#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SAFE_ROOT="$ROOT/safe"
SOURCE_DIR="$SAFE_ROOT"
DIST_DIR="$SAFE_ROOT/dist"
EXPECTED_VERSION="1:4.5.1+git230720-4ubuntu2.5+safelibs1"
INSIDE_CURRENT_ENV=0

die() {
  echo "error: $*" >&2
  exit 1
}

resolve_path() {
  local raw_path="$1"

  if [[ "$raw_path" = /* ]]; then
    printf '%s\n' "$raw_path"
  else
    printf '%s\n' "$ROOT/$raw_path"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-dir)
      SOURCE_DIR="$(resolve_path "$2")"
      shift 2
      ;;
    --out-dir)
      DIST_DIR="$(resolve_path "$2")"
      shift 2
      ;;
    --inside-current-env)
      INSIDE_CURRENT_ENV=1
      shift
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

if [[ "$INSIDE_CURRENT_ENV" -ne 0 ]]; then
  :
fi

SOURCE_DIR="$(realpath "$SOURCE_DIR")"
[[ -d "$SOURCE_DIR" ]] || die "missing source dir: $SOURCE_DIR"

(
  cd "$SOURCE_DIR"
  dpkg-buildpackage -us -uc -b
)

actual_dist_dir="$SOURCE_DIR/dist"
[[ -d "$actual_dist_dir" ]] || die "missing dist dir after build: $actual_dist_dir"

DIST_DIR="$(realpath -m "$DIST_DIR")"
if [[ "$DIST_DIR" != "$actual_dist_dir" ]]; then
  rm -rf "$DIST_DIR"
  mkdir -p "$DIST_DIR"
  find "$actual_dist_dir" -maxdepth 1 -type f \( -name '*.deb' -o -name '*.ddeb' \) \
    -exec cp -f -t "$DIST_DIR" {} +
fi

[[ -d "$DIST_DIR" ]] || die "missing dist dir after build: $DIST_DIR"

for package in libtiff6 libtiffxx6 libtiff-dev libtiff-tools; do
  deb_path=""
  while IFS= read -r candidate; do
    if [[ "$(dpkg-deb -f "$candidate" Package)" == "$package" ]]; then
      deb_path="$candidate"
      break
    fi
  done < <(find "$DIST_DIR" -maxdepth 1 -type f -name '*.deb' | sort)

  [[ -n "$deb_path" ]] || die "missing $package .deb under $DIST_DIR"
  [[ "$(dpkg-deb -f "$deb_path" Version)" == "$EXPECTED_VERSION" ]] || \
    die "$package has unexpected version in $(basename "$deb_path")"
done

printf 'built Debian packages in %s\n' "$DIST_DIR"
