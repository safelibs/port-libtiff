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

find_file_or_die() {
  local search_root="$1"
  local pattern="$2"
  local result

  result="$(find "$search_root" -type f -path "$pattern" -print -quit 2>/dev/null || true)"
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

  assert_uses_packaged_libtiff "$(command -v tiffinfo)" "installed tiffinfo"
}

prepare_fixtures() {
  log_step "Preparing fixtures"

  rm -rf "$FIXTURE_DIR" "$TEST_ROOT"
  mkdir -p "$FIXTURE_DIR" "$TEST_ROOT"

  cp "$ROOT/original/test/images/rgb-3c-8b.tiff" "$FIXTURE_DIR/input.tiff"
  require_valid_tiff "$FIXTURE_DIR/input.tiff"

  convert "$FIXTURE_DIR/input.tiff" "$FIXTURE_DIR/input.pdf" >/tmp/fixture-convert.log 2>&1 || {
    cat /tmp/fixture-convert.log >&2
    exit 1
  }

  require_nonempty_file "$FIXTURE_DIR/input.pdf"
  file "$FIXTURE_DIR/input.pdf" | grep -F 'PDF document' >/dev/null
}

test_gimp() {
  local plugin dir

  log_step "gimp"
  plugin="$(find_file_or_die /usr/lib '*/gimp/*/plug-ins/file-tiff/file-tiff')"
  assert_uses_packaged_libtiff "$plugin" "gimp TIFF plug-in"

  dir="$(reset_test_dir gimp)"
  cp "$FIXTURE_DIR/input.tiff" "$dir/input.tiff"

  (
    cd "$dir"
    timeout 120 gimp-console-2.10 -i -d -f \
      -b "(let* ((image (car (gimp-file-load RUN-NONINTERACTIVE \"$(pwd)/input.tiff\" \"$(pwd)/input.tiff\"))) (drawable (car (gimp-image-get-active-layer image)))) (gimp-file-save RUN-NONINTERACTIVE image drawable \"$(pwd)/output.tiff\" \"$(pwd)/output.tiff\") (gimp-image-delete image))" \
      -b "(gimp-quit 0)" \
      >/tmp/gimp.log 2>&1
  ) || {
    cat /tmp/gimp.log >&2
    exit 1
  }

  require_valid_tiff "$dir/output.tiff"
}

test_imagemagick() {
  local coder dir

  log_step "imagemagick"
  coder="$(find_file_or_die /usr/lib '*/ImageMagick-*/modules-*/coders/tiff.so')"
  assert_uses_packaged_libtiff "$coder" "ImageMagick TIFF coder"

  dir="$(reset_test_dir imagemagick)"
  convert "$FIXTURE_DIR/input.tiff" -rotate 90 "$dir/output.tiff" >/tmp/imagemagick.log 2>&1 || {
    cat /tmp/imagemagick.log >&2
    exit 1
  }

  require_valid_tiff "$dir/output.tiff"
  identify "$dir/output.tiff" >/tmp/imagemagick-identify.log 2>&1 || {
    cat /tmp/imagemagick-identify.log >&2
    exit 1
  }
  require_contains /tmp/imagemagick-identify.log "TIFF"
}

test_graphicsmagick() {
  local lib dir

  log_step "graphicsmagick"
  lib="$(ldconfig -p | awk '/libGraphicsMagick.*\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate GraphicsMagick shared library"
  assert_uses_packaged_libtiff "$lib" "GraphicsMagick shared library"

  dir="$(reset_test_dir graphicsmagick)"
  gm convert "$FIXTURE_DIR/input.tiff" -flip "$dir/output.tiff" >/tmp/graphicsmagick.log 2>&1 || {
    cat /tmp/graphicsmagick.log >&2
    exit 1
  }

  require_valid_tiff "$dir/output.tiff"
  gm identify "$dir/output.tiff" >/tmp/graphicsmagick-identify.log 2>&1 || {
    cat /tmp/graphicsmagick-identify.log >&2
    exit 1
  }
  require_contains /tmp/graphicsmagick-identify.log "TIFF"
}

test_gdal() {
  local lib dir

  log_step "gdal-bin"
  lib="$(ldconfig -p | awk '/libgdal\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libgdal shared library"
  assert_uses_packaged_libtiff "$lib" "libgdal shared library"

  dir="$(reset_test_dir gdal-bin)"
  gdal_translate -of GTiff "$FIXTURE_DIR/input.tiff" "$dir/output.tiff" >/tmp/gdal-translate.log 2>&1 || {
    cat /tmp/gdal-translate.log >&2
    exit 1
  }
  require_valid_tiff "$dir/output.tiff"

  gdalinfo "$dir/output.tiff" >/tmp/gdalinfo.log 2>&1 || {
    cat /tmp/gdalinfo.log >&2
    exit 1
  }
  require_contains /tmp/gdalinfo.log "Driver: GTiff/GeoTIFF"
}

test_poppler() {
  local lib dir

  log_step "poppler-utils"
  lib="$(ldconfig -p | awk '/libpoppler\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libpoppler shared library"
  assert_uses_packaged_libtiff "$lib" "libpoppler shared library"

  dir="$(reset_test_dir poppler-utils)"
  pdftocairo -tiff -singlefile "$FIXTURE_DIR/input.pdf" "$dir/poppler" >/tmp/pdftocairo.log 2>&1 || {
    cat /tmp/pdftocairo.log >&2
    exit 1
  }

  require_valid_tiff "$dir/poppler.tif"
}

test_qt6_image_formats() {
  local plugin dir

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

  g++ -std=c++17 "$dir/qt_tiff_probe.cpp" -o "$dir/qt_tiff_probe" \
    $(pkg-config --cflags --libs Qt6Gui) >/tmp/qt-build.log 2>&1 || {
      cat /tmp/qt-build.log >&2
      exit 1
    }

  "$dir/qt_tiff_probe" "$FIXTURE_DIR/input.tiff" "$dir/output.tiff" >/tmp/qt-run.log 2>&1 || {
    cat /tmp/qt-run.log >&2
    exit 1
  }

  require_valid_tiff "$dir/output.tiff"
}

test_python_pil() {
  local imaging_so dir

  log_step "python3-pil"
  imaging_so="$(python3 - <<'PY'
from PIL import _imaging
print(_imaging.__file__)
PY
)"
  [[ -n "$imaging_so" ]] || die "unable to locate Pillow _imaging extension"
  assert_uses_packaged_libtiff "$imaging_so" "Pillow _imaging extension"

  dir="$(reset_test_dir python3-pil)"
  python3 <<PY >"$dir/pillow.log"
from PIL import Image

src = "$FIXTURE_DIR/input.tiff"
dst = "$dir/output.tiff"

image = Image.open(src)
print(image.format, image.size)
image.transpose(Image.Transpose.FLIP_LEFT_RIGHT).save(dst, format="TIFF")
print(Image.open(dst).size)
PY

  require_contains "$dir/pillow.log" "TIFF"
  require_valid_tiff "$dir/output.tiff"
}

test_netpbm() {
  local dir

  log_step "netpbm"
  assert_uses_packaged_libtiff "$(command -v tifftopnm)" "tifftopnm"

  dir="$(reset_test_dir netpbm)"
  tifftopnm "$FIXTURE_DIR/input.tiff" > "$dir/output.ppm" 2>"$dir/tifftopnm.log" || {
    cat "$dir/tifftopnm.log" >&2
    exit 1
  }
  require_nonempty_file "$dir/output.ppm"

  pnmtotiff "$dir/output.ppm" > "$dir/output.tiff" 2>"$dir/pnmtotiff.log" || {
    cat "$dir/pnmtotiff.log" >&2
    exit 1
  }
  require_valid_tiff "$dir/output.tiff"
}

test_tesseract() {
  local lib dir digits

  log_step "tesseract-ocr"
  lib="$(ldconfig -p | awk '/liblept\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate liblept shared library"
  assert_uses_packaged_libtiff "$lib" "Leptonica shared library"

  dir="$(reset_test_dir tesseract-ocr)"
  pbmtext '12345' | pnmscale 10 > "$dir/ocr-input.pbm"
  pnmtotiff "$dir/ocr-input.pbm" > "$dir/ocr-input.tiff"
  require_valid_tiff "$dir/ocr-input.tiff"

  tesseract "$dir/ocr-input.tiff" stdout --dpi 300 --psm 7 -l eng \
    -c tessedit_char_whitelist=12345 >"$dir/tesseract.log" 2>&1 || {
      cat "$dir/tesseract.log" >&2
      exit 1
    }

  digits="$(tr -cd '0-9' < "$dir/tesseract.log")"
  [[ "$digits" == *"12345"* ]] || {
    cat "$dir/tesseract.log" >&2
    die "tesseract did not recover expected digits from TIFF input"
  }
}

test_ghostscript() {
  local lib dir

  log_step "ghostscript"
  lib="$(ldconfig -p | awk '/libgs\.so/ { print $NF; exit }')"
  [[ -n "$lib" ]] || die "unable to locate libgs shared library"
  assert_uses_packaged_libtiff "$lib" "Ghostscript shared library"

  dir="$(reset_test_dir ghostscript)"
  gs -q -dNOPAUSE -dBATCH -sDEVICE=tiff24nc \
    -sOutputFile="$dir/output.tiff" \
    "$FIXTURE_DIR/input.pdf" >/tmp/ghostscript.log 2>&1 || {
      cat /tmp/ghostscript.log >&2
      exit 1
    }

  require_valid_tiff "$dir/output.tiff"
}

test_gdk_pixbuf() {
  local loader query_loaders thumbnailer dir

  log_step "libgdk-pixbuf-2.0-0"
  loader="$(find_file_or_die /usr/lib '*/gdk-pixbuf-2.0/*/loaders/libpixbufloader-tiff.so')"
  assert_uses_packaged_libtiff "$loader" "GDK Pixbuf TIFF loader"

  query_loaders="$(find_file_or_die /usr '*/gdk-pixbuf-query-loaders')"
  "$query_loaders" >/tmp/gdk-pixbuf-loaders.log 2>&1 || {
    cat /tmp/gdk-pixbuf-loaders.log >&2
    exit 1
  }
  require_contains /tmp/gdk-pixbuf-loaders.log "libpixbufloader-tiff"

  dir="$(reset_test_dir libgdk-pixbuf-2.0-0)"
  thumbnailer="$(find_file_or_die /usr '*/gdk-pixbuf-thumbnailer')"
  "$thumbnailer" -s 64 "$FIXTURE_DIR/input.tiff" "$dir/thumbnail.png" >/tmp/gdk-thumb.log 2>&1 || {
    cat /tmp/gdk-thumb.log >&2
    exit 1
  }

  require_nonempty_file "$dir/thumbnail.png"
  file "$dir/thumbnail.png" | grep -F 'PNG image data' >/dev/null
}

test_opencv() {
  local lib dir

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

  g++ -std=c++17 "$dir/opencv_tiff_probe.cpp" -o "$dir/opencv_tiff_probe" \
    $(pkg-config --cflags --libs opencv4) >/tmp/opencv-build.log 2>&1 || {
      cat /tmp/opencv-build.log >&2
      exit 1
    }

  "$dir/opencv_tiff_probe" "$FIXTURE_DIR/input.tiff" "$dir/output.tiff" >/tmp/opencv-run.log 2>&1 || {
    cat /tmp/opencv-run.log >&2
    exit 1
  }

  require_valid_tiff "$dir/output.tiff"
}

test_sane_airscan() {
  local backend dir

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
  SANE_CONFIG_DIR="$dir/sane.d" SANE_DEBUG_DLL=255 timeout 30 scanimage -L >"$dir/scanimage.log" 2>&1 || {
    cat "$dir/scanimage.log" >&2
    exit 1
  }

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
