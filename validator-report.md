# Validator Checkout And Baseline Safe Matrix

## Summary

- Phase: `impl_validator_baseline`
- Verification date: 2026-05-02, America/Phoenix
- Validator repository: `https://github.com/safelibs/validator`
- Validator commit: `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`
- Safe source commit tested: `61f38826b440c30b5099410a52e1af227832622e`
- Mode: port
- Library: `libtiff`
- Override root: `validator/artifacts/debs/local/libtiff/`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Baseline status: clean. The libtiff `port` matrix passed all 135 testcases; no failures to triage.

## Validator Checkout

- The pre-existing `validator/` checkout was preserved (no clone, no reset).
- Pinned commit verified: `git -C validator rev-parse HEAD` → `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`.
- `/validator/` is present in `.git/info/exclude` so the nested checkout is not committed to the parent repo.
- Validator working tree state: `M workflow.yaml`. This is a pre-existing local edit that renames internal `port-04-test` references to `port` in the validator's own auxiliary `workflow.yaml`. It does not affect `tests/`, `repositories.yml`, `tools/`, `test.sh`, or any matrix runtime path. Per the phase contract, the dirty file was preserved (no `git reset`, no overwrite). Documented here as a non-blocking pre-existing condition; no validator source files, tests, or manifests were modified by this phase.

## Validator Lints

- `make -C validator unit` → 110 unit tests passed.
- `make -C validator check-testcases` → manifest + header lint passed.
- `python3 validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135` → passed.

## Counts

| Source cases | Usage cases | Total cases | Passed | Failed | Casts |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 130 | 135 | 135 | 0 | 135 |

Source cases (5): `c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, `tiffinfo-metadata`.

Usage cases (130): Pillow TIFF open/save/metadata/compression/multipage operations and CLI usage for `tiffcp`, `tiffinfo`, `tiffdump`, `tiff2bw`, `tiff2pdf`, `tiffcrop`, `tiffmedian`, `tiffsplit`.

Proof totals match the result summary:

```json
{
  "cases": 135,
  "casts": 135,
  "failed": 0,
  "libraries": 1,
  "passed": 135,
  "source_cases": 5,
  "usage_cases": 130
}
```

## Package Provenance

- Package source tree: committed `safe/` tree at `61f38826b440c30b5099410a52e1af227832622e` (`git log -1 --format=%H -- safe`).
- Release tag in local lock: `local-61f38826b440`.
- Package version required and verified: `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- Package names required and verified: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`.
- Each `.deb`'s internal Debian `Package` field was verified against the expected canonical name before being recorded in the lock.

| Package | Version | Architecture | Filename | SHA-256 | Size |
| --- | --- | --- | --- | --- | ---: |
| `libtiff6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `b5c7477fb5d99989ce034ecb9558cfc951c0fd23b05bd4be17514e0dc5ac0f29` | 641772 |
| `libtiffxx6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` | 12306 |
| `libtiff-dev` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` | 35732 |
| `libtiff-tools` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` | 200508 |

The proof's `libraries[0].port_commit` (`61f38826b440c30b5099410a52e1af227832622e`) equals the local lock's `libraries[0].commit` and the machine-readable safe source commit below.

## Package Smoke Projects

The historical `original/build/test_cmake*` directories are not present in this workspace. The local replacement smoke projects under `validator/artifacts/libtiff-safe/package-smoke/` were generated to the canonical content (`test.c`, `cmake-target/CMakeLists.txt`, `cmake-targetless/CMakeLists.txt`) and reused by `safe/scripts/check-packaged-install-surface.sh`. Subsequent phases should reuse the same files in place.

## Commands Executed

- `git -C validator rev-parse HEAD` (verified pinned commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`)
- `make -C validator unit`
- `make -C validator check-testcases`
- `python3 validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135`
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff`
- `rm -rf validator/artifacts/debs/local/libtiff && mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof`
- `find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +`
- Synthesized `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` from actual local `.deb` metadata, `dpkg-deb -f` package-name verification, SHA-256 digests, file sizes, architectures, and `git log -1 --format=%H -- safe`.
- `cd validator && bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts`
- `cd validator && python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports`
- Audited every `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` for non-passed status; result paths confirmed present, `summary.json` reports 135 cases / 5 source / 130 usage / 135 passed / 0 failed / 135 casts.

## Result Artifact Paths

- Per-case JSON: `validator/artifacts/libtiff-safe/port/results/libtiff/<testcase>.json` (135 files plus `summary.json`).
- Per-case logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/<testcase>.log` (135 testcase logs plus `docker-build.log`).
- Per-case casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/<testcase>.cast` (135 casts).
- Aggregated proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.

## Triage

The baseline run produced zero validator failures, so there are no source/CLI failures and no usage failures to triage. The downstream phases inherit the following empty buckets:

- Source/CLI bucket (phase `impl_source_cli_failures`): empty. No failing source cases (`c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, `tiffinfo-metadata` all passed).
- Usage bucket (phase `impl_usage_runtime_failures`): empty. All 130 usage cases passed.
- Package-provenance / waiver bucket (phase `impl_package_waiver_gate`): no waiver candidates from this baseline. Validator-bug waivers are exceptional and remain reserved for that phase.

No first-failing symptoms to record; no result/log paths to flag. If a later re-run uncovers regressions, the same triage shape will be used to populate these buckets.

## Required Safe Fixes

None. The current `safe/` tree built, packaged, and ran through the validator port matrix without modification. No packaging or build break blocked the baseline.

## Waivers

No waivers were applied.

## Machine Readable

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: 61f38826b440c30b5099410a52e1af227832622e
Checks executed: validator unit; validator check-testcases; libtiff testcases inventory check; safe build-deb; check-packaged-install-surface (cmake-target, cmake-targetless, pkg-config, C++ smoke, input-tiff round-trip); local override deb copy; local-port-debs-lock generation with dpkg-deb package-name verification; validator libtiff port matrix with --record-casts; verify_proof_artifacts with --require-casts; per-case results audit
Failures found: 0
Waived testcase ids:
Baseline status: clean validator port matrix, zero failures, zero waivers
