# 01-validator-baseline

## Phase Name

Validator Checkout And Baseline Safe Matrix

## Implement Phase ID

`impl_validator_baseline`

## Preexisting Inputs

- `safe/`
- `safe/Cargo.toml`
- `safe/Cargo.lock`
- `safe/CMakeLists.txt`
- `safe/debian/`
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/test/`
- `safe/tools/`
- `original/`
- `dependents.json`
- `all_cves.json`
- `relevant_cves.json`
- `test-original.sh`
- Network access to `https://github.com/safelibs/validator`
- Docker, Git, Python 3, Make, CMake, Ninja, Cargo, dpkg tooling, and Debian package build dependencies available in the execution environment

Consume these artifacts in place. Do not refetch or regenerate `original/`, `safe/test/`, `dependents.json`, `all_cves.json`, `relevant_cves.json`, downstream harnesses, link-compatibility harnesses, or package scripts unless an explicit failure requires a minimal update. The workspace does not contain reusable `original/build/` outputs; do not depend on or recreate original build artifacts. The only external checkout to clone or fast-forward update is `validator/`.

No local `validator/` checkout exists at plan start. Create it under `/home/yans/safelibs/pipeline/ports/port-libtiff/validator`, keep the nested checkout out of the parent repository, and add `/validator/` to the parent `.git/info/exclude` if needed. Do not delete or commit the preexisting untracked Python bytecode directories under `original/.pc/`, `original/cmake/`, `original/doc/`, and `safe/scripts/`.

Do not use `make fetch-port-debs`. That target fetches GitHub release assets. This workflow validates the current local `safe/` tree through locally built `.deb` packages and `--override-deb-root`.

Preserve this existing safe-port layout and package contract:

- `safe/Cargo.toml` builds Rust crate `safe-libtiff` as static library `tiff_safe_core`.
- `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config and CMake metadata, copied tools, and optional tests.
- `safe/scripts/build-deb.sh` builds `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools` packages at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- `safe/scripts/check-packaged-install-surface.sh` verifies package contents, headers, CMake/pkg-config integration, C++ facade, and packaged tools.
- `safe/src/lib.rs` owns lifecycle, open modes, header parsing, memory allocation helpers, C ABI exports, `TIFFOpen`, `TIFFClientOpen`, `TIFFClose`, and `TIFFReadDirectory`; key hotspots are `parse_open_mode` around line 458, `finalize_open` around line 810, allocation exports around line 1156, open entry points around line 1383, and close/read-directory entry points around line 1510.
- `safe/capi/tiff_placeholder.c` owns C varargs marshalling, `TIFFSetField`, `TIFFGetField`, `TIFFGetFieldDefaulted`, error handlers, RGBA wrappers, and `TIFFPrintDirectory`; key hotspots are `safe_default_vset_field` around line 643, `TIFFGetField`/`TIFFSetField` around lines 1160-1229, and `TIFFPrintDirectory` around line 1511.
- `safe/src/core/directory.rs` owns IFD parsing/writing, tag storage/defaults, custom directories, field validation, deferred strile tags, and `TIFFWriteDirectory`; key hotspots are directory traversal around line 1384, deferred strile materialization around line 2010, field setting around line 2935, and directory writes around line 4326.
- `safe/src/strile.rs` owns strip/tile geometry, scanline reads/writes, strile offsets/bytecounts, flushing, and codec integration; key hotspots are size and geometry exports around line 1595, write checks around line 1840, `TIFFWriteScanline` around line 1879, and strip/tile writes around line 1937.
- `safe/src/core/codec.rs`, `safe/src/core/jpeg.rs`, `safe/src/core/color.rs`, and `safe/src/rgba.rs` own compression, JPEG/OJPEG, color conversion, and Pillow-facing RGBA behavior.
- `safe/test/CMakeLists.txt` registers CTest executables and shell/tool tests. Existing regression buckets include `dirread_regressions.c`, `dirwrite_regressions.c`, `strile_regressions.c`, `api_*.c`, `test_rgba_readers.c`, and `test_tile_read_write.c`.

## New Outputs

- `validator/` cloned or fast-forward updated to the actual validator commit used.
- `safe/dist/*.deb` rebuilt from the current safe port.
- `validator/artifacts/debs/local/libtiff/*.deb`
- `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
- `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
- `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- `validator/artifacts/libtiff-safe/package-smoke/` containing generated package-install CMake/pkg-config smoke projects used by `safe/scripts/check-packaged-install-surface.sh`.
- `validator-report.md`
- Parent `.git/info/exclude` contains `/validator/` as a local ignore entry if the parent repository did not already ignore `validator/`.
- A git commit recording `validator-report.md` and any tracked setup/report changes. Do not add the nested `validator/` checkout to the parent repository.

## File Changes

- Create or rewrite `validator-report.md`.
- Create `validator/artifacts/libtiff-safe/package-smoke/` with the exact generated package-smoke source and CMake files shown below.
- Do not change `safe/` unless a packaging/build break blocks the baseline run. If needed, make the minimal build fix and report it.
- Do not modify validator source files or tests.
- Respect these critical-file constraints: `safe/test/images/*`, `safe/test/refs/*`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/*`, package scripts, and link-compatibility scripts may change only for issues actually exposed by this validation work.

## Implementation Details

1. Clone or update validator and record the actual checkout commit. The verified reference commit from the plan is `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`, but the report must record the actual commit used.

```bash
grep -qxF '/validator/' .git/info/exclude || printf '\n/validator/\n' >> .git/info/exclude
if [ -d validator/.git ]; then
  git -C validator diff --quiet
  git -C validator diff --cached --quiet
  git -C validator pull --ff-only
else
  git clone https://github.com/safelibs/validator validator
fi
git -C validator rev-parse HEAD
```

If the dirty-check commands fail in an existing validator checkout, stop and document the blocker in `validator-report.md`. Do not run `git reset`, delete the checkout, or overwrite local validator changes.

2. Check validator tooling and the libtiff inventory.

```bash
make -C validator unit
make -C validator check-testcases
python3 validator/tools/testcases.py \
  --config validator/repositories.yml \
  --tests-root validator/tests \
  --library libtiff \
  --check \
  --min-source-cases 5 \
  --min-usage-cases 130 \
  --min-cases 135
```

The expected inventory is 5 source cases and 130 usage cases, 135 total. Source cases are `c-api-read-write`, `malformed-tiff-rejection`, `tiffcp-copy`, `tiffdump-structure`, and `tiffinfo-metadata`. Usage cases are primarily Pillow TIFF open/save/metadata/compression/multipage operations plus CLI usage for `tiffcp`, `tiffinfo`, `tiffdump`, `tiff2bw`, `tiff2pdf`, `tiffcrop`, `tiffmedian`, and `tiffsplit`.

3. Create local package-smoke projects. The workspace does not contain the historical `original/build/test_cmake` and `original/build/test_cmake_no_target` directories that `safe/scripts/check-packaged-install-surface.sh` uses by default. Do not regenerate the original build tree for those fixtures. Generate replacement smoke projects once under validator artifacts and pass them to every package-install-surface invocation.

```bash
PACKAGE_SMOKE_ROOT=validator/artifacts/libtiff-safe/package-smoke
mkdir -p "$PACKAGE_SMOKE_ROOT/cmake-target" "$PACKAGE_SMOKE_ROOT/cmake-targetless"
cat >"$PACKAGE_SMOKE_ROOT/test.c" <<'EOF'
#include <tiffio.h>

int main(void) {
    const char *version = TIFFGetVersion();
    return (version != 0 && version[0] != '\0') ? 0 : 1;
}
EOF
cat >"$PACKAGE_SMOKE_ROOT/cmake-target/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.16)
project(libtiff_package_smoke_target C)
find_package(TIFF REQUIRED CONFIG)
add_executable(test ../test.c)
target_link_libraries(test PRIVATE TIFF::tiff)
EOF
cat >"$PACKAGE_SMOKE_ROOT/cmake-targetless/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.16)
project(libtiff_package_smoke_targetless C)
find_package(TIFF REQUIRED CONFIG)
add_executable(test ../test.c)
target_include_directories(test PRIVATE ${TIFF_INCLUDE_DIRS})
target_link_libraries(test PRIVATE ${TIFF_LIBRARIES})
EOF
```

4. Build and smoke the safe packages. If this phase required a packaging or build fix under `safe/`, commit that fix before running these commands so the package lock points at the safe-source commit tested.

```bash
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
```

5. Prepare the local override root and generate the port lock from the actual `.deb` files. The script must validate that each selected file's internal Debian `Package` name equals the expected package name. The lock `libraries[0].commit` must be `git log -1 --format=%H -- safe`.

```bash
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
    matches = sorted(leaf.glob(f"{package}_*.deb"))
    if not matches:
        raise SystemExit(f"missing {package} deb")
    path = matches[0]
    actual_package = subprocess.check_output(["dpkg-deb", "-f", str(path), "Package"], text=True).strip()
    arch = subprocess.check_output(["dpkg-deb", "-f", str(path), "Architecture"], text=True).strip()
    if actual_package != package:
        raise SystemExit(f"{path} is {actual_package}, expected {package}")
    data = path.read_bytes()
    debs.append({
        "package": package,
        "filename": path.name,
        "architecture": arch,
        "sha256": hashlib.sha256(data).hexdigest(),
        "size": len(data),
        "asset_api_url": None,
        "browser_download_url": None,
    })

commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
release = f"local-{commit[:12]}"
lock = {
    "schema_version": 1,
    "mode": "port",
    "generated_at": "1970-01-01T00:00:00Z",
    "source_config": "repositories.yml",
    "source_inventory": "local-overrides",
    "libraries": [{
        "library": "libtiff",
        "repository": "safelibs/port-libtiff",
        "url": "https://github.com/safelibs/port-libtiff",
        "tag_ref": f"refs/tags/{release}",
        "commit": commit,
        "release_tag": release,
        "debs": debs,
        "unported_original_packages": [],
    }],
}
Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").write_text(
    json.dumps(lock, indent=2, sort_keys=True) + "\n"
)
PY
```

6. Run the full libtiff `port` matrix only, selecting only `libtiff`, using the local override root, the local port lock, and recorded casts.

```bash
cd validator
bash test.sh \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libtiff-safe \
  --mode port \
  --override-deb-root artifacts/debs/local \
  --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json \
  --library libtiff \
  --record-casts
python3 tools/verify_proof_artifacts.py \
  --config repositories.yml \
  --tests-root tests \
  --artifact-root artifacts/libtiff-safe \
  --proof-output proof/libtiff-safe-port-proof.json \
  --mode port \
  --library libtiff \
  --require-casts \
  --min-source-cases 5 \
  --min-usage-cases 130 \
  --min-cases 135 \
  --ports-root /home/yans/safelibs/pipeline/ports
cd ..
```

In validator `port` mode, `bash test.sh` may exit successfully even when libtiff validator testcases fail. Parse `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json` and per-case JSON files for pass/fail status.

7. Parse results into `validator-report.md`. It must always contain these machine-readable lines, even when empty:

```text
Validator commit: <40-char commit>
Safe source commit tested: <40-char commit>
Checks executed: <short command summary>
Failures found: <count and testcase ids>
Waived testcase ids: <comma-separated testcase ids or empty>
```

Include validator commit, safe commit used to build packages, package filenames/versions/architectures/SHA-256 hashes, commands executed, source/usage/total counts, failed testcase ids, result JSON paths, log paths, first failing symptoms, and initial triage buckets. The `Safe source commit tested:` line must equal `libraries[0].commit` in `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` and `libraries[0].port_commit` in `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.

Validator-bug waivers are exceptional and normally belong to phase 2. Do not edit validator tests or shared scripts to make a check pass.

8. Commit before yielding. If this phase changed `safe/`, commit those changes before the package build and validator run. After the run, commit `validator-report.md` and any tracked setup/report changes. If there are no applicable tracked changes, create an empty phase commit named for `impl_validator_baseline`.

## Verification Phases

### `check_validator_baseline_tester`

- Phase ID: `check_validator_baseline_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_validator_baseline`
- Purpose: Verify the validator checkout, local Debian override package root, local port lock, baseline matrix, proof artifact, and report skeleton. Do not require validator cases to pass.
- Commands:

```bash
test -d validator/.git
git -C validator rev-parse HEAD
test -d validator/artifacts/debs/local/libtiff
test -f validator/artifacts/libtiff-safe/package-smoke/test.c
test -f validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt
test -f validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt
python3 - <<'PY'
import json
import re
from pathlib import Path
for package in ("libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"):
    matches = list(Path("validator/artifacts/debs/local/libtiff").glob(f"{package}_*.deb"))
    assert matches, package
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
assert summary["cases"] == 135, summary
assert summary["source_cases"] == 5, summary
assert summary["usage_cases"] == 130, summary
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
assert proof["mode"] == "port", proof["mode"]
assert proof["totals"]["cases"] == 135, proof["totals"]
lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
safe_commit = lock["libraries"][0]["commit"]
assert proof["libraries"][0]["port_commit"] == safe_commit, (proof["libraries"][0]["port_commit"], safe_commit)
report = Path("validator-report.md").read_text()
match = re.search(r"^Safe source commit tested:\s*([0-9a-f]{40})$", report, re.MULTILINE)
assert match and match.group(1) == safe_commit, (match.group(1) if match else None, safe_commit)
PY
test -s validator-report.md
rg -n "Validator commit:|Safe source commit tested:|Checks executed:|Failures found:|Waived testcase ids:" validator-report.md
```

### `check_validator_baseline_senior`

- Phase ID: `check_validator_baseline_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_validator_baseline`
- Purpose: Confirm the phase followed the validator README flow, left validator tests unchanged, and triaged failures for later phases.
- Commands:

```bash
git status --short
git show --stat --format=fuller HEAD
git -C validator status --short
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools || true
python3 - <<'PY'
import json
from pathlib import Path
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
print("summary:", summary)
failed = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    result = json.loads(path.read_text())
    if result["status"] == "failed":
        failed.append((result["testcase_id"], result["kind"], result["log_path"]))
print("failed:", failed)
PY
rg -n "validator|testcase|failure|waiv|override|package" validator-report.md
```

## Success Criteria

- Validator unit and testcase checks pass.
- Safe package build and install-surface checks pass.
- Validator proof generation succeeds with 5 source cases, 130 usage cases, and 135 total cases.
- `validator-report.md` accurately lists all failed validator testcases or states that the run was clean.
- The nested `validator/` checkout is not committed to the parent repository.

## Git Commit Requirement

The implementer must commit work to git before yielding. If there are no applicable code or report changes, the implementer must create an empty phase commit with a message naming `impl_validator_baseline`.
