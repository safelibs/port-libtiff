# 03-link-package-surface

- Phase Name: Link, Install, And Debian Replacement Surface
- Implement Phase ID: `impl_package_link_surface`

## Preexisting Inputs

- `safe/scripts/check-public-surface.py`
- `safe/abi/public-surface.json`
- `safe/abi/public-surface.inputs.json`
- `safe/abi/platform-excluded-linux.txt`
- `safe/scripts/build-link-compat-objects.sh`
- `safe/scripts/link-and-run-link-compat.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/capi/libtiff-safe.map`
- `safe/capi/libtiffxx-safe.map`
- `safe/debian/control`
- `safe/debian/rules`
- `safe/debian/changelog`
- `safe/debian/libtiff6.symbols`
- `safe/debian/libtiffxx6.symbols`
- `safe/debian/*.install`
- `safe/include/tiff.h`
- `safe/include/tif_config.h`
- `safe/include/tiffconf.h`
- `safe/include/tiffio.h`
- `safe/include/tiffio.hxx`
- `safe/include/tiffvers.h`
- `safe/pkgconfig/libtiff-4.pc.in`
- `safe/cmake/TiffConfig.cmake.in`
- `safe/tools/CMakeLists.txt`
- `safe/tools/*.c`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- `safe/test/api_handle_smoke.c`
- `safe/test/api_directory_read_smoke.c`
- `safe/test/api_field_registry_smoke.c`
- `safe/test/api_strile_smoke.c`
- `safe/test/link_compat_logluv_smoke.c`
- `safe/test/install/tiffxx_staged_smoke.cpp`
- `safe/CMakeLists.txt`
- `original/build/libtiff/libtiff.so.6.0.1`
- `original/build/libtiff/libtiffxx.so.6.0.1`
- `original/build/test_cmake/`
- `original/build/test_cmake/test.c`
- `original/build/test_cmake_no_target/`
- `original/build/test_cmake_no_target/test.c`
- `original/build-step2/`
- `original/build-step2/test/CMakeFiles/*/link.txt`
- `original/test/`

## New Outputs

- Updated link-compat and package-surface scripts
- Corrected copied shell/CTest harness assets when release-build or install-surface validation exposes a harness bug
- Updated Debian metadata, copied-tool install wiring, and symbol files if package replacement semantics or payloads need adjustment
- Release-built safe packages in `safe/dist/` as verification artifacts

## File Changes

- `safe/scripts/build-link-compat-objects.sh`
- `safe/scripts/link-and-run-link-compat.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/debian/control`
- `safe/debian/rules`
- `safe/debian/changelog`
- `safe/debian/libtiff6.symbols`
- `safe/debian/libtiffxx6.symbols`
- `safe/debian/*.install`
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
- `safe/include/tiff.h`
- `safe/include/tif_config.h`
- `safe/include/tiffconf.h`
- `safe/include/tiffio.h`
- `safe/include/tiffio.hxx`
- `safe/include/tiffvers.h`
- `safe/pkgconfig/libtiff-4.pc.in`
- `safe/cmake/TiffConfig.cmake.in`
- `safe/test/api_handle_smoke.c`
- `safe/test/api_directory_read_smoke.c`
- `safe/test/api_field_registry_smoke.c`
- `safe/test/api_strile_smoke.c`
- `safe/test/link_compat_logluv_smoke.c`
- `safe/test/install/tiffxx_staged_smoke.cpp`
- If symbol/export fixes are needed: `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/src/lib.rs`, `safe/capi/tiffxx_placeholder.cxx`
- If link/package verification proves the phase-1 ABI contract itself incomplete: `safe/scripts/check-public-surface.py`, `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`, `safe/abi/platform-excluded-linux.txt`

## Implementation Details

- Keep the current package names and Ubuntu replacement semantics: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`, with version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- Keep package verification package-based, not just staged-install based. The final runtime checks must continue to install the built `.deb` files.
- Consume the checked-in phase-1 ABI contract as input in this phase. Validate it with `check-public-surface.py --check`, but do not regenerate `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`, or `safe/abi/platform-excluded-linux.txt` here unless a verifier proved that the contract itself is wrong.
- Preserve the current object-link flow that consumes `original/build-step2/test/CMakeFiles/*/link.txt` and relinks those objects against the safe shared libraries instead of reconstructing link lines heuristically.
- Treat `safe/test/api_handle_smoke.c`, `safe/test/api_directory_read_smoke.c`, `safe/test/api_field_registry_smoke.c`, `safe/test/api_strile_smoke.c`, `safe/test/link_compat_logluv_smoke.c`, and `safe/test/install/tiffxx_staged_smoke.cpp` as the canonical checked-in relink/install consumers compiled by `safe/scripts/build-link-compat-objects.sh`; update those files in place instead of adding new one-off probes.
- Because `safe/scripts/link-and-run-link-compat.sh` only auto-builds objects when `safe/build/link-compat/objects` does not already exist, every verification path in this phase must explicitly delete `safe/build/link-compat` or rerun `safe/scripts/build-link-compat-objects.sh` before invoking it; stale `.o` files are not acceptable evidence.
- Treat `safe/tools/CMakeLists.txt` as the canonical installed-tool and manpage surface for `libtiff-tools`; if package verification finds missing binaries, missing manpages, broken tool-target wiring, or release-build tool failures, fix that file and the relevant `safe/tools/*.c` sources in this phase instead of compensating after install.
- Keep `safe/scripts/run-upstream-shell-tests.sh` as the canonical shell-test runner in this phase too. If release-mode validation exposes a bug in shell-test discovery, environment setup, or execution orchestration, fix that script here and rerun the matrix.
- Because this phase reruns the full copied CTest and shell-test matrix in release mode before link/package checks, it also owns any harness fixes required to make that matrix truthful and executable in the release/install configuration. If the failure is in `safe/test/*.sh`, `safe/test/refs/*`, `safe/test/TiffTest.cmake`, `safe/test/TiffSplitTest.cmake`, or `safe/test/TiffTestCommon.cmake`, repair that asset here and rerun the matrix instead of deferring the fix.
- Preserve the current install-surface smokes for CMake, pkg-config, and `tiffio.hxx`/`libtiffxx`, and update them in place if a packaging or export fix changes the expected surface; the staged C++ consumer remains `safe/test/install/tiffxx_staged_smoke.cpp`, not a new ad hoc probe.
- If the safe shared objects emit warning-only behavior during link-compat runs, review warning deltas in this phase so unexpected runtime warnings do not hide later compatibility drift.

## Verification Phases

### `check_link_surface_tester`

- Phase ID: `check_link_surface_tester`
- Type: `check`
- Bounce Target: `impl_package_link_surface`
- Purpose: Rebuild the release tree, rerun the copied CTest and shell-test coverage so any tool/install wiring changes are exercised, then clear and rebuild the canonical link-compat consumer objects from current sources and rerun the object-link and versioned-symbol compatibility checks against the prepared original objects and DSOs.
- Commands:

```bash
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
python3 safe/scripts/check-public-surface.py --check --check-versioned-symbols
rm -rf safe/build/link-compat
safe/scripts/build-link-compat-objects.sh
safe/scripts/link-and-run-link-compat.sh
```

### `check_package_surface_tester`

- Phase ID: `check_package_surface_tester`
- Type: `check`
- Bounce Target: `impl_package_link_surface`
- Purpose: Build the Ubuntu packages and verify the staged install surface and package payload split.
- Commands:

```bash
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist
```

### `check_package_surface_senior`

- Phase ID: `check_package_surface_senior`
- Type: `check`
- Bounce Target: `impl_package_link_surface`
- Purpose: Review the commit and confirm that symbol files, version scripts, install metadata, and Debian rules stay consistent.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/scripts/build-link-compat-objects.sh safe/scripts/link-and-run-link-compat.sh safe/scripts/build-deb.sh safe/scripts/check-packaged-install-surface.sh safe/scripts/run-upstream-shell-tests.sh safe/debian safe/CMakeLists.txt safe/tools/CMakeLists.txt safe/tools/*.c safe/test/CMakeLists.txt safe/test/Makefile.am safe/test/*.sh safe/test/refs/* safe/test/TiffTest.cmake safe/test/TiffSplitTest.cmake safe/test/TiffTestCommon.cmake safe/pkgconfig/libtiff-4.pc.in safe/cmake/TiffConfig.cmake.in safe/test/install/tiffxx_staged_smoke.cpp safe/capi/libtiff-safe.map safe/capi/libtiffxx-safe.map
```

## Success Criteria

- A fresh release-mode build passes the copied CTest matrix, the copied shell-test matrix, and `python3 safe/scripts/check-public-surface.py --check --check-versioned-symbols`.
- Link-compat verification clears stale relink artifacts, rebuilds the checked-in consumers from current sources, and passes `safe/scripts/link-and-run-link-compat.sh` against the prepared original artifacts.
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist` and `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist` succeed with package metadata, install surface, and symbol/version-script state kept consistent.

## Git Commit Requirement

- The implementer must commit work to git before yielding.
