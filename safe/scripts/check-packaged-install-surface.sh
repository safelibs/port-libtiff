#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SAFE_ROOT="$ROOT/safe"
DIST_DIR="${LIBTIFF_SAFE_DIST_DIR:-$SAFE_ROOT/dist}"
STAGE_PREFIX=""
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

  if grep -qx 'prefix=/usr' "$pc_file"; then
    sysroot="$(dirname "$prefix_root")"
  fi

  cmake -S "$ROOT/original/build/test_cmake" \
    -B "$tmp_root/test_cmake" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_PREFIX_PATH="$prefix_root" \
    >/dev/null
  cmake --build "$tmp_root/test_cmake" --parallel >/dev/null
  LD_LIBRARY_PATH="$libdir" "$tmp_root/test_cmake/test" >/dev/null

  cmake -S "$ROOT/original/build/test_cmake_no_target" \
    -B "$tmp_root/test_cmake_no_target" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_PREFIX_PATH="$prefix_root" \
    >/dev/null
  cmake --build "$tmp_root/test_cmake_no_target" --parallel >/dev/null
  LD_LIBRARY_PATH="$libdir" "$tmp_root/test_cmake_no_target/test" >/dev/null

  PKG_CONFIG_PATH="$libdir/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" \
  PKG_CONFIG_SYSROOT_DIR="$sysroot" \
  cc "$ROOT/original/build/test_cmake/test.c" \
    -o "$tmp_root/pkg_config_test" \
    $(PKG_CONFIG_PATH="$libdir/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" PKG_CONFIG_SYSROOT_DIR="$sysroot" pkg-config --cflags --libs libtiff-4)
  LD_LIBRARY_PATH="$libdir" "$tmp_root/pkg_config_test" >/dev/null

  c++ -std=c++17 \
    "$SAFE_ROOT/test/install/tiffxx_staged_smoke.cpp" \
    -I"$includedir" \
    -L"$libdir" \
    -Wl,-rpath,"$libdir" \
    -ltiffxx \
    -ltiff \
    -o "$tmp_root/tiffxx_staged_smoke"
  LD_LIBRARY_PATH="$libdir" "$tmp_root/tiffxx_staged_smoke" >/dev/null

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
    *)
      die "unknown argument: $1"
      ;;
  esac
done

DIST_DIR="$(realpath "$DIST_DIR")"
[[ -d "$DIST_DIR" ]] || die "missing dist dir: $DIST_DIR"

if [[ -n "$STAGE_PREFIX" ]]; then
  run_prefix_smokes "$(realpath "$STAGE_PREFIX")" "staged install tree"
fi

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
combined_root="$tmp_root/combined"
mkdir -p "$combined_root"

for package in libtiff6 libtiffxx6 libtiff-dev libtiff-tools; do
  deb_path="$(resolve_deb "$DIST_DIR" "$package")"
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
