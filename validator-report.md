# Validator Baseline Report

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: e9fe9a8b468c1ce8e097186d9e1290cb586a4b95
Checks executed: validator unit/check-testcases/libtiff inventory; safe build-deb; packaged install-surface; validator port matrix with local override debs and casts; verify_proof_artifacts
Failures found: 5: usage-python3-pil-tiff-jpeg-compression-info, usage-python3-pil-tiff-tiff2pdf-conversion, usage-python3-pil-tiff-tiffcp-jpeg-rows-per-strip, usage-python3-pil-tiff-tiffcp-tile-32x32-convert, usage-python3-pil-tiff-tiffcp-tile-convert
Waived testcase ids:

## Summary

- Mode: port
- Cases: 135 total, 5 source, 130 usage
- Results: 130 passed, 5 failed
- Casts recorded: 135
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`

## Commands Executed

- `make -C validator unit`
- `make -C validator check-testcases`
- `python3 validator/tools/testcases.py --config validator/repositories.yml --tests-root validator/tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135`
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff`
- `cd validator && bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts`
- `cd validator && python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports`

## Package Provenance

- Release tag: `local-e9fe9a8b468c`
- Override root: `validator/artifacts/debs/local/libtiff/`

| Package | Version | Architecture | Filename | SHA-256 | Size |
| --- | --- | --- | --- | --- | ---: |
| `libtiff6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `34c43c101aefec13d1f19795d570159529b4f70d910530a4a765915575e84289` | 641192 |
| `libtiffxx6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` | 12306 |
| `libtiff-dev` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` | 35732 |
| `libtiff-tools` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `c5b762edaacb9526b619a180ed414017d8c3c85ac4b79b09135c3b5fa2a9e42a` | 200432 |

## Failed Testcases

| Testcase id | Kind | Result JSON | Log | First symptom | Initial triage |
| --- | --- | --- | --- | --- | --- |
| `usage-python3-pil-tiff-jpeg-compression-info` | `usage` | `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-tiff-jpeg-compression-info.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-tiff-jpeg-compression-info.log` | TIFFFlushData: Codec encode failed.; Traceback (most recent call last):; raise OSError(msg); OSError: encoder error -9 when writing image file | JPEG encode/write path: Pillow save with compression='jpeg' reaches libtiff and fails during flush. |
| `usage-python3-pil-tiff-tiff2pdf-conversion` | `usage` | `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-tiff-tiff2pdf-conversion.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-tiff-tiff2pdf-conversion.log` | Traceback (most recent call last):; AssertionError: b'\x00\x00\x00\x08\x00\x06\x10\x00' | tiff2pdf output generation: command exits 0 but file header is not %PDF-. |
| `usage-python3-pil-tiff-tiffcp-jpeg-rows-per-strip` | `usage` | `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-tiff-tiffcp-jpeg-rows-per-strip.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-tiff-tiffcp-jpeg-rows-per-strip.log` | TIFFFlushData: Codec encode failed.; TIFFFlushData: Codec encode failed. | JPEG encode/write path in tiffcp with explicit RowsPerStrip. |
| `usage-python3-pil-tiff-tiffcp-tile-32x32-convert` | `usage` | `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-tiff-tiffcp-tile-32x32-convert.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-tiff-tiffcp-tile-32x32-convert.log` | Traceback (most recent call last):; AssertionError: tiled TIFF must not have RowsPerStrip | Tile directory/write semantics: tiled output still exposes RowsPerStrip. |
| `usage-python3-pil-tiff-tiffcp-tile-convert` | `usage` | `validator/artifacts/libtiff-safe/port/results/libtiff/usage-python3-pil-tiff-tiffcp-tile-convert.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/usage-python3-pil-tiff-tiffcp-tile-convert.log` | Traceback (most recent call last):; AssertionError: expected no RowsPerStrip in tiled TIFF | Tile directory/write semantics: tiled output still exposes RowsPerStrip. |

## Waivers

No testcase waivers were applied in this phase. The five failures above are left as baseline compatibility regressions for follow-up phases.

## Setup Notes

- `validator/` is a nested checkout and is locally excluded from the parent repository via `.git/info/exclude`.
- `safe/CMakeLists.txt` was minimally updated before the package build to install uppercase `TIFFConfig*.cmake` aliases alongside the existing `TiffConfig*.cmake` files, matching the generated package-smoke project's `find_package(TIFF CONFIG)` call.
- The generated package-smoke projects are under `validator/artifacts/libtiff-safe/package-smoke/` and were used for the install-surface check.
