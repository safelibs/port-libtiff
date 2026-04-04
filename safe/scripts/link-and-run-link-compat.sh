#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
SAFE_ROOT="$ROOT/safe"
SAFE_BUILD_DIR="${SAFE_BUILD_DIR:-$SAFE_ROOT/build}"
SAFE_LIB_DIR="$SAFE_BUILD_DIR/libtiff"
ORIGINAL_STEP2_TEST_DIR="$ROOT/original/build-step2/test"
ORIGINAL_LIBTIFFXX="$ROOT/original/build/libtiff/libtiffxx.so.6.0.1"
OBJECT_DIR="${LINK_COMPAT_OBJECT_DIR:-$SAFE_BUILD_DIR/link-compat/objects}"
BIN_DIR="${LINK_COMPAT_BIN_DIR:-$SAFE_BUILD_DIR/link-compat/bin}"

die() {
  echo "error: $*" >&2
  exit 1
}

require_file() {
  [[ -f "$1" ]] || die "missing required file: $1"
}

require_file "$SAFE_LIB_DIR/libtiff.so.6.0.1"
require_file "$SAFE_LIB_DIR/libtiffxx.so.6.0.1"
require_file "$ORIGINAL_LIBTIFFXX"

if [[ ! -d "$OBJECT_DIR" ]]; then
  "$SAFE_ROOT/scripts/build-link-compat-objects.sh"
fi

mkdir -p "$BIN_DIR"

python3 - "$SAFE_LIB_DIR/libtiffxx.so.6.0.1" "$ORIGINAL_LIBTIFFXX" <<'PY'
import subprocess
import sys

def dynsym_names(path: str):
    out = subprocess.check_output(
        ["readelf", "--dyn-syms", "--wide", path],
        text=True,
    )
    names = []
    for line in out.splitlines():
        tokens = line.split()
        if len(tokens) < 8 or not tokens[0].endswith(":"):
            continue
        names.append(tokens[-1])
    return sorted(names)

safe = dynsym_names(sys.argv[1])
orig = dynsym_names(sys.argv[2])
if safe != orig:
    missing = sorted(set(orig) - set(safe))
    extra = sorted(set(safe) - set(orig))
    if missing:
        print("missing dynsym entries:", file=sys.stderr)
        for item in missing:
            print(f"  {item}", file=sys.stderr)
    if extra:
        print("unexpected dynsym entries:", file=sys.stderr)
        for item in extra:
            print(f"  {item}", file=sys.stderr)
    raise SystemExit(1)

required = {
    "_Z14TIFFStreamOpenPKcPSi@@LIBTIFFXX_4.0",
    "_Z14TIFFStreamOpenPKcPSo@@LIBTIFFXX_4.0",
}
missing = sorted(required - set(safe))
if missing:
    print("missing required TIFFStreamOpen exports:", file=sys.stderr)
    for item in missing:
        print(f"  {item}", file=sys.stderr)
    raise SystemExit(1)
PY

while IFS= read -r link_txt; do
  target_name="$(basename "$(dirname "$link_txt")" .dir)"
  output_path="$BIN_DIR/$target_name"
  python3 - "$link_txt" "$output_path" "$SAFE_LIB_DIR" <<'PY'
import pathlib
import shlex
import subprocess
import sys

link_txt = pathlib.Path(sys.argv[1])
output_path = pathlib.Path(sys.argv[2])
safe_lib_dir = pathlib.Path(sys.argv[3])

args = shlex.split(link_txt.read_text().strip())
rewritten = []
i = 0
while i < len(args):
    arg = args[i]
    if arg == "-o":
        rewritten.extend([arg, str(output_path)])
        i += 2
        continue
    if arg.startswith("-Wl,-rpath,"):
        rewritten.append(f"-Wl,-rpath,{safe_lib_dir}")
    elif arg == "../libtiff/libtiff.so.6.0.1":
        rewritten.append(str(safe_lib_dir / "libtiff.so.6.0.1"))
    else:
        rewritten.append(arg)
    i += 1

subprocess.run(rewritten, cwd=str(link_txt.parent.parent.parent), check=True)
PY
  (
    cd "$ORIGINAL_STEP2_TEST_DIR"
    srcdir="$ROOT/original/test" LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$output_path"
  )
done < <(find "$ORIGINAL_STEP2_TEST_DIR/CMakeFiles" -path '*/link.txt' | sort)

cc -O2 -g -Wl,-rpath,"$SAFE_LIB_DIR" \
  -o "$BIN_DIR/extra_api_handle_smoke" \
  "$OBJECT_DIR/api_handle_smoke.o" \
  "$SAFE_LIB_DIR/libtiff.so.6.0.1"
LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$BIN_DIR/extra_api_handle_smoke"

cc -O2 -g -Wl,-rpath,"$SAFE_LIB_DIR" \
  -o "$BIN_DIR/extra_api_directory_read_smoke" \
  "$OBJECT_DIR/api_directory_read_smoke.o" \
  "$SAFE_LIB_DIR/libtiff.so.6.0.1"
LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$BIN_DIR/extra_api_directory_read_smoke"

cc -O2 -g -Wl,-rpath,"$SAFE_LIB_DIR" \
  -o "$BIN_DIR/extra_api_field_registry_smoke" \
  "$OBJECT_DIR/api_field_registry_smoke.o" \
  "$SAFE_LIB_DIR/libtiff.so.6.0.1"
LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$BIN_DIR/extra_api_field_registry_smoke"

cc -O2 -g -Wl,-rpath,"$SAFE_LIB_DIR" \
  -o "$BIN_DIR/extra_api_strile_smoke" \
  "$OBJECT_DIR/api_strile_smoke.o" \
  "$SAFE_LIB_DIR/libtiff.so.6.0.1"
LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$BIN_DIR/extra_api_strile_smoke"

cc -O2 -g -Wl,-rpath,"$SAFE_LIB_DIR" \
  -o "$BIN_DIR/extra_link_compat_logluv_smoke" \
  "$OBJECT_DIR/link_compat_logluv_smoke.o" \
  "$SAFE_LIB_DIR/libtiff.so.6.0.1"
LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$BIN_DIR/extra_link_compat_logluv_smoke"

c++ -O2 -g -Wl,-rpath,"$SAFE_LIB_DIR" \
  -o "$BIN_DIR/extra_tiffxx_staged_smoke" \
  "$OBJECT_DIR/tiffxx_staged_smoke.o" \
  "$SAFE_LIB_DIR/libtiffxx.so.6.0.1" \
  "$SAFE_LIB_DIR/libtiff.so.6.0.1"
LD_LIBRARY_PATH="$SAFE_LIB_DIR" "$BIN_DIR/extra_tiffxx_staged_smoke"
