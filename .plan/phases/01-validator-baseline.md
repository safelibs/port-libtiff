# Validator Update And Baseline Matrix

## Phase Name

Validator Update And Baseline Matrix

## Implement Phase ID

`impl_validator_baseline`

## Shared Context And Critical Files

- Treat `.plan/plan.md` as the only authoritative implementation plan. Existing `.plan/phases/*.md`, `.plan/workflow-structure.yaml`, and `workflow.yaml` may be stale generated artifacts; do not read, splice, preserve, stage, or modify `workflow.yaml` during validation implementation.
- Validate the memory-safe Rust libtiff port in `safe/` against the safelibs validator. Consume `original/`, `safe/`, `safe/test/`, `safe/test/images/`, `safe/test/refs/`, `safe/abi/*`, `safe/capi/*.map`, package scripts, CVE inventories, dependent inventories, and existing validator artifacts in place.
- `safe/` builds crate `safe-libtiff` as staticlib `tiff_safe_core` via `safe/Cargo.toml`, then links C ABI shims and copied libtiff tools through `safe/CMakeLists.txt`. `safe/debian/control` defines the canonical validator packages: `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`.
- `safe/scripts/build-deb.sh` builds local Debian packages into `safe/dist/`; `scripts/lib/build_port_lock.py` converts them into `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` and `validator/artifacts/debs/local/libtiff/*.deb`.
- Phase 1 owns the only validator checkout update. If `validator/.git` is missing, clone `https://github.com/safelibs/validator`; otherwise fetch `origin main` and check out the fetched commit without deleting the checkout or resetting unrelated local work. Later phases must consume the validator commit selected here.
- The preexisting local validator checkout may be detached at `87b321fe728340d6fc6dd2f638583cca82c667c3` with only 5 source cases, 170 usage cases, and no regression kind. On 2026-05-12, remote `refs/heads/main` resolved to `bde8758883d12061dfb2621b6149949909c803f8`; the selected checkout must derive exact counts and enforce floors of at least 5 source, 240 usage, 10 regression, and 255 total libtiff cases.
- Before any package build, lock generation, validator run, proof generation, or acceptance of existing package/validator artifacts for safe source changes, `git status --porcelain -- safe` must be empty. The lock commit must equal `git log -1 --format=%H -- safe`.
- Validator runtime inputs are read-only: `validator/tests/libtiff/**`, `validator/tests/_shared/**`, `validator/repositories.yml`, `validator/test.sh`, and `validator/tools/**`. Allowed validator-side outputs are generated artifacts under `validator/artifacts/`.
- `validator-report.md` is the authoritative run report, failure ledger, waiver ledger, command log, commit record, count record, package checksum record, and final status. Preserve immutable Phase 1 baseline bucket lines after this phase; update current-failure and waiver lines from the latest result JSON.
- Critical safe implementation files include `safe/src/lib.rs` for lifecycle/open/close/allocation/mode/directory ABI entry points; `safe/src/core/directory.rs` for IFD parsing/writing, tag defaults, custom fields, SubIFDs, BigTIFF, and directory safety; `safe/src/strile.rs` for strip/tile geometry and I/O; `safe/src/core/codec.rs` for compression dispatch; `safe/src/core/jpeg.rs` for JPEG/OJPEG; `safe/src/core/color.rs` and `safe/src/rgba.rs` for color/RGBA behavior; `safe/src/core/field_tables.rs` and `safe/src/core/field_registry.rs` for tag metadata; and `safe/capi/tiff_placeholder.c` for varargs tag APIs, handlers, RGBA wrappers, `TIFFPrintDirectory`, and C glue.
- Critical public surface and packaging files include `safe/include/tiff.h`, `safe/include/tiffio.h`, `safe/include/tiffio.hxx`, `safe/include/tiffconf.h`, `safe/include/tif_config.h`, `safe/include/tiffvers.h`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`, `safe/abi/platform-excluded-linux.txt`, `safe/debian/control`, `safe/debian/rules`, `safe/debian/*.install`, `safe/debian/*.symbols`, `safe/pkgconfig/libtiff-4.pc.in`, and `safe/cmake/TiffConfig.cmake.in`.
- Critical harness files include `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, `safe/scripts/check-public-surface.py`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/run-upstream-shell-tests.sh`, `safe/test/CMakeLists.txt`, the existing regression buckets under `safe/test/`, copied upstream shell tests, `scripts/lib/build_port_lock.py`, and `scripts/run-validation-tests.sh`.
- Hotspot line references from the authoritative plan must be preserved when triaging failures:
  - `safe/src/lib.rs`: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` at line 1583.
  - `safe/src/core/directory.rs`: `read_next_directory` starts at line 1384, and `TIFFWriteDirectory` is at line 4357.
  - `safe/src/strile.rs`: decode/use hotspots around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: `safe_tiff_codec_decode_bytes` at line 3025 and `safe_tiff_codec_encode_bytes` at line 3076.
  - `safe/src/core/jpeg.rs`: `jpeg_decode_bytes` at line 820 and `jpeg_encode_bytes` at line 868.
  - `safe/src/rgba.rs`: RGBA read paths, color conversion, Pillow-facing behavior, and orientation handling.

## Preexisting Inputs

- `validator/.git` if present, otherwise the GitHub repository URL `https://github.com/safelibs/validator`.
- `validator-report.md` history.
- `original/`, `safe/`, `safe/test/`, `safe/test/images/`, `safe/test/refs/`.
- `safe/Cargo.toml`, `safe/Cargo.lock`, `safe/CMakeLists.txt`, `safe/debian/control`, `safe/debian/rules`.
- `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, `safe/scripts/check-public-surface.py`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/run-upstream-shell-tests.sh`.
- `scripts/lib/build_port_lock.py`, `scripts/build-debs.sh`, `scripts/run-validation-tests.sh`, `scripts/check-layout.sh`.
- `all_cves.json`, `relevant_cves.json`, `dependents.json`, `test-original.sh`.
- Existing `validator/artifacts/` contents, including package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/` when present.

## New Outputs

- Updated or newly cloned `validator/` checkout selected to the current fetched `main` commit.
- Verified package-smoke inputs under `validator/artifacts/libtiff-safe/package-smoke/`; if any of the three smoke files is missing, recreate only the missing file with the minimal existing `TIFFGetVersion` smoke source and CMake target/targetless projects.
- Rebuilt `safe/dist/*.deb`.
- `validator/artifacts/debs/local/libtiff/*.deb`.
- `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Baseline `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`, logs, casts, `validator/artifacts/libtiff-safe/port/matrix-status.txt`, `validator/artifacts/libtiff-safe/proof/proof-status.txt`, and `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- `validator-report.md` rewritten with validator commit, safe commit, dynamic counts, executed commands, baseline failures, failure buckets, machine-readable baseline id lines, current-failure id lines, and waiver line.
- Git commit for `impl_validator_baseline`.

## File Changes

- Always update `validator-report.md`.
- Do not change `safe/` in this phase unless a blocking harness issue prevents baseline execution.
- Do not edit `validator/tests/**`, `validator/test.sh`, `validator/repositories.yml`, or `validator/tools/**`.
- Do not modify `.plan/plan.md` or `workflow.yaml`.

## Implementation Details

1. Update the validator checkout without deleting it:

```bash
if [ ! -d validator/.git ]; then
  git clone https://github.com/safelibs/validator validator
fi
git -C validator fetch origin main
git -C validator checkout --detach FETCH_HEAD
VALIDATOR_COMMIT="$(git -C validator rev-parse HEAD)"
```

2. Validate validator metadata and enforce the current libtiff floors:

```bash
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
```

3. Verify package-smoke inputs under `validator/artifacts/libtiff-safe/package-smoke/`. If all three files already exist, leave them untouched. If any are missing, create only the missing file using the existing minimal smoke definitions: `test.c` calls `TIFFGetVersion()` and returns success for a non-empty string; `cmake-target/CMakeLists.txt` uses `find_package(TIFF REQUIRED CONFIG)` and links `TIFF::tiff`; `cmake-targetless/CMakeLists.txt` uses `TIFF_INCLUDE_DIRS` and `TIFF_LIBRARIES`.

4. Run local safe preflight:

```bash
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
```

5. If a blocking baseline-harness issue forced any `safe/` source, test, packaging, or script change during the preflight, commit that `safe/` change before computing `SAFE_COMMIT`, before building packages, before generating `local-port-debs-lock.json`, and before running the validator. Stage only intended safe files. Do not stage `safe/build`, `safe/dist`, generated validator artifacts, `.plan/plan.md`, or `workflow.yaml`.

6. Build packages and synthesize validator override artifacts:

```bash
test -z "$(git status --porcelain -- safe)"
SAFE_COMMIT="$(git log -1 --format=%H -- safe)"
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
rm -rf validator/artifacts/debs/local/libtiff
SAFELIBS_LIBRARY=libtiff \
SAFELIBS_COMMIT_SHA="$SAFE_COMMIT" \
SAFELIBS_DIST_DIR="$PWD/safe/dist" \
SAFELIBS_VALIDATOR_DIR="$PWD/validator" \
SAFELIBS_LOCK_PATH="$PWD/validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json" \
SAFELIBS_OVERRIDE_ROOT="$PWD/validator/artifacts/debs/local" \
python3 scripts/lib/build_port_lock.py
```

7. Run the full libtiff port matrix and proof, preserving artifacts even when the matrix exits non-zero:

```bash
(
  cd validator
  mkdir -p artifacts/libtiff-safe/port artifacts/libtiff-safe/proof
  rm -f \
    artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json \
    artifacts/libtiff-safe/proof/proof-status.txt
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
  printf '%s\n' "$matrix_status" > artifacts/libtiff-safe/port/matrix-status.txt
  if [ -f artifacts/libtiff-safe/port/results/libtiff/summary.json ]; then
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
  else
    proof_status=1
  fi
  printf '%s\n' "$proof_status" > artifacts/libtiff-safe/proof/proof-status.txt
  set -e
  if [ ! -f artifacts/libtiff-safe/port/results/libtiff/summary.json ]; then
    if [ "$matrix_status" -ne 0 ]; then
      exit "$matrix_status"
    fi
    exit 1
  fi
)
```

8. Update `validator-report.md` with validator commit, safe commit, source/usage/regression/total counts, commands executed, package names and SHA-256s, proof status/path, baseline failures, and machine-readable ledger lines:

- `Package/provenance baseline testcase ids:`
- `Source/regression baseline testcase ids:`
- `Usage/runtime baseline testcase ids:`
- `Current package/provenance failed testcase ids:`
- `Current source/regression failed testcase ids:`
- `Current usage/runtime failed testcase ids:`
- `Waived testcase ids:`

The three baseline lines must partition the Phase 1 failed per-case result ids exactly once and must remain immutable in later phases. The current-failure lines plus `Waived testcase ids:` must partition the latest failed per-case result ids exactly once.

9. Before yielding, run `git status --short` and confirm no uncommitted `safe/` changes remain. If any intended `safe/` change remains, stage only those intended `safe/` source, test, packaging, or script files and commit them before rebuilding packages, regenerating the lock, rerunning the validator, regenerating proof, and rewriting `validator-report.md` so all artifacts name the committed safe tree. Then stage only `validator-report.md` for the baseline report commit and commit it with a message naming `impl_validator_baseline`. Do not stage `safe/build`, `safe/dist`, generated validator artifacts, `.plan/plan.md`, or `workflow.yaml`. If there are truly no tracked file changes and no preliminary Phase 1 commit was created, create an empty commit naming `impl_validator_baseline`.

10. Use this waiver-evidence command only if adding or retaining a non-empty `Waived testcase ids:` line:

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

### `check_validator_baseline_tester`

- Type: `check`
- Fixed `bounce_target`: `impl_validator_baseline`
- Purpose: Verify the validator checkout, local safe `.deb` overrides, lock commit, libtiff port matrix, proof artifacts, and `validator-report.md` baseline failure classes. This checker records failures but does not require a clean validator result.
- Commands:

```bash
test -d validator/.git
test -f validator/README.md
test -z "$(git status --porcelain -- safe)"
git -C validator rev-parse HEAD
test -d validator/artifacts/debs/local/libtiff
test -f validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json
test -f validator/artifacts/libtiff-safe/port/matrix-status.txt
test -f validator/artifacts/libtiff-safe/port/results/libtiff/summary.json
test -f validator/artifacts/libtiff-safe/proof/proof-status.txt
test -f validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

root = Path(".")
summary = json.loads((root / "validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
proof_status = int((root / "validator/artifacts/libtiff-safe/proof/proof-status.txt").read_text().strip())
assert proof_status == 0, f"proof verification failed with status {proof_status}"
proof = json.loads((root / "validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
source = len(list((root / "validator/tests/libtiff/tests/cases/source").glob("*.sh")))
usage = len(list((root / "validator/tests/libtiff/tests/cases/usage").glob("*.sh")))
regression_dir = root / "validator/tests/libtiff/tests/cases/regression"
regression = len(list(regression_dir.glob("*.sh"))) if regression_dir.is_dir() else 0
assert source >= 5, source
assert usage >= 240, usage
assert regression >= 10, regression
assert summary["library"] == "libtiff", summary
assert summary["mode"] == "port", summary
assert summary["source_cases"] == source, (summary, source)
assert summary["usage_cases"] == usage, (summary, usage)
assert summary["regression_cases"] == regression, (summary, regression)
assert summary["cases"] == source + usage + regression, summary
assert summary["cases"] >= 255, summary
assert summary["passed"] + summary["failed"] == summary["cases"], summary
assert summary["casts"] == summary["cases"], summary

lock = json.loads((root / "validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
lib = lock["libraries"][0]
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
validator_commit = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
assert lib["library"] == "libtiff", lib
assert lib["commit"] == safe_commit, (lib["commit"], safe_commit)
assert lib["unported_original_packages"] == [], lib
assert [d["package"] for d in lib["debs"]] == ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"], lib
for deb in lib["debs"]:
    path = root / "validator/artifacts/debs/local/libtiff" / deb["filename"]
    assert path.is_file(), path
    assert path.stat().st_size == deb["size"], path

lib_proof = proof["libraries"][0]
assert proof["mode"] == "port", proof
assert lib_proof["library"] == "libtiff", lib_proof
assert lib_proof["port_commit"] == safe_commit, (lib_proof.get("port_commit"), safe_commit)
for field in ("cases", "source_cases", "usage_cases", "regression_cases", "passed", "failed", "casts"):
    assert proof["totals"][field] == summary[field], (field, proof["totals"], summary)
    assert lib_proof["totals"][field] == summary[field], (field, lib_proof["totals"], summary)

report = (root / "validator-report.md").read_text()
assert f"Validator commit: {validator_commit}" in report
assert f"Safe source commit tested: {safe_commit}" in report
assert "Waived testcase ids:" in report
assert "Package/provenance baseline testcase ids:" in report
assert "Source/regression baseline testcase ids:" in report
assert "Usage/runtime baseline testcase ids:" in report
assert "Current package/provenance failed testcase ids:" in report
assert "Current source/regression failed testcase ids:" in report
assert "Current usage/runtime failed testcase ids:" in report
assert re.search(r"Failures found:", report), report
PY
```

### `check_validator_baseline_senior`

- Type: `check`
- Fixed `bounce_target`: `impl_validator_baseline`
- Purpose: Review artifact discipline and failure classification. Confirm validator runtime files were not modified to make checks pass and every failure is assigned to a later phase.
- Commands:

```bash
test -z "$(git status --porcelain -- safe)"
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

root = Path(".")
summary = json.loads((root / "validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
assert summary["cases"] >= 255, summary
assert summary["passed"] + summary["failed"] == summary["cases"], summary
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
validator_commit = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()

proof_status_path = root / "validator/artifacts/libtiff-safe/proof/proof-status.txt"
assert proof_status_path.is_file(), "missing proof verification status"
proof_status = int(proof_status_path.read_text().strip())
assert proof_status == 0, f"proof verification failed with status {proof_status}"
proof = json.loads((root / "validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
lib_proof = proof["libraries"][0]
assert proof["mode"] == "port", proof
assert lib_proof["library"] == "libtiff", lib_proof
assert lib_proof["port_commit"] == safe_commit, (lib_proof.get("port_commit"), safe_commit)
for field in ("cases", "source_cases", "usage_cases", "regression_cases", "passed", "failed", "casts"):
    assert proof["totals"][field] == summary[field], (field, proof["totals"], summary)
    assert lib_proof["totals"][field] == summary[field], (field, lib_proof["totals"], summary)
assert summary["casts"] == summary["cases"], summary

result_dir = root / "validator/artifacts/libtiff-safe/port/results/libtiff"
failed = []
payloads = {}
for path in sorted(result_dir.glob("*.json")):
    if path.name == "summary.json":
        continue
    payload = json.loads(path.read_text())
    payloads[payload.get("testcase_id")] = payload
    assert payload.get("override_debs_installed") is True, path
    assert payload.get("port_commit") == safe_commit, (path, payload.get("port_commit"), safe_commit)
    if payload.get("status") == "failed":
        failed.append((payload.get("testcase_id"), payload.get("kind"), payload.get("error")))
print("failed cases:", failed)
report = (root / "validator-report.md").read_text()
assert f"Validator commit: {validator_commit}" in report
assert f"Safe source commit tested: {safe_commit}" in report
for phrase in ("package/provenance", "source/regression", "usage/runtime", "validator-bug candidates"):
    assert phrase in report, f"report must classify baseline failures by {phrase}"

def parse_ids(label):
    match = re.search(rf"^{re.escape(label)}:\s*(.*)$", report, re.M)
    assert match, f"missing {label} line"
    ids = [item.strip() for item in match.group(1).split(",") if item.strip()]
    assert len(ids) == len(set(ids)), f"duplicate ids in {label}: {ids}"
    return set(ids)

package_ids = parse_ids("Package/provenance baseline testcase ids")
source_regression_ids = parse_ids("Source/regression baseline testcase ids")
usage_ids = parse_ids("Usage/runtime baseline testcase ids")
current_package_ids = parse_ids("Current package/provenance failed testcase ids")
current_source_regression_ids = parse_ids("Current source/regression failed testcase ids")
current_usage_ids = parse_ids("Current usage/runtime failed testcase ids")
waived = parse_ids("Waived testcase ids")
failed_ids = {testcase_id for testcase_id, _, _ in failed}
all_result_ids = set(payloads)
baseline_buckets = {
    "package/provenance baseline": package_ids,
    "source/regression baseline": source_regression_ids,
    "usage/runtime baseline": usage_ids,
}
current_buckets = {
    "current package/provenance": current_package_ids,
    "current source/regression": current_source_regression_ids,
    "current usage/runtime": current_usage_ids,
    "waived": waived,
}

for bucket_name, ids in {**baseline_buckets, **current_buckets}.items():
    unknown = sorted(ids - all_result_ids)
    assert not unknown, f"{bucket_name} ids missing current per-case JSON: {unknown}"
    not_failed = sorted(ids - failed_ids)
    assert not not_failed, f"{bucket_name} ids are not failed in baseline: {not_failed}"

def assert_partition(name, buckets):
    seen_once = {}
    for bucket_name, ids in buckets.items():
        for testcase_id in ids:
            seen_once.setdefault(testcase_id, []).append(bucket_name)
    duplicates = {testcase_id: names for testcase_id, names in seen_once.items() if len(names) != 1}
    assert not duplicates, f"{name} ids appear in multiple buckets: {duplicates}"
    classified = set().union(*buckets.values()) if buckets else set()
    assert classified == failed_ids, {
        "partition": name,
        "missing_from_report": sorted(failed_ids - classified),
        "stale_in_report": sorted(classified - failed_ids),
    }

assert_partition("baseline", baseline_buckets)
assert_partition("current", current_buckets)
for testcase_id in sorted(source_regression_ids | current_source_regression_ids):
    assert payloads[testcase_id].get("kind") in {"source", "regression"}, (testcase_id, payloads[testcase_id].get("kind"))
for testcase_id in sorted(usage_ids | current_usage_ids):
    assert payloads[testcase_id].get("kind") == "usage", (testcase_id, payloads[testcase_id].get("kind"))
if package_ids or current_package_ids:
    package_section = report.lower()
    for required_word in ("package", "override", "lock"):
        assert required_word in package_section, f"package/provenance report lacks {required_word} explanation"
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

- Validator checkout is updated or cloned in place, selected commit is recorded, and libtiff testcase floors are enforced at 5 source, 240 usage, 10 regression, and 255 total.
- Local safe packages, override `.deb` layout, lock file, matrix artifacts, casts, and proof artifacts exist and refer to `git log -1 --format=%H -- safe`.
- `validator-report.md` records the authoritative baseline, immutable baseline buckets, mutable current-failure buckets, waiver line, proof paths, and all commands executed.
- Validator runtime inputs under `validator/tests/**`, `validator/tests/_shared/**`, `validator/repositories.yml`, `validator/test.sh`, and `validator/tools/**` are unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. Before yielding, run `git status --short`. If `safe/` changed, stage only the intended `safe/` source, test, packaging, or script changes and commit them before package rebuild, lock generation, validator execution, and report rewrite. Commit `validator-report.md` separately after the validator run if it changed. Do not stage generated validator artifacts, `safe/build`, `safe/dist`, `.plan/plan.md`, or `workflow.yaml`. If no tracked files changed, create an empty commit naming `impl_validator_baseline`.
