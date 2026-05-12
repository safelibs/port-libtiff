# Validator Baseline Report

Validator commit: bde8758883d12061dfb2621b6149949909c803f8
Safe source commit tested: 95972bf6fd80e21bfaba0fb1685f532674ef299b
Final status: baseline recorded; 253/255 validator cases passed, 2 failed, 255 casts recorded, no waivers.
Failures found: 2 usage/runtime failures; 0 package/provenance failures; 0 source/regression failures; 0 validator-bug candidates.

Package/provenance baseline testcase ids: ,
Source/regression baseline testcase ids: ,
Usage/runtime baseline testcase ids: usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output, usage-python3-pil-r11-tiff-rgba-extra-samples-alpha
Current package/provenance failed testcase ids: ,
Current source/regression failed testcase ids: ,
Current usage/runtime failed testcase ids: usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output, usage-python3-pil-r11-tiff-rgba-extra-samples-alpha
Waived testcase ids: ,

## Phase

- Phase: `impl_validator_baseline`
- Date: 2026-05-12 MST -0700
- Library: `libtiff`
- Validator mode: `port`
- Validator checkout: `validator/`, detached at `bde8758883d12061dfb2621b6149949909c803f8`
- Safe source commit tested: `95972bf6fd80e21bfaba0fb1685f532674ef299b`
- Release tag in local lock: `build-95972bf6fd80`
- Canonical packages: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`

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

`summary.json` reports `passed: 253`, `failed: 2`, `source_cases: 5`, `usage_cases: 240`, `regression_cases: 10`, `cases: 255`, and `casts: 255`. `proof/libtiff-safe-port-proof.json` reports matching totals.

## Commands Executed

Validator update and metadata checks:

```bash
if [ ! -d validator/.git ]; then
  git clone https://github.com/safelibs/validator validator
fi
git -C validator fetch origin main
git -C validator checkout --detach FETCH_HEAD
git -C validator rev-parse HEAD
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

Local safe preflight:

```bash
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
```

Package build and local port lock:

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

Port matrix and proof generation:

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

## Package Artifacts

The local lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` records all four canonical packages, `commit: 95972bf6fd80e21bfaba0fb1685f532674ef299b`, `release_tag: build-95972bf6fd80`, and `unported_original_packages: []`.

| Package | Filename | Size | SHA-256 |
| --- | --- | ---: | --- |
| `libtiff6` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 642830 | `25eaa567091a89711dd202c15c87e2196985afeb0b027306288f747d1e8e6152` |
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
- The proof `mode` is `port`, library is `libtiff`, and `port_commit` is `95972bf6fd80e21bfaba0fb1685f532674ef299b`.

## Baseline Failure Ledger

### Package/provenance

None. Every failed testcase has `override_debs_installed: true`, all four canonical packages were installed from `validator/artifacts/debs/local/libtiff`, the local lock commit matches the safe source commit, and `unported_original_packages` is empty.

#### `impl_package_provenance_fixes`

- Date: 2026-05-12 MST -0700.
- Package/provenance baseline testcase ids remained empty, so this phase did not require a full validator port-matrix rerun.
- Rebuilt `safe/dist/*.deb` from clean `safe/` state with `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`.
- Verified the staged package install surface with `safe/scripts/check-packaged-install-surface.sh`, including C, C++, pkg-config, CMake target, and CMake targetless smoke coverage.
- Verified public surface metadata with `safe/scripts/check-public-surface.py --check --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt`.
- Refreshed `validator/artifacts/debs/local/libtiff/*.deb` and `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`; the lock still records commit `95972bf6fd80e21bfaba0fb1685f532674ef299b`, all four canonical packages, and `unported_original_packages: []`.
- Current failure partition remains unchanged: package/provenance none, source/regression none, usage/runtime `usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output` and `usage-python3-pil-r11-tiff-rgba-extra-samples-alpha`, waivers none.

### Source/regression

None. All 5 source cases and all 10 regression cases passed.

### Usage/runtime

`usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output`

- Kind: `usage`; client: `python3-pil`.
- Result: `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output.json`.
- Log: `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output.log`.
- Cast: `validator/artifacts/libtiff-safe/port/casts/libtiff/usage-python3-pil-r10-tiff-tiff2pdf-jpeg-output.cast`.
- Failure: testcase command exited with status 1 after `tiff2pdf -j` reported `TIFFReadRawStrip() failed` and `An error occurred creating output PDF file`.
- Initial remediation area: JPEG/raw strip read compatibility through `safe/src/core/jpeg.rs`, `safe/src/core/codec.rs`, `safe/src/strile.rs`, and `safe/tools/tiff2pdf.c` integration behavior.

`usage-python3-pil-r11-tiff-rgba-extra-samples-alpha`

- Kind: `usage`; client: `python3-pil`.
- Result: `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-r11-tiff-rgba-extra-samples-alpha.json`.
- Log: `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-r11-tiff-rgba-extra-samples-alpha.log`.
- Cast: `validator/artifacts/libtiff-safe/port/casts/libtiff/usage-python3-pil-r11-tiff-rgba-extra-samples-alpha.cast`.
- Failure: testcase command exited with status 1 while verifying Pillow RGBA TIFF `SamplesPerPixel == 4` and `ExtraSamples == (2,)`, followed by `tiffinfo` checks for `Extra Samples: 1<unassoc-alpha>` and `Samples/Pixel: 4`.
- Initial remediation area: RGBA alpha tag write/read behavior through `safe/src/rgba.rs`, `safe/src/core/directory.rs`, `safe/src/core/field_tables.rs`, `safe/src/core/field_registry.rs`, and the varargs tag paths in `safe/capi/tiff_placeholder.c`.

### Validator-bug candidates

None. No testcase is waived in this baseline. Because `Waived testcase ids:` is empty, no original-mode waiver evidence run was required.

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

No `safe/` source, test, packaging, or script changes were required in this phase. `safe/` was clean before package build and remained clean after the validator run. Validator runtime inputs under `validator/tests/libtiff`, `validator/tests/_shared`, `validator/repositories.yml`, `validator/test.sh`, and `validator/tools` were not modified.

The only unrelated parent worktree modification visible before this phase was `workflow.yaml`; it was not read as authoritative, staged, or committed by this phase.
