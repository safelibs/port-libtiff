# Final Validator Clean Run

## Phase Name

Final Validator Clean Run

## Implement Phase ID

`impl_final_validator_clean_run`

## Shared Context And Critical Files

- Treat `.plan/plan.md` as the only authoritative implementation plan. Do not consume stale generated phase files, `.plan/workflow-structure.yaml`, or `workflow.yaml`; do not stage or modify `workflow.yaml`.
- Consume the validator checkout selected by Phase 1 and all artifacts from Phases 1-4. Do not refetch or move `validator/`, and do not edit validator runtime inputs: `validator/tests/libtiff/**`, `validator/tests/_shared/**`, `validator/repositories.yml`, `validator/test.sh`, or `validator/tools/**`.
- Preserve existing workspace artifacts in place, including `original/`, `safe/test/`, `safe/test/images/`, `safe/test/refs/`, `safe/abi/*`, `safe/capi/*.map`, package scripts, package-smoke projects, link-compatibility harnesses, CVE/dependent inventories, and validator artifacts. Refresh final package, lock, matrix, proof, and report artifacts only for the final committed safe tree.
- The final validator checkout must support source, usage, and regression testcase kinds and satisfy floors of at least 5 source, 240 usage, 10 regression, and 255 total libtiff cases. Final verification must run the validator metadata checks as well as local safe/package/link/original checks and the port-mode validator/proof audit.
- Before package builds, lock generation, validator reruns, proof generation, or accepting existing package/validator artifacts, `git status --porcelain -- safe` must be empty. The final lock, every per-case `port_commit`, proof JSON, and `validator-report.md` safe commit must equal `git log -1 --format=%H -- safe`.
- `validator-report.md` must name the final validator commit and safe commit, preserve immutable Phase 1 baseline bucket lines, update current-failure and waiver lines so they partition final failed result ids exactly once, and include evidence for every waived id.
- Critical safe implementation files for final audit include `safe/src/lib.rs`, `safe/src/core/directory.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/rgba.rs`, `safe/src/core/field_tables.rs`, `safe/src/core/field_registry.rs`, and `safe/capi/tiff_placeholder.c`.
- Hotspot line references from the authoritative plan must be preserved during final audit:
  - `safe/src/lib.rs`: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` at line 1583.
  - `safe/src/core/directory.rs`: `read_next_directory` starts at line 1384, and `TIFFWriteDirectory` is at line 4357.
  - `safe/src/strile.rs`: decode/use hotspots around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: `safe_tiff_codec_decode_bytes` at line 3025 and `safe_tiff_codec_encode_bytes` at line 3076.
  - `safe/src/core/jpeg.rs`: `jpeg_decode_bytes` at line 820 and `jpeg_encode_bytes` at line 868.
  - `safe/src/rgba.rs`: RGBA read paths, color conversion, Pillow-facing behavior, and orientation handling.
- Critical public surface and packaging files are `safe/include/tiff.h`, `safe/include/tiffio.h`, `safe/include/tiffio.hxx`, `safe/include/tiffconf.h`, `safe/include/tif_config.h`, `safe/include/tiffvers.h`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/*`, `safe/debian/control`, `safe/debian/rules`, `safe/debian/*.install`, `safe/debian/*.symbols`, `safe/pkgconfig/libtiff-4.pc.in`, and `safe/cmake/TiffConfig.cmake.in`.
- Critical tests and harnesses are `safe/test/CMakeLists.txt`, the existing regression and usage buckets under `safe/test/`, `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, `safe/scripts/check-public-surface.py`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/run-upstream-shell-tests.sh`, `scripts/lib/build_port_lock.py`, `scripts/run-validation-tests.sh`, and `test-original.sh`.
- Critical validator evidence outputs are `validator/artifacts/debs/local/libtiff/*.deb`, `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`, `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`, casts, `validator/artifacts/libtiff-safe/port/matrix-status.txt`, `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`, `validator/artifacts/libtiff-safe/proof/proof-status.txt`, and `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.

## Preexisting Inputs

- All outputs from Phases 1-4.
- Validator checkout selected by Phase 1.
- Final safe tree after all fixes.
- Current `validator-report.md` with immutable baseline buckets, current-failure dispositions, and waiver line.
- Package-smoke projects verified or created by Phase 1 and safe local test harnesses.
- Existing package, lock, validator result, proof, and report artifacts from earlier phases; consume them as inputs and refresh only for the final committed safe tree.

## New Outputs

- Final rebuilt `safe/dist/*.deb`.
- Final local validator override `.deb` files.
- Final lock, results, logs, casts, and proof artifacts under `validator/artifacts/libtiff-safe/`.
- Final `validator-report.md` with validator commit, safe commit, checks executed, counts, failures found, fixes applied, waivers, package checksums, and final status.
- Git commit for `impl_final_validator_clean_run`.

## File Changes

- Always: `validator-report.md`.
- Conditional: remaining minimal `safe/` fixes and matching regression tests if the catch-all run exposes missed failures.
- No validator runtime file edits.
- Do not modify `.plan/plan.md` or `workflow.yaml`.

## Implementation Details

- Run the entire local safe matrix and package checks before the final validator run.
- Regenerate the lock from final `safe/dist` packages and ensure it records the final safe commit.
- Run the full validator port matrix with casts, then verify proof with `--require-casts`.
- Audit every per-case result for `override_debs_installed: true` and final `port_commit`.
- If failures remain, either fix them in `safe/` with regression tests or document an evidence-backed validator-bug waiver using the waiver-evidence command below. Do not finish with undocumented failures.
- Rewrite `validator-report.md` as the final report. Preserve immutable Phase 1 `* baseline testcase ids:` lines, and update current-failure lines plus `Waived testcase ids:` so they partition final failed testcase ids exactly once.
- Commit any final safe source/test changes before the final package rebuild and validator run. Use the committed `git log -1 --format=%H -- safe` value for the final lock and report.
- After the final validator run, update and commit `validator-report.md` separately if needed.
- Before yielding, run `git status --short`. Stage and commit any final `safe/` changes before the final package rebuild, lock generation, validator execution, proof generation, and report rewrite. After that run, stage only `validator-report.md` for the closure report update and commit it with a message naming `impl_final_validator_clean_run`. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If the final phase only confirms existing artifacts and produces no tracked changes, create an empty commit naming `impl_final_validator_clean_run`.
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

### `check_final_validator_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_final_validator_clean_run`
- Purpose: Verify final artifacts, proof, counts, casts, package lock, and waiver policy. Require zero unexpected failures.
- Commands:

```bash
test -d validator/.git
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
test -z "$(git status --porcelain -- safe)"
make -C validator unit
make -C validator check-testcases
python3 validator/tools/testcases.py \
  --config validator/repositories.yml \
  --tests-root validator/tests \
  --library libtiff \
  --check \
  --min-source-cases 5 \
  --min-usage-cases 240 \
  --min-regression-cases 10 \
  --min-cases 255
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
python3 safe/scripts/check-public-surface.py \
  --check \
  --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen \
  --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
rm -rf safe/build/link-compat
safe/scripts/build-link-compat-objects.sh
safe/scripts/link-and-run-link-compat.sh
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project-no-target validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh
```

### `check_final_validator_senior`

- Type: `check`
- Fixed `bounce_target`: `impl_final_validator_clean_run`
- Purpose: Independently audit proof JSON, summary JSON, per-case results, casts, lock commit, report contents, and waiver policy.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
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
  matrix_status=$?
  mkdir -p artifacts/libtiff-safe/port
  printf '%s\n' "$matrix_status" > artifacts/libtiff-safe/port/matrix-status.txt
  if [ ! -f artifacts/libtiff-safe/port/results/libtiff/summary.json ]; then
    if [ "$matrix_status" -ne 0 ]; then
      exit "$matrix_status"
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
lock = json.loads((root / "validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
report = (root / "validator-report.md").read_text()
matrix_status_path = root / "validator/artifacts/libtiff-safe/port/matrix-status.txt"
assert matrix_status_path.is_file(), "missing validator matrix status"
matrix_status = int(matrix_status_path.read_text().strip())
proof_status_path = root / "validator/artifacts/libtiff-safe/proof/proof-status.txt"
assert proof_status_path.is_file(), "missing proof verification status"
proof_status = int(proof_status_path.read_text().strip())

safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
validator_commit = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
assert f"Validator commit: {validator_commit}" in report
assert f"Safe source commit tested: {safe_commit}" in report

source = len(list((root / "validator/tests/libtiff/tests/cases/source").glob("*.sh")))
usage = len(list((root / "validator/tests/libtiff/tests/cases/usage").glob("*.sh")))
regression = len(list((root / "validator/tests/libtiff/tests/cases/regression").glob("*.sh")))
assert source >= 5 and usage >= 240 and regression >= 10, (source, usage, regression)
assert summary["source_cases"] == source, summary
assert summary["usage_cases"] == usage, summary
assert summary["regression_cases"] == regression, summary
assert summary["cases"] == source + usage + regression, summary
assert summary["casts"] == summary["cases"], summary
assert summary["passed"] + summary["failed"] == summary["cases"], summary

lib_lock = lock["libraries"][0]
assert lib_lock["commit"] == safe_commit, (lib_lock["commit"], safe_commit)
assert lib_lock["unported_original_packages"] == [], lib_lock
assert [d["package"] for d in lib_lock["debs"]] == ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"], lib_lock

def parse_report_ids(label):
    match = re.search(rf"^{re.escape(label)}:\s*(.*)$", report, re.M)
    assert match, f"missing {label} line"
    ids = [item.strip() for item in match.group(1).split(",") if item.strip()]
    assert len(ids) == len(set(ids)), f"duplicate ids in {label}: {ids}"
    return set(ids)

current_package_ids = parse_report_ids("Current package/provenance failed testcase ids")
current_source_regression_ids = parse_report_ids("Current source/regression failed testcase ids")
current_usage_ids = parse_report_ids("Current usage/runtime failed testcase ids")
parse_report_ids("Package/provenance baseline testcase ids")
parse_report_ids("Source/regression baseline testcase ids")
parse_report_ids("Usage/runtime baseline testcase ids")
waived = parse_report_ids("Waived testcase ids")
failed = []
payloads = {}
for path in sorted((root / "validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    payload = json.loads(path.read_text())
    testcase_id = payload.get("testcase_id")
    payloads[testcase_id] = payload
    assert payload.get("override_debs_installed") is True, path
    assert payload.get("port_commit") == safe_commit, path
    if payload.get("status") == "failed":
        failed.append(testcase_id)
failed_set = set(failed)
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
    not_failed = sorted(ids - failed_set)
    assert not not_failed, f"{bucket_name} ids are not failed in current results: {not_failed}"
    for testcase_id in ids:
        seen_once.setdefault(testcase_id, []).append(bucket_name)
duplicates = {testcase_id: names for testcase_id, names in seen_once.items() if len(names) != 1}
assert not duplicates, f"current failed ids appear in multiple buckets: {duplicates}"
classified = set().union(*current_buckets.values()) if current_buckets else set()
assert classified == failed_set, {
    "missing_from_current_report": sorted(failed_set - classified),
    "stale_in_current_report": sorted(classified - failed_set),
}
for testcase_id in sorted(current_source_regression_ids):
    assert payloads[testcase_id].get("kind") in {"source", "regression"}, (testcase_id, payloads[testcase_id].get("kind"))
for testcase_id in sorted(current_usage_ids):
    assert payloads[testcase_id].get("kind") == "usage", (testcase_id, payloads[testcase_id].get("kind"))
unexpected = sorted(set(failed) - waived)
assert not unexpected, unexpected
unused_waivers = sorted(waived - failed_set)
assert not unused_waivers, f"waived testcase ids are no longer failing: {unused_waivers}"
assert summary["failed"] == len(failed), (summary, failed)
if not waived:
    assert summary["failed"] == 0, summary
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

assert proof_status == 0, f"proof verification failed with status {proof_status}"
proof = json.loads((root / "validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
lib_proof = proof["libraries"][0]
assert proof["mode"] == "port", proof
assert lib_proof["library"] == "libtiff", lib_proof
assert lib_proof["port_commit"] == safe_commit, (lib_proof.get("port_commit"), safe_commit)
for field in ("cases", "source_cases", "usage_cases", "regression_cases", "passed", "failed", "casts"):
    assert proof["totals"][field] == summary[field], (field, proof["totals"], summary)
    assert lib_proof["totals"][field] == summary[field], (field, lib_proof["totals"], summary)
PY
```

## Success Criteria

- Full local safe matrix, package checks, link compatibility, and original comparison pass.
- Final lock, per-case result JSON, casts, and proof all refer to the final committed safe tree and selected validator checkout.
- `validator-report.md` names final validator and safe commits, preserves immutable baseline lines, updates final current-failure and waiver lines, and documents every remaining waived id with evidence.
- Zero unexpected failures remain. If `Waived testcase ids:` is empty, `summary["failed"] == 0`; if non-empty, every remaining failed testcase id is in that exact waiver set.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. Before yielding, run `git status --short`. Stage and commit any final `safe/` source/test changes before the final package rebuild, lock generation, validator execution, and report rewrite. After the final validator run, stage only `validator-report.md` and commit it if it changed. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If the final phase only confirms existing artifacts and produces no tracked changes, create an empty commit naming `impl_final_validator_clean_run`.
