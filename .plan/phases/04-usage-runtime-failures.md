# 04-usage-runtime-failures

## Phase Name

Usage Runtime Compatibility Fixes

## Implement Phase ID

`impl_usage_runtime_failures`

## Preexisting Inputs

- `validator/.git` already selected by phase 1. Do not refetch or move it. Expected selected commit for this plan is `87b321fe728340d6fc6dd2f638583cca82c667c3`, where libtiff has 5 source cases, 170 usage cases, and 175 total cases.
- Historical baseline for comparison: the prior validator report recorded a clean run at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`, with 135 of 135 passing, 5 source plus 130 usage, casts present, and no waivers.
- `validator-report.md` after phases 1-3, including package/provenance disposition, source/CLI disposition, remaining usage/runtime failures, current validator commit, safe commit, and `Waived testcase ids:`.
- Validator artifacts after phases 1-3:
  - `validator/artifacts/debs/local/libtiff/*.deb`
  - `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
  - `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
  - `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
  - `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
  - `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Current libtiff usage cases from the selected validator checkout. Validator `main` adds 40 usage testcases beyond the prior 130, for 170 usage cases. New usage coverage includes BigTIFF write/read variants, CCITT RLE, float32 and int32 pixel roundtrips, more metadata tags, explicit rational resolution, ICC profiles, orientation, palette colormap, SubIFD, additional `tiffcp`, `tiffcrop`, `tiff2pdf`, `tiffinfo`, `tiffsplit`, tile, rows-per-strip, and strip byte count checks.
- Existing safe tests and fixtures: `safe/test/CMakeLists.txt`, `safe/test/validator_usage_jpeg_encode.c`, `safe/test/validator_usage_tools.sh`, `safe/test/test_rgba_readers.c`, `safe/test/test_tile_read_write.c`, `safe/test/dirwrite_regressions.c`, `safe/test/dirread_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/images/`, and `safe/test/refs/`.
- Existing reference fixtures under `original/test/images/`.
- Existing package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/test.c`, `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`, and `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`.
- Runtime implementation hotspots:
  - `safe/src/lib.rs` owns handle lifecycle, open modes, header parsing, allocation helpers, and public C ABI exports. Relevant entry points: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c` owns C varargs marshalling, field get/set wrappers, error/warning handlers, RGBA wrappers, and directory printing. Relevant functions: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` around line 1511.
  - `safe/src/core/directory.rs` owns IFD parsing/writing, tag storage, defaulted fields, custom directories, deferred strile tags, and `TIFFWriteDirectory`. Relevant functions: `read_next_directory` at line 1384 and `TIFFWriteDirectory` at line 4357. Use it for IFD read/write, custom dirs, BigTIFF/SubIFD/multipage, and rewrite behavior.
  - `safe/src/core/field_tables.rs` and `safe/src/core/field_registry.rs` for tag definitions, custom fields, defaulted field behavior, and field lookup.
  - `safe/src/strile.rs` owns strip/tile geometry, scanline I/O, offset/bytecount management, flushing, and codec integration. Relevant functions include codec decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs` owns compression dispatch. Relevant functions: `safe_tiff_codec_decode_bytes` at line 3019 and `safe_tiff_codec_encode_bytes` at line 3070.
  - `safe/src/core/jpeg.rs` owns JPEG/OJPEG encode and decode. Relevant functions: `jpeg_decode_bytes` at line 755 and `jpeg_encode_bytes` at line 803.
  - `safe/src/core/color.rs` and `safe/src/rgba.rs` own color conversion, pixel/raster behavior, Pillow-facing RGBA read paths, and orientation handling.
- Safe port build facts: `safe/Cargo.toml` builds crate `safe-libtiff` as static library `tiff_safe_core`; `safe/CMakeLists.txt` enumerates `SAFE_RUST_SOURCES` and builds shared libraries, copied upstream tools, CMake/package metadata, and optional tests; `safe/scripts/build-deb.sh` builds version `1:4.5.1+git230720-4ubuntu2.5+safelibs1` packages into `safe/dist/`.

Consume these artifacts in place. Do not refetch, recollect, rediscover, or regenerate prepared artifacts. Do not refetch or move the validator checkout. Do not edit validator tests, shared scripts, manifests, runner code, or tools.

## New Outputs

- Regression tests for each non-CLI usage failure or failure class.
- Safe implementation fixes.
- Rebuilt `safe/dist/*.deb`.
- Refreshed `validator/artifacts/debs/local/libtiff/*.deb`, `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`, `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`, `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`, `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`, and `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Updated `validator-report.md` with usage failures found, tests added, files changed, fixes applied, waiver state, and final non-CLI usage disposition.
- A git commit before yielding.

## File Changes

- Test changes: `safe/test/CMakeLists.txt`, `safe/test/test_rgba_readers.c`, `safe/test/test_tile_read_write.c`, `safe/test/validator_usage_jpeg_encode.c`, `safe/test/validator_usage_tools.sh`, `safe/test/dirwrite_regressions.c`, `safe/test/dirread_regressions.c`, `safe/test/strile_regressions.c`, or new fixtures under `safe/test/images/`.
- Runtime fixes: `safe/src/core/directory.rs`, `safe/src/core/field_tables.rs`, `safe/src/core/field_registry.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/rgba.rs`, and `safe/capi/tiff_placeholder.c`.
- `safe/CMakeLists.txt` if tests, build wiring, or new Rust source files require it.
- Always update `validator-report.md`.
- Do not modify validator runtime files.

Critical file guidance for this phase:

- Add only minimal deterministic fixtures or references under `safe/test/images/*` and `safe/test/refs/*` needed to reproduce validator failures.
- Avoid dependency additions in `safe/Cargo.toml` unless a failure cannot be fixed with existing dependencies (`fax`, `flate2`, `libc`, and `weezl`).
- Public headers and ABI maps should change only for missing or incompatible public API declarations/exports discovered through usage failures.

## Implementation Details

Workflow-generation contract preserved for this implement block:

- Execute phases linearly. Do not generate `parallel_groups`.
- Preserve the source-plan generation boundary for downstream workflow generation: generate only `.plan/plan.md`; do not generate or edit `.plan/phases/*`, `.plan/workflow-structure.yaml`, or `workflow.yaml` from inside phase-level prompts because workflow-generation stages own those files.
- Use self-contained inline-only YAML. Do not use a top-level `include`.
- Do not use phase-level `prompt_file`, `workflow_file`, `workflow_dir`, `checks`, `source`, or any other YAML-source indirection.
- Do not generate `bounce_targets` lists. Each verifier has exactly one fixed `bounce_target`.
- Every verifier is an explicit top-level `check` phase, stays inside the implement block it verifies, and bounces only to `impl_usage_runtime_failures`.
- Put verifier commands directly in the checker instructions; do not model tests, builds, proof generation, artifact parsing, or review commands as non-agentic phases.
- Consume existing artifacts in place: `original/`, `safe/test/`, CVE/dependent inventories, package scripts, ABI inventories, link-compatibility harnesses, downstream smoke harnesses, package-smoke projects, prior validator artifacts, and the phase-1-selected validator checkout.
- Do not refetch or move `validator/`; phase 1 is the only phase that may fetch or checkout the validator repository.

- Group usage failures by behavior, not individual testcase file:
  - Metadata/tag failures: fix field table definitions, varargs marshalling, defaulted fields, ASCII/count/rational storage, `TIFFPrintDirectory`, and directory write ordering.
  - BigTIFF/SubIFD/multipage failures: fix header version, 64-bit offsets/counts, next-directory links, subdirectory traversal, and rewrite logic in `safe/src/core/directory.rs`.
  - Strip/tile/rows-per-strip failures: fix strip/tile geometry, byte counts, offset materialization, deferred strile loading/writing, and tiled-vs-stripped tag interactions in `safe/src/strile.rs`.
  - Compression failures: fix codec dispatch and codec-specific encode/decode for LZW, Deflate, PackBits, CCITT, JPEG/OJPEG, LZMA, ZSTD, WEBP, LERC, and predictors.
  - Pillow image mode/pixel failures: fix RGBA/color conversion, sample format, alpha/extrasamples, palette colormap, CMYK, ICC, orientation, and 16-bit/float/int sample handling.
- For each failure class, add one minimal local regression that fails before the fix and passes after. Prefer deterministic C tests for API-level bugs; prefer shell tests for copied CLI behavior.
- Before rebuilding packages for a validator run, commit all `safe/` source, test, packaging, or script changes represented in those packages. The lock must record the safe-source commit actually tested.
- Rebuild packages and rerun the full libtiff validator matrix after every coherent failure class or before yielding. The rerun must refresh the local lock/override, per-case JSON/log/cast artifacts, and proof.
- Update `validator-report.md` with failures found, tests added, files changed, fixes applied, artifact paths, and final disposition.
- If there are no non-CLI usage failures in this bucket, update `validator-report.md` with the clean disposition and create an empty or report-only commit named for `impl_usage_runtime_failures`.
- In `port` mode, parse result JSON and per-case JSON instead of trusting `bash test.sh` process exit status.

## Verification Phases

### `check_usage_runtime_tester`

- Phase ID: `check_usage_runtime_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_usage_runtime_failures`
- Purpose: Verify non-CLI usage failures from the 170 validator usage cases are fixed or waived and have minimal safe regression coverage.
- Commands:

```bash
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
safe/scripts/build-link-compat-objects.sh
safe/scripts/link-and-run-link-compat.sh
LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh
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
cli_terms = ("tiffcp", "tiffdump", "tiffinfo", "tiff2bw", "tiff2pdf", "tiffcrop", "tiffmedian", "tiffsplit")
remaining_usage = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    r = json.loads(path.read_text())
    tid = r["testcase_id"]
    assert r.get("port_commit") == safe_commit, (tid, r.get("port_commit"), safe_commit)
    if r.get("status") == "failed" and r.get("kind") == "usage" and tid not in waived and not any(term in tid for term in cli_terms):
        remaining_usage.append(tid)
assert not remaining_usage, remaining_usage
PY
```

### `check_usage_runtime_senior`

- Phase ID: `check_usage_runtime_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_usage_runtime_failures`
- Purpose: Review Pillow/dependent-client compatibility fixes for API behavior, metadata fidelity, compression correctness, BigTIFF/SubIFD handling, and memory-safety risk.
- Commands:

```bash
git show --stat --format=fuller HEAD
git diff HEAD~1..HEAD -- safe validator-report.md
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
rg -n "usage|Pillow|BigTIFF|SubIFD|compression|metadata|ICC|rational|orientation|colormap|rowsperstrip|tile|strip|regression|fixed|waiver" validator-report.md
```

## Success Criteria

- Cargo tests, CMake release build, CTest, upstream shell tests, link compatibility, and downstream-style `test-original.sh` pass.
- Full validator rerun has no unwaived non-CLI usage failures.
- Usage fixes have focused local regression coverage.
- The lock, proof, per-case results, and report all name the safe-source commit actually tested.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If the phase only updates the report or has no code change, commit the report change or create an empty phase commit naming `impl_usage_runtime_failures`.
