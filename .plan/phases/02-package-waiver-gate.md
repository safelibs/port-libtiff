# 02-package-waiver-gate

## Phase Name

Package Override And Waiver Gate

## Implement Phase ID

`impl_package_provenance_waiver_gate`

## Preexisting Inputs

- Baseline `validator-report.md` from `impl_validator_baseline`.
- Baseline validator artifacts under `validator/artifacts/libtiff-safe/port/`.
- Baseline local override packages under `validator/artifacts/debs/local/libtiff/`.
- Baseline local port lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Baseline port proof at `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/`, including `test.c`, `cmake-target/CMakeLists.txt`, and `cmake-targetless/CMakeLists.txt`.
- The external validator checkout under `validator/`.
- Safe Debian packaging in `safe/debian/`.
- `safe/scripts/build-deb.sh`
- `safe/scripts/check-packaged-install-surface.sh`
- `safe/test/install/tiffxx_staged_smoke.cpp`
- `original/test/images/rgb-3c-8b.tiff`
- Original-package validator behavior if a validator-bug waiver must be proved.

Consume these artifacts in place. Do not refetch or regenerate `original/`, `safe/test/`, CVE data, dependent inventories, package scripts, link-compatibility harnesses, downstream smoke harnesses, or the package-smoke projects. Do not recreate missing `original/build/` outputs. Do not edit validator tests or shared scripts to make a check pass.

Do not use `make fetch-port-debs`. That target fetches GitHub release assets. This workflow uses locally built `.deb` packages from `safe/` and `--override-deb-root`.

Preserve this package and build-surface contract while making any package/provenance fixes:

- `safe/Cargo.toml` builds Rust crate `safe-libtiff` as static library `tiff_safe_core`; package fixes must not accidentally replace that core artifact with a different build product.
- `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config and CMake metadata, copied tools, and optional tests.
- `safe/scripts/build-deb.sh` builds exactly `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools` packages at Debian version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- `safe/scripts/check-packaged-install-surface.sh` must keep verifying package contents, headers, CMake/pkg-config integration, C++ facade, packaged tools, and the generated package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/`.
- Do not regenerate missing `original/build/test_cmake*` fixtures; consume the phase-1 package-smoke projects in place.

## New Outputs

- Package/install/link fixes in `safe/` if local override installation fails.
- Regenerated local override artifacts under `validator/artifacts/debs/local/libtiff/`.
- Regenerated local port lock at `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`.
- Regenerated port proof at `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`.
- Original-mode validator artifacts under `validator/artifacts/libtiff-original/` only if a validator-bug waiver must be proved.
- Original-mode proof at `validator/artifacts/libtiff-original/proof/libtiff-original-proof.json` only if a validator-bug waiver must be proved.
- `validator-report.md` updated with exact package versions, hashes, override install status, and any validator-bug waiver.
- A git commit containing package/provenance/report changes.

## File Changes

- Possible package/provenance fixes: `safe/debian/control`, `safe/debian/rules`, `safe/debian/changelog`, `safe/debian/*.install`, `safe/debian/libtiff6.symbols`, `safe/debian/libtiffxx6.symbols`.
- Possible package integration fixes: `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`, `safe/include/*.h`, `safe/include/*.hxx`, `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`.
- Possible ABI/link files only for a package/link issue actually exposed here: `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/abi/*`.
- Update `validator-report.md`.
- Do not edit validator tests or shared scripts.
- Do not add the nested `validator/` checkout to the parent repository.

## Implementation Details

- Verify that the parent `safe/` tree is clean before rebuilding packages. Package fixes must be committed before the rebuild so the generated lock records the safe-source commit actually tested.
- If result JSON shows `override_debs_installed` is false, fix the local override root, package filenames, package metadata, package dependencies, maintainer scripts, or package splitting in `safe/`.
- The validator expects `validator/artifacts/debs/local/libtiff/*.deb` and a lock whose `.debs` entries exactly match filenames, sizes, and SHA-256 hashes. The lock `commit` must be the latest commit that touched `safe/`, after any package fixes have been committed. The `Safe source commit tested:` report line, result JSON `port_commit`, and proof `libraries[0].port_commit` must all refer to that same safe-source commit.
- Regenerate the local port lock from the actual `.deb` files. Keep the phase-1 package-name validation: for each of `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`, read `dpkg-deb -f <path> Package` and fail if it does not match the expected package name. Record filename, architecture, SHA-256, and size.
- Rebuild `safe/dist/*.deb`, rerun `safe/scripts/check-packaged-install-surface.sh` using the existing package-smoke projects, copy local packages into `validator/artifacts/debs/local/libtiff/`, regenerate `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`, rerun the full libtiff validator `port` matrix, and regenerate proof artifacts.
- Allow ordinary libtiff-safe behavioral failures to remain for later phases. This phase owns package provenance and validator-bug waiver adjudication, not source/runtime bug fixing.
- In validator `port` mode, `bash test.sh` may exit successfully even when libtiff validator testcases fail. Parse `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json` and per-case JSON files for pass/fail status.
- Package/provenance and validator-bug waiver adjudication must run before source and usage fix phases. Later source and usage checkers may allow failures only if the exact testcase ids are already documented on `Waived testcase ids:` with evidence.

If a validator failure is clearly due to a validator bug, prove it before adding a waiver. Run the validator `original` matrix for libtiff, capture the exit status even if it is nonzero, run proof generation for the original artifacts, and compare original and safe result JSON/logs for the exact testcase.

```bash
cd validator
set +e
bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-original --mode original --library libtiff --record-casts
original_matrix_status=$?
set -e
python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-original --proof-output proof/libtiff-original-proof.json --mode original --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135
printf 'original matrix exit status: %s\n' "$original_matrix_status"
cd ..
```

A waiver is valid only when the validator expectation is demonstrably invalid or inapplicable, and a libtiff-safe code change would be wrong. Record every waiver on the single `Waived testcase ids:` line, with detailed justification below it. Each waiver must include testcase id, validator commit, safe result, original-package result when relevant, log/result paths, why the validator expectation is wrong, and why no libtiff-safe change is appropriate. Final parsers may ignore only testcase ids named on the `Waived testcase ids:` line.

`validator-report.md` must continue to contain these machine-readable lines:

```text
Validator commit: <40-char commit>
Safe source commit tested: <40-char commit>
Checks executed: <short command summary>
Failures found: <count and testcase ids>
Waived testcase ids: <comma-separated testcase ids or empty>
```

Commit any `safe/` package/provenance fixes before rebuilding `.deb` files and regenerating the port lock. After the validator run, commit `validator-report.md`. If no package or waiver work is needed, update the report and create an empty phase commit named for `impl_package_provenance_waiver_gate`.

## Verification Phases

### `check_package_waiver_tester`

- Phase ID: `check_package_waiver_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_package_provenance_waiver_gate`
- Purpose: Verify Debian package contents, local override installation metadata, port-lock provenance, and any documented validator-bug waiver before source/usage fixes. Allow ordinary libtiff-safe behavioral failures to remain for later phases.
- Commands:

```bash
git diff --quiet -- safe
git diff --cached --quiet -- safe
safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
python3 - <<'PY'
import subprocess
from pathlib import Path
for package in ("libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"):
    matches = sorted(Path("safe/dist").glob(f"{package}_*.deb"))
    assert matches, package
    path = matches[0]
    print(package, subprocess.check_output(["dpkg-deb", "-f", str(path), "Version"], text=True).strip(), path)
PY
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
lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
safe_commit = lock["libraries"][0]["commit"]
assert proof["libraries"][0]["port_commit"] == safe_commit, (proof["libraries"][0]["port_commit"], safe_commit)
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    result = json.loads(path.read_text())
    assert result["override_debs_installed"] is True, (path, result.get("error"))
    assert result["port_commit"] == safe_commit, (path, result["port_commit"], safe_commit)
    installed = {row["package"] for row in result["override_installed_packages"]}
    assert {"libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"}.issubset(installed), (path, installed)
report = Path("validator-report.md").read_text()
assert "Waived testcase ids:" in report
match = re.search(r"^Safe source commit tested:\s*([0-9a-f]{40})$", report, re.MULTILINE)
assert match and match.group(1) == safe_commit, (match.group(1) if match else None, safe_commit)
print(summary)
PY
```

### `check_package_waiver_senior`

- Phase ID: `check_package_waiver_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_package_provenance_waiver_gate`
- Purpose: Review package/provenance changes and any waiver for evidence, scope, and absence of validator-suite edits.
- Commands:

```bash
git show --stat --format=fuller HEAD
git show -- safe/debian safe/CMakeLists.txt safe/pkgconfig safe/cmake safe/include safe/scripts validator-report.md
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools || true
python3 - <<'PY'
import json
from pathlib import Path
summary = json.loads(Path("validator/artifacts/libtiff-safe/port/results/libtiff/summary.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
print("summary:", summary)
print("proof totals:", proof["totals"])
print("port commit:", proof["libraries"][0]["port_commit"])
PY
rg -n "Validator bug|waiver|Waived testcase ids|override|port lock|debian|package|original result|safe result" validator-report.md
```

## Success Criteria

- Safe package build and install-surface checks pass.
- Local override installation succeeds in every result JSON.
- Port lock metadata matches the actual local `.deb` files and the latest commit that touched `safe/`.
- Any waiver is documented before source and usage checkers rely on it.
- Validator tests and shared scripts remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If there are no applicable code or report changes, the implementer must create an empty phase commit with a message naming `impl_package_provenance_waiver_gate`.
