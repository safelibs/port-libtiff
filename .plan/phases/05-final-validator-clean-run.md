# 05-final-validator-clean-run

## Phase Name

Final Validator Hardening And Report Closure

## Implement Phase ID

`impl_final_validator_clean_run`

## Preexisting Inputs

- All prior phase commits, tests, validator artifacts, and `validator-report.md`.
- Local override packages under `validator/artifacts/debs/local/libtiff/`.
- Local port lock and proof artifacts under `validator/artifacts/libtiff-safe/proof/`.
- Package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/`.
- The external validator checkout under `validator/`.
- Existing compatibility harnesses: ABI check, CTest, upstream shell tests, link compatibility, package install surface, and downstream `test-original.sh`.
- Existing fixtures and references under `safe/test/images/`, `safe/test/refs/`, and `original/test/images/`.

Consume prior artifacts in place. Do not broaden scope, regenerate original build outputs, refetch prepared inventories, regenerate downstream harnesses, or edit validator tests or shared scripts. Do not add the nested `validator/` checkout to the parent repository.

Do not use `make fetch-port-debs`. The final run must use locally built `.deb` packages from `safe/`, `validator/artifacts/debs/local/libtiff/`, and `--override-deb-root`.

Preserve the completed safe-port surface while closing final failures:

- `safe/Cargo.toml` builds Rust crate `safe-libtiff` as static library `tiff_safe_core`.
- `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config and CMake metadata, copied tools, and optional tests.
- `safe/scripts/build-deb.sh` builds `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools` packages at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- `safe/src/lib.rs` hotspots include `parse_open_mode` around line 458, `finalize_open` around line 810, allocation exports around line 1156, open entry points around line 1383, and close/read-directory entry points around line 1510.
- `safe/capi/tiff_placeholder.c` hotspots include `safe_default_vset_field` around line 643, `TIFFGetField`/`TIFFSetField` around lines 1160-1229, and `TIFFPrintDirectory` around line 1511.
- `safe/src/core/directory.rs` hotspots include directory traversal around line 1384, deferred strile materialization around line 2010, field setting around line 2935, and directory writes around line 4326.
- `safe/src/strile.rs` hotspots include size and geometry exports around line 1595, write checks around line 1840, `TIFFWriteScanline` around line 1879, and strip/tile writes around line 1937.
- `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, and `safe/src/rgba.rs` own compression, JPEG/OJPEG, color conversion, and Pillow-facing RGBA behavior.

## New Outputs

- Final catch-all fixes for any remaining unwaived validator or local compatibility issue.
- Rebuilt `safe/dist/*.deb`.
- Regenerated local override packages, local port lock, and final port proof under `validator/artifacts/`.
- Final `validator-report.md` summarizing validator commit, safe commit/package hashes, checks executed, failures found, fixes applied, waivers if any, and final status.
- Final git commit containing all final changes.

## File Changes

- Any safe source/test/package file necessary to close remaining unwaived failures.
- Possible critical files only when the final checks expose the need: `safe/src/lib.rs`, `safe/capi/tiff_placeholder.c`, `safe/src/core/directory.rs`, `safe/src/strile.rs`, `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, `safe/src/rgba.rs`, `safe/src/core/field_registry.rs`, `safe/src/core/field_tables.rs`, `safe/tools/*.c`, `safe/debian/*`, `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`, `safe/include/*`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, and `safe/abi/*`.
- Add or update `safe/test/*.c`, `safe/test/*.sh`, `safe/test/images/*`, or `safe/test/refs/*` only when required by a focused regression, and document why in `validator-report.md`.
- Package/link scripts `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, `safe/scripts/build-link-compat-objects.sh`, and `safe/scripts/link-and-run-link-compat.sh` may change only if local package or link-surface verification is inadequate for a validator-exposed issue.
- Update `validator-report.md`.
- Do not edit validator tests or shared scripts.

## Implementation Details

- Consume only concrete failures from previous phases and final test output. Do not broaden scope.
- If a final failure is a duplicate root cause, add or update one minimal regression that proves the root behavior.
- Rebuild packages and regenerate the validator local lock after final code changes.
- Regenerate the local port lock from actual `.deb` files, including package-name validation for `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`; the lock commit must be `git log -1 --format=%H -- safe`.
- The rebuilt packages must still be the local Debian packages `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools` at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`, produced from the committed `safe/` tree.
- Run the full safe compatibility matrix and the full libtiff validator `port` matrix selecting only `--library libtiff`, using `--override-deb-root artifacts/debs/local`, `--port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json`, and `--record-casts`.
- Update `validator-report.md` with validator repository URL and exact commit, safe source commit tested, package filenames/versions/architectures/SHA-256 hashes, override root, commands executed, source/usage/total/passed/failed/cast counts, every failure found and final disposition, safe fixes with file/test references, any validator-bug waiver with evidence, final verification date, and final status.
- The final verifier must require zero unexpected failures. If `Waived testcase ids:` is empty, require `summary["failed"] == 0`. If waivers exist, every remaining failed testcase must be in that exact waived set and must have detailed waiver evidence in `validator-report.md`.
- `validator-report.md` must continue to contain these machine-readable lines:

```text
Validator commit: <40-char commit>
Safe source commit tested: <40-char commit>
Checks executed: <short command summary>
Failures found: <count and testcase ids>
Waived testcase ids: <comma-separated testcase ids or empty>
```

Commit any final `safe/` fixes before rebuilding `.deb` files and regenerating the final port lock. After the final validator run, commit the completed `validator-report.md`. If no final tracked changes are needed, create an empty phase commit named for `impl_final_validator_clean_run`.

## Verification Phases

### `check_final_validator_tester`

- Phase ID: `check_final_validator_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_final_validator_clean_run`
- Purpose: Run the full safe compatibility matrix and final validator matrix. Require a clean or waiver-limited result.
- Commands:

```bash
git diff --quiet -- safe
git diff --cached --quiet -- safe
cargo test --manifest-path safe/Cargo.toml
cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON
cmake --build safe/build --parallel
python3 safe/scripts/check-public-surface.py --check --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt
ctest --test-dir safe/build --output-on-failure
safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build
rm -rf safe/build/link-compat
safe/scripts/build-link-compat-objects.sh
safe/scripts/link-and-run-link-compat.sh
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh
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
make unit
make check-testcases
python3 tools/testcases.py --config repositories.yml --tests-root tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135
bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts
python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports
cd ..
python3 - <<'PY'
import json
import re
from pathlib import Path
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
report = Path("validator-report.md").read_text()
lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
safe_commit = lock["libraries"][0]["commit"]
commit_match = re.search(r"^Safe source commit tested:\s*([0-9a-f]{40})$", report, re.MULTILINE)
assert commit_match and commit_match.group(1) == safe_commit, (commit_match.group(1) if commit_match else None, safe_commit)
assert proof["libraries"][0]["port_commit"] == safe_commit, (proof["libraries"][0]["port_commit"], safe_commit)
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
assert summary["cases"] == 135 and summary["source_cases"] == 5 and summary["usage_cases"] == 130, summary
PY
```

### `check_final_validator_senior`

- Phase ID: `check_final_validator_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_final_validator_clean_run`
- Purpose: Review all validator-related changes, report completeness, residual risk, and workspace hygiene. Distinguish new unintended tracked changes from preexisting untracked Python bytecode directories and the intentionally excluded nested `validator/` checkout.
- Commands:

```bash
git status --short
git log --oneline -n 8
git show --stat --format=fuller HEAD
git show -- safe/src safe/capi safe/test safe/tools safe/debian safe/scripts validator-report.md
git -C validator status --short
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools || true
python3 - <<'PY'
import json
from pathlib import Path
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
print("summary:", summary)
print("proof totals:", proof["totals"])
print("validator report bytes:", Path("validator-report.md").stat().st_size)
PY
rg -n "TODO|FIXME|panic!|unimplemented!" safe/src safe/capi safe/test validator-report.md || true
rg -n "Validator commit:|Safe source commit tested:|Checks executed:|Failures found:|Waived testcase ids:|Final status" validator-report.md
```

## Success Criteria

- Full safe compatibility matrix passes.
- Full libtiff validator matrix passes with zero unexpected failures.
- Final report matches the artifacts on disk.
- The final `validator-report.md` states the validator commit, safe source commit, commands executed, checks executed, failures found, fixes applied, waivers if any, and final clean or waiver-limited status.
- Validator tests and shared scripts remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If there are no applicable code or report changes, the implementer must create an empty phase commit with a message naming `impl_final_validator_clean_run`.
