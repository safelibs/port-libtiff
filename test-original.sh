#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_TAG="${LIBTIFF_ORIGINAL_TEST_IMAGE:-libtiff-original-test:ubuntu24.04}"
SAFE_DIST_INPUT="${LIBTIFF_SAFE_DIST_DIR:-}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required to run $0" >&2
  exit 1
fi

if [[ ! -d "$ROOT/original" ]]; then
  echo "missing original source tree" >&2
  exit 1
fi

if [[ ! -f "$ROOT/dependents.json" ]]; then
  echo "missing dependents.json" >&2
  exit 1
fi

if [[ -z "$SAFE_DIST_INPUT" ]]; then
  echo "LIBTIFF_SAFE_DIST_DIR must point at the generated safe .deb directory" >&2
  exit 1
fi

if [[ "$SAFE_DIST_INPUT" = /* ]]; then
  SAFE_DIST_HOST_DIR="$SAFE_DIST_INPUT"
else
  SAFE_DIST_HOST_DIR="$ROOT/$SAFE_DIST_INPUT"
fi
SAFE_DIST_HOST_DIR="$(realpath "$SAFE_DIST_HOST_DIR")"

if [[ ! -d "$SAFE_DIST_HOST_DIR" ]]; then
  echo "missing safe dist dir: $SAFE_DIST_HOST_DIR" >&2
  exit 1
fi

docker build -t "$IMAGE_TAG" - <<'DOCKERFILE'
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      cmake \
      file \
      gdal-bin \
      ghostscript \
      gimp \
      graphicsmagick \
      imagemagick \
      libgdk-pixbuf-2.0-0 \
      libgdk-pixbuf2.0-bin \
      libjbig-dev \
      libjpeg-dev \
      liblzma-dev \
      libopencv-dev \
      libtiff-dev \
      libtiff-tools \
      libwebp-dev \
      libzstd-dev \
      netpbm \
      ninja-build \
      pkg-config \
      poppler-utils \
      python3 \
      python3-pil \
      qt6-base-dev \
      qt6-image-formats-plugins \
      sane-airscan \
      sane-utils \
      tesseract-ocr \
      tesseract-ocr-eng \
      zlib1g-dev \
 && rm -rf /var/lib/apt/lists/*
DOCKERFILE

docker run --rm -i \
  -v "$ROOT":/work:ro \
  -v "$SAFE_DIST_HOST_DIR":/dist:ro \
  "$IMAGE_TAG" \
  bash -s <<'CONTAINER_SCRIPT'
set -euo pipefail

export LANG=C.UTF-8
export LC_ALL=C.UTF-8

ROOT=/work
DIST_ROOT=/dist
FIXTURE_DIR=/tmp/libtiff-fixtures
TEST_ROOT=/tmp/libtiff-dependent-tests
MULTIARCH="$(gcc -print-multiarch)"
EXPECTED_SAFE_VERSION="1:4.5.1+git230720-4ubuntu2.5+safelibs1"
ARCHIVE_BASELINE_VERSION="4.5.1+git230720-4ubuntu2.5"
PROBE_TIMEOUT_SEC="${LIBTIFF_DOWNSTREAM_TIMEOUT_SEC:-120}"
VALIDATION_TIMEOUT_SEC="${LIBTIFF_DOWNSTREAM_VALIDATION_TIMEOUT_SEC:-30}"
PROBE_KILL_AFTER_SEC="${LIBTIFF_DOWNSTREAM_KILL_AFTER_SEC:-10}"

log_step() {
  printf '\n==> %s\n' "$1"
}

die() {
  echo "error: $*" >&2
  exit 1
}

require_nonempty_file() {
  local path="$1"

  [[ -s "$path" ]] || die "expected non-empty file: $path"
}

require_contains() {
  local path="$1"
  local needle="$2"

  if ! grep -F -- "$needle" "$path" >/dev/null 2>&1; then
    printf 'missing expected text in %s: %s\n' "$path" "$needle" >&2
    printf -- '--- %s ---\n' "$path" >&2
    cat "$path" >&2
    exit 1
  fi
}

report_probe_failure() {
  local app="$1"
  local command_desc="$2"
  local artifact="$3"
  local behavior="$4"
  local log_path="$5"
  local status="$6"

  printf 'downstream probe failed\n' >&2
  printf 'application: %s\n' "$app" >&2
  printf 'command: %s\n' "$command_desc" >&2
  printf 'artifact: %s\n' "$artifact" >&2
  printf 'libtiff behavior: %s\n' "$behavior" >&2
  printf 'exit status: %s\n' "$status" >&2

  if [[ -f "$log_path" ]]; then
    printf -- '--- %s ---\n' "$log_path" >&2
    cat "$log_path" >&2
  fi

  exit "$status"
}

run_probe() {
  local app="$1"
  local command_desc="$2"
  local artifact="$3"
  local behavior="$4"
  local log_path="$5"
  local timeout_sec="$6"
  shift 6

  mkdir -p "$(dirname "$log_path")"
  timeout --signal=TERM --kill-after="${PROBE_KILL_AFTER_SEC}s" "${timeout_sec}s" "$@" \
    >"$log_path" 2>&1 || {
      local status=$?
      report_probe_failure "$app" "$command_desc" "$artifact" "$behavior" "$log_path" "$status"
    }
}

build_isolated_env() {
  local dir="$1"
  local out_name="$2"
  local -n out_ref="$out_name"

  mkdir -p "$dir/home" "$dir/cache" "$dir/config" "$dir/runtime" "$dir/tmp"
  chmod 700 "$dir/runtime"

  out_ref=(
    env
    HOME="$dir/home"
    XDG_CACHE_HOME="$dir/cache"
    XDG_CONFIG_HOME="$dir/config"
    XDG_RUNTIME_DIR="$dir/runtime"
    TMPDIR="$dir/tmp"
  )
}

find_file_or_die() {
  local search_root="$1"
  local pattern="$2"
  local result

  result="$(find "$search_root" -type f -path "$pattern" -print 2>/dev/null | LC_ALL=C sort | head -n 1 || true)"
  [[ -n "$result" ]] || die "unable to locate file matching $pattern under $search_root"
  printf '%s\n' "$result"
}

reset_test_dir() {
  local name="$1"
  local dir="$TEST_ROOT/$name"

  rm -rf "$dir"
  mkdir -p "$dir"
  printf '%s\n' "$dir"
}

find_deb_by_package() {
  local package="$1"
  local deb

  while IFS= read -r candidate; do
    if [[ "$(dpkg-deb -f "$candidate" Package)" == "$package" ]]; then
      deb="$candidate"
      break
    fi
  done < <(find "$DIST_ROOT" -maxdepth 1 -type f -name '*.deb' | sort)

  [[ -n "${deb:-}" ]] || die "unable to locate ${package}.deb under $DIST_ROOT"
  printf '%s\n' "$deb"
}

assert_installed_path_owned_by_package() {
  local path="$1"
  local package="$2"

  dpkg -S "$path" >/tmp/dpkg-owner.log 2>&1 || {
    cat /tmp/dpkg-owner.log >&2
    exit 1
  }
  require_contains /tmp/dpkg-owner.log "${package}:"
}

assert_uses_packaged_libtiff() {
  local target="$1"
  local label="$2"
  local runtime_path
  local resolved

  ldd "$target" >/tmp/ldd-check.log 2>&1 || {
    cat /tmp/ldd-check.log >&2
    exit 1
  }
  runtime_path="$(awk '$1 == "libtiff.so.6" { print $3; exit }' /tmp/ldd-check.log)"
  [[ -n "$runtime_path" ]] || die "$label does not resolve libtiff.so.6"
  resolved="$(readlink -f "$runtime_path")"

  case "$resolved" in
    /usr/lib/"$MULTIARCH"/*)
      ;;
    *)
      printf '%s resolved libtiff.so.6 to %s instead of /usr/lib/%s\n' \
        "$label" "$resolved" "$MULTIARCH" >&2
      ldd "$target" >&2
      exit 1
      ;;
  esac

  dpkg -S "$resolved" >/tmp/dpkg-owner.log 2>&1 || {
    cat /tmp/dpkg-owner.log >&2
    exit 1
  }
  require_contains /tmp/dpkg-owner.log "libtiff6:"

  require_contains /tmp/ldd-check.log "$runtime_path"
}

require_valid_tiff() {
  local path="$1"

  require_nonempty_file "$path"
  timeout --signal=TERM --kill-after="${PROBE_KILL_AFTER_SEC}s" "${VALIDATION_TIMEOUT_SEC}s" \
    tiffinfo "$path" >/tmp/tiffinfo-check.log 2>&1 || {
    cat /tmp/tiffinfo-check.log >&2
    exit 1
  }
}

validate_dependents_inventory() {
  python3 <<'PY'
import json
from pathlib import Path

expected = [
    "gimp",
    "imagemagick",
    "graphicsmagick",
    "gdal-bin",
    "poppler-utils",
    "qt6-image-formats-plugins",
    "python3-pil",
    "netpbm",
    "tesseract-ocr",
    "ghostscript",
    "libgdk-pixbuf-2.0-0",
    "libopencv-imgcodecs406t64",
    "sane-airscan",
]

data = json.loads(Path("/work/dependents.json").read_text(encoding="utf-8"))
actual = [entry["package"] for entry in data["dependents"]]

if actual != expected:
    raise SystemExit(
        f"unexpected dependents.json contents: expected {expected}, found {actual}"
    )
PY
}

install_safe_packages() {
  local libtiff6_deb
  local libtiffxx6_deb
  local libtiff_dev_deb
  local libtiff_tools_deb
  local package
  local version
  local archive_version

  log_step "Installing safe libtiff packages"

  libtiff6_deb="$(find_deb_by_package libtiff6)"
  libtiffxx6_deb="$(find_deb_by_package libtiffxx6)"
  libtiff_dev_deb="$(find_deb_by_package libtiff-dev)"
  libtiff_tools_deb="$(find_deb_by_package libtiff-tools)"

  for package in \
    "$libtiff6_deb" \
    "$libtiffxx6_deb" \
    "$libtiff_dev_deb" \
    "$libtiff_tools_deb"; do
    version="$(dpkg-deb -f "$package" Version)"
    [[ "$version" == "$EXPECTED_SAFE_VERSION" ]] || \
      die "unexpected package version for $package: $version"
    dpkg --compare-versions "$version" gt "$ARCHIVE_BASELINE_VERSION" || \
      die "version $version does not sort above $ARCHIVE_BASELINE_VERSION"
  done

  for package in libtiff6 libtiffxx6 libtiff-dev libtiff-tools; do
    archive_version="$(dpkg-query -W -f='${Version}' "$package")"
    dpkg --compare-versions "$archive_version" eq "$ARCHIVE_BASELINE_VERSION" || \
      die "unexpected archive version for $package: $archive_version"
  done

  apt-get install -y \
    "$libtiff6_deb" \
    "$libtiffxx6_deb" \
    "$libtiff_dev_deb" \
    "$libtiff_tools_deb" >/tmp/apt-local-debs.log 2>&1 || {
      cat /tmp/apt-local-debs.log >&2
      exit 1
    }

  for package in libtiff6 libtiffxx6 libtiff-dev libtiff-tools; do
    version="$(dpkg-query -W -f='${Version}' "$package")"
    [[ "$version" == "$EXPECTED_SAFE_VERSION" ]] || \
      die "failed to install $package at $EXPECTED_SAFE_VERSION"
  done

  assert_installed_path_owned_by_package "/usr/lib/$MULTIARCH/libtiff.so.6.0.1" "libtiff6"
  assert_installed_path_owned_by_package "/usr/lib/$MULTIARCH/libtiffxx.so.6.0.1" "libtiffxx6"
  assert_installed_path_owned_by_package "$(command -v tiffinfo)" "libtiff-tools"
  assert_installed_path_owned_by_package "$(command -v tiffcp)" "libtiff-tools"
  assert_uses_packaged_libtiff "$(command -v tiffinfo)" "installed tiffinfo"
  assert_uses_packaged_libtiff "$(command -v tiffcp)" "installed tiffcp"
}

prepare_fixtures() {
  local -a fixture_env

  log_step "Preparing fixtures"

  rm -rf "$FIXTURE_DIR" "$TEST_ROOT"
  mkdir -p "$FIXTURE_DIR" "$TEST_ROOT"
  build_isolated_env "$FIXTURE_DIR" fixture_env

  cp "$ROOT/original/test/images/rgb-3c-8b.tiff" "$FIXTURE_DIR/input.tiff"
  require_valid_tiff "$FIXTURE_DIR/input.tiff"

  fixture_env+=(MAGICK_TEMPORARY_PATH="$FIXTURE_DIR/tmp")
  run_probe \
    "fixture-setup" \
    "convert input.tiff input.pdf" \
    "$FIXTURE_DIR/input.tiff -> $FIXTURE_DIR/input.pdf" \
    "prepare a deterministic PDF fixture from the canonical TIFF input" \
    /tmp/fixture-convert.log \
    "$PROBE_TIMEOUT_SEC" \
    "${fixture_env[@]}" \
    convert "$FIXTURE_DIR/input.tiff" "$FIXTURE_DIR/input.pdf"

  require_nonempty_file "$FIXTURE_DIR/input.pdf"
  file "$FIXTURE_DIR/input.pdf" | grep -F 'PDF document' >/dev/null
}

test_gimp() {
  local plugin dir
  local -a app_env

  log_step "gimp"
  plugin="$(find_file_or_die /usr/lib '*/gimp/*/plug-ins/file-tiff/file-tiff')"
  assert_uses_packaged_libtiff "$plugin" "gimp TIFF plug-in"

  dir="$(reset_test_dir gimp)"
  build_isolated_env "$dir" app_env
  app_env+=(GIMP2_DIRECTORY="$dir/gimp")
  cp "$FIXTURE_DIR/input.tiff" "$dir/input.tiff"

  run_probe \
    "gimp" \
    "gimp-console-2.10 scripted TIFF load/save" \
    "$dir/input.tiff -> $dir/output.tiff" \
    "load and save through the GIMP TIFF plug-in linked against the packaged libtiff" \
    /tmp/gimp.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    bash -lc 'cd "$1" && gimp-console-2.10 -i -d -f \
      -b "(let* ((image (car (gimp-file-load RUN-NONINTERACTIVE \"$2\" \"$2\"))) (drawable (car (gimp-image-get-active-layer image)))) (gimp-file-save RUN-NONINTERACTIVE image drawable \"$3\" \"$3\") (gimp-image-delete image))" \
      -b "(gimp-quit 0)"' \
    bash "$dir" "$dir/input.tiff" "$dir/output.tiff"

  require_valid_tiff "$dir/output.tiff"
}

test_imagemagick() {
  local coder dir
  local -a app_env

  log_step "imagemagick"
  coder="$(find_file_or_die /usr/lib '*/ImageMagick-*/modules-*/coders/tiff.so')"
  assert_uses_packaged_libtiff "$coder" "ImageMagick TIFF coder"

  dir="$(reset_test_dir imagemagick)"
  build_isolated_env "$dir" app_env
  app_env+=(MAGICK_TEMPORARY_PATH="$dir/tmp")
  run_probe \
    "imagemagick" \
    "convert input.tiff -rotate 90 output.tiff" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.tiff" \
    "load the TIFF coder and write a rotated TIFF using the packaged libtiff" \
    /tmp/imagemagick.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    convert "$FIXTURE_DIR/input.tiff" -rotate 90 "$dir/output.tiff"

  require_valid_tiff "$dir/output.tiff"
  run_probe \
    "imagemagick" \
    "identify output.tiff" \
    "$dir/output.tiff" \
    "reopen the generated TIFF through ImageMagick metadata inspection" \
    /tmp/imagemagick-identify.log \
    "$VALIDATION_TIMEOUT_SEC" \
    "${app_env[@]}" \
    identify "$dir/output.tiff"
  require_contains /tmp/imagemagick-identify.log "TIFF"
}

test_graphicsmagick() {
  local lib dir
  local -a app_env

  log_step "graphicsmagick"
  lib="$(ldconfig -p | awk '/libGraphicsMagick.*\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate GraphicsMagick shared library"
  assert_uses_packaged_libtiff "$lib" "GraphicsMagick shared library"

  dir="$(reset_test_dir graphicsmagick)"
  build_isolated_env "$dir" app_env
  run_probe \
    "graphicsmagick" \
    "gm convert input.tiff -flip output.tiff" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.tiff" \
    "load and write TIFFs through the GraphicsMagick shared library bound to the packaged libtiff" \
    /tmp/graphicsmagick.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    gm convert "$FIXTURE_DIR/input.tiff" -flip "$dir/output.tiff"

  require_valid_tiff "$dir/output.tiff"
  run_probe \
    "graphicsmagick" \
    "gm identify output.tiff" \
    "$dir/output.tiff" \
    "reopen the generated TIFF through GraphicsMagick metadata inspection" \
    /tmp/graphicsmagick-identify.log \
    "$VALIDATION_TIMEOUT_SEC" \
    "${app_env[@]}" \
    gm identify "$dir/output.tiff"
  require_contains /tmp/graphicsmagick-identify.log "TIFF"
}

test_gdal() {
  local lib dir
  local -a app_env

  log_step "gdal-bin"
  lib="$(ldconfig -p | awk '/libgdal\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libgdal shared library"
  assert_uses_packaged_libtiff "$lib" "libgdal shared library"

  dir="$(reset_test_dir gdal-bin)"
  build_isolated_env "$dir" app_env
  run_probe \
    "gdal-bin" \
    "gdal_translate -of GTiff input.tiff output.tiff" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.tiff" \
    "round-trip a TIFF through the GTiff driver linked against the packaged libtiff" \
    /tmp/gdal-translate.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    gdal_translate -of GTiff "$FIXTURE_DIR/input.tiff" "$dir/output.tiff"
  require_valid_tiff "$dir/output.tiff"

  run_probe \
    "gdal-bin" \
    "gdalinfo output.tiff" \
    "$dir/output.tiff" \
    "read the generated TIFF back through GDAL metadata inspection" \
    /tmp/gdalinfo.log \
    "$VALIDATION_TIMEOUT_SEC" \
    "${app_env[@]}" \
    gdalinfo "$dir/output.tiff"
  require_contains /tmp/gdalinfo.log "Driver: GTiff/GeoTIFF"
}

test_poppler() {
  local lib dir
  local -a app_env

  log_step "poppler-utils"
  lib="$(ldconfig -p | awk '/libpoppler\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libpoppler shared library"
  assert_uses_packaged_libtiff "$lib" "libpoppler shared library"

  dir="$(reset_test_dir poppler-utils)"
  build_isolated_env "$dir" app_env
  run_probe \
    "poppler-utils" \
    "pdftocairo -tiff -singlefile input.pdf poppler" \
    "$FIXTURE_DIR/input.pdf -> $dir/poppler.tif" \
    "render PDF content to TIFF through Poppler's TIFF backend using the packaged libtiff" \
    /tmp/pdftocairo.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    pdftocairo -tiff -singlefile "$FIXTURE_DIR/input.pdf" "$dir/poppler"

  require_valid_tiff "$dir/poppler.tif"
}

test_qt6_image_formats() {
  local plugin dir
  local -a app_env

  log_step "qt6-image-formats-plugins"
  plugin="$(find_file_or_die /usr/lib '*/qt6/plugins/imageformats/libqtiff.so')"
  assert_uses_packaged_libtiff "$plugin" "Qt TIFF image plug-in"

  dir="$(reset_test_dir qt6-image-formats-plugins)"
  cat > "$dir/qt_tiff_probe.cpp" <<'CPP'
#include <QCoreApplication>
#include <QImage>
#include <QImageReader>
#include <QImageWriter>
#include <QTextStream>

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    if (argc != 3) {
        return 2;
    }

    bool hasTiff = false;
    for (const QByteArray &format : QImageReader::supportedImageFormats()) {
        if (format == "tif" || format == "tiff") {
            hasTiff = true;
            break;
        }
    }
    if (!hasTiff) {
        QTextStream(stderr) << "missing TIFF support\n";
        return 1;
    }

    QImage image(argv[1]);
    if (image.isNull()) {
        QTextStream(stderr) << "failed to load input TIFF\n";
        return 1;
    }

    image = image.mirrored(true, false);
    QImageWriter writer(argv[2], "tiff");
    if (!writer.write(image)) {
        QTextStream(stderr) << writer.errorString() << '\n';
        return 1;
    }

    QTextStream(stdout) << image.width() << "x" << image.height() << '\n';
    return 0;
}
CPP

  build_isolated_env "$dir" app_env
  app_env+=(QT_QPA_PLATFORM=offscreen)
  run_probe \
    "qt6-image-formats-plugins" \
    "build qt_tiff_probe.cpp" \
    "$dir/qt_tiff_probe.cpp -> $dir/qt_tiff_probe" \
    "compile a Qt imageformats probe against the system Qt stack" \
    /tmp/qt-build.log \
    "$PROBE_TIMEOUT_SEC" \
    bash -lc 'g++ -std=c++17 "$1" -o "$2" $(pkg-config --cflags --libs Qt6Gui)' \
    bash "$dir/qt_tiff_probe.cpp" "$dir/qt_tiff_probe"

  run_probe \
    "qt6-image-formats-plugins" \
    "qt_tiff_probe input.tiff output.tiff" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.tiff" \
    "load the Qt TIFF plug-in and save a mirrored TIFF through QImageWriter" \
    /tmp/qt-run.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    "$dir/qt_tiff_probe" "$FIXTURE_DIR/input.tiff" "$dir/output.tiff"

  require_valid_tiff "$dir/output.tiff"
}

test_python_pil() {
  local imaging_so dir
  local -a app_env

  log_step "python3-pil"
  imaging_so="$(python3 - <<'PY'
from PIL import _imaging
print(_imaging.__file__)
PY
)"
  [[ -n "$imaging_so" ]] || die "unable to locate Pillow _imaging extension"
  assert_uses_packaged_libtiff "$imaging_so" "Pillow _imaging extension"

  dir="$(reset_test_dir python3-pil)"
  build_isolated_env "$dir" app_env
  cat > "$dir/pillow_probe.py" <<PY
from PIL import Image

src = "$FIXTURE_DIR/input.tiff"
dst = "$dir/output.tiff"

image = Image.open(src)
print(image.format, image.size)
image.transpose(Image.Transpose.FLIP_LEFT_RIGHT).save(dst, format="TIFF")
print(Image.open(dst).size)
PY
  run_probe \
    "python3-pil" \
    "python3 pillow_probe.py" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.tiff" \
    "load and save TIFFs through Pillow's _imaging extension linked against the packaged libtiff" \
    "$dir/pillow.log" \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    python3 "$dir/pillow_probe.py"

  require_contains "$dir/pillow.log" "TIFF"
  require_valid_tiff "$dir/output.tiff"
}

test_netpbm() {
  local dir
  local -a app_env

  log_step "netpbm"
  assert_uses_packaged_libtiff "$(command -v tifftopnm)" "tifftopnm"

  dir="$(reset_test_dir netpbm)"
  build_isolated_env "$dir" app_env
  run_probe \
    "netpbm" \
    "tifftopnm input.tiff > output.ppm" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.ppm" \
    "decode TIFF input through tifftopnm backed by the packaged libtiff" \
    "$dir/tifftopnm.log" \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    bash -lc 'tifftopnm "$1" > "$2"' \
    bash "$FIXTURE_DIR/input.tiff" "$dir/output.ppm"
  require_nonempty_file "$dir/output.ppm"

  run_probe \
    "netpbm" \
    "pnmtotiff output.ppm > output.tiff" \
    "$dir/output.ppm -> $dir/output.tiff" \
    "re-encode the intermediate PPM into TIFF for a downstream round-trip" \
    "$dir/pnmtotiff.log" \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    bash -lc 'pnmtotiff "$1" > "$2"' \
    bash "$dir/output.ppm" "$dir/output.tiff"
  require_valid_tiff "$dir/output.tiff"
}

test_tesseract() {
  local lib dir digits
  local -a app_env

  log_step "tesseract-ocr"
  lib="$(ldconfig -p | awk '/liblept\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate liblept shared library"
  assert_uses_packaged_libtiff "$lib" "Leptonica shared library"

  dir="$(reset_test_dir tesseract-ocr)"
  build_isolated_env "$dir" app_env
  run_probe \
    "tesseract-ocr" \
    "pbmtext 12345 | pnmscale 10 > ocr-input.pbm" \
    "$dir/ocr-input.pbm" \
    "generate a deterministic PBM input for OCR smoke coverage" \
    "$dir/pbmtext.log" \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    bash -lc 'pbmtext "12345" | pnmscale 10 > "$1"' \
    bash "$dir/ocr-input.pbm"
  run_probe \
    "tesseract-ocr" \
    "pnmtotiff ocr-input.pbm > ocr-input.tiff" \
    "$dir/ocr-input.pbm -> $dir/ocr-input.tiff" \
    "encode the OCR fixture as TIFF before handing it to Tesseract" \
    "$dir/pnmtotiff.log" \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    bash -lc 'pnmtotiff "$1" > "$2"' \
    bash "$dir/ocr-input.pbm" "$dir/ocr-input.tiff"
  require_valid_tiff "$dir/ocr-input.tiff"

  run_probe \
    "tesseract-ocr" \
    "tesseract ocr-input.tiff stdout --dpi 300 --psm 7 -l eng" \
    "$dir/ocr-input.tiff" \
    "read TIFF input through Leptonica and recover the expected digits" \
    "$dir/tesseract.log" \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    tesseract "$dir/ocr-input.tiff" stdout --dpi 300 --psm 7 -l eng \
      -c tessedit_char_whitelist=12345

  digits="$(tr -cd '0-9' < "$dir/tesseract.log")"
  [[ "$digits" == *"12345"* ]] || {
    cat "$dir/tesseract.log" >&2
    die "tesseract did not recover expected digits from TIFF input"
  }
}

test_ghostscript() {
  local lib dir
  local -a app_env

  log_step "ghostscript"
  lib="$(ldconfig -p | awk '/libgs\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libgs shared library"
  assert_uses_packaged_libtiff "$lib" "Ghostscript shared library"

  dir="$(reset_test_dir ghostscript)"
  build_isolated_env "$dir" app_env
  run_probe \
    "ghostscript" \
    "gs -sDEVICE=tiff24nc -sOutputFile=output.tiff input.pdf" \
    "$FIXTURE_DIR/input.pdf -> $dir/output.tiff" \
    "render PDF content to TIFF through Ghostscript linked against the packaged libtiff" \
    /tmp/ghostscript.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    gs -q -dNOPAUSE -dBATCH -sDEVICE=tiff24nc \
      -sOutputFile="$dir/output.tiff" \
      "$FIXTURE_DIR/input.pdf"

  require_valid_tiff "$dir/output.tiff"
}

test_gdk_pixbuf() {
  local loader query_loaders thumbnailer dir
  local -a app_env

  log_step "libgdk-pixbuf-2.0-0"
  loader="$(find_file_or_die /usr/lib '*/gdk-pixbuf-2.0/*/loaders/libpixbufloader-tiff.so')"
  assert_uses_packaged_libtiff "$loader" "GDK Pixbuf TIFF loader"

  query_loaders="$(find_file_or_die /usr '*/gdk-pixbuf-query-loaders')"
  dir="$(reset_test_dir libgdk-pixbuf-2.0-0)"
  build_isolated_env "$dir" app_env
  run_probe \
    "libgdk-pixbuf-2.0-0" \
    "gdk-pixbuf-query-loaders > loaders.cache" \
    "$dir/loaders.cache" \
    "build a deterministic loader cache that includes the TIFF loader linked against the packaged libtiff" \
    /tmp/gdk-pixbuf-loaders.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    bash -lc '"$1" > "$2"' \
    bash "$query_loaders" "$dir/loaders.cache"
  require_contains "$dir/loaders.cache" "libpixbufloader-tiff"

  thumbnailer="$(find_file_or_die /usr '*/gdk-pixbuf-thumbnailer')"
  app_env+=(GDK_PIXBUF_MODULE_FILE="$dir/loaders.cache")
  run_probe \
    "libgdk-pixbuf-2.0-0" \
    "gdk-pixbuf-thumbnailer -s 64 input.tiff thumbnail.png" \
    "$FIXTURE_DIR/input.tiff -> $dir/thumbnail.png" \
    "load TIFF input through the GDK Pixbuf TIFF loader and render a PNG thumbnail" \
    /tmp/gdk-thumb.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    "$thumbnailer" -s 64 "$FIXTURE_DIR/input.tiff" "$dir/thumbnail.png"

  require_nonempty_file "$dir/thumbnail.png"
  file "$dir/thumbnail.png" | grep -F 'PNG image data' >/dev/null
}

test_opencv() {
  local lib dir
  local -a app_env

  log_step "libopencv-imgcodecs406t64"
  lib="$(ldconfig -p | awk '/libopencv_imgcodecs\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libopencv_imgcodecs shared library"
  assert_uses_packaged_libtiff "$lib" "OpenCV imgcodecs shared library"

  dir="$(reset_test_dir libopencv-imgcodecs406t64)"
  cat > "$dir/opencv_tiff_probe.cpp" <<'CPP'
#include <opencv2/imgcodecs.hpp>
#include <opencv2/imgproc.hpp>
#include <iostream>

int main(int argc, char **argv)
{
    if (argc != 3) {
        return 2;
    }

    cv::Mat image = cv::imread(argv[1], cv::IMREAD_UNCHANGED);
    if (image.empty()) {
        std::cerr << "failed to read TIFF input\n";
        return 1;
    }

    cv::Mat rotated;
    cv::rotate(image, rotated, cv::ROTATE_90_CLOCKWISE);
    if (!cv::imwrite(argv[2], rotated)) {
        std::cerr << "failed to write TIFF output\n";
        return 1;
    }

    std::cout << rotated.cols << "x" << rotated.rows << '\n';
    return 0;
}
CPP

  build_isolated_env "$dir" app_env
  run_probe \
    "libopencv-imgcodecs406t64" \
    "build opencv_tiff_probe.cpp" \
    "$dir/opencv_tiff_probe.cpp -> $dir/opencv_tiff_probe" \
    "compile an OpenCV imgcodecs probe against the system OpenCV stack" \
    /tmp/opencv-build.log \
    "$PROBE_TIMEOUT_SEC" \
    bash -lc 'g++ -std=c++17 "$1" -o "$2" $(pkg-config --cflags --libs opencv4)' \
    bash "$dir/opencv_tiff_probe.cpp" "$dir/opencv_tiff_probe"

  run_probe \
    "libopencv-imgcodecs406t64" \
    "opencv_tiff_probe input.tiff output.tiff" \
    "$FIXTURE_DIR/input.tiff -> $dir/output.tiff" \
    "load and write TIFFs through OpenCV imgcodecs backed by the packaged libtiff" \
    /tmp/opencv-run.log \
    "$PROBE_TIMEOUT_SEC" \
    "${app_env[@]}" \
    "$dir/opencv_tiff_probe" "$FIXTURE_DIR/input.tiff" "$dir/output.tiff"

  require_valid_tiff "$dir/output.tiff"
}

test_sane_airscan() {
  local backend dir
  local -a app_env

  log_step "sane-airscan"
  backend="$(find_file_or_die /usr/lib '*/sane/libsane-airscan.so*')"
  assert_uses_packaged_libtiff "$backend" "sane-airscan backend"

  dir="$(reset_test_dir sane-airscan)"
  mkdir -p "$dir/sane.d"
  printf 'airscan\n' > "$dir/sane.d/dll.conf"
  cat > "$dir/sane.d/airscan.conf" <<'EOF'
[devices]

[options]
discovery = disable
EOF

  # There is no physical network scanner in CI, so the backend smoke test is
  # limited to loading the airscan backend through SANE with discovery disabled.
  build_isolated_env "$dir" app_env
  app_env+=(SANE_CONFIG_DIR="$dir/sane.d" SANE_DEBUG_DLL=255)
  run_probe \
    "sane-airscan" \
    "scanimage -L" \
    "$dir/sane.d/airscan.conf" \
    "load the sane-airscan backend and enumerate devices with discovery disabled" \
    "$dir/scanimage.log" \
    "$VALIDATION_TIMEOUT_SEC" \
    "${app_env[@]}" \
    scanimage -L

  require_contains "$dir/scanimage.log" "libsane-airscan"
}

main() {
  validate_dependents_inventory
  install_safe_packages
  prepare_fixtures

  test_gimp
  test_imagemagick
  test_graphicsmagick
  test_gdal
  test_poppler
  test_qt6_image_formats
  test_python_pil
  test_netpbm
  test_tesseract
  test_ghostscript
  test_gdk_pixbuf
  test_opencv
  test_sane_airscan

  log_step "All downstream smoke tests passed"
}

main "$@"
CONTAINER_SCRIPT
