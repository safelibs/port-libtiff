# Source And CLI Failures Report

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: 24ad2b59c73f73a30e974b100e6453644290f10d
Checks executed: safe tree diff check; CMake Release build with tools/tests; CTest; selected upstream shell tests for tiffinfo, tiffdump, tiffcp, ppm2tiff, and fax2tiff; safe build-deb with validator image preload path exercised; packaged install-surface; local override copy and port-lock regeneration with package-name validation; validator port matrix with local override debs/casts; verify_proof_artifacts; source-case and override result JSON audits
Failures found: 5 usage-only: usage-python3-pil-tiff-jpeg-compression-info, usage-python3-pil-tiff-tiff2pdf-conversion, usage-python3-pil-tiff-tiffcp-jpeg-rows-per-strip, usage-python3-pil-tiff-tiffcp-tile-32x32-convert, usage-python3-pil-tiff-tiffcp-tile-convert
Waived testcase ids:

## Summary

- Phase: `impl_source_cli_failures`
- Source/CLI result: no source/CLI failures. All five source cases passed in the refreshed port matrix.
- Mode: port
- Cases: 135 total, 5 source, 130 usage
- Results: 130 passed, 5 failed
- Casts recorded: 135
- Override installation: 135/135 result JSON files have `override_debs_installed: true`
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- No libtiff API/source or regression-test edits were needed because the source-facing validator cases were already passing; this phase added a `safe/scripts/build-deb.sh` Docker Buildx preload guard so the fixed verifier command can find `validator-libtiff-shared:latest` before usage cases run.

## Commands Executed

- `git diff --quiet -- safe`
- `git diff --cached --quiet -- safe`
- `cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON`
- `cmake --build safe/build --parallel`
- `ctest --test-dir safe/build --output-on-failure`
- `safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build --include-regex 'tiff(info|dump|cp)|ppm2tiff|fax2tiff'`
- `LIBTIFF_SAFE_PRELOAD_VALIDATOR_IMAGE=1 safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff`
- `rm -rf validator/artifacts/debs/local/libtiff && mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof && find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +`
- `python3 - <<'PY' ... validate package names and regenerate validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json from local .deb metadata, sizes, and SHA-256 hashes ... PY`
- `cd validator && bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts`
- `cd validator && python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports`
- `python3 - <<'PY' ... assert no unwaived source failures in validator/artifacts/libtiff-safe/port/results/libtiff/*.json ... PY`
- `python3 - <<'PY' ... audit summary, proof port_commit, per-case port_commit, override_debs_installed, and override_installed_packages ... PY`

## Package Provenance

- Release tag: `local-24ad2b59c73f`
- Override root: `validator/artifacts/debs/local/libtiff/`
- Local port lock commit: `24ad2b59c73f73a30e974b100e6453644290f10d`
- Proof `port_commit`: `24ad2b59c73f73a30e974b100e6453644290f10d`
- Override install status: all 135 result JSON files report `override_debs_installed: true`, `port_commit: 24ad2b59c73f73a30e974b100e6453644290f10d`, and installed local packages include `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`.

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

## Source/CLI Testcases

| Testcase id | Kind | Status | Result JSON | Log |
| --- | --- | --- | --- | --- |
| `c-api-read-write` | `source` | passed | `validator/artifacts/libtiff-safe/port/results/libtiff/c-api-read-write.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/c-api-read-write.log` |
| `malformed-tiff-rejection` | `source` | passed | `validator/artifacts/libtiff-safe/port/results/libtiff/malformed-tiff-rejection.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/malformed-tiff-rejection.log` |
| `tiffcp-copy` | `source` | passed | `validator/artifacts/libtiff-safe/port/results/libtiff/tiffcp-copy.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/tiffcp-copy.log` |
| `tiffdump-structure` | `source` | passed | `validator/artifacts/libtiff-safe/port/results/libtiff/tiffdump-structure.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/tiffdump-structure.log` |
| `tiffinfo-metadata` | `source` | passed | `validator/artifacts/libtiff-safe/port/results/libtiff/tiffinfo-metadata.json` | `validator/artifacts/libtiff-safe/port/logs/libtiff/tiffinfo-metadata.log` |

## Waivers

No testcase waivers were applied in this phase. The source/CLI cases passed without libtiff behavior changes, and the five failures above are ordinary libtiff-safe usage compatibility regressions left for the next usage-focused phase; no validator-bug waiver was needed, so the original-mode validator matrix was not run.

## Setup Notes

- `validator/` is a nested checkout and is locally excluded from the parent repository via `.git/info/exclude`.
- `safe/scripts/build-deb.sh` now preloads `validator-libtiff-shared:latest` with `docker buildx build --load` when Docker Buildx uses a non-loading driver such as `docker-container`; the local rerun forced this path with `LIBTIFF_SAFE_PRELOAD_VALIDATOR_IMAGE=1`, while the verifier's normal command uses the script's automatic detection.
- `safe/CMakeLists.txt` was minimally updated before the package build to install uppercase `TIFFConfig*.cmake` aliases alongside the existing `TiffConfig*.cmake` files, matching the generated package-smoke project's `find_package(TIFF CONFIG)` call.
- The generated package-smoke projects are under `validator/artifacts/libtiff-safe/package-smoke/` and were used for the install-surface check.
