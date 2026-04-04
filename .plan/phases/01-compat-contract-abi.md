# 01-compat-contract-abi

- Phase Name: Compatibility Contract And ABI Surface
- Implement Phase ID: `impl_compat_contract`

## Preexisting Inputs

- `safe/scripts/check-public-surface.py`
- `safe/abi/public-surface.json`
- `safe/abi/public-surface.inputs.json`
- `safe/abi/platform-excluded-linux.txt`
- `safe/capi/libtiff-safe.map`
- `safe/capi/libtiffxx-safe.map`
- `safe/src/abi/mod.rs`
- `safe/test/public_abi_layout_smoke.c`
- `safe/test/abi_layout_probe_bridge.c`
- `safe/include/tiffio.h`
- `safe/include/tiffio.hxx`
- `original/libtiff/tiffio.h`
- `original/libtiff/tiffio.hxx`
- `original/build/libtiff/tif_config.h`
- `original/build/libtiff/tiffconf.h`
- `original/libtiff/libtiff.map`
- `original/libtiff/libtiffxx.map`
- `original/debian/libtiff6.symbols`
- `original/debian/libtiffxx6.symbols`
- `original/build/libtiff/libtiff.so.6.0.1`
- `original/build/libtiff/libtiffxx.so.6.0.1`

## New Outputs

- Refreshed and validated `safe/abi/public-surface.json`
- Refreshed and validated `safe/abi/public-surface.inputs.json`
- Refreshed and validated `safe/abi/platform-excluded-linux.txt`
- Any required export-map or ABI-layout fixes needed to make those artifacts true

## File Changes

- `safe/scripts/check-public-surface.py`
- `safe/abi/public-surface.json`
- `safe/abi/public-surface.inputs.json`
- `safe/abi/platform-excluded-linux.txt`
- `safe/capi/libtiff-safe.map`
- `safe/capi/libtiffxx-safe.map`
- `safe/src/abi/mod.rs`
- `safe/test/public_abi_layout_smoke.c`
- `safe/test/abi_layout_probe_bridge.c`
- If a missing export or layout mismatch is found: `safe/src/lib.rs`, `safe/capi/tiff_placeholder.c`, or `safe/capi/tiffxx_placeholder.cxx`

## Implementation Details

- Keep `check-public-surface.py` as the single source of truth for generating and validating the ABI contract. Do not introduce a second collector script.
- Ensure the inventory explicitly records header provenance, generated-config-header provenance (`original/build/libtiff/tif_config.h` and `original/build/libtiff/tiffconf.h`), Debian-symbol provenance, original-version-script provenance, safe-version-script provenance, and observed-export provenance for both `libtiff.so.6` and `libtiffxx.so.6`.
- Preserve Linux exclusions for `TIFFOpenW` and `TIFFOpenWExt`; they must remain recorded and must not be exported from the Linux safe DSOs.
- Keep public `#[repr(C)]` struct layout locked to the copied headers for `TIFFTagMethods`, `TIFFFieldInfo`, `TIFFCodec`, `TIFFDisplay`, `TIFFYCbCrToRGB`, `TIFFCIELabToRGB`, and `TIFFRGBAImage`.
- If an export gap exists, prefer the narrowest fix: add the missing Rust export, expose it through the C shim if it is variadic/C++-only, and then record it in the version script and checked-in inventory.
- This phase owns the contract artifacts that all later phases consume; later phases may validate them, but should not silently rewrite them.

## Verification Phases

### `check_compat_contract_tester`

- Phase ID: `check_compat_contract_tester`
- Type: `check`
- Bounce Target: `impl_compat_contract`
- Purpose: Validate the checked-in ABI inventory, Linux exclusions, versioned symbol parity, and public struct layout against the current safe build and the prepared original DSOs.
- Commands:

```bash
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Debug -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
python3 safe/scripts/check-public-surface.py --check --check-versioned-symbols --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt
ctest --test-dir safe/build --output-on-failure -R 'public_abi_layout_smoke|api_handle_smoke|api_open_mode_smoke|api_field_registry_smoke|api_codec_smoke|api_rgba_image_helpers_smoke|api_strile_smoke'
```

### `check_compat_contract_senior`

- Phase ID: `check_compat_contract_senior`
- Type: `check`
- Bounce Target: `impl_compat_contract`
- Purpose: Review the implementor commit and confirm that ABI-contract edits stayed limited to inventory, version scripts, public struct mirrors, and minimal export glue.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/abi safe/scripts/check-public-surface.py safe/capi/libtiff-safe.map safe/capi/libtiffxx-safe.map safe/src/abi/mod.rs safe/test/public_abi_layout_smoke.c safe/test/abi_layout_probe_bridge.c
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Debug -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
python3 safe/scripts/check-public-surface.py --check --check-versioned-symbols
```

## Success Criteria

- `safe/scripts/check-public-surface.py --check --check-versioned-symbols` validates the checked-in ABI inventory and versioned-symbol contract against the current safe build.
- `TIFFOpenW` and `TIFFOpenWExt` remain recorded as Linux exclusions, and the required Linux-visible exports include `_TIFFcalloc`, `TIFFReadTile`, `TIFFWriteTile`, `TIFFReadFromUserBuffer`, and `TIFFStreamOpen`.
- Public `#[repr(C)]` layout remains locked to the copied header contract for the named ABI-visible structs, with any required minimal export or shim fixes landed in the same phase.

## Git Commit Requirement

- The implementer must commit work to git before yielding.
