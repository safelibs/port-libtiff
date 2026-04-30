# 03-source-cli-failures

## Phase Name

Fix Source-Facing And CLI Regressions

## Implement Phase ID

`impl_source_cli_failures`

## Preexisting Inputs

- Updated `validator-report.md` from phases 1 and 2.
- Validator artifacts from phases 1 and 2 under `validator/artifacts/libtiff-safe/port/`.
- Local override packages under `validator/artifacts/debs/local/libtiff/`.
- Local port lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/`.
- Package/provenance gate complete with no override installation failures.
- The external validator checkout under `validator/`.
- `safe/src/lib.rs`, especially open/header/read-directory/close functions.
- `safe/capi/tiff_placeholder.c`, especially varargs field marshalling, `TIFFGetField`, `TIFFSetField`, and `TIFFPrintDirectory`.
- `safe/src/core/directory.rs`, especially directory traversal, tag reads, field setting, and directory writes.
- `safe/src/strile.rs`, especially `TIFFWriteScanline` and read/write strip/tile APIs.
- Existing copied tools in `safe/tools/*.c`.
- Existing test registration in `safe/test/CMakeLists.txt` and `safe/test/Makefile.am`.
- Existing fixtures under `safe/test/images/`, `safe/test/refs/`, and `original/test/images/`.

Consume these artifacts in place. Do not refetch or regenerate `original/`, `safe/test/`, CVE data, dependent inventories, package scripts, link-compatibility harnesses, downstream smoke harnesses, or package-smoke projects. Do not recreate missing `original/build/` outputs. Do not edit validator tests or shared scripts.

Do not use `make fetch-port-debs`. This phase must continue to validate locally built packages via `validator/artifacts/debs/local/libtiff/` and `--override-deb-root`.

Relevant safe-port implementation hotspots to consume before editing:

- `safe/Cargo.toml` builds Rust crate `safe-libtiff` as static library `tiff_safe_core`; keep source fixes inside that existing crate/library layout.
- `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, copied tools, package metadata, and optional tests.
- `safe/src/lib.rs` owns lifecycle, open modes, header parsing, memory allocation helpers, C ABI exports, `TIFFOpen`, `TIFFClientOpen`, `TIFFClose`, and `TIFFReadDirectory`; inspect `parse_open_mode` around line 458, `finalize_open` around line 810, allocation exports around line 1156, open entry points around line 1383, and close/read-directory entry points around line 1510.
- `safe/capi/tiff_placeholder.c` owns C varargs marshalling, `TIFFSetField`, `TIFFGetField`, `TIFFGetFieldDefaulted`, error handlers, RGBA wrappers, and `TIFFPrintDirectory`; inspect `safe_default_vset_field` around line 643, `TIFFGetField`/`TIFFSetField` around lines 1160-1229, and `TIFFPrintDirectory` around line 1511.
- `safe/src/core/directory.rs` owns IFD parsing and writing, tag storage/defaults, custom directories, field validation, deferred strile tags, and `TIFFWriteDirectory`; inspect directory traversal around line 1384, deferred strile materialization around line 2010, field setting around line 2935, and directory writes around line 4326.
- `safe/src/strile.rs` owns strip/tile geometry, scanline reads/writes, strile offsets/bytecounts, flushing, and codec integration; inspect size and geometry exports around line 1595, write checks around line 1840, `TIFFWriteScanline` around line 1879, and strip/tile writes around line 1937.
- Existing regression buckets include `dirread_regressions.c`, `dirwrite_regressions.c`, `strile_regressions.c`, `api_*.c`, `test_rgba_readers.c`, and `test_tile_read_write.c`.

## New Outputs

- Minimal safe regression tests for each source/CLI validator failure root cause.
- Source/CLI fixes in `safe/` only.
- Rebuilt `safe/dist/*.deb`.
- Regenerated local override packages, local port lock, and port proof under `validator/artifacts/`.
- Updated `validator-report.md` marking source-case failures fixed or waived with evidence.
- A git commit containing source/CLI fixes and report updates.

## File Changes

- Likely: `safe/src/lib.rs`, `safe/capi/tiff_placeholder.c`, `safe/src/core/directory.rs`, `safe/src/strile.rs`, `safe/tools/tiffinfo.c`, `safe/tools/tiffdump.c`, `safe/tools/tiffcp.c`.
- Regression tests: add `safe/test/validator_source_*.c` or `safe/test/validator_source_*.sh`, or extend `safe/test/api_*`, `safe/test/dirwrite_regressions.c`, `safe/test/dirread_regressions.c`, or `safe/test/strile_regressions.c`.
- Register tests in `safe/test/CMakeLists.txt`; for shell-style tests also update `safe/test/Makefile.am`.
- Add or update `safe/test/images/*` and `safe/test/refs/*` only when a small fixture/reference is required by a regression, and document why in `validator-report.md`.
- Possible ABI files only if a real missing exported symbol is exposed: `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/public-surface.json`, `safe/abi/platform-excluded-linux.txt`.
- Update `validator-report.md`.
- Do not edit `validator/tests/libtiff/**`, validator shared scripts, or the validator test harness.

## Implementation Details

- Use `validator-report.md` and result JSON to identify failed source cases. Expected source-case ids are `c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, and `tiffinfo-metadata`.
- For `c-api-read-write`, add a C regression that writes a 1x1 RGB image through `TIFFOpen`, `TIFFSetField`, `TIFFWriteScanline`, closes it, reopens it, and verifies `TIFFGetField` returns width and height. Fix marshalling, directory writes, flush, scanline, or open-mode logic as needed.
- For `malformed-tiff-rejection`, add a regression that feeds non-TIFF bytes to `TIFFOpen` or `tiffinfo` and asserts failure. Fix header parsing or tool error propagation; do not make invalid bytes acceptable.
- For `tiffcp-copy`, reproduce with a small fixture such as `safe/test/images/rgb-3c-8b.tiff`, then fix strip/tile read/write, directory copying, or tool integration.
- For `tiffdump-structure` and `tiffinfo-metadata`, reproduce expected metadata output with existing fixtures and fix tag parsing, `TIFFPrintDirectory`, `TIFFGetFieldDefaulted`, directory traversal, or tool formatting.
- Keep C ABI and symbol maps stable unless the validator exposes a genuine missing exported symbol.
- If a source testcase is a validator bug, prove and document the waiver before relying on it. Use the original-mode validator command from phase 2, capture nonzero original-mode exit status, generate original proof artifacts, and document the exact id on `Waived testcase ids:`.
- Later source and usage checkers may allow failures only if the exact testcase ids are already documented on `Waived testcase ids:` with evidence. Do not require `summary["failed"] == 0` before waiver handling has occurred.
- Before rebuilding packages for the validator run, commit all `safe/` source, test, package, and script changes that should be represented in those packages. Regenerate the local port lock from the actual `.deb` files, including package-name validation for `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`; the lock commit must be `git log -1 --format=%H -- safe`.
- The rebuilt packages must still be the local Debian packages `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools` at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`, produced from the committed `safe/` tree.
- Rerun the full libtiff validator `port` matrix selecting only `--library libtiff`, using `--override-deb-root artifacts/debs/local`, `--port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json`, and `--record-casts`.
- Update `validator-report.md` with source-case fixes, test references, result JSON/log paths, any waiver evidence, and the current machine-readable lines:

```text
Validator commit: <40-char commit>
Safe source commit tested: <40-char commit>
Checks executed: <short command summary>
Failures found: <count and testcase ids>
Waived testcase ids: <comma-separated testcase ids or empty>
```

Commit any source/CLI fixes and regression tests before rebuilding `.deb` files and regenerating the port lock. After the validator run, commit `validator-report.md`. If no source/CLI failures exist, update the report with "no source/CLI failures" and create an empty phase commit named for `impl_source_cli_failures`.

## Verification Phases

### `check_source_cli_tester`

- Phase ID: `check_source_cli_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_source_cli_failures`
- Purpose: Rebuild safe, run local source/CLI regressions, rerun the full validator matrix, and require no unwaived source-case failures. Usage failures may remain for the next phase only when they are not source cases.
- Commands:

```bash
git diff --quiet -- safe
git diff --cached --quiet -- safe
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build --include-regex 'tiff(info|dump|cp)|ppm2tiff|fax2tiff'
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
rm -rf validator/artifacts/debs/local/libtiff
mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof
find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +
python3 - <<'PY'
import hashlib
import json
import subprocess
from pathlib import Path
packages = ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"]
leaf = Path("validator/artifacts/debs/local/libtiff")
debs = []
for package in packages:
    path = sorted(leaf.glob(f"{package}_*.deb"))[0]
    arch = subprocess.check_output(["dpkg-deb", "-f", str(path), "Architecture"], text=True).strip()
    data = path.read_bytes()
    debs.append({"package": package, "filename": path.name, "architecture": arch, "sha256": hashlib.sha256(data).hexdigest(), "size": len(data), "asset_api_url": None, "browser_download_url": None})
commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
release = f"local-{commit[:12]}"
lock = {"schema_version": 1, "mode": "port", "generated_at": "1970-01-01T00:00:00Z", "source_config": "repositories.yml", "source_inventory": "local-overrides", "libraries": [{"library": "libtiff", "repository": "safelibs/port-libtiff", "url": "https://github.com/safelibs/port-libtiff", "tag_ref": f"refs/tags/{release}", "commit": commit, "release_tag": release, "debs": debs, "unported_original_packages": []}]}
Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n")
PY
cd validator
bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts
python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports
cd ..
python3 - <<'PY'
import json
import re
from pathlib import Path
report = Path("validator-report.md").read_text()
match = re.search(r"^Waived testcase ids:\s*(.*)$", report, re.MULTILINE)
waived = {item.strip() for item in (match.group(1) if match else "").split(",") if item.strip()}
source_failures = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    result = json.loads(path.read_text())
    if result["kind"] == "source" and result["status"] == "failed" and result["testcase_id"] not in waived:
        source_failures.append(result["testcase_id"])
assert not source_failures, {"unwaived_source_failures": source_failures, "waived": sorted(waived)}
PY
```

### `check_source_cli_senior`

- Phase ID: `check_source_cli_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_source_cli_failures`
- Purpose: Review source/CLI fixes for minimality, ABI compatibility, regression quality, and absence of validator-suite edits.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/src safe/capi safe/tools safe/test safe/CMakeLists.txt safe/capi/libtiff-safe.map safe/abi validator-report.md
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools || true
rg -n "validator|source|c-api-read-write|malformed-tiff-rejection|tiffcp-copy|tiffdump-structure|tiffinfo-metadata|Waived testcase ids" validator-report.md safe/test safe/src safe/capi safe/tools
```

## Success Criteria

- Safe CMake release build passes with tools/tests enabled.
- CTest passes.
- Relevant upstream shell tests for `tiffinfo`, `tiffdump`, and `tiffcp` pass.
- Safe Debian packages build and install-surface smokes pass.
- Full libtiff validator matrix has no unwaived source failures.
- Validator tests and shared scripts remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If there are no applicable code or report changes, the implementer must create an empty phase commit with a message naming `impl_source_cli_failures`.
