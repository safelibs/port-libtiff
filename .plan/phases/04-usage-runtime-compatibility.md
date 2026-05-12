# Usage Runtime Compatibility

## Phase Name

Usage Runtime Compatibility

## Implement Phase ID

`impl_usage_runtime_fixes`

## Shared Context And Critical Files

- Treat `.plan/plan.md` as the only authoritative implementation plan. Do not consume stale generated phase files, `.plan/workflow-structure.yaml`, or `workflow.yaml`; do not stage or modify `workflow.yaml`.
- Consume the validator checkout selected by Phase 1 and earlier package/proof/report artifacts. Do not refetch or move `validator/`, and do not edit validator runtime inputs: `validator/tests/libtiff/**`, `validator/tests/_shared/**`, `validator/repositories.yml`, `validator/test.sh`, or `validator/tools/**`.
- Preserve existing workspace artifacts in place, including `original/`, `safe/test/`, `safe/test/images/`, `safe/test/refs/`, `safe/abi/*`, `safe/capi/*.map`, CVE/dependent inventories, package-smoke projects, link-compatibility harnesses, and existing validator artifacts. Rebuild package, lock, matrix, proof, and report artifacts only for the committed safe tree.
- The selected validator checkout must derive exact libtiff counts and enforce floors of at least 5 source, 240 usage, 10 regression, and 255 total cases. Usage coverage includes Pillow read/write/metadata, BigTIFF, CCITT Group 3/4/RLE, float32 and int32 roundtrips, ICC profiles, palette colormaps, SubIFDs, multipage workflows, tiled output, and copied libtiff tools.
- Before package builds, lock generation, validator reruns, proof generation, or accepting package/validator artifacts, `git status --porcelain -- safe` must be empty. The lock and per-case `port_commit` must match `git log -1 --format=%H -- safe`.
- `validator-report.md` preserves immutable Phase 1 baseline bucket lines. This phase consumes `Usage/runtime baseline testcase ids:` and updates current-failure and waiver lines from the latest per-case JSON.
- Critical runtime/API files and hotspot references are:
  - `safe/src/lib.rs`: handle lifecycle, open modes, header parsing, allocation helpers, and public C ABI exports. Hotspots include `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: C varargs marshalling, tag get/set wrappers, error/warning handlers, RGBA wrappers, and directory printing. Hotspots include `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` at line 1583.
  - `safe/src/core/directory.rs`: IFD parsing/writing, tag storage, defaulted fields, custom directories, deferred strile tags, and `TIFFWriteDirectory` at line 4357. `read_next_directory` starts at line 1384.
  - `safe/src/strile.rs`: strip/tile geometry, scanline I/O, raw and encoded strile management, and codec integration. Hotspots include decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: compression dispatch, especially `safe_tiff_codec_decode_bytes` at line 3025 and `safe_tiff_codec_encode_bytes` at line 3076.
  - `safe/src/core/jpeg.rs`: JPEG/OJPEG encode and decode, especially `jpeg_decode_bytes` at line 820 and `jpeg_encode_bytes` at line 868.
  - `safe/src/core/color.rs` and `safe/src/rgba.rs`: color conversion, RGBA read paths, Pillow-facing behavior, and orientation handling.
  - `safe/src/core/field_tables.rs` and `safe/src/core/field_registry.rs`: tag lookup, defaulted field behavior, custom fields, and registry metadata.
- Critical copied-tool files are `safe/tools/tiffcp.c`, `safe/tools/tiffcrop.c`, `safe/tools/tiff2pdf.c`, `safe/tools/tiffinfo.c`, `safe/tools/tiffdump.c`, `safe/tools/tiffmedian.c`, and `safe/tools/tiffsplit.c`; edit them only after confirming a validator failure is genuinely tool-level.
- Critical test and harness files are `safe/test/CMakeLists.txt`, `safe/test/test_rgba_readers.c`, `safe/test/test_tile_read_write.c`, `safe/test/validator_usage_jpeg_encode.c`, `safe/test/validator_usage_tools.sh`, `safe/test/strile_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/scripts/run-upstream-shell-tests.sh`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/build-deb.sh`, and `test-original.sh`.
- Critical packaging/public surface files remain relevant when runtime failures expose install or ABI drift: `safe/include/*.h`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/*`, `safe/debian/*`, `safe/pkgconfig/libtiff-4.pc.in`, and `safe/cmake/TiffConfig.cmake.in`.

## Preexisting Inputs

- Remaining usage/runtime failures from `validator-report.md`.
- Validator checkout selected by Phase 1 and artifacts from Phases 1-3.
- Validator usage scripts from the selected checkout. Expected coverage includes Pillow read/write/metadata cases, BigTIFF cases, CCITT Group 3/4/RLE compression, float32 and int32 roundtrips, ICC profile handling, palette colormap handling, SubIFDs, multipage workflows, tiled output, `tiffcp`, `tiffcrop`, `tiff2pdf`, `tiffinfo`, `tiffdump`, `tiffmedian`, and `tiffsplit`.
- Existing safe tests: `safe/test/test_rgba_readers.c`, `safe/test/test_tile_read_write.c`, `safe/test/validator_usage_jpeg_encode.c`, `safe/test/validator_usage_tools.sh`, `safe/test/strile_regressions.c`, `safe/test/dirwrite_regressions.c`, and copied upstream shell tests.
- Existing package, lock, validator result, proof, and `validator-report.md` artifacts from earlier phases; consume them as inputs and refresh only after committed safe changes.

## New Outputs

- Minimal local regression tests for each usage/runtime failure class.
- Safe runtime/API/codec/tool fixes.
- Rebuilt package and validator artifacts.
- Updated `validator-report.md` usage/runtime section.
- Git commit for `impl_usage_runtime_fixes`.

## File Changes

- Likely: `safe/src/rgba.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/core/directory.rs`, `safe/capi/tiff_placeholder.c`.
- Possible: `safe/tools/tiffcp.c`, `safe/tools/tiffcrop.c`, `safe/tools/tiff2pdf.c`, `safe/tools/tiffinfo.c`, `safe/tools/tiffdump.c`, `safe/tools/tiffmedian.c`, `safe/tools/tiffsplit.c`.
- Tests: `safe/test/test_rgba_readers.c`, `safe/test/test_tile_read_write.c`, `safe/test/validator_usage_jpeg_encode.c`, `safe/test/validator_usage_tools.sh`, `safe/test/CMakeLists.txt`, and new fixtures only if necessary.
- Always: `validator-report.md`.
- Do not edit validator usage scripts, shared scripts, manifests, runner code, tools, or `workflow.yaml`.

## Implementation Details

- For Pillow cases, classify failures by the C API behavior used by Pillow: open modes, tag get/set/defaults, scanline/strip/tile reads, RGBA helpers, compression, endian handling, resolution rationals, or multipage directory traversal.
- Use `Usage/runtime baseline testcase ids:` as the immutable Phase 1 usage/runtime scope, and also inspect the latest per-case result JSON for any new usage failures introduced by later changes.
- For tool cases, determine whether the tool is exposing a library defect or copied-tool behavior drift. Prefer library fixes when the same behavior is reachable through public APIs.
- For codec failures, add tests that exercise both encoded bytes and tag side effects such as `Compression`, `Predictor`, `JPEGTables`, strip byte counts, rows per strip, tile width/length, and extrasamples.
- Preserve upstream-compatible observable output for `tiffinfo`, `tiffdump`, and `TIFFPrintDirectory` because validator checks logs and textual output.
- Keep regression tests minimal and deterministic. Avoid duplicating whole validator scripts inside `safe/test`.
- After local compatibility tests pass, commit safe source/test/tool changes before rebuilding packages or running any validator rerun. Use the committed `git log -1 --format=%H -- safe` value for the lock.
- After the validator rerun, update and commit `validator-report.md` separately if needed.
- Leave all three `* baseline testcase ids:` lines unchanged. Update `Current package/provenance failed testcase ids:`, `Current source/regression failed testcase ids:`, `Current usage/runtime failed testcase ids:`, and `Waived testcase ids:` from the latest validator result JSON so those four current lines partition current failed testcase ids exactly once.
- Before yielding, run `git status --short`. Stage and commit any `safe/` runtime/API/codec/tool/test changes before the final package and validator rerun for this phase. After that rerun, stage only `validator-report.md` for the report update and commit it with a message naming `impl_usage_runtime_fixes`. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If a later edit touches `safe/` again, repeat the package build, lock generation, validator run, proof generation, and report update against the new committed safe tree. If there are no tracked file changes because no usage/runtime failures remained, create an empty commit naming `impl_usage_runtime_fixes`.
- If waivers are added or retained, collect original-mode evidence from the Phase 1 validator checkout and reference both original and port result/log paths in `validator-report.md`. Use this command only when `Waived testcase ids:` is non-empty:

```bash
(
  cd validator
  set +e
  bash test.sh \
    --config repositories.yml \
    --tests-root tests \
    --artifact-root artifacts/libtiff-original-waiver \
    --mode original \
    --library libtiff \
    --record-casts
  original_status=$?
  mkdir -p artifacts/libtiff-original-waiver/original
  printf '%s\n' "$original_status" > artifacts/libtiff-original-waiver/original/matrix-status.txt
  if [ ! -f artifacts/libtiff-original-waiver/results/libtiff/summary.json ]; then
    if [ "$original_status" -ne 0 ]; then
      exit "$original_status"
    fi
    exit 1
  fi
  python3 tools/verify_proof_artifacts.py \
    --config repositories.yml \
    --tests-root tests \
    --artifact-root artifacts/libtiff-original-waiver \
    --proof-output proof/libtiff-original-waiver-proof.json \
    --mode original \
    --library libtiff \
    --require-casts \
    --min-source-cases 5 \
    --min-usage-cases 240 \
    --min-regression-cases 10 \
    --min-cases 255 \
    --ports-root /home/yans/safelibs/pipeline/ports
  original_proof_status=$?
  mkdir -p artifacts/libtiff-original-waiver/proof
  printf '%s\n' "$original_proof_status" > artifacts/libtiff-original-waiver/proof/proof-status.txt
  set -e
)
```

## Verification Phases

### `check_usage_runtime_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_usage_runtime_fixes`
- Purpose: Verify each usage failure has a minimal local regression and the main local compatibility matrix passes.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
rm -rf safe/build/link-compat
safe/scripts/build-link-compat-objects.sh
safe/scripts/link-and-run-link-compat.sh
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh
```

### `check_usage_runtime_senior`

- Type: `check`
- Fixed `bounce_target`: `impl_usage_runtime_fixes`
- Purpose: Review runtime fixes for broad compatibility and no validator-specific hacks. Rerun the validator and fail on remaining non-waived usage failures.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
SAFE_COMMIT="$(git log -1 --format=%H -- safe)"
rm -rf validator/artifacts/debs/local/libtiff
SAFELIBS_LIBRARY=libtiff \
SAFELIBS_COMMIT_SHA="$SAFE_COMMIT" \
SAFELIBS_DIST_DIR="$PWD/safe/dist" \
SAFELIBS_VALIDATOR_DIR="$PWD/validator" \
SAFELIBS_LOCK_PATH="$PWD/validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json" \
SAFELIBS_OVERRIDE_ROOT="$PWD/validator/artifacts/debs/local" \
python3 scripts/lib/build_port_lock.py
(
  cd validator
  set +e
  bash test.sh \
    --config repositories.yml \
    --tests-root tests \
    --artifact-root artifacts/libtiff-safe \
    --mode port \
    --override-deb-root artifacts/debs/local \
    --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json \
    --library libtiff \
    --record-casts
  status=$?
  mkdir -p artifacts/libtiff-safe/port
  printf '%s\n' "$status" > artifacts/libtiff-safe/port/matrix-status.txt
  if [ ! -f artifacts/libtiff-safe/port/results/libtiff/summary.json ]; then
    if [ "$status" -ne 0 ]; then
      exit "$status"
    fi
    exit 1
  fi
  python3 tools/verify_proof_artifacts.py \
    --config repositories.yml \
    --tests-root tests \
    --artifact-root artifacts/libtiff-safe \
    --proof-output proof/libtiff-safe-port-proof.json \
    --mode port \
    --library libtiff \
    --require-casts \
    --min-source-cases 5 \
    --min-usage-cases 240 \
    --min-regression-cases 10 \
    --min-cases 255 \
    --ports-root /home/yans/safelibs/pipeline/ports
  proof_status=$?
  mkdir -p artifacts/libtiff-safe/proof
  printf '%s\n' "$proof_status" > artifacts/libtiff-safe/proof/proof-status.txt
  set -e
)
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

root = Path(".")
summary = json.loads((root / "validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
assert summary["usage_cases"] >= 240, summary
report = (root / "validator-report.md").read_text()
def parse_report_ids(label):
    match = re.search(rf"^{re.escape(label)}:\s*(.*)$", report, re.M)
    assert match, f"missing {label} line"
    ids = [item.strip() for item in match.group(1).split(",") if item.strip()]
    assert len(ids) == len(set(ids)), f"duplicate ids in {label}: {ids}"
    return set(ids)

current_package_ids = parse_report_ids("Current package/provenance failed testcase ids")
current_source_regression_ids = parse_report_ids("Current source/regression failed testcase ids")
current_usage_ids = parse_report_ids("Current usage/runtime failed testcase ids")
usage_baseline_ids = parse_report_ids("Usage/runtime baseline testcase ids")
waived = parse_report_ids("Waived testcase ids")
validator_commit = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
proof_status_path = root / "validator/artifacts/libtiff-safe/proof/proof-status.txt"
assert proof_status_path.is_file(), "missing proof verification status"
proof_status = int(proof_status_path.read_text().strip())
proof = json.loads((root / "validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
lib_proof = proof["libraries"][0]
source = len(list((root / "validator/tests/libtiff/tests/cases/source").glob("*.sh")))
usage = len(list((root / "validator/tests/libtiff/tests/cases/usage").glob("*.sh")))
regression = len(list((root / "validator/tests/libtiff/tests/cases/regression").glob("*.sh")))
assert source >= 5 and usage >= 240 and regression >= 10, (source, usage, regression)
assert summary["library"] == "libtiff", summary
assert summary["mode"] == "port", summary
assert summary["source_cases"] == source, (summary, source)
assert summary["usage_cases"] == usage, (summary, usage)
assert summary["regression_cases"] == regression, (summary, regression)
assert summary["cases"] == source + usage + regression, summary
assert summary["passed"] + summary["failed"] == summary["cases"], summary
assert summary["casts"] == summary["cases"], summary
assert proof["mode"] == "port", proof
assert lib_proof["library"] == "libtiff", lib_proof
assert lib_proof["port_commit"] == safe_commit, (lib_proof.get("port_commit"), safe_commit)
for field in ("cases", "source_cases", "usage_cases", "regression_cases", "passed", "failed", "casts"):
    assert proof["totals"][field] == summary[field], (field, proof["totals"], summary)
    assert lib_proof["totals"][field] == summary[field], (field, lib_proof["totals"], summary)
failures = []
all_failed = set()
payloads = {}
seen_baseline_ids = set()
remaining_baseline_failures = set()
for path in (root / "validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json"):
    if path.name == "summary.json":
        continue
    payload = json.loads(path.read_text())
    testcase_id = payload.get("testcase_id")
    payloads[testcase_id] = payload
    assert payload.get("override_debs_installed") is True, path
    assert payload.get("port_commit") == safe_commit, (path, payload.get("port_commit"), safe_commit)
    if testcase_id in usage_baseline_ids:
        seen_baseline_ids.add(testcase_id)
        if payload.get("status") == "failed":
            remaining_baseline_failures.add(testcase_id)
    if payload.get("status") == "failed":
        all_failed.add(testcase_id)
    if payload.get("status") == "failed" and payload.get("kind") == "usage":
        failures.append(testcase_id)
current_buckets = {
    "current package/provenance": current_package_ids,
    "current source/regression": current_source_regression_ids,
    "current usage/runtime": current_usage_ids,
    "waived": waived,
}
seen_once = {}
all_result_ids = set(payloads)
for bucket_name, ids in current_buckets.items():
    unknown = sorted(ids - all_result_ids)
    assert not unknown, f"{bucket_name} ids missing current per-case JSON: {unknown}"
    not_failed = sorted(ids - all_failed)
    assert not not_failed, f"{bucket_name} ids are not failed in current results: {not_failed}"
    for testcase_id in ids:
        seen_once.setdefault(testcase_id, []).append(bucket_name)
duplicates = {testcase_id: names for testcase_id, names in seen_once.items() if len(names) != 1}
assert not duplicates, f"current failed ids appear in multiple buckets: {duplicates}"
classified = set().union(*current_buckets.values()) if current_buckets else set()
assert classified == all_failed, {
    "missing_from_current_report": sorted(all_failed - classified),
    "stale_in_current_report": sorted(classified - all_failed),
}
for testcase_id in sorted(current_source_regression_ids):
    assert payloads[testcase_id].get("kind") in {"source", "regression"}, (testcase_id, payloads[testcase_id].get("kind"))
for testcase_id in sorted(current_usage_ids):
    assert payloads[testcase_id].get("kind") == "usage", (testcase_id, payloads[testcase_id].get("kind"))
assert seen_baseline_ids == usage_baseline_ids, {
    "missing_usage_baseline_ids": sorted(usage_baseline_ids - seen_baseline_ids)
}
unexpected_baseline = sorted(remaining_baseline_failures - waived)
assert not unexpected_baseline, {
    "remaining_usage_baseline_failures": unexpected_baseline,
    "waived": sorted(waived),
}
unexpected = sorted(set(failures) - waived)
assert not unexpected, {"unexpected": unexpected, "waived": sorted(waived)}
assert proof_status == 0, f"proof verification failed with status {proof_status}"
unused_waivers = sorted(waived - all_failed)
assert not unused_waivers, f"waived testcase ids are no longer failing: {unused_waivers}"
if waived:
    assert "Waiver evidence:" in report, "missing waiver evidence section"
    original_proof_status_path = root / "validator/artifacts/libtiff-original-waiver/proof/proof-status.txt"
    assert original_proof_status_path.is_file(), "missing original waiver proof status"
    assert int(original_proof_status_path.read_text().strip()) == 0, "original waiver proof verification failed"
for testcase_id in sorted(waived):
    port_result = root / "validator/artifacts/libtiff-safe/port/results/libtiff" / f"{testcase_id}.json"
    port_log = root / "validator/artifacts/libtiff-safe/port/logs/libtiff" / f"{testcase_id}.log"
    original_result = root / "validator/artifacts/libtiff-original-waiver/results/libtiff" / f"{testcase_id}.json"
    original_log = root / "validator/artifacts/libtiff-original-waiver/logs/libtiff" / f"{testcase_id}.log"
    for evidence_path in (port_result, port_log, original_result, original_log):
        assert evidence_path.is_file(), evidence_path
        assert str(evidence_path) in report, f"{evidence_path} not referenced in report"
    port_payload = payloads[testcase_id]
    original_payload = json.loads(original_result.read_text())
    assert port_payload.get("status") == "failed", testcase_id
    assert port_payload.get("port_commit") == safe_commit, testcase_id
    assert validator_commit in report, "validator commit missing from waiver report"
    assert safe_commit in report, "safe commit missing from waiver report"
    if original_payload.get("status") == "passed":
        assert f"Original result exception for {testcase_id}:" in report
PY
```

## Success Criteria

- Every usage/runtime baseline failure is fixed in `safe/` with a minimal local regression test, or remains only as an evidence-backed waiver.
- Local compatibility matrix passes, including Rust tests, CMake build, CTest, upstream shell tests, link compatibility, package build, and `test-original.sh`.
- Full validator port run has no usage failures except evidence-backed waived ids.
- `validator-report.md` current-failure lines and waiver line partition current failed result ids exactly once.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. Before yielding, run `git status --short`. Stage and commit any `safe/` runtime/API/codec/tool/test changes before package rebuild, lock generation, validator execution, and report rewrite. After the validator rerun, stage only `validator-report.md` and commit it if it changed. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If no tracked files changed because no usage/runtime failures remained, create an empty commit naming `impl_usage_runtime_fixes`.
