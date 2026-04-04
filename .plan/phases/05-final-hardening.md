# 05-final-hardening

- Phase Name: Catch-All Compatibility Fixes, Unsafe Reduction, And Final Sweep
- Implement Phase ID: `impl_final_compat_hardening`

## Preexisting Inputs

- `relevant_cves.json`
- `all_cves.json`
- `safe/CMakeLists.txt`
- `safe/tools/CMakeLists.txt`
- `safe/tools/*.c`
- `safe/pkgconfig/libtiff-4.pc.in`
- `safe/cmake/TiffConfig.cmake.in`
- `safe/include/tiff.h`
- `safe/include/tif_config.h`
- `safe/include/tiffconf.h`
- `safe/include/tiffio.h`
- `safe/include/tiffio.hxx`
- `safe/include/tiffvers.h`
- `safe/abi/public-surface.json`
- `safe/abi/public-surface.inputs.json`
- `safe/abi/platform-excluded-linux.txt`
- `safe/src/lib.rs`
- `safe/src/abi/mod.rs`
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
- `safe/capi/tiffxx_placeholder.cxx`
- `safe/capi/jpeg_helper.c`
- `safe/capi/external_codec_helper.c`
- `safe/capi/libtiff-safe.map`
- `safe/capi/libtiffxx-safe.map`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- `safe/test/images/*`
- `safe/test/*.c`
- `safe/test/public_abi_layout_smoke.c`
- `safe/test/abi_layout_probe_bridge.c`
- `safe/test/dirread_regressions.c`
- `safe/test/dirwrite_regressions.c`
- `safe/test/strile_regressions.c`
- `safe/test/api_*.c`
- `safe/test/api_handle_smoke.c`
- `safe/test/api_directory_read_smoke.c`
- `safe/test/api_field_registry_smoke.c`
- `safe/test/api_strile_smoke.c`
- `safe/test/link_compat_logluv_smoke.c`
- `safe/test/install/tiffxx_staged_smoke.cpp`
- `safe/scripts/check-public-surface.py`
- `safe/scripts/build-link-compat-objects.sh`
- `safe/scripts/link-and-run-link-compat.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/scripts/*.sh`
- `safe/debian/control`
- `safe/debian/rules`
- `safe/debian/changelog`
- `safe/debian/libtiff6.symbols`
- `safe/debian/libtiffxx6.symbols`
- `safe/debian/*.install`
- `safe/debian/`
- `dependents.json`
- `test-original.sh`
- `original/libtiff/tiffio.h`
- `original/libtiff/tiffio.hxx`
- `original/libtiff/libtiff.map`
- `original/libtiff/libtiffxx.map`
- `original/debian/libtiff6.symbols`
- `original/debian/libtiffxx6.symbols`
- `original/build/libtiff/tif_config.h`
- `original/build/libtiff/tiffconf.h`
- `original/build/libtiff/libtiff.so.6.0.1`
- `original/build/libtiff/libtiffxx.so.6.0.1`
- `original/build/test_cmake/`
- `original/build/test_cmake/test.c`
- `original/build/test_cmake_no_target/`
- `original/build/test_cmake_no_target/test.c`
- `original/build-step2/`
- `original/build-step2/test/CMakeFiles/*/link.txt`
- `original/test/`
- `original/tools/`
- `original/test/images/rgb-3c-8b.tiff`

## New Outputs

- Final catch-all compatibility fixes across ABI, upstream-test, link/install, package, and downstream-runtime surfaces
- Final regression additions plus any required updates to checked-in test registration, shell/CTest harness assets, fixtures, or install/link consumer smokes for anything found by the end-to-end matrix
- Final synchronized state of the ABI inventories, package metadata, install-surface probes, and downstream harness artifacts

## File Changes

- `safe/CMakeLists.txt`
- `safe/tools/CMakeLists.txt`
- Any affected `safe/tools/*.c`
- `safe/pkgconfig/libtiff-4.pc.in`
- `safe/cmake/TiffConfig.cmake.in`
- `safe/include/tiff.h`
- `safe/include/tif_config.h`
- `safe/include/tiffconf.h`
- `safe/include/tiffio.h`
- `safe/include/tiffio.hxx`
- `safe/include/tiffvers.h`
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
- `safe/capi/tiffxx_placeholder.cxx`
- `safe/capi/jpeg_helper.c`
- `safe/capi/external_codec_helper.c`
- `safe/capi/libtiff-safe.map`
- `safe/capi/libtiffxx-safe.map`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- `safe/test/images/*`
- `safe/scripts/check-public-surface.py`
- `safe/abi/public-surface.json`
- `safe/abi/public-surface.inputs.json`
- `safe/abi/platform-excluded-linux.txt`
- `safe/test/*.c`
- `safe/test/install/tiffxx_staged_smoke.cpp`
- `safe/scripts/build-link-compat-objects.sh`
- `safe/scripts/link-and-run-link-compat.sh`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/scripts/*.sh`
- `safe/debian/*`
- `test-original.sh`

## Implementation Details

- This phase closes remaining issues from earlier tester phases. It must consume those concrete failures instead of inventing new scope.
- Prefer moving invariants and validation into safe Rust helpers and keeping `unsafe` confined to FFI, raw-pointer marshalling, and direct OS/library calls.
- Use `relevant_cves.json` as a checklist for the classes most likely to survive a Rust port: divide-by-zero, null-state handling, resource exhaustion, memory leaks, and validation/arithmetic mistakes. Any fix in directory parsing, directory writing, strip/tile math, JPEG/OJPEG handling, or validation logic should be reviewed against that file.
- Ensure every downstream-visible fix is backed by checked-in regression coverage: ideally in `safe/test/`, and secondarily in `test-original.sh` when the behavior is only observable through an application/plugin integration.
- If the final sweep finds a remaining link/install/package issue, fix it in the existing install surface rather than inventing a new harness: update `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`, `safe/include/*`, `safe/test/install/tiffxx_staged_smoke.cpp`, `safe/scripts/build-link-compat-objects.sh`, `safe/scripts/link-and-run-link-compat.sh`, `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, or `safe/debian/*` as appropriate.
- If the final sweep finds a remaining copied-tool build, install, or behavior issue exposed by CTest, the shell-test runner, packaged-install validation, or the downstream container, fix `safe/CMakeLists.txt`, `safe/tools/CMakeLists.txt`, and the relevant `safe/tools/*.c` in place rather than bypassing the shipped tool surface.
- If the final sweep finds that a copied shell script, a golden reference in `safe/test/refs/*`, or a helper CTest driver (`safe/test/TiffTest.cmake`, `safe/test/TiffSplitTest.cmake`, `safe/test/TiffTestCommon.cmake`) is itself the remaining problem, fix that harness asset in place in this phase rather than creating a wrapper or leaving the issue unresolved.
- If the final sweep adds or reshapes a checked-in regression under `safe/test/`, update `safe/test/CMakeLists.txt`, `safe/test/Makefile.am`, `safe/test/common.sh`, `safe/test/*.sh`, `safe/test/refs/*`, `safe/test/TiffTest.cmake`, `safe/test/TiffSplitTest.cmake`, `safe/test/TiffTestCommon.cmake`, and `safe/test/images/*` in place so the full CTest and shell-test inventories continue to discover it.
- Keep the phase-1 ABI contract artifacts stable by default. Only edit `safe/scripts/check-public-surface.py`, `safe/abi/public-surface.json`, `safe/abi/public-surface.inputs.json`, or `safe/abi/platform-excluded-linux.txt` here if an earlier verifier already demonstrated that the checked-in contract is wrong and the final sweep is closing that known gap.
- Do not broaden the public ABI or package surface during this phase unless a previous verifier demonstrated that the broadening is required for compatibility.
- As in phase 3, any final-sweep verification that invokes `safe/scripts/link-and-run-link-compat.sh` must first clear `safe/build/link-compat` or rerun `safe/scripts/build-link-compat-objects.sh` so the relink consumers come from the current source tree instead of stale objects.
- The ordered 13-package downstream inventory remains fixed in this phase. The final sweep may tighten `test-original.sh` assertions or deterministic setup, but it must not change `dependents.json` or remove the matching inline `validate_dependents_inventory()` package-list assertion; a matrix change requires returning to planning/workflow regeneration.
- The final sweep must consume the prepared original artifacts already in the workspace: `original/build/libtiff/libtiff.so.6.0.1`, `original/build/libtiff/libtiffxx.so.6.0.1`, `original/build/test_cmake/`, `original/build/test_cmake_no_target/`, `original/build-step2/test/CMakeFiles/*/link.txt`, and `original/test/`. It must not rebuild or rediscover those upstream artifacts.

## Verification Phases

### `check_final_matrix_tester`

- Phase ID: `check_final_matrix_tester`
- Type: `check`
- Bounce Target: `impl_final_compat_hardening`
- Purpose: Rerun the full compatibility matrix end to end after the catch-all fix phase.
- Commands:

```bash
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
python3 safe/scripts/check-public-surface.py --check --check-versioned-symbols --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
rm -rf safe/build/link-compat
safe/scripts/build-link-compat-objects.sh
safe/scripts/link-and-run-link-compat.sh
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist
LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh
```

### `check_final_matrix_senior`

- Phase ID: `check_final_matrix_senior`
- Type: `check`
- Bounce Target: `impl_final_compat_hardening`
- Purpose: Review the final commit for remaining unsafe boundaries, regression completeness, and alignment with the CVE-derived validation categories.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/CMakeLists.txt safe/tools/CMakeLists.txt safe/tools/*.c safe/src safe/capi safe/test safe/scripts safe/debian dependents.json test-original.sh
rg -n "unsafe" safe/src safe/capi
python3 - <<'PY'
import json
from pathlib import Path
data = json.loads(Path('relevant_cves.json').read_text())
print(data['summary'])
PY
```

## Success Criteria

- The full release-mode compatibility matrix passes end to end, including ABI validation, CTest, shell tests, link-compat relink checks, package-surface validation, and the packaged downstream application matrix.
- Remaining `unsafe` usage is confined to justified FFI or low-level boundaries, and final fixes are reviewed against the categories summarized in `relevant_cves.json`.
- Any remaining ABI, package, tool, harness, or downstream-runtime issues are closed in the existing checked-in surfaces without broadening the public ABI or changing the fixed 13-package downstream inventory.

## Git Commit Requirement

- The implementer must commit work to git before yielding.
