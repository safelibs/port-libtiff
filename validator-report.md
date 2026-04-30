# Usage Runtime Failures Report

## Summary

- Phase: `impl_usage_runtime_failures`
- Final usage-case status: clean. All 130 usage cases passed, including the five Pillow/tool regressions from the previous report.
- Mode: port
- Cases: 135 total, 5 source, 130 usage
- Results: 135 passed, 0 failed
- Casts recorded: 135
- Override installation: 135/135 result JSON files have `override_debs_installed: true`
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`

## Fixes

| Testcase id | Root cause | Fix | Regression |
| --- | --- | --- | --- |
| `usage-python3-pil-tiff-jpeg-compression-info` | JPEG TIFF writes reached the codec path without a working encoder, so Pillow save failed during flush. | Added `safe_tiff_jpeg_encode` through libjpeg and wired Rust JPEG encoding through `safe/src/core/jpeg.rs`. RGB JPEG decode now asks libjpeg for RGB output for Photometric RGB data. | `validator_usage_jpeg_encode` |
| `usage-python3-pil-tiff-tiffcp-jpeg-rows-per-strip` | `tiffcp -c jpeg -r 16` hit the same missing JPEG encoder path. | Same JPEG encode implementation; the tool can now write JPEG-compressed strips while preserving the requested rows-per-strip. | `validator_usage_tools` |
| `usage-python3-pil-tiff-tiff2pdf-conversion` | The custom seek callback returned `fseek` status instead of the current offset, causing TIFF client I/O to corrupt the output stream. | Changed `t2p_seekproc` to return `ftell` after a successful seek and reject negative positions. | `validator_usage_tools` |
| `usage-python3-pil-tiff-tiffcp-tile-convert` | Tiled `tiffcp` output copied/created `RowsPerStrip`, which should not appear in tiled directories. | Stopped setting `ROWSPERSTRIP` in the tiled output branch. | `validator_usage_jpeg_encode`, `validator_usage_tools` |
| `usage-python3-pil-tiff-tiffcp-tile-32x32-convert` | Same tiled-directory metadata issue with explicit tile geometry. | Same `tiffcp` tile metadata fix. | `validator_usage_tools` |

No binary fixtures or reference files were added; the new regressions generate temporary TIFF inputs through the public API and tools.

## Regression Coverage

- `safe/test/validator_usage_jpeg_encode.c` writes a small JPEG-compressed TIFF, reads it back through RGBA APIs, checks JPEG compression and rows-per-strip metadata, and verifies a tiled directory omits `RowsPerStrip`.
- `safe/test/validator_usage_tools.sh` checks `tiff2pdf -o` emits a `%PDF-` file, `tiffcp -t` emits tile metadata without `RowsPerStrip`, and `tiffcp -c jpeg -r 16` writes JPEG compression with `RowsPerStrip: 16`.
- Tests are registered in `safe/test/CMakeLists.txt` and `safe/test/Makefile.am`.

## Commands Executed

- `cargo test --manifest-path safe/Cargo.toml`
- `cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON`
- `cmake --build safe/build --parallel`
- `ctest --test-dir safe/build --output-on-failure`
- `safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build`
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff`
- `rm -rf validator/artifacts/debs/local/libtiff && mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof && find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +`
- `python3 - <<'PY' ... validate package names and versions, then regenerate validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json from local .deb metadata, sizes, and SHA-256 hashes ... PY`
- `cd validator && bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts`
- `cd validator && python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports`
- `python3 - <<'PY' ... assert 135 total cases, 5 source cases, 130 usage cases, and zero unwaived failures ... PY`

## Package Provenance

- Release tag: `local-b5303914957c`
- Override root: `validator/artifacts/debs/local/libtiff/`
- Local port lock commit: `b5303914957cd5f1dee0237f6ee2f92bfa665ec0`
- Proof `port_commit`: `b5303914957cd5f1dee0237f6ee2f92bfa665ec0`
- Override install status: all 135 result JSON files report `override_debs_installed: true`, `port_commit: b5303914957cd5f1dee0237f6ee2f92bfa665ec0`, and installed local packages include `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`.

| Package | Version | Architecture | Filename | SHA-256 | Size |
| --- | --- | --- | --- | --- | ---: |
| `libtiff6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `b5c7477fb5d99989ce034ecb9558cfc951c0fd23b05bd4be17514e0dc5ac0f29` | 641772 |
| `libtiffxx6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` | 12306 |
| `libtiff-dev` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` | 35732 |
| `libtiff-tools` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` | 200508 |

## Validator Result

The final validator matrix passed with no waivers:

```json
{
  "cases": 135,
  "casts": 135,
  "failed": 0,
  "library": "libtiff",
  "mode": "port",
  "passed": 135,
  "source_cases": 5,
  "usage_cases": 130
}
```

The report covers usage failures, Pillow JPEG compression behavior, metadata for tiled output, and the source/usage mix including multipage cases. No multipage regressions remained after the run, and no testcase waivers were applied.

## Machine Readable

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: b5303914957cd5f1dee0237f6ee2f92bfa665ec0
Checks executed: cargo test; CMake Release build with tools/tests; CTest; upstream shell tests; build-deb; packaged install-surface; local override copy and port-lock regeneration; validator port matrix with local override debs/casts; verify_proof_artifacts; no-unwaived-failure audit
Failures found: 0
Waived testcase ids:
