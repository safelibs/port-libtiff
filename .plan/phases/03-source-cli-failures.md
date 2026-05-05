# 03-source-cli-failures

## Phase Name

Source And CLI Compatibility Fixes

## Implement Phase ID

`impl_source_cli_failures`

## Preexisting Inputs

- `validator/.git` already selected by phase 1. Do not refetch or move it. Expected selected commit for this plan is `87b321fe728340d6fc6dd2f638583cca82c667c3`, where libtiff has 5 source cases, 170 usage cases, and 175 total cases.
- Historical baseline for comparison: the prior validator report recorded a clean run at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`, with 135 of 135 passing, 5 source plus 130 usage, casts present, and no waivers.
- `validator-report.md` after phases 1 and 2, including baseline failure buckets, package gate disposition, current validator commit, safe commit, and `Waived testcase ids:`.
- Validator artifacts after phases 1 and 2:
  - `validator/artifacts/debs/local/libtiff/*.deb`
  - `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
  - `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
  - `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
  - `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
  - `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Existing source testcase identities from the validator suite: `c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, and `tiffinfo-metadata`.
- Existing CLI usage failures identified in `validator-report.md`. CLI-oriented testcase ids include tools such as `tiffcp`, `tiffdump`, `tiffinfo`, `tiff2bw`, `tiff2pdf`, `tiffcrop`, `tiffmedian`, and `tiffsplit`.
- New validator usage coverage on `origin/main` includes additional `tiffcp`, `tiffcrop`, `tiff2pdf`, `tiffinfo`, `tiffsplit`, BigTIFF, tile, rows-per-strip, strip byte count, metadata, rational resolution, ICC, orientation, palette colormap, SubIFD, CCITT RLE, float32, and int32 checks; source/CLI work should only address the source cases and CLI-oriented failures from that broader matrix.
- Existing safe CTest and shell test infrastructure: `safe/test/CMakeLists.txt`, `safe/test/dirread_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/validator_usage_tools.sh`, `safe/test/api_*.c`, `safe/test/images/`, and `safe/test/refs/`.
- Existing package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/test.c`, `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`, and `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`.
- Safe source hotspots relevant to source and CLI compatibility:
  - `safe/src/lib.rs` owns handle lifecycle, open modes, header parsing, allocation helpers, and public C ABI exports. Relevant entry points: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c` owns C varargs marshalling, field get/set wrappers, error/warning handlers, RGBA wrappers, and directory printing. Relevant functions: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` around line 1511.
  - `safe/src/core/directory.rs` owns IFD parsing/writing, tag storage, defaulted fields, custom directories, deferred strile tags, and `TIFFWriteDirectory`. Relevant functions: `read_next_directory` at line 1384 and `TIFFWriteDirectory` at line 4357.
  - `safe/src/strile.rs` owns strip/tile geometry, scanline I/O, offset/bytecount management, flushing, and codec integration. Relevant functions include codec decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs` owns compression dispatch. Relevant functions: `safe_tiff_codec_decode_bytes` at line 3019 and `safe_tiff_codec_encode_bytes` at line 3070.
  - `safe/src/core/jpeg.rs` owns JPEG/OJPEG encode and decode. Relevant functions: `jpeg_decode_bytes` at line 755 and `jpeg_encode_bytes` at line 803.
  - `safe/src/rgba.rs` owns Pillow-facing RGBA read paths, color conversion, and orientation handling.
  - Copied CLI tool files under `safe/tools/`.
- Safe port build facts: `safe/Cargo.toml` builds crate `safe-libtiff` as static library `tiff_safe_core`; `safe/CMakeLists.txt` enumerates `SAFE_RUST_SOURCES` and builds the shared libraries, tools, pkg-config, CMake package metadata, and optional tests; add new Rust source files to `SAFE_RUST_SOURCES`.

Consume these artifacts in place. Do not refetch, recollect, rediscover, or regenerate `original/`, safe test fixtures, CVE inventories, dependent inventories, package scripts, ABI inventories, link-compatibility harnesses, downstream smoke harnesses, package-smoke projects, or validator checkout state. Do not edit validator runtime files.

## New Outputs

- Minimal safe regression tests for each source or CLI failure.
- Safe implementation fixes in relevant modules.
- Rebuilt `safe/dist/*.deb`.
- Refreshed `validator/artifacts/debs/local/libtiff/*.deb`, `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`, `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`, `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`, `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`, and `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Updated `validator-report.md` with source/CLI failures found, regression tests added, files changed, and final source/CLI disposition.
- A git commit before yielding.

## File Changes

- Test changes: `safe/test/CMakeLists.txt`, `safe/test/dirread_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/validator_usage_tools.sh`, `safe/test/api_*.c`, or new focused files under `safe/test/`.
- Source fixes: `safe/src/lib.rs`, `safe/capi/tiff_placeholder.c`, `safe/src/core/directory.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/rgba.rs`, or copied tool files under `safe/tools/`.
- ABI/export fixes may touch `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/include/tiffio.h`, or `safe/include/tiffio.hxx`.
- Always update `validator-report.md`.
- Do not edit validator tests, validator shared scripts, manifests, runner code, or tools.

Critical file guidance for this phase:

- Prefer fixing library behavior before copied CLI tool behavior.
- Update `safe/CMakeLists.txt` when new tests, tools, install wiring, or Rust source files are added.
- Update public headers or ABI maps only for missing or incompatible public API declarations/exports.
- `validator/artifacts/libtiff-safe/**/*` and `validator/artifacts/debs/local/libtiff/*` are generated external validation evidence and are not committed to the parent repo.

## Implementation Details

Workflow-generation contract preserved for this implement block:

- Execute phases linearly. Do not generate `parallel_groups`.
- Preserve the source-plan generation boundary for downstream workflow generation: generate only `.plan/plan.md`; do not generate or edit `.plan/phases/*`, `.plan/workflow-structure.yaml`, or `workflow.yaml` from inside phase-level prompts because workflow-generation stages own those files.
- Use self-contained inline-only YAML. Do not use a top-level `include`.
- Do not use phase-level `prompt_file`, `workflow_file`, `workflow_dir`, `checks`, `source`, or any other YAML-source indirection.
- Do not generate `bounce_targets` lists. Each verifier has exactly one fixed `bounce_target`.
- Every verifier is an explicit top-level `check` phase, stays inside the implement block it verifies, and bounces only to `impl_source_cli_failures`.
- Put verifier commands directly in the checker instructions; do not model tests, builds, proof generation, artifact parsing, or review commands as non-agentic phases.
- Consume existing artifacts in place: `original/`, `safe/test/`, CVE/dependent inventories, package scripts, ABI inventories, link-compatibility harnesses, downstream smoke harnesses, package-smoke projects, prior validator artifacts, and the phase-1-selected validator checkout.
- Do not refetch or move `validator/`; phase 1 is the only phase that may fetch or checkout the validator repository.

- Map each source/CLI validator failure to the smallest safe regression:
  - `c-api-read-write`: add a C regression for field setting, directory write, scanline/tile write, close/reopen, and field readback.
  - `malformed-tiff-rejection`: add a fixture or byte-level generated test in `dirread_regressions.c`; ensure malformed IFD, offset, type, count, and loop cases fail cleanly.
  - `tiffcp-copy` and CLI `tiffcp-*`: add shell or C tests for compression, rows-per-strip, tiling, BigTIFF, endian options, and strip/tile byte counts as relevant.
  - `tiffdump-structure` and CLI `tiffinfo-*`: add shell tests that assert exact structural/tag lines without depending on validator internals.
  - `tiff2pdf`, `tiff2bw`, `tiffcrop`, `tiffmedian`, and `tiffsplit`: add shell tests in the existing upstream-test style and prefer existing fixtures under `safe/test/images/`.
- Fix the underlying safe implementation, not copied validator scripts.
- Before rebuilding packages for a validator run, commit all `safe/` source, test, packaging, or script changes represented in those packages. The lock must record the safe-source commit actually tested.
- Rebuild packages, regenerate the local lock/override using `scripts/lib/build_port_lock.py`, rerun the full libtiff port matrix with `--library libtiff --record-casts`, regenerate proof, update `validator-report.md`, and commit.
- If there are no source or CLI failures in this bucket, update `validator-report.md` with the clean disposition and create an empty or report-only commit named for `impl_source_cli_failures`.
- In `port` mode, parse result JSON and per-case JSON instead of trusting `bash test.sh` process exit status.

## Verification Phases

### `check_source_cli_tester`

- Phase ID: `check_source_cli_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_source_cli_failures`
- Purpose: Verify all source testcases and CLI-oriented validator failures are fixed or explicitly waived, and each fixed failure has a safe regression test.
- Commands:

```bash
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
python3 safe/scripts/check-public-surface.py \
  --check \
  --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen \
  --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
assert lock["libraries"][0]["commit"] == safe_commit, (lock["libraries"][0]["commit"], safe_commit)
assert proof["libraries"][0]["port_commit"] == safe_commit, (proof["libraries"][0], safe_commit)

report = Path("validator-report.md").read_text()
waived_line = re.search(r"^Waived testcase ids:\s*(.*)$", report, re.MULTILINE)
waived = {x.strip() for x in (waived_line.group(1) if waived_line else "").split(",") if x.strip()}
source_failures = []
cli_failures = []
cli_terms = ("tiffcp", "tiffdump", "tiffinfo", "tiff2bw", "tiff2pdf", "tiffcrop", "tiffmedian", "tiffsplit")
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    r = json.loads(path.read_text())
    tid = r["testcase_id"]
    assert r.get("port_commit") == safe_commit, (tid, r.get("port_commit"), safe_commit)
    if r.get("status") != "failed" or tid in waived:
        continue
    if r.get("kind") == "source":
        source_failures.append(tid)
    if any(term in tid for term in cli_terms):
        cli_failures.append(tid)
assert not source_failures, source_failures
assert not cli_failures, cli_failures
PY
```

### `check_source_cli_senior`

- Phase ID: `check_source_cli_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_source_cli_failures`
- Purpose: Review that fixes are minimal, regression tests map directly to validator failures, ABI/export changes are intentional, and no validator checks were loosened.
- Commands:

```bash
git show --stat --format=fuller HEAD
git diff HEAD~1..HEAD -- safe validator-report.md
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
rg -n "source|CLI|tiffcp|tiffdump|tiffinfo|tiff2bw|tiff2pdf|tiffcrop|tiffmedian|tiffsplit|regression|fixed|waiver" validator-report.md
```

## Success Criteria

- Cargo tests, CMake release build, CTest, upstream shell tests, and public surface checks pass.
- Full validator rerun has no unwaived source failures and no unwaived CLI-oriented failures.
- Each fixed source or CLI failure has focused local regression coverage.
- The lock, proof, per-case results, and report all name the safe-source commit actually tested.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If the phase only updates the report or has no code change, commit the report change or create an empty phase commit naming `impl_source_cli_failures`.
