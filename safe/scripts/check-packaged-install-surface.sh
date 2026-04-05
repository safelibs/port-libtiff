#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SAFE_ROOT="$ROOT/safe"
DIST_DIR="${LIBTIFF_SAFE_DIST_DIR:-$SAFE_ROOT/dist}"
STAGE_PREFIX=""
declare -A DEB_PATHS=()
declare -a CMAKE_PROJECT_OVERRIDES=()
CMAKE_PROJECT="$ROOT/original/build/test_cmake"
CMAKE_PROJECT_NO_TARGET="$ROOT/original/build/test_cmake_no_target"
PKGCONFIG_SOURCE="$ROOT/original/build/test_cmake/test.c"
CXX_SMOKE="$SAFE_ROOT/test/install/tiffxx_staged_smoke.cpp"
INPUT_TIFF="$ROOT/original/test/images/rgb-3c-8b.tiff"
MULTIARCH="$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || gcc -print-multiarch)"
EXPECTED_VERSION="1:4.5.1+git230720-4ubuntu2.5+safelibs1"
BASELINE_VERSION="4.5.1+git230720-4ubuntu2.5"

die() {
  echo "error: $*" >&2
  exit 1
}

assert_exists() {
  [[ -e "$1" || -L "$1" ]] || die "expected path to exist: $1"
}

assert_absent() {
  [[ ! -e "$1" && ! -L "$1" ]] || die "expected path to be absent: $1"
}

assert_nonempty_file() {
  [[ -s "$1" ]] || die "expected non-empty file: $1"
}

resolve_deb() {
  local dir="$1"
  local package="$2"
  local path

  while IFS= read -r candidate; do
    if [[ "$(dpkg-deb -f "$candidate" Package)" == "$package" ]]; then
      path="$candidate"
      break
    fi
  done < <(find "$dir" -maxdepth 1 -type f -name '*.deb' | sort)

  [[ -n "${path:-}" ]] || die "unable to locate $package .deb under $dir"
  printf '%s\n' "$path"
}

record_deb_path() {
  local package="$1"
  local deb_path="$2"

  [[ -f "$deb_path" ]] || die "missing $package .deb: $deb_path"
  [[ "$(dpkg-deb -f "$deb_path" Package)" == "$package" ]] || \
    die "$deb_path is not a $package package"
  DEB_PATHS["$package"]="$(realpath "$deb_path")"
}

detect_libdir() {
  local prefix_root="$1"
  if [[ -d "$prefix_root/lib/$MULTIARCH" ]]; then
    printf '%s\n' "$prefix_root/lib/$MULTIARCH"
  else
    printf '%s\n' "$prefix_root/lib"
  fi
}

detect_includedir() {
  local prefix_root="$1"
  if [[ -d "$prefix_root/include/$MULTIARCH" ]]; then
    printf '%s\n' "$prefix_root/include/$MULTIARCH"
  else
    printf '%s\n' "$prefix_root/include"
  fi
}

run_prefix_smokes() {
  local prefix_root="$1"
  local label="$2"
  local tmp_root
  local libdir
  local includedir
  local pc_file
  local sysroot=""

  tmp_root="$(mktemp -d)"
  trap 'rm -rf "$tmp_root"' RETURN

  libdir="$(detect_libdir "$prefix_root")"
  includedir="$(detect_includedir "$prefix_root")"
  pc_file="$libdir/pkgconfig/libtiff-4.pc"

  assert_exists "$libdir/libtiff.so.6.0.1"
  assert_exists "$libdir/libtiffxx.so.6.0.1"
  assert_exists "$pc_file"
  assert_exists "$includedir/tiffio.h"
  assert_exists "$includedir/tiffio.hxx"
  assert_exists "$CMAKE_PROJECT/CMakeLists.txt"
  assert_exists "$CMAKE_PROJECT_NO_TARGET/CMakeLists.txt"
  assert_exists "$PKGCONFIG_SOURCE"
  assert_exists "$CXX_SMOKE"
  if [[ -n "$INPUT_TIFF" ]]; then
    assert_exists "$INPUT_TIFF"
  fi

  if grep -qx 'prefix=/usr' "$pc_file"; then
    sysroot="$(dirname "$prefix_root")"
  fi

  cmake -S "$CMAKE_PROJECT" \
    -B "$tmp_root/test_cmake" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_PREFIX_PATH="$prefix_root" \
    >/dev/null
  cmake --build "$tmp_root/test_cmake" --parallel >/dev/null
  LD_LIBRARY_PATH="$libdir" "$tmp_root/test_cmake/test" >/dev/null

  cmake -S "$CMAKE_PROJECT_NO_TARGET" \
    -B "$tmp_root/test_cmake_no_target" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_PREFIX_PATH="$prefix_root" \
    >/dev/null
  cmake --build "$tmp_root/test_cmake_no_target" --parallel >/dev/null
  LD_LIBRARY_PATH="$libdir" "$tmp_root/test_cmake_no_target/test" >/dev/null

  PKG_CONFIG_PATH="$libdir/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" \
  PKG_CONFIG_SYSROOT_DIR="$sysroot" \
  cc "$PKGCONFIG_SOURCE" \
    -o "$tmp_root/pkg_config_test" \
    $(PKG_CONFIG_PATH="$libdir/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" PKG_CONFIG_SYSROOT_DIR="$sysroot" pkg-config --cflags --libs libtiff-4)
  LD_LIBRARY_PATH="$libdir" "$tmp_root/pkg_config_test" >/dev/null

  c++ -std=c++17 \
    "$CXX_SMOKE" \
    -I"$includedir" \
    -L"$libdir" \
    -Wl,-rpath,"$libdir" \
    -ltiffxx \
    -ltiff \
    -o "$tmp_root/tiffxx_staged_smoke"
  LD_LIBRARY_PATH="$libdir" "$tmp_root/tiffxx_staged_smoke" >/dev/null

  if [[ -n "$INPUT_TIFF" && -x "$prefix_root/bin/tiffinfo" && -x "$prefix_root/bin/tiffcp" ]]; then
    LD_LIBRARY_PATH="$libdir" "$prefix_root/bin/tiffinfo" "$INPUT_TIFF" \
      >"$tmp_root/tiffinfo.stdout"
    grep -F 'Image Width' "$tmp_root/tiffinfo.stdout" >/dev/null || \
      die "packaged tiffinfo did not inspect $INPUT_TIFF successfully under $label"

    LD_LIBRARY_PATH="$libdir" "$prefix_root/bin/tiffcp" "$INPUT_TIFF" "$tmp_root/tool-smoke.tiff" \
      >/dev/null
    assert_nonempty_file "$tmp_root/tool-smoke.tiff"
  fi

  printf 'verified install surface: %s\n' "$label"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dist-dir)
      DIST_DIR="$2"
      shift 2
      ;;
    --stage-prefix)
      STAGE_PREFIX="$2"
      shift 2
      ;;
    --libtiff-deb)
      record_deb_path libtiff6 "$2"
      shift 2
      ;;
    --libtiffxx-deb)
      record_deb_path libtiffxx6 "$2"
      shift 2
      ;;
    --libtiff-dev-deb)
      record_deb_path libtiff-dev "$2"
      shift 2
      ;;
    --libtiff-tools-deb)
      record_deb_path libtiff-tools "$2"
      shift 2
      ;;
    --cmake-project)
      CMAKE_PROJECT_OVERRIDES+=("$2")
      shift 2
      ;;
    --cmake-project-no-target|--cmake-project-targetless|--cmake-no-target-project)
      CMAKE_PROJECT_NO_TARGET="$2"
      shift 2
      ;;
    --pkgconfig-source)
      PKGCONFIG_SOURCE="$2"
      shift 2
      ;;
    --cxx-smoke)
      CXX_SMOKE="$2"
      shift 2
      ;;
    --input-tiff)
      INPUT_TIFF="$2"
      shift 2
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

if [[ "${#CMAKE_PROJECT_OVERRIDES[@]}" -gt 2 ]]; then
  die "expected at most two --cmake-project arguments"
fi
if [[ "${#CMAKE_PROJECT_OVERRIDES[@]}" -ge 1 ]]; then
  CMAKE_PROJECT="${CMAKE_PROJECT_OVERRIDES[0]}"
fi
if [[ "${#CMAKE_PROJECT_OVERRIDES[@]}" -ge 2 ]]; then
  CMAKE_PROJECT_NO_TARGET="${CMAKE_PROJECT_OVERRIDES[1]}"
fi

if [[ -d "$DIST_DIR" ]]; then
  DIST_DIR="$(realpath "$DIST_DIR")"
elif [[ "${#DEB_PATHS[@]}" -ne 4 ]]; then
  die "missing dist dir: $DIST_DIR"
fi

if [[ -n "$STAGE_PREFIX" ]]; then
  run_prefix_smokes "$(realpath "$STAGE_PREFIX")" "staged install tree"
fi

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
combined_root="$tmp_root/combined"
mkdir -p "$combined_root"

for package in libtiff6 libtiffxx6 libtiff-dev libtiff-tools; do
  deb_path="${DEB_PATHS[$package]:-}"
  if [[ -z "$deb_path" ]]; then
    deb_path="$(resolve_deb "$DIST_DIR" "$package")"
  fi
  version="$(dpkg-deb -f "$deb_path" Version)"
  [[ "$version" == "$EXPECTED_VERSION" ]] || die "$package has unexpected version $version"
  dpkg --compare-versions "$version" gt "$BASELINE_VERSION" || \
    die "$package version does not sort above $BASELINE_VERSION"
  package_root="$tmp_root/$package"
  mkdir -p "$package_root"
  dpkg-deb -x "$deb_path" "$package_root"
  dpkg-deb -x "$deb_path" "$combined_root"
done

assert_exists "$tmp_root/libtiff6/usr/lib/$MULTIARCH/libtiff.so.6.0.1"
assert_exists "$tmp_root/libtiff6/usr/lib/$MULTIARCH/libtiff.so.6"
assert_absent "$tmp_root/libtiff6/usr/include"
assert_absent "$tmp_root/libtiff6/usr/lib/$MULTIARCH/libtiffxx.so.6.0.1"

assert_exists "$tmp_root/libtiffxx6/usr/lib/$MULTIARCH/libtiffxx.so.6.0.1"
assert_exists "$tmp_root/libtiffxx6/usr/lib/$MULTIARCH/libtiffxx.so.6"
assert_absent "$tmp_root/libtiffxx6/usr/include"
assert_absent "$tmp_root/libtiffxx6/usr/lib/$MULTIARCH/libtiff.so.6.0.1"

assert_exists "$tmp_root/libtiff-dev/usr/include/$MULTIARCH/tiffio.h"
assert_exists "$tmp_root/libtiff-dev/usr/include/$MULTIARCH/tiffio.hxx"
assert_exists "$tmp_root/libtiff-dev/usr/lib/$MULTIARCH/libtiff.so"
assert_exists "$tmp_root/libtiff-dev/usr/lib/$MULTIARCH/libtiffxx.so"
assert_exists "$tmp_root/libtiff-dev/usr/lib/$MULTIARCH/pkgconfig/libtiff-4.pc"
assert_exists "$tmp_root/libtiff-dev/usr/lib/$MULTIARCH/cmake/tiff/TiffConfig.cmake"
assert_exists "$tmp_root/libtiff-dev/usr/lib/$MULTIARCH/cmake/tiff/TiffTargets.cmake"
assert_absent "$tmp_root/libtiff-dev/usr/bin"

for tool in fax2ps fax2tiff pal2rgb ppm2tiff raw2tiff tiff2bw tiff2pdf tiff2ps tiff2rgba tiffcmp tiffcp tiffcrop tiffdither tiffdump tiffinfo tiffmedian tiffset tiffsplit; do
  assert_exists "$tmp_root/libtiff-tools/usr/bin/$tool"
  assert_exists "$tmp_root/libtiff-tools/usr/share/man/man1/$tool.1.gz"
done
assert_absent "$tmp_root/libtiff-tools/usr/lib"

run_prefix_smokes "$combined_root/usr" "extracted package root"
