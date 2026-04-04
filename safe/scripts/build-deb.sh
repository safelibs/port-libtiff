#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SAFE_ROOT="$ROOT/safe"
DIST_DIR="$SAFE_ROOT/dist"
EXPECTED_VERSION="1:4.5.1+git230720-4ubuntu2.5+safelibs1"
INSIDE_CURRENT_ENV=0

die() {
  echo "error: $*" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
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

(
  cd "$SAFE_ROOT"
  dpkg-buildpackage -us -uc -b
)

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
