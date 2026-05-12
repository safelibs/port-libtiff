# Package Provenance And Install Surface

## Phase Name

Package Provenance And Install Surface

## Implement Phase ID

`impl_package_provenance_fixes`

## Shared Context And Critical Files

- Treat `.plan/plan.md` as the only authoritative implementation plan. Existing generated `.plan/phases/*.md`, `.plan/workflow-structure.yaml`, and `workflow.yaml` are not implementation inputs; do not read, splice, preserve, stage, or modify `workflow.yaml`.
- Consume the validator checkout selected by Phase 1. Do not refetch, move, delete, or reset `validator/`; do not edit `validator/tests/libtiff/**`, `validator/tests/_shared/**`, `validator/repositories.yml`, `validator/test.sh`, or `validator/tools/**`.
- Preserve existing artifacts in place: `original/`, `safe/`, `safe/test/`, `safe/test/images/`, `safe/test/refs/`, `safe/abi/*`, `safe/capi/*.map`, package scripts, link-compatibility harnesses, CVE/dependent inventories, package-smoke projects, and existing validator artifacts. Refresh package, lock, matrix, proof, and report artifacts only when this phase instructs a rerun for the committed safe tree.
- The selected validator checkout must support source, usage, and regression testcase kinds and must satisfy floors of at least 5 source, 240 usage, 10 regression, and 255 total libtiff cases. Do not relax thresholds if metadata checks fail.
- `safe/debian/control` must continue to provide the canonical package set `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`. `safe/scripts/build-deb.sh` emits `safe/dist/*.deb`; `scripts/lib/build_port_lock.py` emits the local lock and `validator/artifacts/debs/local/libtiff/*.deb`.
- Before rebuilding packages, regenerating `local-port-debs-lock.json`, running the validator, or accepting existing package/validator artifacts, fail if `git status --porcelain -- safe` is non-empty. The local lock commit must equal `git log -1 --format=%H -- safe`.
- `validator-report.md` carries immutable Phase 1 baseline bucket lines and mutable current-failure/waiver lines. This phase may update package/provenance analysis and current lines, but must not rewrite the baseline bucket lines.
- Critical implementation files for this phase are `safe/debian/control`, `safe/debian/rules`, `safe/debian/*.install`, `safe/debian/*.symbols`, `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`, and `safe/abi/platform-excluded-linux.txt`.
- Critical harnesses are `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, `safe/scripts/check-public-surface.py`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/run-upstream-shell-tests.sh`, `scripts/lib/build_port_lock.py`, and the package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/`.
- Broader safe surfaces remain in scope only when packaging exposes a real ABI/API issue: `safe/include/*.h`, `safe/src/lib.rs`, `safe/src/core/directory.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/rgba.rs`, `safe/src/core/field_tables.rs`, `safe/src/core/field_registry.rs`, and `safe/capi/tiff_placeholder.c`.
- Hotspot line references from the authoritative plan must be preserved if package/install failures expose ABI or runtime behavior:
  - `safe/src/lib.rs`: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` at line 1583.
  - `safe/src/core/directory.rs`: `read_next_directory` starts at line 1384, and `TIFFWriteDirectory` is at line 4357.
  - `safe/src/strile.rs`: decode/use hotspots around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: `safe_tiff_codec_decode_bytes` at line 3025 and `safe_tiff_codec_encode_bytes` at line 3076.
  - `safe/src/core/jpeg.rs`: `jpeg_decode_bytes` at line 820 and `jpeg_encode_bytes` at line 868.
  - `safe/src/rgba.rs`: RGBA read paths, color conversion, Pillow-facing behavior, and orientation handling.

## Preexisting Inputs

- Validator commit selected by Phase 1.
- Baseline report and artifacts from Phase 1.
- `safe/debian/control`, `safe/debian/rules`, `safe/debian/*.install`, `safe/debian/*.symbols`.
- `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`.
- `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/*`.
- `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`.
- Package-smoke projects verified or created by Phase 1 under `validator/artifacts/libtiff-safe/package-smoke/`.
- Existing generated package, lock, matrix, proof, and report artifacts from Phase 1; consume them as baseline evidence and refresh only as required by this phase.

## New Outputs

- Packaging or install-surface fixes in `safe/`.
- Regression smoke coverage when a package-surface failure is reproducible locally.
- Rebuilt `safe/dist/*.deb`.
- Refreshed local lock and override `.deb` artifacts.
- Current validator rerun artifacts under `validator/artifacts/libtiff-safe/` when Phase 1 recorded package/provenance baseline ids.
- Updated `validator-report.md` package/provenance section.
- Git commit for `impl_package_provenance_fixes`.

## File Changes

- Possible: `safe/debian/control`, `safe/debian/rules`, `safe/debian/*.install`, `safe/debian/*.symbols`.
- Possible: `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`.
- Possible: `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`.
- Possible: `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh` only for true local harness defects.
- Always: `validator-report.md`.
- Do not edit validator runtime files or `workflow.yaml`.

## Implementation Details

- Compare validator canonical packages from `validator/repositories.yml` with `dpkg-deb -f safe/dist/*.deb Package`; all four canonical packages must be present: `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`.
- If a package install fails inside validator, reproduce it with `safe/scripts/check-packaged-install-surface.sh` before changing implementation code.
- If a header, symbol, pkg-config, or CMake target is missing, fix the safe install surface rather than validator test setup.
- For public ABI changes, update version scripts and `safe/abi/*` together and rerun `safe/scripts/check-public-surface.py --check`.
- Keep private build-only headers under `safe/libtiff/`; do not install private headers unless the upstream package installs them.
- Commit safe packaging or install-surface changes before rebuilding packages, regenerating the lock, or rerunning the validator. After a validator rerun, commit only `validator-report.md` if the report changed.
- If `Package/provenance baseline testcase ids:` is non-empty in `validator-report.md`, run the full validator port matrix after rebuilding packages and update the package/provenance section with current per-case package fields: `override_debs_installed`, `port_commit`, `port_debs`, and `unported_original_packages`.
- Leave all three `* baseline testcase ids:` lines unchanged. Update `Current package/provenance failed testcase ids:`, `Current source/regression failed testcase ids:`, `Current usage/runtime failed testcase ids:`, and `Waived testcase ids:` from the latest validator result JSON so those four current lines partition current failed testcase ids exactly once.
- Before yielding, run `git status --short`. Stage and commit any `safe/` packaging/install-surface changes before the final package and validator rerun for this phase. After that rerun, stage only `validator-report.md` for the report update and commit it with a message naming `impl_package_provenance_fixes`. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If a later edit touches `safe/` again, repeat the package build, lock generation, validator run, proof generation, and report update against the new committed safe tree. If there are no tracked file changes because the baseline had no package/provenance failures, create an empty commit naming `impl_package_provenance_fixes`.
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

### `check_package_provenance_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_package_provenance_fixes`
- Purpose: Verify package fixes, local `.deb` metadata, C/C++/pkg-config/CMake install surfaces, and the current safe commit in the local lock.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project-no-target validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
python3 - <<'PY'
import subprocess
from pathlib import Path

expected = {"libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"}
seen = {}
for deb in Path("safe/dist").glob("*.deb"):
    package = subprocess.check_output(["dpkg-deb", "-f", str(deb), "Package"], text=True).strip()
    version = subprocess.check_output(["dpkg-deb", "-f", str(deb), "Version"], text=True).strip()
    arch = subprocess.check_output(["dpkg-deb", "-f", str(deb), "Architecture"], text=True).strip()
    if package in expected:
        seen[package] = (deb.name, version, arch)
assert set(seen) == expected, seen
for package, (_, version, arch) in seen.items():
    assert "+safelibs" in version, (package, version)
    assert arch in {"amd64", "all"}, (package, arch)
print(seen)
PY
```

### `check_package_provenance_senior`

- Type: `check`
- Fixed `bounce_target`: `impl_package_provenance_fixes`
- Purpose: Review packaging minimality and provenance correctness. Confirm no package failure is masked by altering validator code or omitting canonical packages.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
python3 safe/scripts/check-public-surface.py \
  --check \
  --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen \
  --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt
SAFE_COMMIT="$(git log -1 --format=%H -- safe)"
rm -rf validator/artifacts/debs/local/libtiff
SAFELIBS_LIBRARY=libtiff \
SAFELIBS_COMMIT_SHA="$SAFE_COMMIT" \
SAFELIBS_DIST_DIR="$PWD/safe/dist" \
SAFELIBS_VALIDATOR_DIR="$PWD/validator" \
SAFELIBS_LOCK_PATH="$PWD/validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json" \
SAFELIBS_OVERRIDE_ROOT="$PWD/validator/artifacts/debs/local" \
python3 scripts/lib/build_port_lock.py
python3 - <<'PY'
import json
import subprocess
from pathlib import Path

lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
lib = lock["libraries"][0]
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
assert lib["commit"] == safe_commit, (lib["commit"], safe_commit)
assert lib["unported_original_packages"] == [], lib
assert [d["package"] for d in lib["debs"]] == ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"], lib
PY
python3 - <<'PY' > /tmp/libtiff-package-provenance-baseline-ids.txt
import re
from pathlib import Path

report = Path("validator-report.md").read_text()
match = re.search(r"^Package/provenance baseline testcase ids:\s*(.*)$", report, re.M)
assert match, "missing package/provenance baseline id line"
ids = [item.strip() for item in match.group(1).split(",") if item.strip()]
print("\n".join(ids))
PY
if [ -s /tmp/libtiff-package-provenance-baseline-ids.txt ]; then
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
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
validator_commit = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
baseline_ids = {
    line.strip()
    for line in Path("/tmp/libtiff-package-provenance-baseline-ids.txt").read_text().splitlines()
    if line.strip()
}
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
waived = parse_report_ids("Waived testcase ids")
summary = json.loads((root / "validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
assert summary["cases"] >= 255, summary
assert summary["passed"] + summary["failed"] == summary["cases"], summary
proof_status = int((root / "validator/artifacts/libtiff-safe/proof/proof-status.txt").read_text().strip())
assert proof_status == 0, f"proof verification failed with status {proof_status}"
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
assert summary["casts"] == summary["cases"], summary
assert proof["mode"] == "port", proof
assert lib_proof["library"] == "libtiff", lib_proof
assert lib_proof["port_commit"] == safe_commit, (lib_proof.get("port_commit"), safe_commit)
for field in ("cases", "source_cases", "usage_cases", "regression_cases", "passed", "failed", "casts"):
    assert proof["totals"][field] == summary[field], (field, proof["totals"], summary)
    assert lib_proof["totals"][field] == summary[field], (field, lib_proof["totals"], summary)
canonical = ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"]
seen_baseline_ids = set()
remaining_baseline_failures = set()
payloads = {}
for path in sorted((root / "validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    payload = json.loads(path.read_text())
    testcase_id = payload.get("testcase_id")
    payloads[testcase_id] = payload
    if testcase_id in baseline_ids:
        seen_baseline_ids.add(testcase_id)
        if payload.get("status") == "failed":
            remaining_baseline_failures.add(testcase_id)
    assert payload.get("override_debs_installed") is True, path
    assert payload.get("port_commit") == safe_commit, (path, payload.get("port_commit"), safe_commit)
    assert payload.get("unported_original_packages") == [], path
    assert [d["package"] for d in payload.get("port_debs", [])] == canonical, path
assert seen_baseline_ids == baseline_ids, {"missing": sorted(baseline_ids - seen_baseline_ids)}
failed_ids = {testcase_id for testcase_id, payload in payloads.items() if payload.get("status") == "failed"}
current_buckets = {
    "current package/provenance": current_package_ids,
    "current source/regression": current_source_regression_ids,
    "current usage/runtime": current_usage_ids,
    "waived": waived,
}
all_result_ids = set(payloads)
seen_once = {}
for bucket_name, ids in current_buckets.items():
    unknown = sorted(ids - all_result_ids)
    assert not unknown, f"{bucket_name} ids missing current per-case JSON: {unknown}"
    not_failed = sorted(ids - failed_ids)
    assert not not_failed, f"{bucket_name} ids are not failed in current results: {not_failed}"
    for testcase_id in ids:
        seen_once.setdefault(testcase_id, []).append(bucket_name)
duplicates = {testcase_id: names for testcase_id, names in seen_once.items() if len(names) != 1}
assert not duplicates, f"current failed ids appear in multiple buckets: {duplicates}"
classified = set().union(*current_buckets.values()) if current_buckets else set()
assert classified == failed_ids, {
    "missing_from_current_report": sorted(failed_ids - classified),
    "stale_in_current_report": sorted(classified - failed_ids),
}
for testcase_id in sorted(current_source_regression_ids):
    assert payloads[testcase_id].get("kind") in {"source", "regression"}, (testcase_id, payloads[testcase_id].get("kind"))
for testcase_id in sorted(current_usage_ids):
    assert payloads[testcase_id].get("kind") == "usage", (testcase_id, payloads[testcase_id].get("kind"))
unexpected = sorted(remaining_baseline_failures - waived)
assert not unexpected, {"remaining_package_provenance_failures": unexpected, "waived": sorted(waived)}
remaining_waived = sorted(remaining_baseline_failures & waived)
if remaining_waived:
    assert "Waiver evidence:" in report, "missing waiver evidence section"
    original_proof_status_path = root / "validator/artifacts/libtiff-original-waiver/proof/proof-status.txt"
    assert original_proof_status_path.is_file(), "missing original waiver proof status"
    assert int(original_proof_status_path.read_text().strip()) == 0, "original waiver proof verification failed"
for testcase_id in remaining_waived:
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
fi
```

## Success Criteria

- All canonical libtiff Debian packages are present, named correctly, versioned with `+safelibs`, and installed through the local package-surface smoke harness.
- The lock records the current committed safe tree and has no unported canonical packages.
- Package/provenance baseline failures are resolved or explicitly waived with evidence, and current failed ids are partitioned in `validator-report.md`.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. Before yielding, run `git status --short`. Stage and commit any `safe/` packaging, ABI, install-surface, or harness changes before package rebuild, lock generation, validator execution, and report rewrite. After the validator rerun, stage only `validator-report.md` and commit it if it changed. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If no tracked files changed because no package/provenance failures existed, create an empty commit naming `impl_package_provenance_fixes`.
