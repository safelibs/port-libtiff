# 01-validator-baseline

## Phase Name

Validator Update And Baseline Matrix

## Implement Phase ID

`impl_validator_baseline`

## Preexisting Inputs

- Existing external validator checkout at `validator/`. Current known state before this phase: detached at `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`; `validator/origin/main` is expected to resolve to `87b321fe728340d6fc6dd2f638583cca82c667c3`; `validator/workflow.yaml` has a preexisting local non-runtime edit that renames internal `port-04-test` text to `port`.
- Historical `validator-report.md`, which records the prior clean run at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`: 135 of 135 passed, 5 source plus 130 usage, with casts and no waivers.
- Prior validator artifacts under `validator/artifacts/` as historical context only. The updated validator run at `87b321fe728340d6fc6dd2f638583cca82c667c3` is authoritative for this plan.
- Existing package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/test.c`, `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`, and `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`. These replace missing historical `original/build/test_cmake*` fixtures and must be passed explicitly to `safe/scripts/check-packaged-install-surface.sh`.
- `original/`, including already prepared build/test material. Treat it as a reference input; do not refetch or regenerate it.
- `safe/`, `safe/test/`, `safe/test/images/`, and `safe/test/refs/`.
- Existing generated `safe/dist/` package artifacts as prior local build output. Treat them as evidence/stale inputs only; phase 1 still rebuilds `safe/dist/*.deb` from the current safe tree before generating the validator lock.
- `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`, and `safe/abi/platform-excluded-linux.txt`.
- `safe/capi/libtiff-safe.map` and `safe/capi/libtiffxx-safe.map`.
- `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/check-public-surface.py`, and `safe/scripts/run-upstream-shell-tests.sh`.
- `scripts/check-layout.sh`, `scripts/build-debs.sh`, `scripts/run-validation-tests.sh`, and `scripts/lib/build_port_lock.py`.
- `test-original.sh`, `dependents.json`, `all_cves.json`, and `relevant_cves.json`.
- Safe port implementation facts: `safe/Cargo.toml` builds crate `safe-libtiff` as static library `tiff_safe_core` with dependencies `fax`, `flate2`, `libc`, and `weezl`; `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config metadata, CMake package metadata, copied upstream tools, and optional tests; `safe/debian/control` packages the canonical validator set `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`; `safe/scripts/build-deb.sh` builds packages into `safe/dist/` at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- Safe implementation hotspots to preserve for diagnosis if the baseline cannot build or package:
  - `safe/src/lib.rs` owns handle lifecycle, open modes, header parsing, allocation helpers, and public C ABI exports. Relevant entry points: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c` owns C varargs marshalling, field get/set wrappers, error/warning handlers, RGBA wrappers, and directory printing. Relevant functions: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` around line 1511.
  - `safe/src/core/directory.rs` owns IFD parsing/writing, tag storage, defaulted fields, custom directories, deferred strile tags, and `TIFFWriteDirectory`. Relevant functions: `read_next_directory` at line 1384 and `TIFFWriteDirectory` at line 4357.
  - `safe/src/strile.rs` owns strip/tile geometry, scanline I/O, offset/bytecount management, flushing, and codec integration. Relevant functions include codec decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs` owns compression dispatch. Relevant functions: `safe_tiff_codec_decode_bytes` at line 3019 and `safe_tiff_codec_encode_bytes` at line 3070.
  - `safe/src/core/jpeg.rs` owns JPEG/OJPEG encode and decode. Relevant functions: `jpeg_decode_bytes` at line 755 and `jpeg_encode_bytes` at line 803.
  - `safe/src/rgba.rs` owns Pillow-facing RGBA read paths, color conversion, and orientation handling.
- Network access to `https://github.com/safelibs/validator`.
- Docker, Git, Python 3, Make, CMake, Ninja, Cargo/Rust, dpkg tooling, and libtiff package build dependencies.

Consume these artifacts in place. Do not use `make fetch-port-debs`; that fetches GitHub release assets and is not the local source-under-test flow. Do not refetch, recollect, rediscover, or regenerate prepared artifacts from scratch. Phase 1 is the only phase that may fetch or checkout the validator repository.

## New Outputs

- `validator/` updated to current `origin/main` or cloned if missing. Expected selected commit for this plan is `87b321fe728340d6fc6dd2f638583cca82c667c3`.
- Validator case-count confirmation from the checked-out files: at least 5 source cases, 170 usage cases, and 175 total cases. Validator `main` adds 40 libtiff usage testcases compared with the prior report; new coverage includes BigTIFF write/read variants, CCITT RLE, float32 and int32 pixel roundtrips, more metadata tags, explicit rational resolution, ICC profiles, orientation, palette colormap, SubIFD, additional `tiffcp`, `tiffcrop`, `tiff2pdf`, `tiffinfo`, `tiffsplit`, tile, rows-per-strip, and strip byte count checks.
- `safe/dist/*.deb` rebuilt from the current safe tree.
- `validator/artifacts/debs/local/libtiff/*.deb`.
- `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`.
- `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`.
- `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`.
- `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- `validator/artifacts/libtiff-safe/package-smoke/{test.c,cmake-target/CMakeLists.txt,cmake-targetless/CMakeLists.txt}` only if those existing smoke files are missing and must be recreated with the exact contents embedded below.
- Updated `validator-report.md` with validator commit, safe commit, commands run, counts, failure buckets, package hashes, result/log/cast/proof paths, and waiver state.
- A git commit for this phase. Do not add the nested `validator/` directory to the parent repository.

## File Changes

- Rewrite `validator-report.md`.
- `.git/info/exclude` may receive `/validator/` locally if needed so the nested checkout is not committed.
- Do not edit `safe/` unless the baseline cannot build or package.
- Do not edit validator runtime files: `validator/tests/libtiff`, `validator/tests/_shared`, `validator/repositories.yml`, `validator/test.sh`, or `validator/tools`.

Critical file guidance for this phase:

- `scripts/lib/build_port_lock.py` is consumed as-is to synthesize the validator-compatible local `port` lock and copy matching canonical packages into `<override-root>/libtiff/`.
- `scripts/run-validation-tests.sh`, `scripts/build-debs.sh`, `safe/scripts/build-deb.sh`, and `safe/scripts/check-packaged-install-surface.sh` are existing harnesses. Edit them only for true local harness bugs that block validation.
- `validator/tests/libtiff/**/*`, `validator/tests/_shared/**/*`, `validator/repositories.yml`, `validator/test.sh`, and `validator/tools/**/*` must not be modified to pass tests. Read them only for diagnosis.

## Implementation Details

Workflow-generation contract preserved for this implement block:

- Execute phases linearly. Do not generate `parallel_groups`.
- Preserve the source-plan generation boundary for downstream workflow generation: generate only `.plan/plan.md`; do not generate or edit `.plan/phases/*`, `.plan/workflow-structure.yaml`, or `workflow.yaml` from inside phase-level prompts because workflow-generation stages own those files.
- Use self-contained inline-only YAML. Do not use a top-level `include`.
- Do not use phase-level `prompt_file`, `workflow_file`, `workflow_dir`, `checks`, `source`, or any other YAML-source indirection.
- Do not generate `bounce_targets` lists. Each verifier has exactly one fixed `bounce_target`.
- Every verifier is an explicit top-level `check` phase, stays inside the implement block it verifies, and bounces only to that implement phase.
- If a verifier runs tests, lint, build, validator commands, proof generation, artifact parsing, or review commands, write those commands directly in the checker instructions. Do not model them as non-agentic phases.
- Existing workspace artifacts are inputs. Consume or update them in place. Do not refetch, recollect, rediscover, or regenerate prepared artifacts from scratch.
- Generate the local validator package override root from locally built safe Debian packages as `validator/artifacts/debs/local/libtiff/*.deb`; do not use `make fetch-port-debs`.
- Phase 1 is the only phase that may fetch or checkout the validator repository. Later phases and final verification consume the validator commit selected here and must not refetch or move the checkout.

1. Preserve dirty validator runtime state. Fail early if `validator/` has uncommitted or untracked changes under `tests/libtiff`, `tests/_shared`, `repositories.yml`, `test.sh`, or `tools`. Record existing non-runtime validator dirt, currently `workflow.yaml`, in `validator-report.md` before checkout updates. If `workflow.yaml` is dirty, stash only that file before moving the checkout and leave the stash unapplied during validation. Do not reset validator runtime files and do not reapply the workflow edit into the validation working tree.

```bash
if [ ! -d validator/.git ]; then
  git clone https://github.com/safelibs/validator validator
fi
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
if [ -n "$(git -C validator status --porcelain -- workflow.yaml)" ]; then
  git -C validator stash push -m "preexisting non-runtime workflow.yaml edit before libtiff validation" -- workflow.yaml
fi
git -C validator fetch origin main
git -C validator checkout --detach origin/main
VALIDATOR_COMMIT="$(git -C validator rev-parse HEAD)"
```

2. Determine current libtiff case counts from the checked-out validator and enforce the current floor. Case-count checkers must derive exact expected counts from `validator/tests/libtiff/tests/cases/{source,usage}` and enforce floors of at least 5 source, 170 usage, and 175 total cases.

```bash
python3 validator/tools/testcases.py \
  --config validator/repositories.yml \
  --tests-root validator/tests \
  --library libtiff \
  --check \
  --min-source-cases 5 \
  --min-usage-cases 170 \
  --min-cases 175
```

3. Run validator unit and manifest checks.

```bash
make -C validator unit
make -C validator check-testcases
```

4. Build local safe packages and smoke their installed surface. Use explicit package-smoke arguments because this workspace does not contain reusable `original/build/test_cmake*` fixtures. Reuse `validator/artifacts/libtiff-safe/package-smoke/`; recreate only the three files below if missing and document recreation in `validator-report.md`.

`validator/artifacts/libtiff-safe/package-smoke/test.c`:

```c
#include <tiffio.h>

int main(void) {
    const char *version = TIFFGetVersion();
    return (version != 0 && version[0] != '\0') ? 0 : 1;
}
```

`validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.16)
project(libtiff_package_smoke_target C)
find_package(TIFF REQUIRED CONFIG)
add_executable(test ../test.c)
target_link_libraries(test PRIVATE TIFF::tiff)
```

`validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.16)
project(libtiff_package_smoke_targetless C)
find_package(TIFF REQUIRED CONFIG)
add_executable(test ../test.c)
target_include_directories(test PRIVATE ${TIFF_INCLUDE_DIRS})
target_link_libraries(test PRIVATE ${TIFF_LIBRARIES})
```

Build and smoke:

```bash
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project-no-target validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
```

5. Generate the local override root and lock with the existing helper. Before rebuilding `.deb` packages for a validator run, commit all `safe/` source, test, packaging, or script changes that should be represented in those packages. The lock must record the safe-source commit actually tested.

```bash
SAFE_COMMIT="$(git log -1 --format=%H -- safe)"
rm -rf validator/artifacts/debs/local/libtiff
SAFELIBS_LIBRARY=libtiff \
SAFELIBS_COMMIT_SHA="$SAFE_COMMIT" \
SAFELIBS_DIST_DIR="$PWD/safe/dist" \
SAFELIBS_VALIDATOR_DIR="$PWD/validator" \
SAFELIBS_LOCK_PATH="$PWD/validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json" \
SAFELIBS_OVERRIDE_ROOT="$PWD/validator/artifacts/debs/local" \
python3 scripts/lib/build_port_lock.py
```

6. Run the full libtiff port matrix and proof in local override mode. The run must use `--library libtiff`; this selects the full validator suite relevant to libtiff-safe. In `port` mode, `bash test.sh` can complete while behavioral failures are recorded in JSON, so always parse `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json` and per-case JSON files instead of trusting process exit status alone.

```bash
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

7. Rewrite `validator-report.md` with machine-readable provenance lines and failure buckets: package/provenance, source/CLI, usage/runtime, suspected validator bug, and remaining unknown. The report must include `Validator commit:`, `Safe source commit tested:`, `Checks executed:`, `Failures found:`, and `Waived testcase ids:`. Validator-bug waivers are exceptional and require original-package evidence, safe-package evidence, testcase id, validator commit, logs, result paths, and a concrete explanation. The validator does not expose a per-test skip flag for this local flow, so checkers may ignore only testcase ids listed on the machine-readable `Waived testcase ids:` line.

8. Commit before yielding.

```bash
git add validator-report.md
git commit -m "impl_validator_baseline: record updated libtiff validator baseline"
```

## Verification Phases

### `check_validator_baseline_tester`

- Phase ID: `check_validator_baseline_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_validator_baseline`
- Purpose: Verify the validator checkout is updated, local `.deb` overrides and lock exist, the libtiff matrix ran, the proof was generated, and the report has machine-readable provenance. Record failures; do not require all testcases to pass.
- Commands:

```bash
test -d validator/.git
git -C validator rev-parse HEAD
test -d validator/artifacts/debs/local/libtiff
test -f validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json
test -f validator/artifacts/libtiff-safe/port/results/libtiff/summary.json
test -f validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json
python3 - <<'PY'
import json
import re
import subprocess
from pathlib import Path

summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
assert summary["mode"] == "port", summary
assert summary["library"] == "libtiff", summary
expected_source = len(list(Path("validator/tests/libtiff/tests/cases/source").glob("*.sh")))
expected_usage = len(list(Path("validator/tests/libtiff/tests/cases/usage").glob("*.sh")))
assert expected_source >= 5, expected_source
assert expected_usage >= 170, expected_usage
assert summary["source_cases"] == expected_source, (summary, expected_source)
assert summary["usage_cases"] == expected_usage, (summary, expected_usage)
assert summary["cases"] == expected_source + expected_usage, summary

lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
lib = lock["libraries"][0]
assert lib["library"] == "libtiff", lib
assert lib["unported_original_packages"] == [], lib
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
assert lib["commit"] == safe_commit, (lib["commit"], safe_commit)
assert [d["package"] for d in lib["debs"]] == ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"], lib
for deb in lib["debs"]:
    path = Path("validator/artifacts/debs/local/libtiff") / deb["filename"]
    assert path.is_file(), path
    assert path.stat().st_size == deb["size"], path
for path in (
    "validator/artifacts/libtiff-safe/package-smoke/test.c",
    "validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt",
    "validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt",
):
    assert Path(path).is_file(), path

proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
assert proof["mode"] == "port", proof
assert proof["totals"]["cases"] == summary["cases"], (proof["totals"], summary)
assert proof["libraries"][0]["port_commit"] == lib["commit"], (proof["libraries"][0], lib)

report = Path("validator-report.md").read_text()
for header in ("Validator commit:", "Safe source commit tested:", "Checks executed:", "Failures found:", "Waived testcase ids:"):
    assert re.search(rf"^{re.escape(header)}", report, re.MULTILINE), header
assert re.search(r"^Validator commit:\s*[0-9a-f]{40}$", report, re.MULTILINE)
assert re.search(r"^Safe source commit tested:\s*[0-9a-f]{40}$", report, re.MULTILINE)

head = subprocess.check_output(["git", "-C", "validator", "rev-parse", "HEAD"], text=True).strip()
assert f"Validator commit: {head}" in report
PY
```

### `check_validator_baseline_senior`

- Phase ID: `check_validator_baseline_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_validator_baseline`
- Purpose: Review that the baseline followed the validator README local override flow, preserved existing artifacts, did not change validator runtime code, and categorized failures for later phases.
- Commands:

```bash
git status --short
git show --stat --format=fuller HEAD
git -C validator status --short
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
print("summary", summary)
failed = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    result = json.loads(path.read_text())
    if result.get("status") == "failed":
        failed.append((result["testcase_id"], result["kind"], result.get("log_path")))
print("failed", failed)
PY
rg -n "Validator commit:|Safe source commit tested:|Checks executed:|Failures found:|Waived testcase ids:|Failure buckets|package|source|usage|waiver" validator-report.md
```

## Success Criteria

- Validator checkout is selected and recorded without modifying validator runtime files.
- The expected validator commit for this plan, `87b321fe728340d6fc6dd2f638583cca82c667c3`, is used if it is the resolved `origin/main`; otherwise the actual selected commit is recorded and all counts are derived from that checked-out tree.
- Current libtiff case counts are derived from checked-out files and satisfy at least 5 source, 170 usage, and 175 total.
- Local safe packages build, package-smoke checks run with the existing smoke projects, local override `.deb` files and lock are generated, per-case JSON/log/cast artifacts exist, and proof artifacts exist.
- `validator-report.md` records machine-readable provenance, the prior clean-run context, current failure buckets, artifact paths, and waiver state.
- Baseline testcase failures are allowed as inputs to later phases.

## Git Commit Requirement

The implementer must commit work to git before yielding. If this phase only updates the report or has no code change, commit the report change or create an empty phase commit naming `impl_validator_baseline`.
