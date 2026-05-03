# Usage-Case Runtime Regressions

## Summary

- Phase: `impl_usage_runtime_failures`
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
- Usage-case gate status: **no usage failures**. The phase 1 baseline, the phase 2 package gate, and the phase 3 source/CLI gate each produced 0 of 135 failures with `override_debs_installed: true` in every per-case result. All 130 usage testcases (read-path, save/roundtrip, compression, metadata, multipage/SubIFD/BigTIFF, CLI usage) passed under the local override `.deb` set pinned at safe-source commit `61f38826b440c30b5099410a52e1af227832622e`. No `safe/src/`, `safe/capi/`, `safe/tools/`, `safe/test/`, ABI, or symbol-map change was warranted by this phase, no validator-bug waiver was adjudicated, and `safe/`, `safe/dist/*.deb`, the override `.deb` tree, and the local port lock were not regenerated because the inputs from phases 2 and 3 already match byte-for-byte.

## Validator Checkout

- The pre-existing `validator/` checkout was preserved (no clone, no force-reset).
- Pinned commit reapplied via `git -C validator fetch --tags origin && git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e`; `git -C validator rev-parse HEAD` → `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`.
- Validator working tree state: `M workflow.yaml` only. This is the same pre-existing local edit documented in phases 1, 2, and 3 (an upstream-superseded rename of internal `port-04-test` references to `port` in the validator's auxiliary `workflow.yaml`); it does not touch `tests/`, `repositories.yml`, `tools/`, or `test.sh` and has no effect on the matrix runtime path. No validator source files, tests, manifests, or runner code were modified by this phase.

## Usage Case Status

All 130 usage testcases enumerated by the libtiff inventory passed in the phase 2 port matrix and remain passing under the local override `.deb` set pinned at safe-source commit `61f38826b440c30b5099410a52e1af227832622e`. No fix or waiver was applied. By behavior class:

| Class | Coverage | Status |
| --- | --- | --- |
| Read-path pixel/mode (RGB/RGBA, palette, YCbCr, CMYK, OJPEG, alpha, region) | exercised across `tiff2bw-*`, `tiff2pdf-*`, `tiffcrop-*`, `pillow-*`, `vips-*`, `imagemagick-*`, `pdfio-*` usage testcases | passed |
| Save/roundtrip (`TIFFSetField`, directory write, strile write, flush/rewrite) | exercised across `tiffcp-*`, `tiff2pdf-*`, `tiffcrop-*`, `pillow-save-*`, `vips-save-*` usage testcases | passed |
| Compression (LZW, deflate, PackBits, Fax, JPEG/OJPEG, LZMA, ZSTD, WEBP, LERC, predictor, pseudo-tags) | exercised across `tiffcp-compress-*`, `pillow-compression-*`, `vips-compression-*`, `imagemagick-compress-*`, `gdal-*` usage testcases | passed |
| Metadata/tag (defaults, rationals, ASCII counts, field tables, `TIFFGetFieldDefaulted`) | exercised across `tiffinfo-*`, `tiffdump-*`, `pillow-metadata-*`, `vips-metadata-*`, `pdfio-metadata-*` usage testcases | passed |
| Multipage / SubIFD / BigTIFF (directory traversal/write offsets, next-directory links, BigTIFF header, `tiffsplit`) | exercised across `tiffcp-multipage-*`, `tiffsplit-*`, `tiff2pdf-multipage-*`, `pillow-multipage-*`, `vips-multipage-*`, `bigtiff-*` usage testcases | passed |
| CLI usage (shipped tool integration: `tiff2bw`, `tiff2pdf`, `tiffcrop`, `tiffmedian`, `tiffsplit`, `tiffcp`, `tiffdump`, `tiffinfo`) | exercised across the matching `tiff*-*` usage testcases | passed |

Per-case JSON for every usage testcase reports `status: "passed"`, `override_debs_installed: true`, all four canonical packages (`libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`) in `override_installed_packages`, and `port_commit == 61f38826b440c30b5099410a52e1af227832622e`.

## Counts

| Source cases | Usage cases | Total cases | Passed | Failed | Casts |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 130 | 135 | 135 | 0 | 135 |

Source cases (5): `c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, `tiffinfo-metadata`. Usage cases (130): all remaining libtiff testcase ids under `validator/tests/libtiff/` enumerated by the validator inventory; every one is recorded as `passed` in `validator/artifacts/libtiff-safe/port/results/libtiff/`.

Proof totals match the phase 2/3 result summary (no rerun was required because `safe/` did not change in this phase):

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

Every per-case result JSON under `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` reports `override_debs_installed: true` with all four canonical packages (`libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`) in `override_installed_packages`. 0 of 135 testcases reported a local override installation failure. The 130 usage cases were exercised against the safe override packages — no validator failure exposed a read-path mismatch, save/roundtrip drift, compression failure, metadata/tag drift, multipage/SubIFD/BigTIFF traversal fault, or CLI tool regression that would have warranted a `safe/src/rgba.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/core/directory.rs`, `safe/src/core/field_registry.rs`, `safe/src/core/field_tables.rs`, `safe/capi/tiff_placeholder.c`, or `safe/tools/*.c` change.

## Lock And Proof Provenance

- Local port lock release tag: `local-61f38826b440`.
- `libraries[0].commit` in `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` equals `git log -1 --format=%H -- safe` (`61f38826b440c30b5099410a52e1af227832622e`), equals `proof.libraries[0].port_commit`, equals every per-case `result["port_commit"]`.
- Each `debs[]` entry's `filename`, `size`, and `sha256` was recomputed against the on-disk `.deb` files under `validator/artifacts/debs/local/libtiff/` and matched exactly:
  - `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` → 641772 bytes, sha256 `b5c7477fb5d99989ce034ecb9558cfc951c0fd23b05bd4be17514e0dc5ac0f29`.
  - `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` → 12306 bytes, sha256 `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141`.
  - `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` → 35732 bytes, sha256 `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323`.
  - `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` → 200508 bytes, sha256 `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b`.
- Because no usage/runtime fix was applied, no `safe/dist/*.deb` rebuild, no override-tree refresh, and no lock regeneration was performed in this phase. The lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` and the proof at `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json` are inherited unchanged from phases 2 and 3.

## Commands Executed

- `git -C validator diff --quiet` / `git -C validator diff --cached --quiet` (working-tree pre-checks; only `M workflow.yaml` is permitted by the documented pre-existing exception).
- `git -C validator fetch --tags origin && git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e` (reapply pinned validator checkout; no reclone, no force-reset).
- `git -C validator rev-parse HEAD | grep -qxF 5d908be26e33f071e119ffe1a52e3149f1e5ec4e` (verified pinned commit).
- `git diff --quiet -- safe` / `git diff --cached --quiet -- safe` (verified `safe/` clean) and `git log -1 --format=%H -- safe` → `61f38826b440c30b5099410a52e1af227832622e`.
- Audit pass over every `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`: each file reports `status: "passed"`, `override_debs_installed: true`, the four canonical packages in `override_installed_packages`, and `port_commit == 61f38826b440c30b5099410a52e1af227832622e`. Counted 130 non-source testcases, 0 with a non-passed status, 0 with a missing override, 0 with a mismatched `port_commit`.
- Lock and proof reverification: recomputed sha256/size for each `.deb` under `validator/artifacts/debs/local/libtiff/` and matched against the lock; cross-checked `lock.libraries[0].commit ↔ HEAD(safe) ↔ proof.libraries[0].port_commit ↔ result.port_commit`.

## Result Artifact Paths

- Per-case JSON: `validator/artifacts/libtiff-safe/port/results/libtiff/<testcase>.json` (135 files plus `summary.json`).
- Per-case logs: `validator/artifacts/libtiff-safe/port/logs/libtiff/<testcase>.log`.
- Per-case casts: `validator/artifacts/libtiff-safe/port/casts/libtiff/<testcase>.cast` (135 casts).
- Aggregated proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.

## Failed Testcase Ids

None. All 5 source cases and all 130 usage cases passed (`status: "passed"`) under the local override `.deb` set.

## Triage

The phase 1 baseline produced zero validator failures, the phase 2 package gate produced zero override installation failures, and the phase 3 source/CLI gate produced zero source-case failures. The usage-case bucket inherited by this phase was already empty across read-path, save/roundtrip, compression, metadata, multipage/SubIFD/BigTIFF, and CLI usage classes, and after revalidation it remains empty. No regression test was added under `safe/test/` (no new `validator_usage_*.c` or `*.sh`), no fix was applied to `safe/src/rgba.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/core/directory.rs`, `safe/src/core/field_registry.rs`, `safe/src/core/field_tables.rs`, `safe/capi/tiff_placeholder.c`, or `safe/tools/*.c`, no ABI/symbol-map change was made under `safe/capi/libtiff-safe.map`, `safe/abi/public-surface.json`, or `safe/abi/platform-excluded-linux.txt`, and no validator-bug waiver was adjudicated in `original` mode (per the phase contract, the original-package validator behavior is needed only when proving a validator-bug waiver, and there is nothing to waive).

If a later re-run uncovers usage regressions, this phase will repopulate the table above with `failed` rows, attach one minimal regression per root cause under `safe/test/validator_usage_*.{c,sh}` (or extend existing buckets such as `safe/test/api_*.c`, `safe/test/dirwrite_regressions.c`, `safe/test/dirread_regressions.c`, `safe/test/strile_regressions.c`, `safe/test/codec_regressions.c`), commit the fix, rebuild `.deb`s, regenerate the lock against the post-fix safe-source commit, rerun the matrix, and update the report with the new `port_commit`.

## Required Safe Fixes

None. The `safe/` tree at `61f38826b440c30b5099410a52e1af227832622e` already passes every usage case under the validator's local override mechanism. No `safe/src/rgba.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/core/directory.rs`, `safe/src/core/field_registry.rs`, `safe/src/core/field_tables.rs`, `safe/capi/tiff_placeholder.c`, `safe/tools/tiff2bw.c`, `safe/tools/tiff2pdf.c`, `safe/tools/tiffcrop.c`, `safe/tools/tiffmedian.c`, `safe/tools/tiffsplit.c`, `safe/tools/tiffcp.c`, `safe/tools/tiffdump.c`, `safe/tools/tiffinfo.c`, `safe/test/*.c`, `safe/test/*.sh`, `safe/test/CMakeLists.txt`, `safe/test/Makefile.am`, `safe/test/common.sh`, `safe/capi/libtiff-safe.map`, `safe/abi/public-surface.json`, or `safe/abi/platform-excluded-linux.txt` change was applied in this phase.

## Waivers

No waivers were applied. With `summary["failed"] == 0` and zero unwaived usage-case failures, there is nothing to adjudicate. The `Waived testcase ids:` line below is therefore intentionally empty and the downstream final-clean phase must not treat any testcase id as waived.

## Usage-Case Failure Adjudication

- Inputs consumed in place (no rebuild, no reclone): the phase 2 validator run's `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`, `summary.json`, `casts/`, `logs/`, `proof/libtiff-safe-port-proof.json`, `proof/local-port-debs-lock.json`, and the `validator/artifacts/debs/local/libtiff/*.deb` tree, all carried forward unchanged through phase 3.
- Pre-validator-command checkout reapply (no reclone, no force-reset): `git -C validator diff --quiet`, `git -C validator diff --cached --quiet`, `git -C validator fetch --tags origin`, `git -C validator checkout 5d908be26e33f071e119ffe1a52e3149f1e5ec4e`, `git -C validator rev-parse HEAD` → `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`. The `M workflow.yaml` exception in the validator working tree was preserved.
- Usage-case audit (per behavior class): all 130 usage testcases report `status: "passed"`, `override_debs_installed: true`, all four canonical packages in `override_installed_packages`, and `port_commit == 61f38826b440c30b5099410a52e1af227832622e`. No read-path pixel/mode mismatch, no save/roundtrip failure, no compression failure, no metadata/tag failure, no multipage/SubIFD/BigTIFF failure, and no CLI usage failure was exposed.
- Lock provenance reverification: lock filename/size/sha256 recomputed from on-disk `.deb` files and matched (`b5c7477f…` / 641772 for `libtiff6`, `4e8990c2…` / 12306 for `libtiffxx6`, `c18f9474…` / 35732 for `libtiff-dev`, `10df893f…` / 200508 for `libtiff-tools`). `lock.commit ↔ HEAD(safe) ↔ proof.port_commit ↔ result.port_commit` cross-check passed. No regeneration of the `.deb` tree or lock was performed because `safe/` did not change in this phase, in line with the rule "Rebuild packages only when `safe/` changes, not to align the lock with a report-only commit."
- Validator-bug waiver adjudication: not triggered. With zero usage-case failures there is no failing testcase to compare against `original` mode, no log to cite, and no validator expectation to challenge.

## Machine Readable

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: 61f38826b440c30b5099410a52e1af227832622e
Checks executed: validator dirty-check + pinned-commit reapply; safe-tree clean check; per-case status / override_debs_installed / override_installed_packages / port_commit audit across all 130 usage testcases; behavior-class enumeration for read-path, save/roundtrip, compression, metadata, multipage/SubIFD/BigTIFF, and CLI usage; lock filename/size/sha256 reverification against on-disk .deb files; lock.commit ↔ HEAD(safe) ↔ proof.port_commit ↔ result.port_commit cross-check
Failures found: 0
Usage-case failures: 0
Override install failures: 0
Waived testcase ids:
Usage-case gate status: no usage failures; all 130 usage cases (read-path, save/roundtrip, compression, metadata, multipage/SubIFD/BigTIFF, CLI usage) passed under the local override .deb set pinned at safe-source commit 61f38826b440c30b5099410a52e1af227832622e; no safe/, packaging, ABI, symbol-map, regression-test, or waiver change applied
