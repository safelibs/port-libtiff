# Final Validator Clean Run Report

Validator commit: bde8758883d12061dfb2621b6149949909c803f8
Safe source commit tested: 3dd81b54dcd6698241f39489d45e0dd92c6b382d
Final status: clean final validator run; 255/255 validator cases passed, 0 failed, 255 casts recorded, no waivers.
Failures found: 0 package/provenance failures; 0 source/regression failures; 0 usage/runtime failures; 0 validator-bug candidates.

Package/provenance baseline testcase ids: ,
Source/regression baseline testcase ids: ,
Usage/runtime baseline testcase ids: usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output, usage-python3-pil-r11-tiff-rgba-extra-samples-alpha
Current package/provenance failed testcase ids: ,
Current source/regression failed testcase ids: ,
Current usage/runtime failed testcase ids: ,
Waived testcase ids: ,

## Phase

- Phase: `impl_final_validator_clean_run`
- Date: 2026-05-12 MST -0700
- Library: `libtiff`
- Validator mode: `port`
- Validator checkout: `validator/`, detached at `bde8758883d12061dfb2621b6149949909c803f8`
- Safe source commit tested: `3dd81b54dcd6698241f39489d45e0dd92c6b382d`
- Release tag in local lock: `build-3dd81b54dcd6`
- Canonical packages: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`

## Checks Executed

Validator metadata and testcase floors were checked with:

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

Local safe, package, link-compatibility, install-surface, and original downstream checks were run before the final validator matrix:

```bash
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

The final lock, port matrix, and proof were regenerated from the committed safe tree:

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
    --min-usage-cases 240 \
    --min-regression-cases 10 \
    --min-cases 255 \
    --ports-root /home/yans/safelibs/pipeline/ports
)
```

## Counts

Counts were derived from the selected validator checkout and enforced with floors of at least 5 source, 240 usage, 10 regression, and 255 total cases.

| Source cases | Usage cases | Regression cases | Total cases |
| ---: | ---: | ---: | ---: |
| 5 | 240 | 10 | 255 |

Final artifact counts:

| Artifact set | Count | Notes |
| --- | ---: | --- |
| `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` | 256 | 255 testcase JSON files plus `summary.json` |
| `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log` | 256 | 255 testcase logs plus `summary.log` |
| `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast` | 255 | one cast per testcase |

`summary.json` reports `passed: 255`, `failed: 0`, `source_cases: 5`, `usage_cases: 240`, `regression_cases: 10`, `cases: 255`, and `casts: 255`. `proof/libtiff-safe-port-proof.json` reports matching totals.

## Package Artifacts

The local lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` records `commit: 3dd81b54dcd6698241f39489d45e0dd92c6b382d`, `release_tag: build-3dd81b54dcd6`, all four canonical packages, and `unported_original_packages: []`.

| Package | Filename | Size | SHA-256 |
| --- | --- | ---: | --- |
| `libtiff6` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 643506 | `daeee5f546ad8f0c1da0998c999271a5b11105b0c32995ab61cc97fc252cd986` |
| `libtiffxx6` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 12306 | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` |
| `libtiff-dev` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 35732 | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` |
| `libtiff-tools` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 200508 | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` |

Package-smoke inputs were present and left untouched:

- `validator/artifacts/libtiff-safe/package-smoke/test.c`
- `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`
- `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`

## Proof

- Matrix status: `validator/artifacts/libtiff-safe/port/matrix-status.txt` contains `0`.
- Proof status: `validator/artifacts/libtiff-safe/proof/proof-status.txt` contains `0`.
- Proof JSON: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- The proof `mode` is `port`, library is `libtiff`, and `port_commit` is `3dd81b54dcd6698241f39489d45e0dd92c6b382d`.
- Every per-case result records `override_debs_installed: true` and `port_commit: 3dd81b54dcd6698241f39489d45e0dd92c6b382d`.

## Fixes Applied

The final clean-run phase refreshed `safe/abi/public-surface.inputs.json` with the current `safe/build/libtiff/libtiff.so.6` hash after the final Release build. The public exported surface did not change. That safe-tree update was committed before the final package rebuild, local lock generation, validator run, and proof generation.

Prior phase fixes remained in place for the two immutable Phase 1 usage/runtime baseline ids:

- `usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output`: `TIFFReadRawStrip(..., -1)` compatibility for full raw strip reads.
- `usage-python3-pil-r11-tiff-rgba-extra-samples-alpha`: upstream-compatible `TIFFPrintDirectory` output for `ExtraSamples`.

## Failure Ledger

Package/provenance: none. Every validator testcase installed local override packages, all canonical packages came from `validator/artifacts/debs/local/libtiff`, the lock commit matches the safe source commit, and `unported_original_packages` is empty.

Source/regression: none. All 5 source cases and all 10 regression cases passed.

Usage/runtime: none. All 240 usage cases passed.

Validator-bug candidates: none. `Waived testcase ids:` is empty, so no original-mode waiver evidence run was required.

## Final Artifacts

- Packages: `safe/dist/*.deb`
- Local validator package override: `validator/artifacts/debs/local/libtiff/*.deb`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Results: `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
- Logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
- Casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Matrix status: `validator/artifacts/libtiff-safe/port/matrix-status.txt`
- Proof status: `validator/artifacts/libtiff-safe/proof/proof-status.txt`
- Proof JSON: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`

## Artifact Discipline

`safe/` was clean before package build, lock generation, validator execution, proof generation, and report rewrite. Validator runtime inputs under `validator/tests/libtiff`, `validator/tests/_shared`, `validator/repositories.yml`, `validator/test.sh`, and `validator/tools` were not modified.

`workflow.yaml`, `.plan/plan.md`, and generated validator/package artifacts were not staged for the final report commit.
