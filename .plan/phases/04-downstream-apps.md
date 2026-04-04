# 04-downstream-apps

- Phase Name: Downstream Application Matrix And Reduction
- Implement Phase ID: `impl_downstream_apps`

## Preexisting Inputs

- `dependents.json`
- `test-original.sh`
- `safe/CMakeLists.txt`
- `safe/tools/CMakeLists.txt`
- `safe/tools/*.c`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/debian/`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/images/`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- `original/test/images/rgb-3c-8b.tiff`
- `safe/test/dirread_regressions.c`
- `safe/test/dirwrite_regressions.c`
- `safe/test/strile_regressions.c`
- `safe/test/api_*.c`
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

## New Outputs

- Updated downstream harness probes or deterministic setup in `test-original.sh`, while preserving the fixed 13-package inventory
- Updated package-build or Debian metadata artifacts if the Ubuntu 24.04 container exposes a `.deb` replacement issue
- Corrected copied shell/CTest harness assets when the best reduction or fix lives in an existing shell script, golden reference, or helper CMake driver
- New or extended regression tests in `safe/test/`, wired into the existing CTest and shell-test registration files, for any application-visible compatibility bug found in the container matrix
- Rust/C, copied-tool, or packaging fixes required for packaged, drop-in runtime replacement

## File Changes

- `test-original.sh`
- `safe/CMakeLists.txt`
- `safe/tools/CMakeLists.txt`
- Any affected `safe/tools/*.c`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/scripts/run-upstream-shell-tests.sh`
- `safe/debian/*`
- `safe/test/CMakeLists.txt`
- `safe/test/Makefile.am`
- `safe/test/common.sh`
- `safe/test/*.sh`
- `safe/test/refs/*`
- `safe/test/TiffTest.cmake`
- `safe/test/TiffSplitTest.cmake`
- `safe/test/TiffTestCommon.cmake`
- Any affected `safe/test/dirread_regressions.c`
- Any affected `safe/test/dirwrite_regressions.c`
- Any affected `safe/test/strile_regressions.c`
- Any affected `safe/test/api_*.c`
- If new fixtures are unavoidable: `safe/test/images/*`
- Any affected `safe/src/lib.rs`
- Any affected `safe/src/core/mod.rs`
- Any affected `safe/src/core/directory.rs`
- Any affected `safe/src/core/codec.rs`
- Any affected `safe/src/core/field_registry.rs`
- Any affected `safe/src/core/field_tables.rs`
- Any affected `safe/src/core/color.rs`
- Any affected `safe/src/core/jpeg.rs`
- Any affected `safe/src/rgba.rs`
- Any affected `safe/src/strile.rs`
- Any affected `safe/capi/tiff_placeholder.c`
- Any affected `safe/capi/tiffxx_placeholder.cxx`
- Any affected `safe/capi/jpeg_helper.c`
- Any affected `safe/capi/external_codec_helper.c`

## Implementation Details

- Keep `dependents.json` and `test-original.sh` as the authoritative downstream matrix. The harness has one function per dependent and builds the Ubuntu 24.04 image inline; update `test-original.sh` in place for stronger probes or deterministic fixes, but do not replace the harness, remove the inline `validate_dependents_inventory()` exact-list assertion, or change the ordered package list during this workflow.
- Preserve the package-installed replacement model already used by `test-original.sh`: the applications must resolve `/usr/lib/$MULTIARCH/libtiff.so.6*` from the locally built safe package, not from an injected build tree.
- Because this is the first phase that proves real apt-installed replacement, packaging failures discovered here belong to this phase. If the container run exposes incorrect package dependencies, version ordering, payload split, install behavior, or other `.deb` replacement issues, update `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, and the relevant `safe/debian/*` files in the same commit as the downstream fix, then rebuild `safe/dist` and rerun both package-surface validation and the downstream container.
- Treat the current ordered 13-package inventory as the explicit contract for this workflow: `gimp`, `imagemagick`, `graphicsmagick`, `gdal-bin`, `poppler-utils`, `qt6-image-formats-plugins`, `python3-pil`, `netpbm`, `tesseract-ocr`, `ghostscript`, `libgdk-pixbuf-2.0-0`, `libopencv-imgcodecs406t64`, and `sane-airscan`. Do not weaken this to a count-based check, and do not edit `dependents.json` or the matching `validate_dependents_inventory()` package list in `test-original.sh` during this workflow. If the matrix itself must change, stop and return to planning/workflow regeneration rather than rewriting checker prompts mid-run.
- For each downstream failure, record the application, the failing command, the artifact being read or written, and the libtiff behavior at fault. If the issue can be reduced to a public libtiff reproducer, add that reproducer to `safe/test/` before or together with the actual fix.
- Downstream reductions must land in the existing regression buckets (`safe/test/dirread_regressions.c`, `safe/test/dirwrite_regressions.c`, `safe/test/strile_regressions.c`) or in an existing-or-new `safe/test/api_*.c` smoke, and they must be registered through `safe/test/CMakeLists.txt` and `safe/test/Makefile.am` in the same commit so the phase-4 regression checker can rebuild and execute them immediately.
- Do not artificially constrain downstream fixes to directory and codec files. Runtime issues may legitimately need edits in `safe/src/lib.rs`, `safe/src/core/mod.rs`, `safe/src/core/field_tables.rs`, `safe/src/core/color.rs`, or the C/C++ ABI shims when the application-visible behavior flows through exported entry points, field metadata, or color conversion helpers.
- If a downstream reduction is best expressed by an existing copied shell test, a golden output under `safe/test/refs/*`, or a tool-driven helper CMake driver, update that harness asset in place rather than wrapping it in a new helper script.
- If a downstream reduction adds or reshapes a checked-in `safe/test/` reproducer, update `safe/test/CMakeLists.txt`, `safe/test/Makefile.am`, `safe/test/common.sh`, `safe/test/*.sh`, `safe/test/refs/*`, `safe/test/TiffTest.cmake`, `safe/test/TiffSplitTest.cmake`, `safe/test/TiffTestCommon.cmake`, and `safe/test/images/*` in place as needed so the existing CTest and shell-test inventories continue to discover the coverage.
- Keep `safe/scripts/run-upstream-shell-tests.sh` as the canonical shell-test runner for this phase’s regression pass. If a new downstream reduction needs discovery or execution support there, update that script in place and rerun the local shell-test matrix before returning to the container run.
- If the downstream container or the package-installed validation path exposes a bug in shipped tools such as `tiffinfo`, fix `safe/CMakeLists.txt`, `safe/tools/CMakeLists.txt`, or the relevant `safe/tools/*.c` in the same commit as the downstream fix, then rebuild `safe/dist`, rerun package-surface validation, and rerun the downstream container.
- Keep the application probes non-interactive, time-bounded, and deterministic. Continue to prefer CLI/plugin-level workflows that exercise real TIFF import/export or TIFF-backed functionality.

## Verification Phases

### `check_downstream_regressions_tester`

- Phase ID: `check_downstream_regressions_tester`
- Type: `check`
- Bounce Target: `impl_downstream_apps`
- Purpose: Rebuild the safe test tree from current sources and rerun the checked-in CTest and shell-test coverage so any downstream reductions added to `safe/test/` are compiled, registered, and executed in this phase instead of waiting for the final sweep.
- Commands:

```bash
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Debug -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
```

### `check_downstream_apps_tester`

- Phase ID: `check_downstream_apps_tester`
- Type: `check`
- Bounce Target: `impl_downstream_apps`
- Purpose: Build the safe packages, rerun packaged-install-surface validation, and run the Docker-based downstream matrix from `test-original.sh`.
- Commands:

```bash
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist
LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh
```

### `check_downstream_apps_senior`

- Phase ID: `check_downstream_apps_senior`
- Type: `check`
- Bounce Target: `impl_downstream_apps`
- Purpose: Review the implementor commit, confirm the fixed dependent inventory was preserved, confirm that any app failure reductions were encoded as checked-in regression tests, and inspect any packaging or tool changes that affect the `.deb` replacement path.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- dependents.json test-original.sh safe/CMakeLists.txt safe/tools/CMakeLists.txt safe/tools/*.c safe/test safe/src safe/capi safe/scripts/build-deb.sh safe/scripts/check-packaged-install-surface.sh safe/scripts/run-upstream-shell-tests.sh safe/debian
python3 - <<'PY'
import ast
import json
import re
from pathlib import Path

expected = [
    "gimp",
    "imagemagick",
    "graphicsmagick",
    "gdal-bin",
    "poppler-utils",
    "qt6-image-formats-plugins",
    "python3-pil",
    "netpbm",
    "tesseract-ocr",
    "ghostscript",
    "libgdk-pixbuf-2.0-0",
    "libopencv-imgcodecs406t64",
    "sane-airscan",
]

data = json.loads(Path('dependents.json').read_text())
actual = [entry['package'] for entry in data['dependents']]
assert actual == expected, (expected, actual)

script = Path('test-original.sh').read_text()
match = re.search(
    r"validate_dependents_inventory\(\) \{.*?expected = \[(.*?)\]\n\s*\n",
    script,
    re.S,
)
assert match, 'missing validate_dependents_inventory expected list'
script_expected = ast.literal_eval('[' + match.group(1) + ']')
assert script_expected == expected, (expected, script_expected)
print(actual)
PY
```

## Success Criteria

- The local regression pass rebuilds `safe/build` and reruns the checked-in CTest and shell-test inventories so any downstream reductions are compiled and executed immediately.
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`, `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist`, and `LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh` succeed against the package-installed replacement path.
- The ordered 13-package downstream inventory remains unchanged in both `dependents.json` and `test-original.sh`, and each downstream-visible fix is reduced to checked-in regression coverage when possible.

## Git Commit Requirement

- The implementer must commit work to git before yielding.
