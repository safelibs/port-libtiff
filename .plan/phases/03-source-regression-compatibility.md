# Source And CVE Regression Compatibility

## Phase Name

Source And CVE Regression Compatibility

## Implement Phase ID

`impl_source_regression_fixes`

## Shared Context And Critical Files

- Treat `.plan/plan.md` as the only authoritative implementation plan. Do not consume stale generated phase files, `.plan/workflow-structure.yaml`, or `workflow.yaml`; do not stage or modify `workflow.yaml`.
- Consume the validator checkout selected by Phase 1 and the package/proof/report artifacts produced by earlier phases. Do not refetch or move `validator/`, and do not edit validator runtime inputs: `validator/tests/libtiff/**`, `validator/tests/_shared/**`, `validator/repositories.yml`, `validator/test.sh`, or `validator/tools/**`.
- Preserve existing workspace artifacts in place, including `original/`, `safe/test/`, `safe/test/images/`, `safe/test/refs/`, `safe/abi/*`, `safe/capi/*.map`, CVE inventories, dependent inventories, link-compatibility harnesses, package-smoke inputs, and existing validator artifacts. Rebuild derived package, lock, matrix, proof, and report artifacts only for the committed safe tree.
- The selected validator checkout must derive exact libtiff counts and enforce floors of at least 5 source, 240 usage, 10 regression, and 255 total cases. Source/regression failures cover both source scripts and CVE regression reproducers selected by that checkout.
- Before package builds, lock generation, validator reruns, proof generation, or accepting package/validator artifacts, `git status --porcelain -- safe` must be empty. The lock and per-case `port_commit` must match `git log -1 --format=%H -- safe`.
- `validator-report.md` preserves the immutable Phase 1 baseline bucket lines. This phase consumes `Source/regression baseline testcase ids:` and updates the current-failure and waiver lines from the latest per-case JSON.
- Critical source/regression implementation files and hotspot references are:
  - `safe/src/lib.rs`: handle lifecycle, open modes, header parsing, allocation helpers, and public C ABI exports. Hotspots include `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: C varargs marshalling, tag get/set wrappers, error/warning handlers, RGBA wrappers, and directory printing. Hotspots include `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` at line 1583.
  - `safe/src/core/directory.rs`: IFD parsing/writing, tag storage, defaulted fields, custom directories, deferred strile tags, and `TIFFWriteDirectory` at line 4357. `read_next_directory` starts at line 1384.
  - `safe/src/strile.rs`: strip/tile geometry, scanline I/O, raw and encoded strile management, and codec integration. Hotspots include decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: compression dispatch, especially `safe_tiff_codec_decode_bytes` at line 3025 and `safe_tiff_codec_encode_bytes` at line 3076.
  - `safe/src/core/jpeg.rs`: JPEG/OJPEG encode and decode, especially `jpeg_decode_bytes` at line 820 and `jpeg_encode_bytes` at line 868.
  - `safe/src/rgba.rs`: RGBA read paths, color conversion, Pillow-facing behavior, and orientation handling.
- Critical metadata and public surface files are `safe/src/core/field_tables.rs`, `safe/src/core/field_registry.rs`, `safe/include/tiff.h`, `safe/include/tiffio.h`, `safe/include/tiffio.hxx`, `safe/include/tiffconf.h`, `safe/include/tif_config.h`, `safe/include/tiffvers.h`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, and `safe/abi/*`.
- Critical test locations are `safe/test/CMakeLists.txt`, `safe/test/dirread_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/test_ifd_loop_detection.c`, `safe/test/test_open_options.c`, `safe/test/test_write_read_tags.c`, `safe/test/validator_usage_tools.sh`, and fixtures under `safe/test/images/`.
- Critical harnesses are `safe/scripts/build-deb.sh`, `safe/scripts/check-public-surface.py`, `safe/scripts/run-upstream-shell-tests.sh`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `scripts/lib/build_port_lock.py`, and `scripts/run-validation-tests.sh`.

## Preexisting Inputs

- Phase 1 baseline failure classification.
- Validator checkout selected by Phase 1 and updated artifacts from Phases 1-2.
- Validator source cases: `c-api-read-write.sh`, `malformed-tiff-rejection.sh`, `tiffcp-copy.sh`, `tiffdump-structure.sh`, and `tiffinfo-metadata.sh`.
- Validator regression cases selected by the updated checkout. Expected CVE reproducers include `CVE-2004-0804`, `CVE-2005-2452`, `CVE-2006-3463`, `CVE-2014-8130`, `CVE-2017-11613`, `CVE-2018-5784`, `CVE-2019-14973`, `CVE-2022-40090`, `CVE-2023-52355`, and `CVE-2023-6277`.
- Local safe regression buckets: `safe/test/dirread_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/test_ifd_loop_detection.c`, `safe/test/test_open_options.c`, `safe/test/test_write_read_tags.c`, `safe/test/validator_usage_tools.sh`, and fixtures under `safe/test/images/`.
- Existing package, lock, validator result, proof, and `validator-report.md` artifacts from earlier phases; consume them as inputs and refresh only after committed safe changes.

## New Outputs

- Minimal local regression tests for each source/regression failure class.
- Safe implementation fixes.
- Updated package artifacts and validator artifacts after rerun.
- Updated `validator-report.md` source/regression section.
- Git commit for `impl_source_regression_fixes`.

## File Changes

- Likely: `safe/src/lib.rs`, `safe/src/core/directory.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/capi/tiff_placeholder.c`.
- Likely tests: `safe/test/dirread_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/CMakeLists.txt`, and required new fixtures under `safe/test/images/`.
- Possible: `safe/tools/*.c` for CLI behavior only after confirming the issue is tool-level.
- Always: `validator-report.md`.
- Do not edit validator source/regression scripts, shared scripts, manifests, runner code, tools, or `workflow.yaml`.

## Implementation Details

- For each failed source/regression testcase, inspect its validator script and log under `validator/artifacts/libtiff-safe/port/logs/libtiff/<id>.log`.
- Use `Source/regression baseline testcase ids:` as the immutable Phase 1 source/regression scope, and also inspect the latest per-case result JSON for any new source or regression failures introduced by later changes.
- Reproduce locally with the smallest C test or shell test that covers the failure. Prefer an existing CTest bucket.
- Fix malformed input handling by returning upstream-compatible errors, not by panicking or silently accepting invalid state.
- Use checked arithmetic for directory offsets, strile counts, image dimensions, and allocation sizes.
- Treat CVE regression failures as safety regressions. If a validator CVE reproducer is questionable, run the waiver-evidence command below before considering a waiver.
- Do not edit validator regression scripts.
- After local regression tests pass, commit the safe source/test changes before rebuilding packages or running any validator rerun. Use the committed `git log -1 --format=%H -- safe` value for the lock.
- After the validator rerun, update and commit `validator-report.md` separately if needed.
- Leave all three `* baseline testcase ids:` lines unchanged. Update `Current package/provenance failed testcase ids:`, `Current source/regression failed testcase ids:`, `Current usage/runtime failed testcase ids:`, and `Waived testcase ids:` from the latest validator result JSON so those four current lines partition current failed testcase ids exactly once.
- Before yielding, run `git status --short`. Stage and commit any `safe/` source/test changes before the final package and validator rerun for this phase. After that rerun, stage only `validator-report.md` for the report update and commit it with a message naming `impl_source_regression_fixes`. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If a later edit touches `safe/` again, repeat the package build, lock generation, validator run, proof generation, and report update against the new committed safe tree. If there are no tracked file changes because no source/regression failures remained, create an empty commit naming `impl_source_regression_fixes`.
- Use this waiver-evidence command only when adding or retaining a non-empty `Waived testcase ids:` line:

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

### `check_source_regression_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_source_regression_fixes`
- Purpose: Verify each source-facing or regression testcase failure has a minimal local regression test, a safe implementation fix, and passing local tests.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
```

### `check_source_regression_senior`

- Type: `check`
- Fixed `bounce_target`: `impl_source_regression_fixes`
- Purpose: Review fixes for safety and compatibility. Confirm failures were fixed in `safe/`, not by weakening tests, and run validator proof checks against the selected validator checkout.
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
assert summary["source_cases"] >= 5, summary
assert summary["regression_cases"] >= 10, summary
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
source_regression_baseline_ids = parse_report_ids("Source/regression baseline testcase ids")
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
    if testcase_id in source_regression_baseline_ids:
        seen_baseline_ids.add(testcase_id)
        if payload.get("status") == "failed":
            remaining_baseline_failures.add(testcase_id)
    if payload.get("status") == "failed":
        all_failed.add(testcase_id)
    if payload.get("status") == "failed" and payload.get("kind") in {"source", "regression"}:
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
assert seen_baseline_ids == source_regression_baseline_ids, {
    "missing_source_regression_baseline_ids": sorted(source_regression_baseline_ids - seen_baseline_ids)
}
unexpected_baseline = sorted(remaining_baseline_failures - waived)
assert not unexpected_baseline, {
    "remaining_source_regression_baseline_failures": unexpected_baseline,
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

- Every source/regression baseline failure is fixed in `safe/` with a minimal local regression test, or remains only as an evidence-backed waiver.
- Local Rust, CMake, CTest, and upstream shell tests pass.
- Full validator port artifacts and proof match the committed safe tree and selected validator checkout.
- No source or regression kind failures remain in validator artifacts unless listed as evidence-backed waivers in `validator-report.md`.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. Before yielding, run `git status --short`. Stage and commit any `safe/` source/test/tool changes before package rebuild, lock generation, validator execution, and report rewrite. After the validator rerun, stage only `validator-report.md` and commit it if it changed. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If no tracked files changed because no source/regression failures remained, create an empty commit naming `impl_source_regression_fixes`.
