# 02-upstream-matrix

- Phase Name: Upstream Test Matrix And Regression Infrastructure
- Implement Phase ID: `impl_upstream_matrix`

## Preexisting Inputs

- `safe/CMakeLists.txt`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/images/`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/tools/CMakeLists.txt`
- `safe/tools/*.c`
- `safe/src/lib.rs`
- `safe/src/core/mod.rs`
- `safe/src/core/directory.rs`
- `safe/src/core/codec.rs`
- `safe/src/core/field_registry.rs`
- `safe/src/core/field_tables.rs`
- `safe/src/core/color.rs`
- `safe/src/core/jpeg.rs`
- `safe/src/rgba.rs`
- `safe/src/strile.rs`
- `safe/capi/tiff_placeholder.c`
- `safe/capi/jpeg_helper.c`
- `safe/capi/external_codec_helper.c`
- `original/test/`
- `original/tools/`

## New Outputs

- Expanded or corrected regression coverage in existing `safe/test/*_regressions.c` and `safe/test/api_*.c`
- Corrected copied shell/CTest harness assets when the failure is in the harness itself rather than in libtiff behavior
- Any missing fixture files under `safe/test/images/` that cannot be synthesized in-code
- Rust/C compatibility fixes and any required copied-tool or build-wiring fixes needed to make the full upstream matrix green

## File Changes

- `safe/CMakeLists.txt`
- `safe/tools/CMakeLists.txt`
- Any affected `safe/tools/*.c`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/test/dirread_regressions.c`
- `safe/test/dirwrite_regressions.c`
- `safe/test/strile_regressions.c`
- Any affected `safe/test/api_*.c`
- Any affected `safe/test/images/*`
- `safe/src/lib.rs`
- `safe/src/core/mod.rs`
- `safe/src/core/directory.rs`
- `safe/src/core/codec.rs`
- `safe/src/core/field_registry.rs`
- `safe/src/core/field_tables.rs`
- `safe/src/core/color.rs`
- `safe/src/core/jpeg.rs`
- `safe/src/rgba.rs`
- `safe/src/strile.rs`
- `safe/capi/tiff_placeholder.c`
- `safe/capi/jpeg_helper.c`
- `safe/capi/external_codec_helper.c`

## Implementation Details

- Use the existing `simple_tests` and tool-driven test registration in `safe/test/CMakeLists.txt` as the canonical CTest surface; do not create a parallel bespoke test driver.
- Use `safe/tools/CMakeLists.txt` and the copied `safe/tools/*.c` sources as the canonical executable surface for shell tests and tool-driven CTest coverage; do not wrap failing tool behavior in bespoke helper scripts.
- Preserve the current `safe/test/common.sh` assumption that the shell tests execute from `safe/build/test` and resolve tools via `../tools`.
- Keep `safe/scripts/run-upstream-shell-tests.sh` as the canonical shell-test discovery/execution entry point. If the failure is in test discovery, filtering, environment setup, or build-tree execution semantics, fix that script in this phase instead of replacing it.
- If a copied shell script, its golden output under `safe/test/refs/*`, or a helper CTest driver (`safe/test/TiffTest.cmake`, `safe/test/TiffSplitTest.cmake`, `safe/test/TiffTestCommon.cmake`) is itself wrong or missing required coverage, fix that harness asset in this phase and rerun both `ctest` and `safe/scripts/run-upstream-shell-tests.sh`.
- If a tool-driven CTest or copied shell test fails because of tool behavior or tool build wiring, fix `safe/CMakeLists.txt`, `safe/tools/CMakeLists.txt`, or the relevant `safe/tools/*.c` in this phase and rerun the full matrix.
- When a failure is about malformed directory/tag input, extend `dirread_regressions.c`.
- When a failure is about directory serialization, custom directories, rational validation, or tag rewrite behavior, extend `dirwrite_regressions.c`.
- When a failure is about strips, tiles, fill order, byte swapping, `TIFFReadFromUserBuffer`, or deferred strile handling, extend `strile_regressions.c`.
- Reserve new `api_*.c` tests for public API contract gaps that are not naturally “malformed input” regressions.
- Keep most safety-sensitive logic in Rust (`safe/src/lib.rs`, `safe/src/core/mod.rs`, `safe/src/core/*.rs`, `safe/src/rgba.rs`, `safe/src/strile.rs`) and use `safe/capi/tiff_placeholder.c` only for C varargs or glue that cannot be expressed in Rust.

## Verification Phases

### `check_upstream_matrix_tester`

- Phase ID: `check_upstream_matrix_tester`
- Type: `check`
- Bounce Target: `impl_upstream_matrix`
- Purpose: Rebuild the safe tree with tools/tests enabled and run the full copied CTest plus shell-test matrix.
- Commands:

```bash
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Debug -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
```

### `check_upstream_matrix_senior`

- Phase ID: `check_upstream_matrix_senior`
- Type: `check`
- Bounce Target: `impl_upstream_matrix`
- Purpose: Inspect the implementor commit and confirm that new reproductions land in the checked-in regression buckets rather than only in ad hoc scripts.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/CMakeLists.txt safe/tools/CMakeLists.txt safe/tools/*.c safe/test/CMakeLists.txt safe/test/Makefile.am safe/test/*.sh safe/test/refs/* safe/test/TiffTest.cmake safe/test/TiffSplitTest.cmake safe/test/TiffTestCommon.cmake safe/scripts/run-upstream-shell-tests.sh safe/test/dirread_regressions.c safe/test/dirwrite_regressions.c safe/test/strile_regressions.c safe/test/api_*.c safe/src safe/capi
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Debug -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure -R 'dirread_regressions|dirwrite_regressions|strile_regressions|test_directory|test_tile_read_write|test_rgba_readers|test_open_options'
```

## Success Criteria

- A fresh `safe/build` with `tiff-tools` and `tiff-tests` enabled passes the copied CTest matrix and `safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build`.
- Any failure reductions land in the checked-in regression buckets or `safe/test/api_*.c`, and remain registered through the existing `safe/test/CMakeLists.txt` and `safe/test/Makefile.am` surfaces.
- Any harness or tool fixes needed to make the matrix truthful are applied in the canonical copied assets rather than via ad hoc wrapper scripts or parallel test drivers.

## Git Commit Requirement

- The implementer must commit work to git before yielding.
