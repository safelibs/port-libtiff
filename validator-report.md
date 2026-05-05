# Validator Closure Report

Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
Safe source commit tested: 95972bf6fd80e21bfaba0fb1685f532674ef299b
Final status: clean; 175/175 validator cases passed, 0 failed, 175 casts recorded, no waivers.
Checks executed: full local safe compatibility matrix; validator unit tests; validator testcase manifest checks; checked-out libtiff testcase count check; package rebuild; packaged install-surface smoke tests; local deb lock regeneration; full validator port matrix with --record-casts; proof verification with --require-casts; final JSON/proof/lock/cast audit.
Failures found: none in the final run.
Waived testcase ids: ,

The machine-readable waiver line above intentionally contains only a comma so the verifier parser reads an empty set. Waiver disposition in prose: none. No validator-bug waiver remains, and no original-mode waiver evidence was needed in this final phase.

## Phase

- Phase: `impl_final_validator_clean_run`
- Date: 2026-05-04 MST -0700
- Library: `libtiff`
- Validator mode: `port`
- Validator checkout: `validator/`, detached at `87b321fe728340d6fc6dd2f638583cca82c667c3`
- Final safe source commit tested: `95972bf6fd80e21bfaba0fb1685f532674ef299b`
- Final release tag in local lock: `build-95972bf6fd80`
- Canonical packages: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`

## Safe Port Facts

- `safe/Cargo.toml` builds crate `safe-libtiff` as static library `tiff_safe_core`.
- `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config metadata, CMake package metadata, copied upstream tools, and optional tests.
- `safe/debian/control` packages `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`.
- `safe/scripts/build-deb.sh` built version `1:4.5.1+git230720-4ubuntu2.5+safelibs1` packages into `safe/dist/`.

## Counts

Counts were derived from the checked-out validator testcase files:

| Source cases | Usage cases | Total cases |
| ---: | ---: | ---: |
| 5 | 170 | 175 |

Final artifact counts:

| Artifact set | Count | Notes |
| --- | ---: | --- |
| `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` | 176 | 175 testcase JSON files plus `summary.json` |
| `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log` | 176 | 175 testcase logs plus `summary.log` |
| `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast` | 175 | one cast per testcase |

`summary.json` reports `passed: 175`, `failed: 0`, `source_cases: 5`, `usage_cases: 170`, `cases: 175`, and `casts: 175`. `proof/libtiff-safe-port-proof.json` reports matching totals, including `casts: 175`.

## Checks Executed

Local safe compatibility matrix:

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

Validator-side checks:

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

Final local deb lock and validator proof run:

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
    --min-usage-cases 170 \
    --min-cases 175 \
    --ports-root /home/yans/safelibs/pipeline/ports
)
```

Final artifact audit:

```bash
python3 - <<'PY'
# Inline audit from the phase contract:
# - lock commit equals git log -1 --format=%H -- safe
# - proof port_commit equals the safe commit
# - report contains the validator and safe commits
# - every per-case result has override_debs_installed: true
# - every per-case result has the final safe port_commit
# - checked-out source/usage testcase counts match summary.json
# - proof totals match summary totals
# - proof casts equal total cases
# - every failed testcase is listed on Waived testcase ids
# - because the waiver set is empty, failed == 0 and summary["failed"] == 0
PY
```

All checks above passed.

## Package Artifacts

Final `safe/dist/*.deb` and `validator/artifacts/debs/local/libtiff/*.deb` contain the same four canonical package files and SHA-256 values:

| Package | Filename | Size | SHA-256 |
| --- | --- | ---: | --- |
| `libtiff6` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 642830 | `25eaa567091a89711dd202c15c87e2196985afeb0b027306288f747d1e8e6152` |
| `libtiffxx6` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 12306 | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` |
| `libtiff-dev` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 35732 | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` |
| `libtiff-tools` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 200508 | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` |

The local lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` records all four canonical packages, `commit: 95972bf6fd80e21bfaba0fb1685f532674ef299b`, `release_tag: build-95972bf6fd80`, and `unported_original_packages: []`.

## Final Artifacts

- Packages: `safe/dist/*.deb`
- Local validator package override: `validator/artifacts/debs/local/libtiff/*.deb`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Results: `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
- Logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
- Casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Package-smoke inputs reused in place:
  - `validator/artifacts/libtiff-safe/package-smoke/test.c`
  - `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`
  - `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`

## Failures And Fixes

Failures found in this final phase: none. No `safe/` implementation change was needed in `impl_final_validator_clean_run`.

fixes applied before this final phase and preserved in the tested safe source:

| Commit | Scope |
| --- | --- |
| `9552cf46de26e98d55bde8dd286eb0012f63cf5f` | Fixed CCITT RLE decode lookahead padding and upstream-compatible `TIFFPrintDirectory` names/strip summaries. |
| `0bb04537cc14dce0ddb952f5232681e12d60353c` | Added JPEG table extraction during JPEG encode and printed `JPEG Tables: (N bytes)` in `TIFFPrintDirectory`. |
| `794a27d0176788ab04e04b827a0cb7b3af41026a` | Aligned the default Compression print regression with upstream `Compression Scheme: None` output. |
| `95972bf6fd80e21bfaba0fb1685f532674ef299b` | Refreshed the public-surface input manifest digest for the Release `libtiff.so.6` used by verifier checks. |

Historical clean-run context: an earlier validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e` had a clean `135/135` run with `5` source cases, `130` usage cases, and no waivers. The selected validator checkout now has `175` cases: `5` source plus `170` usage.

## Validator Runtime Cleanliness

The final run consumed the prepared `validator/` checkout in place. It did not refetch, move, or edit validator runtime files. This check passed:

```bash
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
```

The only unrelated parent worktree modification visible before and after this phase is `workflow.yaml`; it was not staged or committed by this phase.
