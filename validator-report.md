# Final Validator Clean Run — Closure Report

## Summary

- Phase: `impl_final_validator_clean_run`
- Final verification date: 2026-05-03, America/Phoenix
- Validator repository: `https://github.com/safelibs/validator`
- Validator commit: `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`
- Safe source commit tested: `61f38826b440c30b5099410a52e1af227832622e`
- Mode: port
- Library: `libtiff`
- Override root: `validator/artifacts/debs/local/libtiff/`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Aggregated proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Final status: **clean — 135 of 135 testcases passed (5 source + 130 usage), 0 failed, 135 of 135 casts recorded, 0 overrides missing, 0 waivers, no remaining unwaived validator or local compatibility issue**.

## Validator Checkout

- The pre-existing `validator/` checkout was preserved (no clone, no force-reset).
- Pinned commit reapplied via `git -C validator fetch --tags origin && git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e`.
- `git -C validator rev-parse HEAD` → `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`.
- Validator working tree state: `M workflow.yaml` only. This is the same pre-existing local edit documented in phases 1–4 (a workflow-only rename of internal `port-04-test` references to `port` to match the validator's stable mode name); it does not touch `tests/`, `repositories.yml`, `tools/`, `test.sh`, the `_shared` runtime helpers, the package-smoke fixtures, or the runner code, and has no effect on the matrix runtime path.
- No validator source files, tests, manifests, or runner code were modified by this phase.

## Safe Source Commit

- `git log -1 --format=%H -- safe` → `61f38826b440c30b5099410a52e1af227832622e`.
- `git diff --quiet -- safe` and `git diff --cached --quiet -- safe` both returned 0 immediately before the rebuild and the validator run; the safe tree was clean with no in-flight modifications.
- No additional `safe/src/`, `safe/capi/`, `safe/tools/`, `safe/test/`, `safe/scripts/`, `safe/debian/`, `safe/abi/`, `safe/CMakeLists.txt`, or `packaging/` change was needed in this phase: every prior gate (phase 1 baseline, phase 2 package waiver gate, phase 3 source/CLI failures, phase 4 usage runtime failures) was already clean at this commit, the full safe compatibility matrix re-ran clean against this commit, and the rebuilt `.deb` set produced the same byte-identical artifact hashes already recorded in the local port lock.

## Package Artifacts

Override root `validator/artifacts/debs/local/libtiff/` and `safe/dist/` agree byte-for-byte after the final `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist` and the deterministic copy into the override leaf:

| Package | Filename | Architecture | Version | SHA-256 | Size (bytes) |
| --- | --- | --- | --- | --- | ---: |
| `libtiff6` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `amd64` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `b5c7477fb5d99989ce034ecb9558cfc951c0fd23b05bd4be17514e0dc5ac0f29` | 641772 |
| `libtiffxx6` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `amd64` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` | 12306 |
| `libtiff-dev` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `amd64` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` | 35732 |
| `libtiff-tools` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `amd64` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` | 200508 |

The four canonical packages match the validator's `libtiff` `apt_packages` list exactly. `unported_original_packages` is `[]`. `safe/dist/` also contains the corresponding `*-dbgsym_*.ddeb` debug companions, which the validator deliberately ignores (non-canonical packages are ignored by `tools/run_matrix.py`).

## Lock And Proof Provenance

- Local port lock release tag: `local-61f38826b440`.
- `lock.libraries[0].commit` = `61f38826b440c30b5099410a52e1af227832622e` = `git log -1 --format=%H -- safe` = `proof.libraries[0].port_commit` = every per-case `result["port_commit"]`.
- Each `lock.libraries[0].debs[]` entry's `filename`, `size`, and `sha256` was recomputed against the on-disk `.deb` files under `validator/artifacts/debs/local/libtiff/` after the final rebuild and matched the table above byte-for-byte.
- Aggregated proof totals (`validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`):

```json
{
  "libraries": 1,
  "cases": 135,
  "source_cases": 5,
  "usage_cases": 130,
  "passed": 135,
  "failed": 0,
  "casts": 135
}
```

## Counts

| Source cases | Usage cases | Total cases | Passed | Failed | Casts |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 130 | 135 | 135 | 0 | 135 |

Source cases (5): `c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, `tiffinfo-metadata`. Usage cases (130): every remaining libtiff testcase id under `validator/tests/libtiff/tests/cases/usage/`, covering read-path pixel/mode (RGB/RGBA, palette, YCbCr, CMYK, OJPEG, alpha, region), save/roundtrip (`TIFFSetField`, directory write, strile write, flush/rewrite), compression (LZW, deflate, PackBits, Fax, JPEG/OJPEG, LZMA, ZSTD, WEBP, LERC, predictor, pseudo-tags), metadata/tag (defaults, rationals, ASCII counts, field tables, `TIFFGetFieldDefaulted`), multipage / SubIFD / BigTIFF (directory traversal/write offsets, next-directory links, BigTIFF header, `tiffsplit`), and CLI usage (`tiff2bw`, `tiff2pdf`, `tiffcrop`, `tiffmedian`, `tiffsplit`, `tiffcp`, `tiffdump`, `tiffinfo`).

Per-case JSON for every one of the 135 testcases reports `status: "passed"`, `override_debs_installed: true`, all four canonical packages in `override_installed_packages`, and `port_commit == 61f38826b440c30b5099410a52e1af227832622e`. 0 of 135 testcases reported a local override installation failure.

## Checks Executed

Safe compatibility matrix (all green at `safe` commit `61f38826b440c30b5099410a52e1af227832622e`):

- `git diff --quiet -- safe` and `git diff --cached --quiet -- safe` (clean tree pre-checks).
- `cargo test --manifest-path safe/Cargo.toml` (Rust core unit tests).
- `cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON` and `cmake --build safe/build --parallel` (full Release build with tools and tests).
- `python3 safe/scripts/check-public-surface.py --check --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt` (public ABI surface).
- `ctest --test-dir safe/build --output-on-failure` (150 of 150 tests passed).
- `safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build` (84 upstream shell tests passed).
- `rm -rf safe/build/link-compat && safe/scripts/build-link-compat-objects.sh && safe/scripts/link-and-run-link-compat.sh` (link compatibility against original-libtiff-headers objects, plus the hardened `libtiffxx` link path from earlier in this phase chain).
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist` (rebuilt the four canonical `.deb` files plus `*-dbgsym` debug companions).
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff` (CMake target, CMake targetless, pkg-config, and tiffxx smoke against the staged `.deb` payload).
- `LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh` (downstream smoke: every dependent client image exercised against the rebuilt `.deb` set).

Validator final port matrix (all green at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`):

- `git -C validator diff --quiet`, `git -C validator diff --cached --quiet` — only the documented `M workflow.yaml` exception.
- `git -C validator fetch --tags origin && git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e && git -C validator rev-parse HEAD` (pinned-commit reapply, no reclone, no force-reset).
- `rm -rf validator/artifacts/debs/local/libtiff && mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof && find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +` (refresh override leaf from final rebuild).
- Inline Python: regenerated `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` from the on-disk `.deb` filenames, `dpkg-deb -f Architecture` values, byte sizes, and recomputed SHA-256s, with `release_tag = local-<safe_commit[:12]>` and `commit = git log -1 --format=%H -- safe`.
- `cd validator && make unit` (110 unit tests passed).
- `make check-testcases` and `python3 tools/testcases.py --config repositories.yml --tests-root tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135` (manifest + header lint).
- `bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts` (full libtiff port matrix with cast recording).
- `python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports` (proof regenerated and verified against the per-case results, casts, and lock).

Cross-checks at the end of the run:

- `lock.libraries[0].commit ↔ HEAD(safe) ↔ proof.libraries[0].port_commit ↔ every result.port_commit` all equal `61f38826b440c30b5099410a52e1af227832622e`.
- 135 of 135 per-case results report `status: "passed"`, `override_debs_installed: true`, the four canonical packages in `override_installed_packages`, and `port_commit == 61f38826b440c30b5099410a52e1af227832622e`.
- Cast count: 135 of 135 (`port/casts/libtiff/*.cast` matches the result count one-for-one).

## Failures Found And Final Disposition

| Phase | Bucket | Failures found | Final disposition |
| --- | --- | ---: | --- |
| `impl_validator_baseline` | Whole-matrix baseline | 0 | Clean baseline at `61f38826b440`; no fix or waiver. |
| `impl_package_provenance_waiver_gate` | Override install / canonical-package mismatch | 0 | All 4 canonical packages installed in every case; no fix or waiver. |
| `impl_source_cli_failures` | Source / CLI testcases | 0 | All 5 source testcases (`c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, `tiffinfo-metadata`) and CLI tool testcases passed; no fix or waiver. |
| `impl_usage_runtime_failures` | Usage testcases (read-path, save/roundtrip, compression, metadata, multipage/SubIFD/BigTIFF, CLI usage) | 0 | All 130 usage testcases passed; no fix or waiver. |
| `impl_final_validator_clean_run` | Final catch-all + closure | 0 | Final rebuild and rerun produced 135/135 passed, 0 failed, 0 missing overrides, 135/135 casts; no remaining unwaived issue; no fix or waiver. |

Per-case `status == "failed"` count across `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`: **0**.

## Safe Fixes Applied In This Phase

None. The full safe compatibility matrix and the validator port matrix were both clean at the entering safe commit `61f38826b440c30b5099410a52e1af227832622e`. No `safe/src/*`, `safe/capi/*`, `safe/tools/*`, `safe/test/*`, `safe/scripts/*`, `safe/debian/*`, `safe/abi/*`, `safe/CMakeLists.txt`, or `packaging/*` change was warranted by any final failure, because there was no final failure to consume. Per the phase contract, no broadening of scope was attempted.

The cumulative safe-tree state at `61f38826b440c30b5099410a52e1af227832622e` already incorporates the libtiffxx link compatibility hardening committed earlier in the chain (commit `61f38826b440 — impl_final_validator_clean_run: harden libtiffxx link compatibility check`), so the `safe/scripts/link-and-run-link-compat.sh` step exercises the hardened path without any further code change in this final phase.

## Waivers

None. `summary["failed"] == 0`, every per-case `status == "passed"`, every per-case `override_debs_installed == true`, every per-case `port_commit == 61f38826b440c30b5099410a52e1af227832622e`. There is no failing testcase to compare against `original` mode, no log to cite, and no validator expectation to challenge. No validator-bug waiver was adjudicated.

The `Waived testcase ids:` line in the machine-readable block below is therefore intentionally empty, and the `check_final_validator_tester` invariant `summary["failed"] == 0 if no waived ids else every failed id is in the waived set` reduces to the strict `summary["failed"] == 0` branch, which holds.

## Result Artifact Paths

- Per-case JSON: `validator/artifacts/libtiff-safe/port/results/libtiff/<testcase>.json` (135 files plus `summary.json`).
- Per-case logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/<testcase>.log`.
- Per-case casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/<testcase>.cast` (135 casts).
- Aggregated proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Override `.deb` tree: `validator/artifacts/debs/local/libtiff/{libtiff6,libtiffxx6,libtiff-dev,libtiff-tools}_*.deb`.
- Reference `safe/dist/` build: `safe/dist/{libtiff6,libtiffxx6,libtiff-dev,libtiff-tools}_*.deb` plus `*-dbgsym_*.ddeb` debug companions.

## Failed Testcase Ids

None. All 5 source cases and all 130 usage cases passed (`status: "passed"`) under the local override `.deb` set after the final rebuild, with `override_debs_installed: true` everywhere and `port_commit == 61f38826b440c30b5099410a52e1af227832622e` everywhere.

## Machine Readable

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: 61f38826b440c30b5099410a52e1af227832622e
Checks executed: validator dirty-check + pinned-commit reapply; safe-tree clean check; cargo test; cmake Release build with tools and tests; public-surface ABI check; ctest; upstream shell tests; link-compat object build and link/run; safe/dist .deb rebuild; packaged install surface (cmake-target, cmake-targetless, pkg-config, tiffxx smoke); test-original downstream smoke; override leaf refresh from safe/dist; lock regeneration with sha256/size/architecture from on-disk .deb files; validator make unit; validator check-testcases; libtiff-only testcases lint; full validator port matrix with --record-casts; verify_proof_artifacts with --require-casts; per-case status / override_debs_installed / override_installed_packages / port_commit audit; lock.commit ↔ HEAD(safe) ↔ proof.port_commit ↔ result.port_commit cross-check; cast count audit
Failures found: 0
Source-case failures: 0
Usage-case failures: 0
Override install failures: 0
Cast recording failures: 0
Waived testcase ids:
Final status: clean — 135 of 135 testcases passed (5 source + 130 usage), 0 failed, 135 of 135 casts recorded, 0 overrides missing, 0 waivers, no remaining unwaived validator or local compatibility issue at safe commit 61f38826b440c30b5099410a52e1af227832622e against validator commit 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
