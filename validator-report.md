# Validator Baseline Report

Validator commit: 87b321fe728340d6fc6dd2f638583cca82c667c3
Safe source commit tested: 0bb04537cc14dce0ddb952f5232681e12d60353c
Checks executed: validator runtime dirty-check; stash preexisting validator workflow.yaml edit; fetch origin main; checkout detached origin/main; libtiff testcase count check; make -C validator unit; make -C validator check-testcases; safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist; safe/scripts/check-packaged-install-surface.sh with existing package-smoke CMake/pkg-config fixtures; scripts/lib/build_port_lock.py local override lock generation; full validator port matrix with --record-casts; verify_proof_artifacts with --require-casts; focused cargo/CMake/docker reproductions for safe fixes; final package rebuild, lock regeneration, matrix, proof, and JSON artifact audit; impl_package_provenance_waiver_gate lock/proof/per-case provenance audit; impl_package_provenance_waiver_gate packaged install-surface smoke check
Failures found: 0 final failures; 0 package/provenance failures in impl_package_provenance_waiver_gate; 8 failures found on the first 175-case baseline and fixed in safe, then 1 remaining JPEG-table failure fixed in safe before the final clean run
Waived testcase ids:

## Summary

- Phase: `impl_validator_baseline`
- Date: 2026-05-04 MST
- Library: `libtiff`
- Mode: `port`
- Validator checkout: `validator/`, detached at `87b321fe728340d6fc6dd2f638583cca82c667c3`
- Safe commit tested in final validator run: `0bb04537cc14dce0ddb952f5232681e12d60353c`
- Final result: `175/175` passed, `0` failed, `5` source cases, `170` usage cases, `175` casts
- Waivers: none

## Package Provenance Gate

- Phase: `impl_package_provenance_waiver_gate`
- Date: 2026-05-04 MST
- Disposition: clean; no safe packaging change, package rebuild, validator rerun, or waiver was required in this phase.
- Validator commit: `87b321fe728340d6fc6dd2f638583cca82c667c3`
- Safe source commit tested: `0bb04537cc14dce0ddb952f5232681e12d60353c`
- Canonical package set: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`

The package gate consumed the prepared phase 1 artifacts in place. `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` records all four canonical packages, `unported_original_packages: []`, and release tag `build-0bb04537cc14`. The lock sizes and SHA-256 values match the `.deb` files under `validator/artifacts/debs/local/libtiff/`, and `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json` records the same safe source commit.

Every per-case result JSON under `validator/artifacts/libtiff-safe/port/results/libtiff/` reports `override_debs_installed: true`, the canonical packages in order, and `port_commit: 0bb04537cc14dce0ddb952f5232681e12d60353c`. The existing packaged install-surface smoke check also passed with:

```bash
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project-no-target validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
```

Package/provenance failures found: none. Waiver candidates: none. Original-mode evidence was not collected in this phase because there are no validator-bug waiver candidates.

The previous report recorded a clean run at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`: `135/135` passed, `5` source plus `130` usage, with casts and no waivers. Validator `main` now contributes `40` additional libtiff usage cases, bringing the checked-out tree to `175` total cases. The added coverage includes BigTIFF write/read variants, CCITT RLE, float32 and int32 roundtrips, metadata tags, rational resolution, ICC profiles, orientation, palette colormap, SubIFD, additional `tiffcp`, `tiffcrop`, `tiff2pdf`, `tiffinfo`, `tiffsplit`, tile, rows-per-strip, and strip byte count checks.

## Validator Checkout

- `validator/.git` already existed and was reused.
- Runtime dirty check passed for `tests/libtiff`, `tests/_shared`, `repositories.yml`, `test.sh`, and `tools`.
- Preexisting non-runtime validator dirt was limited to `workflow.yaml`. It was stashed before checkout with message `preexisting non-runtime workflow.yaml edit before libtiff validation` and left unapplied during validation.
- Commands:
  - `git -C validator stash push -m "preexisting non-runtime workflow.yaml edit before libtiff validation" -- workflow.yaml`
  - `git -C validator fetch origin main`
  - `git -C validator checkout --detach origin/main`
  - `git -C validator rev-parse HEAD`
- Selected commit: `87b321fe728340d6fc6dd2f638583cca82c667c3`
- `git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools` is empty.

## Case Counts

Counts were derived from the checked-out validator files and enforced with:

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

| Source cases | Usage cases | Total cases |
| ---: | ---: | ---: |
| 5 | 170 | 175 |

Validator self-checks also passed:

```bash
make -C validator unit
make -C validator check-testcases
```

## Safe Fixes

Two safe-port commits were made before the final package rebuild:

- `9552cf46de26e98d55bde8dd286eb0012f63cf5f` — fixed CCITT RLE decode lookahead padding and upstream-compatible `TIFFPrintDirectory` names/strip summaries.
- `0bb04537cc14dce0ddb952f5232681e12d60353c` — added JPEG table extraction during JPEG encode and printed `JPEG Tables: (N bytes)` in `TIFFPrintDirectory`.

Focused verification included:

```bash
cargo test --manifest-path safe/Cargo.toml
cmake --build safe/build-debian --parallel
docker run ... validator-check-libtiff:latest ... LD_LIBRARY_PATH=/safe-build python3 ... compression="jpeg" ...
```

The focused JPEG-table reproduction against the rebuilt local library printed `JPEG Tables: (574 bytes)` and Pillow observed `Compression=7` plus a non-empty tag `347`.

## Package Build

Final packages were rebuilt after committing the safe fixes:

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

The existing package-smoke files were present and reused; none were recreated.

| Package | Filename | Size | SHA-256 |
| --- | --- | ---: | --- |
| `libtiff6` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 642830 | `25eaa567091a89711dd202c15c87e2196985afeb0b027306288f747d1e8e6152` |
| `libtiffxx6` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 12306 | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` |
| `libtiff-dev` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 35732 | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` |
| `libtiff-tools` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | 200508 | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` |

The local lock records all four canonical packages and `unported_original_packages: []`.

## Lock And Artifacts

Lock generation:

```bash
SAFELIBS_LIBRARY=libtiff \
SAFELIBS_COMMIT_SHA=0bb04537cc14dce0ddb952f5232681e12d60353c \
SAFELIBS_DIST_DIR="$PWD/safe/dist" \
SAFELIBS_VALIDATOR_DIR="$PWD/validator" \
SAFELIBS_LOCK_PATH="$PWD/validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json" \
SAFELIBS_OVERRIDE_ROOT="$PWD/validator/artifacts/debs/local" \
python3 scripts/lib/build_port_lock.py
```

- Override `.deb` root: `validator/artifacts/debs/local/libtiff/`
- Local lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Lock release tag: `build-0bb04537cc14`
- Result JSON: `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
- Logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
- Casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`

## Final Validator Run

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

Final `summary.json`:

```json
{
  "cases": 175,
  "casts": 175,
  "failed": 0,
  "library": "libtiff",
  "mode": "port",
  "passed": 175,
  "source_cases": 5,
  "usage_cases": 170
}
```

Final proof totals match the summary: `175` cases, `175` passed, `0` failed, `175` casts. `proof.libraries[0].port_commit`, `lock.libraries[0].commit`, and `git log -1 --format=%H -- safe` all equal `0bb04537cc14dce0ddb952f5232681e12d60353c`.

## Failure Buckets

| Bucket | Baseline finding | Final state |
| --- | --- | --- |
| Package/provenance | No canonical package mismatch; all four packages locked and installed. | Clean |
| Source/CLI | `tiffinfo` formatting gaps for compression, photometric, orientation, strip summaries, and JPEG tables. | Fixed in safe |
| Usage/runtime | CCITT RLE decode failed on a valid Pillow bilevel RLE image. | Fixed in safe |
| Suspected validator bug | None. Original package evidence was not needed because all observed failures were safe compatibility gaps. | No waivers |
| Remaining unknown | None. | Clean |

Initial failing testcase ids fixed during this phase:

- `usage-python3-pil-tiff-ccitt-rle-bilevel-compression`
- `usage-python3-pil-tiff-orientation-tag-bottomright`
- `usage-python3-pil-tiff-rowsperstrip-tag-tiffcp-r-16`
- `usage-python3-pil-tiff-tiffcp-bigtiff-bigendian`
- `usage-python3-pil-tiff-tiffcp-r-32-mid-strip`
- `usage-python3-pil-tiff-tiffinfo-bps-16bit-gray`
- `usage-python3-pil-tiff-tiffinfo-j-jpeg-tables`
- `usage-python3-pil-tiff-tiffinfo-photometric-line`

Final failed testcase ids: none.

## Waivers

No validator-bug waivers were used. The machine-readable `Waived testcase ids:` line is intentionally empty because the final matrix has `summary["failed"] == 0`.
