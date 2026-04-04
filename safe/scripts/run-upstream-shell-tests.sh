#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SAFE_ROOT=$(cd "${SCRIPT_DIR}/.." && pwd)
SAFE_TEST_DIR="${SAFE_ROOT}/test"
SAFE_BUILD_DIR="${SAFE_ROOT}/build"

declare -a tests_filters=()
include_regex=""

usage() {
  cat <<'EOF'
Usage: run-upstream-shell-tests.sh [--build-dir dir] [--test-dir dir] [--tests test1.sh [test2.sh ...]] [--include-regex regex]
EOF
}

discover_tests() {
  awk '
    /^[A-Z_]*TESTSCRIPTS[[:space:]]*=/ { capture = 1 }
    capture {
      line = $0
      sub(/#.*/, "", line)
      sub(/^[A-Z_]*TESTSCRIPTS[[:space:]]*=[[:space:]]*/, "", line)
      gsub(/\\/, " ", line)
      n = split(line, parts, /[[:space:]]+/)
      for (i = 1; i <= n; ++i) {
        if (parts[i] ~ /\.sh$/) {
          print parts[i]
        }
      }
      if ($0 !~ /\\[[:space:]]*$/) {
        capture = 0
      }
    }
  ' "${MAKEFILE_AM}" | sort -u
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tests)
      shift
      while [[ $# -gt 0 && "$1" != --* ]]; do
        tests_filters+=("$1")
        shift
      done
      ;;
    --build-dir)
      SAFE_BUILD_DIR=$(cd "${2:-}" && pwd)
      shift 2
      ;;
    --test-dir)
      SAFE_TEST_DIR=$(cd "${2:-}" && pwd)
      shift 2
      ;;
    --include-regex)
      include_regex="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

SAFE_ROOT=$(cd "${SAFE_TEST_DIR}/.." && pwd)
SAFE_BUILD_TEST_DIR="${SAFE_BUILD_DIR}/test"
SAFE_BUILD_TOOLS_DIR="${SAFE_BUILD_DIR}/tools"
MAKEFILE_AM="${SAFE_TEST_DIR}/Makefile.am"

mapfile -t discovered_tests < <(discover_tests)
if [[ ${#discovered_tests[@]} -eq 0 ]]; then
  echo "no upstream shell tests discovered in ${MAKEFILE_AM}" >&2
  exit 1
fi

declare -A requested=()
if [[ ${#tests_filters[@]} -gt 0 ]]; then
  for filter_value in "${tests_filters[@]}"; do
    IFS=',' read -r -a requested_list <<<"${filter_value}"
    for test_name in "${requested_list[@]}"; do
      if [[ -n "${test_name}" ]]; then
        requested["${test_name}"]=1
      fi
    done
  done
fi

selected_tests=()
for test_name in "${discovered_tests[@]}"; do
  if [[ ${#tests_filters[@]} -gt 0 && -z "${requested[${test_name}]:-}" ]]; then
    continue
  fi
  if [[ -n "${include_regex}" && ! "${test_name}" =~ ${include_regex} ]]; then
    continue
  fi
  selected_tests+=("${test_name}")
done

if [[ ${#selected_tests[@]} -eq 0 ]]; then
  echo "no upstream shell tests matched the requested selectors" >&2
  exit 1
fi

if [[ ${#tests_filters[@]} -gt 0 ]]; then
  for requested_name in "${!requested[@]}"; do
    if [[ ! " ${discovered_tests[*]} " =~ [[:space:]]${requested_name}[[:space:]] ]]; then
      echo "unknown upstream shell test: ${requested_name}" >&2
      exit 1
    fi
  done
fi

mkdir -p "${SAFE_BUILD_TEST_DIR}"

export srcdir="${SAFE_TEST_DIR}"
export top_srcdir="${SAFE_ROOT}"
export top_builddir="${SAFE_BUILD_DIR}"
export LD_LIBRARY_PATH="${SAFE_BUILD_DIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

export FAX2PS="${SAFE_BUILD_TOOLS_DIR}/fax2ps"
export FAX2TIFF="${SAFE_BUILD_TOOLS_DIR}/fax2tiff"
export PAL2RGB="${SAFE_BUILD_TOOLS_DIR}/pal2rgb"
export PPM2TIFF="${SAFE_BUILD_TOOLS_DIR}/ppm2tiff"
export RAW2TIFF="${SAFE_BUILD_TOOLS_DIR}/raw2tiff"
export RGB2YCBCR="${SAFE_BUILD_TOOLS_DIR}/rgb2ycbcr"
export THUMBNAIL="${SAFE_BUILD_TOOLS_DIR}/thumbnail"
export TIFF2BW="${SAFE_BUILD_TOOLS_DIR}/tiff2bw"
export TIFF2PDF="${SAFE_BUILD_TOOLS_DIR}/tiff2pdf"
export TIFF2PS="${SAFE_BUILD_TOOLS_DIR}/tiff2ps"
export TIFF2RGBA="${SAFE_BUILD_TOOLS_DIR}/tiff2rgba"
export TIFFCMP="${SAFE_BUILD_TOOLS_DIR}/tiffcmp"
export TIFFCP="${SAFE_BUILD_TOOLS_DIR}/tiffcp"
export TIFFCROP="${SAFE_BUILD_TOOLS_DIR}/tiffcrop"
export TIFFDITHER="${SAFE_BUILD_TOOLS_DIR}/tiffdither"
export TIFFDUMP="${SAFE_BUILD_TOOLS_DIR}/tiffdump"
export TIFFINFO="${SAFE_BUILD_TOOLS_DIR}/tiffinfo"
export TIFFMEDIAN="${SAFE_BUILD_TOOLS_DIR}/tiffmedian"
export TIFFSET="${SAFE_BUILD_TOOLS_DIR}/tiffset"
export TIFFSPLIT="${SAFE_BUILD_TOOLS_DIR}/tiffsplit"

for test_name in "${selected_tests[@]}"; do
  test_path="${SAFE_TEST_DIR}/${test_name}"
  if [[ ! -f "${test_path}" ]]; then
    echo "missing test script: ${test_path}" >&2
    exit 1
  fi
  find "${SAFE_BUILD_TEST_DIR}" -maxdepth 1 -mindepth 1 -name 'o-*' -exec rm -rf -- {} +
  echo "==> ${test_name}"
  (
    cd "${SAFE_BUILD_TEST_DIR}"
    "${test_path}"
  )
done
