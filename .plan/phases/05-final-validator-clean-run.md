# 05-final-validator-clean-run

## Phase Name

Final Validator Clean Run

## Implement Phase ID

`impl_final_validator_clean_run`

## Preexisting Inputs

- All outputs from phases 1-4.
- `validator/.git` selected in phase 1 and not moved since. Expected selected commit for this plan is `87b321fe728340d6fc6dd2f638583cca82c667c3`, where libtiff has at least 5 source cases, 170 usage cases, and 175 total cases.
- Final safe tree after fixes.
- `validator-report.md` after phases 1-4, including historical prior clean-run context at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`, the current validator commit, current safe source commit, checks executed, failure buckets, fixes applied, and `Waived testcase ids:`.
- Current validator artifacts:
  - `validator/artifacts/debs/local/libtiff/*.deb`
  - `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
  - `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
  - `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
  - `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
  - `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Existing package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/test.c`, `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`, and `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`.
- Safe test and verification harnesses:
  - `safe/scripts/check-public-surface.py`
  - `safe/scripts/run-upstream-shell-tests.sh`
  - `safe/scripts/build-link-compat-objects.sh`
  - `safe/scripts/link-and-run-link-compat.sh`
  - `safe/scripts/build-deb.sh`
  - `safe/scripts/check-packaged-install-surface.sh`
  - `safe/test/images/`
  - `safe/test/refs/`
  - `original/test/images/`
  - `test-original.sh`
- Safe port facts to preserve in the closure report and review: `safe/Cargo.toml` builds crate `safe-libtiff` as static library `tiff_safe_core`; `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config metadata, CMake package metadata, copied upstream tools, and optional tests; `safe/debian/control` packages `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`; `safe/scripts/build-deb.sh` builds version `1:4.5.1+git230720-4ubuntu2.5+safelibs1` packages into `safe/dist/`.

Consume these artifacts in place. Do not refetch, recollect, rediscover, regenerate prepared artifacts, move the validator checkout, or edit validator tests/shared scripts/manifests/runner code/tools. Do not use `make fetch-port-debs`.

## New Outputs

- Final rebuilt `safe/dist/*.deb`.
- Final `validator/artifacts/debs/local/libtiff/*.deb`.
- Final `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Final `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`.
- Final `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`.
- Final `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`.
- Final `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Closure-grade `validator-report.md` summarizing validator commit, safe commit, exact checks executed, counts derived from checked-out testcase files, failures found, fixes applied, waiver disposition, artifact paths, and final status.
- Final git commit before yielding.

## File Changes

- Always `validator-report.md`.
- Conditional final fixes in `safe/` if failures remain.
- No validator runtime file edits.

Critical file guidance for this phase:

- The source plan artifact is not edited by this phase.
- `validator-report.md` is the durable cross-phase report and must contain closure-grade provenance, commands, failures, fixes, waiver state, artifact paths, and final status.
- `safe/Cargo.toml` changes only if a new Rust module or dependency is required; avoid dependency additions unless a failure cannot be fixed with existing libraries.
- `safe/CMakeLists.txt` must reflect any new Rust sources, build/test options, install rules, or tool/test wiring required by implementation changes.
- Exact safe implementation hotspots remain the primary source-level anchors for final failures:
  - `safe/src/lib.rs`: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` around line 1511.
  - `safe/src/core/directory.rs`: `read_next_directory` at line 1384 and `TIFFWriteDirectory` at line 4357.
  - `safe/src/core/field_tables.rs` and `safe/src/core/field_registry.rs`: tag definitions, custom fields, defaulted field behavior, and field lookup.
  - `safe/src/strile.rs`: codec decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: `safe_tiff_codec_decode_bytes` at line 3019 and `safe_tiff_codec_encode_bytes` at line 3070.
  - `safe/src/core/jpeg.rs`: `jpeg_decode_bytes` at line 755 and `jpeg_encode_bytes` at line 803.
  - `safe/src/core/color.rs` and `safe/src/rgba.rs`: color conversion, pixel/raster behavior, Pillow-facing RGBA read paths, and orientation handling.
- `safe/tools/*.c` may be touched for CLI compatibility regressions, but prefer library fixes first.
- `safe/include/tiffio.h`, `safe/include/tiffio.hxx`, `safe/capi/libtiff-safe.map`, and `safe/capi/libtiffxx-safe.map` should change only for public API or ABI export corrections.
- `safe/debian/control`, `safe/debian/*.install`, `safe/debian/rules`, `safe/cmake/TiffConfig.cmake.in`, and `safe/pkgconfig/libtiff-4.pc.in` are package/install-surface fix locations.
- `scripts/lib/build_port_lock.py`, `scripts/run-validation-tests.sh`, `scripts/build-debs.sh`, `safe/scripts/build-deb.sh`, and `safe/scripts/check-packaged-install-surface.sh` should be consumed as existing harnesses unless a true local harness bug blocks validation.
- `validator/artifacts/debs/local/libtiff/*` and `validator/artifacts/libtiff-safe/**/*` are generated external validation artifacts and primary evidence for check phases. They are not committed to the parent repo.
- `validator/tests/libtiff/**/*`, `validator/tests/_shared/**/*`, `validator/repositories.yml`, `validator/test.sh`, and `validator/tools/**/*` must not be modified to pass tests.

## Implementation Details

Workflow-generation contract preserved for this implement block:

- Execute phases linearly. Do not generate `parallel_groups`.
- Preserve the source-plan generation boundary for downstream workflow generation: generate only `.plan/plan.md`; do not generate or edit `.plan/phases/*`, `.plan/workflow-structure.yaml`, or `workflow.yaml` from inside phase-level prompts because workflow-generation stages own those files.
- Use self-contained inline-only YAML. Do not use a top-level `include`.
- Do not use phase-level `prompt_file`, `workflow_file`, `workflow_dir`, `checks`, `source`, or any other YAML-source indirection.
- Do not generate `bounce_targets` lists. Each verifier has exactly one fixed `bounce_target`.
- Every verifier is an explicit top-level `check` phase, stays inside the implement block it verifies, and bounces only to `impl_final_validator_clean_run`.
- Put verifier commands directly in the checker instructions; do not model tests, builds, proof generation, artifact parsing, or review commands as non-agentic phases.
- Consume all phase outputs and existing workspace artifacts in place. Do not refetch, recollect, rediscover, or regenerate prepared artifacts from scratch.
- Do not refetch or move `validator/`; phase 1 is the only phase that may fetch or checkout the validator repository.

1. Re-run the full local safe compatibility matrix before final validator.

```bash
test -d validator/.git
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
VALIDATOR_COMMIT="$(git -C validator rev-parse HEAD)"
SAFE_COMMIT="$(git log -1 --format=%H -- safe)"

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

2. Run the validator-side checks from the final verification contract. Later phases must consume the validator commit selected by phase 1 and must not refetch or move the checkout.

```bash
make -C validator unit
make -C validator check-testcases
python3 validator/tools/testcases.py \
  --config validator/repositories.yml \
  --tests-root validator/tests \
  --library libtiff \
  --check \
  --min-source-cases 5 \
  --min-usage-cases 170 \
  --min-cases 175
```

3. Refresh the local lock/override and rerun the validator matrix and proof exactly as in phase 1, using the final safe commit and current validator commit.

```bash
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
  bash test.sh \
    --config repositories.yml \
    --tests-root tests \
    --artifact-root artifacts/libtiff-safe \
    --mode port \
    --override-deb-root artifacts/debs/local \
    --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json \
    --library libtiff \
    --record-casts

  python3 tools/verify_proof_artifacts.py \
    --config repositories.yml \
    --tests-root tests \
    --artifact-root artifacts/libtiff-safe \
    --proof-output proof/libtiff-safe-port-proof.json \
    --mode port \
    --library libtiff \
    --require-casts \
    --min-source-cases 5 \
    --min-usage-cases 170 \
    --min-cases 175 \
    --ports-root /home/yans/safelibs/pipeline/ports
)
```

4. Parse final artifacts, enforcing zero unexpected failures. If `Waived testcase ids:` is empty, require `summary["failed"] == 0`. If it is non-empty, every remaining failed testcase must be in that exact waived set. Proof totals must match summary counts, and casts must equal total cases.

```bash
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
report = Path("validator-report.md").read_text()
waived_line = re.search(r"^Waived testcase ids:\s*(.*)$", report, re.MULTILINE)
waived = {x.strip() for x in (waived_line.group(1) if waived_line else "").split(",") if x.strip()}
expected_source = len(list(Path("validator/tests/libtiff/tests/cases/source").glob("*.sh")))
expected_usage = len(list(Path("validator/tests/libtiff/tests/cases/usage").glob("*.sh")))
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
validator_commit = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
assert lock["libraries"][0]["commit"] == safe_commit, (lock["libraries"][0]["commit"], safe_commit)
assert proof["libraries"][0]["port_commit"] == safe_commit, (proof["libraries"][0], safe_commit)
assert f"Validator commit: {validator_commit}" in report
assert f"Safe source commit tested: {safe_commit}" in report
failed = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    result = json.loads(path.read_text())
    assert result.get("override_debs_installed") is True, result["testcase_id"]
    assert result.get("port_commit") == safe_commit, (result["testcase_id"], result.get("port_commit"), safe_commit)
    if result.get("status") == "failed":
        failed.append(result["testcase_id"])
assert expected_source >= 5, expected_source
assert expected_usage >= 170, expected_usage
assert summary["source_cases"] == expected_source, (summary, expected_source)
assert summary["usage_cases"] == expected_usage, (summary, expected_usage)
assert summary["cases"] == expected_source + expected_usage, summary
assert proof["totals"]["cases"] == summary["cases"], proof["totals"]
assert proof["totals"]["casts"] == summary["cases"], proof["totals"]
assert set(failed).issubset(waived), (failed, waived)
if not waived:
    assert not failed, failed
    assert summary["failed"] == 0, summary
PY
```

5. If unexpected failures remain, fix them in `safe/` and repeat this phase. If a validator bug remains, ensure the waiver has original-mode evidence and is listed on the machine-readable waiver line.

6. Rewrite `validator-report.md` as the closure report. It must state the final validator commit selected in phase 1, final safe source commit, exact checks executed, final counts derived from checked-out libtiff testcase files, failure list, fixes applied, waiver list, artifact paths, and whether the final status is clean.

7. Commit before yielding.

```bash
git add validator-report.md safe
git commit -m "impl_final_validator_clean_run: close libtiff validator run"
```

## Verification Phases

### `check_final_validator_tester`

- Phase ID: `check_final_validator_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_final_validator_clean_run`
- Purpose: Enforce a clean final validator run or exactly documented waivers, with package/proof/cast provenance matching the final safe commit.
- Commands:

```bash
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
report = Path("validator-report.md").read_text()
waived_line = re.search(r"^Waived testcase ids:\s*(.*)$", report, re.MULTILINE)
waived = {x.strip() for x in (waived_line.group(1) if waived_line else "").split(",") if x.strip()}

expected_source = len(list(Path("validator/tests/libtiff/tests/cases/source").glob("*.sh")))
expected_usage = len(list(Path("validator/tests/libtiff/tests/cases/usage").glob("*.sh")))
assert expected_source >= 5, expected_source
assert expected_usage >= 170, expected_usage
assert summary["source_cases"] == expected_source, (summary, expected_source)
assert summary["usage_cases"] == expected_usage, (summary, expected_usage)
assert summary["cases"] == expected_source + expected_usage, summary
assert proof["totals"]["cases"] == summary["cases"], (proof["totals"], summary)
assert proof["totals"]["casts"] == summary["cases"], proof["totals"]
assert proof["libraries"][0]["port_commit"] == lock["libraries"][0]["commit"], (proof["libraries"][0], lock["libraries"][0])

failed = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    r = json.loads(path.read_text())
    if r.get("status") == "failed":
        failed.append(r["testcase_id"])
    assert r.get("override_debs_installed") is True, r["testcase_id"]
    assert r.get("port_commit") == lock["libraries"][0]["commit"], r["testcase_id"]
assert set(failed).issubset(waived), (failed, waived)
if not waived:
    assert not failed, failed
    assert summary["failed"] == 0, summary

head = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
safe_head = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
assert f"Validator commit: {head}" in report
assert f"Safe source commit tested: {safe_head}" in report
PY
```

### `check_final_validator_senior`

- Phase ID: `check_final_validator_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_final_validator_clean_run`
- Purpose: Final architectural review of scope, provenance, test coverage, compatibility risk, and report completeness.
- Commands:

```bash
git status --short
git show --stat --format=fuller HEAD
git -C validator status --short
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
rg -n "Final status|Validator commit:|Safe source commit tested:|Checks executed:|Failures found:|Waived testcase ids:|135|175|source|usage|casts|proof|package|fixes applied" validator-report.md
```

## Success Criteria

- Full safe compatibility matrix passes.
- Validator unit, testcase manifest, and case-count checks pass against the phase-1-selected validator checkout.
- Final validator artifacts match the final safe commit and validator commit.
- If there are no waivers, `summary["failed"] == 0`; if waivers exist, every remaining failed testcase is exactly listed on `Waived testcase ids:`.
- Proof totals match summary counts and casts equal total cases.
- Closure report states validator commit, safe source commit, checks executed, final counts, failures, fixes, waivers, artifact paths, and final status.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If the phase only updates the report or has no code change, commit the report change or create an empty phase commit naming `impl_final_validator_clean_run`.
