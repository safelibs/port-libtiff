# 04-usage-runtime-failures

## Phase Name

Fix Usage-Case Runtime Regressions

## Implement Phase ID

`impl_usage_runtime_failures`

## Preexisting Inputs

- Updated `validator-report.md` and validator artifacts from phases 1-3.
- Local override packages under `validator/artifacts/debs/local/libtiff/`.
- Local port lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/`.
- The external validator checkout under `validator/`.
- `safe/src/rgba.rs` for RGB/RGBA/Pillow-facing reads, region conversion, OJPEG handling, palette/YCbCr/CMYK, and alpha behavior.
- `safe/src/core/codec.rs` for LZW, deflate, PackBits, Fax, JPEG/OJPEG, LZMA, ZSTD, WEBP, LERC, and codec pseudo-tags.
- `safe/src/core/jpeg.rs` for JPEG and OJPEG decode/encode.
- `safe/src/core/color.rs` for LogLuv, CIE Lab, YCbCr, and color transforms.
- `safe/src/core/directory.rs` for metadata tags, defaults, rational values, resolution units, BigTIFF, SubIFD/multipage traversal, and directory writes.
- `safe/src/strile.rs` for strip/tile geometry and data I/O.
- Existing fixtures in `safe/test/images/`, `safe/test/refs/`, and `original/test/images/`.
- Existing test registration in `safe/test/CMakeLists.txt` and `safe/test/Makefile.am`.

Consume these artifacts in place. Do not refetch or regenerate `original/`, `safe/test/`, CVE data, dependent inventories, package scripts, link-compatibility harnesses, downstream smoke harnesses, or package-smoke projects. Do not recreate missing `original/build/` outputs. Do not edit validator tests or shared scripts.

Do not use `make fetch-port-debs`. This phase validates the current local `safe/` tree through locally built `.deb` packages and `--override-deb-root`.

Relevant safe-port implementation hotspots to consume before editing:

- `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, and `safe/src/rgba.rs` own compression, JPEG/OJPEG, color conversion, and Pillow-facing RGBA behavior.
- `safe/src/rgba.rs` owns RGB/RGBA/Pillow-facing reads, region conversion, OJPEG handling, palette/YCbCr/CMYK, alpha behavior, tiled reads, orientations, and photometric handling.
- `safe/src/core/codec.rs` owns LZW, deflate, PackBits, Fax, JBIG, LZMA, ZSTD, WEBP, LERC, predictor behavior, and codec pseudo-tags.
- `safe/src/core/jpeg.rs` owns JPEG and OJPEG decode/encode behavior.
- `safe/src/core/color.rs` owns LogLuv, CIE Lab, YCbCr, CMYK, and RGB conversions.
- `safe/src/core/directory.rs` owns metadata tags, defaults, rational values, resolution units, BigTIFF, SubIFD/multipage traversal, directory writes, custom tags, and deferred strile tags; key hotspots are traversal around line 1384, deferred strile materialization around line 2010, field setting around line 2935, and directory writes around line 4326.
- `safe/capi/tiff_placeholder.c` owns `TIFFSetField`, `TIFFGetField`, `TIFFGetFieldDefaulted`, varargs marshalling, `TIFFPrintDirectory`, and RGBA C wrappers; key hotspots are `safe_default_vset_field` around line 643, `TIFFGetField`/`TIFFSetField` around lines 1160-1229, and `TIFFPrintDirectory` around line 1511.
- `safe/src/strile.rs` owns strip/tile geometry and data I/O; key hotspots are size and geometry exports around line 1595, write checks around line 1840, `TIFFWriteScanline` around line 1879, and strip/tile writes around line 1937.
- `safe/Cargo.toml` still builds Rust crate `safe-libtiff` as static library `tiff_safe_core`; `safe/CMakeLists.txt` still builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, tools, metadata, and optional tests.

## New Outputs

- Regression tests for each usage failure class, preferably under existing regression buckets or new `safe/test/validator_usage_*.c` files.
- Small fixtures under `safe/test/images/` only when a fixture cannot be generated in a test.
- Small references under `safe/test/refs/` only when needed by a regression.
- Safe runtime fixes in Rust, C facade, or copied tools.
- Rebuilt `safe/dist/*.deb`.
- Regenerated local override packages, local port lock, and port proof under `validator/artifacts/`.
- Updated validator report with per-failure fixes and final usage-case status.
- A git commit containing usage fixes and tests.

## File Changes

- Likely: `safe/src/rgba.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/core/directory.rs`, `safe/src/core/field_registry.rs`, `safe/src/core/field_tables.rs`, `safe/capi/tiff_placeholder.c`.
- Possible: `safe/tools/tiff2bw.c`, `safe/tools/tiff2pdf.c`, `safe/tools/tiffcrop.c`, `safe/tools/tiffmedian.c`, `safe/tools/tiffsplit.c`, `safe/tools/tiffcp.c`, `safe/tools/tiffdump.c`, `safe/tools/tiffinfo.c`.
- Test registration: `safe/test/CMakeLists.txt`, `safe/test/Makefile.am`, `safe/test/common.sh`, and new or existing `safe/test/*.c` or `safe/test/*.sh`.
- Add or update `safe/test/images/*` and `safe/test/refs/*` only when required by a regression and document why in `validator-report.md`.
- Possible ABI/link files only if a real package/link/ABI issue is exposed while fixing usage failures: `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/*`.
- Update `validator-report.md`.
- Do not edit validator tests, validator shared scripts, or validator test expectations.

## Implementation Details

- Classify failed usage cases by observable behavior before editing:
  - Read-path pixel or mode mismatch: inspect `safe/src/rgba.rs`, `safe/src/strile.rs`, and photometric/color defaults.
  - Save or roundtrip failure: inspect `TIFFSetField` marshalling, directory write, strile write, flush/rewrite, and tool integration.
  - Compression failure: inspect `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, and pending strile flush logic.
  - Metadata/tag failure: inspect `safe/capi/tiff_placeholder.c` varargs handling and `safe/src/core/directory.rs` defaulted values, rational storage, ASCII counts, and field tables.
  - Multipage/SubIFD/BigTIFF failure: inspect directory traversal/write offsets, next-directory links, BigTIFF header handling, and `tiffsplit`.
  - CLI usage failure: reproduce using the shipped tool and fixture, then fix the tool integration or library behavior.
- Add one minimal regression per root cause, not one per duplicate validator testcase. Name tests after behavior, such as `validator_usage_lzw_predictor_roundtrip.c`, `validator_usage_resolution_rational_roundtrip.c`, or `validator_usage_multipage_seek.c`.
- Prefer generating tiny TIFF files inside C tests through the public API. Add binary fixtures only for malformed, compression, or multipage cases that cannot be generated reliably.
- Keep fixes in safe Rust helpers when possible, with `unsafe` confined to FFI/raw-pointer boundaries.
- If a usage testcase is a validator bug discovered only in this phase, prove it with the original-package validator command from phase 2 and document the exact id on `Waived testcase ids:` before yielding. Capture nonzero original-mode exit status and run proof generation so original failure evidence remains available.
- The full libtiff validator matrix must have zero unwaived failures after this phase. If `Waived testcase ids:` is empty, require `summary["failed"] == 0`; if waivers exist, every remaining failed testcase must be in that exact waived set and have detailed evidence in `validator-report.md`.
- Before rebuilding packages for the validator run, commit all `safe/` source, test, package, and script changes that should be represented in those packages. Regenerate the local port lock from actual `.deb` files, including package-name validation for `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`; the lock commit must be `git log -1 --format=%H -- safe`.
- The rebuilt packages must still be the local Debian packages `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools` at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`, produced from the committed `safe/` tree.
- Rerun the full libtiff validator `port` matrix selecting only `--library libtiff`, using `--override-deb-root artifacts/debs/local`, `--port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json`, and `--record-casts`.
- Update `validator-report.md` with per-failure fixes, test references, package hashes, result JSON/log paths, final usage-case status, and these machine-readable lines:

```text
Validator commit: <40-char commit>
Safe source commit tested: <40-char commit>
Checks executed: <short command summary>
Failures found: <count and testcase ids>
Waived testcase ids: <comma-separated testcase ids or empty>
```

Commit any usage/runtime fixes and regression tests before rebuilding `.deb` files and regenerating the port lock. After the validator run, commit `validator-report.md`. If no usage failures exist, update the report with "no usage failures" and create an empty phase commit named for `impl_usage_runtime_failures`.

## Verification Phases

### `check_usage_runtime_tester`

- Phase ID: `check_usage_runtime_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_usage_runtime_failures`
- Purpose: Verify usage-case fixes with focused safe regressions, upstream tests, downstream-like package tests, and a full validator rerun. Require zero unwaived failures across the full libtiff matrix.
- Commands:

```bash
git diff --quiet -- safe
git diff --cached --quiet -- safe
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
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
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
assert summary["cases"] == 135 and summary["source_cases"] == 5 and summary["usage_cases"] == 130, summary
report = Path("validator-report.md").read_text()
match = re.search(r"^Waived testcase ids:\s*(.*)$", report, re.MULTILINE)
waived = {item.strip() for item in (match.group(1) if match else "").split(",") if item.strip()}
failed = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    result = json.loads(path.read_text())
    if result["status"] == "failed":
        failed.append(result["testcase_id"])
unexpected = sorted(set(failed) - waived)
assert not unexpected, {"unexpected": unexpected, "failed": sorted(failed), "waived": sorted(waived), "summary": summary}
if not waived:
    assert summary["failed"] == 0, summary
PY
```

### `check_usage_runtime_senior`

- Phase ID: `check_usage_runtime_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_usage_runtime_failures`
- Purpose: Review usage fixes for data correctness, metadata semantics, compression safety, regression coverage, and strict waiver handling.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/src/core safe/src/rgba.rs safe/src/strile.rs safe/capi safe/test safe/tools validator-report.md
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools || true
rg -n "unsafe|unwrap\\(|expect\\(" safe/src safe/capi
python3 - <<'PY'
import json
from pathlib import Path
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
print(summary)
report = Path("validator-report.md").read_text().lower()
for needle in ("usage", "pillow", "compression", "metadata", "multipage", "waived testcase ids"):
    print(needle, needle in report)
PY
```

## Success Criteria

- Cargo tests pass.
- Release CMake build with tools/tests passes.
- Full CTest and upstream shell tests pass.
- Safe Debian packages build and install-surface smokes pass.
- Full libtiff validator matrix has zero unwaived failures.
- Usage fixes have focused regressions in the safe tree.
- Validator tests and shared scripts remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If there are no applicable code or report changes, the implementer must create an empty phase commit with a message naming `impl_usage_runtime_failures`.
