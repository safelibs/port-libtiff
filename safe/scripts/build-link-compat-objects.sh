#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SAFE_ROOT="$ROOT/safe"
SAFE_BUILD_DIR="${SAFE_BUILD_DIR:-$SAFE_ROOT/build}"
OUT_DIR="${LINK_COMPAT_OBJECT_DIR:-$SAFE_BUILD_DIR/link-compat/objects}"

mkdir -p "$OUT_DIR"

COMMON_CFLAGS=(
  -I"$SAFE_ROOT/include"
  -I"$SAFE_ROOT/libtiff"
  -O2
  -g
  -fPIC
)
COMMON_CXXFLAGS=(
  -I"$SAFE_ROOT/include"
  -I"$SAFE_ROOT/libtiff"
  -O2
  -g
  -fPIC
  -std=c++17
)

cc "${COMMON_CFLAGS[@]}" \
  -c "$SAFE_ROOT/test/api_handle_smoke.c" \
  -o "$OUT_DIR/api_handle_smoke.o"

cc "${COMMON_CFLAGS[@]}" \
  -DSOURCE_DIR="\"$ROOT/original/test\"" \
  -c "$SAFE_ROOT/test/api_directory_read_smoke.c" \
  -o "$OUT_DIR/api_directory_read_smoke.o"

cc "${COMMON_CFLAGS[@]}" \
  -c "$SAFE_ROOT/test/api_field_registry_smoke.c" \
  -o "$OUT_DIR/api_field_registry_smoke.o"

cc "${COMMON_CFLAGS[@]}" \
  -c "$SAFE_ROOT/test/api_strile_smoke.c" \
  -o "$OUT_DIR/api_strile_smoke.o"

cc "${COMMON_CFLAGS[@]}" \
  -c "$SAFE_ROOT/test/link_compat_logluv_smoke.c" \
  -o "$OUT_DIR/link_compat_logluv_smoke.o"

c++ "${COMMON_CXXFLAGS[@]}" \
  -c "$SAFE_ROOT/test/install/tiffxx_staged_smoke.cpp" \
  -o "$OUT_DIR/tiffxx_staged_smoke.o"

printf '%s\n' "$OUT_DIR"
