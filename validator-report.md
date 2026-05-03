# Validator Checkout And Baseline Safe Matrix

## Summary

- Phase: `impl_validator_baseline`
- Verification date: 2026-05-03, America/Phoenix
- Validator repository: `https://github.com/safelibs/validator`
- Validator commit: `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`
- Safe source commit tested: `61f38826b440c30b5099410a52e1af227832622e`
- Mode: port
- Library: `libtiff`
- Override root: `validator/artifacts/debs/local/libtiff/`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Baseline status: clean. Locally built `.deb` packages installed in every testcase, the libtiff `port` matrix passed all 135 cases (5 source + 130 usage), and no `safe/` build or packaging fix was required.

## Validator Checkout

- The pre-existing `validator/` checkout was preserved (no clone, no force-reset).
- Pinned commit reapplied via `git -C validator fetch --tags origin && git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e`; `git -C validator rev-parse HEAD` → `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`.
- `/validator/` remains in `.git/info/exclude`; the nested checkout is not committed to the parent repo.
- Validator working tree state: `M workflow.yaml` only. This is a pre-existing local edit that renames internal `port-04-test` references to `port` in the validator's own auxiliary `workflow.yaml`. The same rename has since landed upstream (`c58e3e2 rename port-04-test mode to port`) and the local edit is now redundant, but it was preserved (no `git reset`, no overwrite). The dirty file does not touch `tests/`, `repositories.yml`, `tools/`, `test.sh`, or any matrix runtime path, so the baseline run is unaffected. No validator source files, tests, manifests, or runner code were modified by this phase.

## Inventory Lint

- `make -C validator unit` → 110 unit tests passed.
- `make -C validator check-testcases` → manifest lint passed (115 source / 1683 usage / 1803 total across all libraries at the pinned commit).
- `python3 validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135` → libtiff inventory satisfies the 5/130/135 floor required by every phase at this pinned commit.

## Package Smoke Projects

The historical `original/build/test_cmake*` directories are not present in this workspace. The local replacement smoke projects under `validator/artifacts/libtiff-safe/package-smoke/` (`cmake-target/CMakeLists.txt`, `cmake-targetless/CMakeLists.txt`, `test.c`) exist with the exact contents required by `safe/scripts/check-packaged-install-surface.sh` (a `find_package(TIFF REQUIRED CONFIG)` target-link smoke, a targetless `${TIFF_INCLUDE_DIRS}` / `${TIFF_LIBRARIES}` smoke, and a minimal `TIFFGetVersion` C source). No regeneration was needed; later phases will reuse the same files in place.

## Package Build And Install-Surface Smoke

- Package source tree: committed `safe/` tree at `61f38826b440c30b5099410a52e1af227832622e` (`git log -1 --format=%H -- safe`). `git diff --quiet -- safe` and `git diff --cached --quiet -- safe` both passed before any build.
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist` rebuilt all four canonical binaries (plus three dbgsym sidecars) at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`, architecture `amd64`.
- `safe/scripts/check-packaged-install-surface.sh` ran with the local package-smoke projects and `original/test/images/rgb-3c-8b.tiff`; final line was `verified install surface: extracted package root`. CMake config + pkg-config integration, the `libtiffxx` C++ facade, and the packaged tools all verified clean.

## Package Provenance

- Release tag in local lock: `local-61f38826b440`.
- Package version required and verified: `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- Package names required and verified: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`.
- Each `.deb`'s internal Debian `Package` field was verified against the expected canonical name when the lock was generated.

| Package | Version | Architecture | Filename | SHA-256 | Size |
| --- | --- | --- | --- | --- | ---: |
| `libtiff6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `b5c7477fb5d99989ce034ecb9558cfc951c0fd23b05bd4be17514e0dc5ac0f29` | 641772 |
| `libtiffxx6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` | 12306 |
| `libtiff-dev` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` | 35732 |
| `libtiff-tools` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` | 200508 |

The proof's `libraries[0].port_commit` (`61f38826b440c30b5099410a52e1af227832622e`) equals the local lock's `libraries[0].commit` and the machine-readable safe source commit below.

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

## Override Installation Status

Every per-case result JSON under `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` reports `override_debs_installed: true` and `override_installed_packages` covering all four canonical packages (`libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`). 0 of 135 testcases reported a local override installation failure.

## Commands Executed

- `git -C validator diff --quiet` / `git -C validator diff --cached --quiet` (working-tree pre-checks; only `M workflow.yaml` is permitted by the documented pre-existing exception).
- `git -C validator fetch --tags origin && git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e` (reapply pinned validator checkout; no reclone, no force-reset).
- `git -C validator rev-parse HEAD | grep -qxF 5d908be26e33f071e119ffe1a52e3149f1e5ec4e` (verified pinned commit).
- `make -C validator unit` and `make -C validator check-testcases` (validator unit tests + manifest lint).
- `python3 validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135` (libtiff inventory floor).
- `git diff --quiet -- safe` / `git diff --cached --quiet -- safe` and `git log -1 --format=%H -- safe` (verified `safe/` clean, captured commit pinned in the lock).
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist` (rebuilt all four canonical packages).
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff` (install-surface smoke).
- `rm -rf validator/artifacts/debs/local/libtiff && mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof && find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +` (synthesize override root from freshly built debs).
- Inline `python3` block (per the phase plan): for each canonical package, verified the deb's internal `Package`/`Architecture`, then wrote `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` with deterministic `schema_version: 1`, `mode: "port"`, `generated_at: "1970-01-01T00:00:00Z"`, `libraries[0].commit = git log -1 --format=%H -- safe`, `release_tag = local-<12-char>`.
- `cd validator && bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts`.
- `cd validator && python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports`.
- Audited every `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` for `status: "passed"`, `override_debs_installed: true`, and presence of all four canonical packages in `override_installed_packages`; 0 failures across 135 cases.

## Result Artifact Paths

- Per-case JSON: `validator/artifacts/libtiff-safe/port/results/libtiff/<testcase>.json` (135 files plus `summary.json`).
- Per-case logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/<testcase>.log`.
- Per-case casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/<testcase>.cast` (135 casts).
- Aggregated proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.

## Failed Testcase Ids

None. All 5 source cases and all 130 usage cases passed (`status: "passed"`) under the local override `.deb` set.

## Triage

The baseline run produced zero validator failures and zero local override installation failures, so no triage buckets were populated by this phase. The downstream phases inherit the following empty buckets:

- Source/CLI bucket (phase `impl_source_cli_failures`): empty. No failing source cases (`c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, `tiffinfo-metadata` all passed).
- Usage bucket (phase `impl_usage_runtime_failures`): empty. All 130 usage cases passed.
- Package/provenance bucket (phase `impl_package_provenance_waiver_gate`): empty. Override installation succeeded in every case, the lock matches the on-disk `.deb` set and the safe-source commit, and no package metadata, layout, dependency, maintainer-script, or split fix was required.

If a later re-run uncovers regressions, the same triage shape will be used to populate these buckets.

## Required Safe Fixes

None. The current `safe/` tree built and packaged cleanly, installed under the validator's local override mechanism, and ran the entire `port` matrix without failure. No `safe/debian/`, packaging-script, CMake, pkg-config, header, or install-surface fix was applied in this phase.

## Waivers

No waivers were applied. No validator failure occurred, so the `original` mode matrix was not run in this phase and no validator-bug adjudication was needed.

## Machine Readable

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: 61f38826b440c30b5099410a52e1af227832622e
Checks executed: validator dirty-check + pinned-commit reapply; make unit; make check-testcases; libtiff inventory floor (5/130/135); safe-tree clean check; safe/scripts/build-deb.sh; safe/scripts/check-packaged-install-surface.sh; local override deb tree synthesis; deterministic local-port-debs-lock.json synthesis; validator libtiff port matrix with --record-casts; verify_proof_artifacts with --require-casts; per-case status / override_debs_installed / port_commit audit
Failures found: 0
Override install failures: 0
Waived testcase ids:
Baseline status: clean local override install, clean validator port matrix, zero failures, zero waivers
