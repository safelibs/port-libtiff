# 02-package-waiver-gate

## Phase Name

Package Provenance And Validator Waiver Gate

## Implement Phase ID

`impl_package_provenance_waiver_gate`

## Preexisting Inputs

- `validator/.git` already selected by phase 1. Do not refetch or move it in this phase. Expected selected commit for this plan is `87b321fe728340d6fc6dd2f638583cca82c667c3`, where libtiff has 5 source cases, 170 usage cases, and 175 total cases.
- Historical baseline for comparison: `validator-report.md` previously recorded a clean run at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`, with 135 of 135 passing, 5 source plus 130 usage, casts present, and no waivers.
- Phase 1 `validator-report.md`, including the prior clean-run context at validator commit `5d908be26e33f071e119ffe1a52e3149f1e5ec4e` and safe commit `61f38826b440c30b5099410a52e1af227832622e`, plus the current validator commit, safe commit, counts, failure buckets, and `Waived testcase ids:` line.
- Phase 1 local package and proof artifacts:
  - `safe/dist/*.deb`
  - `validator/artifacts/debs/local/libtiff/*.deb`
  - `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
  - `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
  - `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`
  - `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`
  - `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`
- Validator `repositories.yml` canonical package list. The canonical package set for this port is `libtiff6`, `libtiffxx6`, `libtiff-dev`, and `libtiff-tools`.
- Existing package-smoke projects under `validator/artifacts/libtiff-safe/package-smoke/test.c`, `validator/artifacts/libtiff-safe/package-smoke/cmake-target/CMakeLists.txt`, and `validator/artifacts/libtiff-safe/package-smoke/cmake-targetless/CMakeLists.txt`.
- Safe packaging inputs: `safe/debian/control`, `safe/debian/*.install`, `safe/debian/rules`, `safe/CMakeLists.txt`, install files, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, `safe/scripts/build-deb.sh`, `safe/scripts/check-packaged-install-surface.sh`, and `scripts/lib/build_port_lock.py`.
- Safe port package facts: `safe/Cargo.toml` builds crate `safe-libtiff` as static library `tiff_safe_core`; `safe/CMakeLists.txt` builds `libtiff.so.6.0.1`, `libtiffxx.so.6.0.1`, pkg-config metadata, CMake package metadata, copied upstream tools, and optional tests; `safe/scripts/build-deb.sh` builds packages into `safe/dist/` at version `1:4.5.1+git230720-4ubuntu2.5+safelibs1`.
- Safe implementation hotspots to consult only if a package/ABI/export problem requires source-level diagnosis:
  - `safe/src/lib.rs`: `parse_open_mode` at line 458, `finalize_open` at line 810, `TIFFClientOpen` at line 1384, `TIFFClientOpenExt` at line 1414, `TIFFOpen` at line 1471, `TIFFOpenExt` at line 1476, `TIFFClose` at line 1548, and `TIFFReadDirectory` at line 1563.
  - `safe/capi/tiff_placeholder.c`: `safe_default_vset_field` at line 643, `TIFFGetField` at line 1160, `TIFFGetFieldDefaulted` at line 1184, `TIFFSetField` at line 1208, and `TIFFPrintDirectory` around line 1511.
  - `safe/src/core/directory.rs`: `read_next_directory` at line 1384 and `TIFFWriteDirectory` at line 4357.
  - `safe/src/strile.rs`: codec decode/use around lines 1262 and 2485, `TIFFWriteScanline` at line 1879, `TIFFWriteTile` at line 2007, and `TIFFReadTile` at line 2200.
  - `safe/src/core/codec.rs`: `safe_tiff_codec_decode_bytes` at line 3019 and `safe_tiff_codec_encode_bytes` at line 3070.
  - `safe/src/core/jpeg.rs`: `jpeg_decode_bytes` at line 755 and `jpeg_encode_bytes` at line 803.
- Original-mode validator artifacts under `validator/artifacts/libtiff-original/` only if a validator-bug waiver must be proved; otherwise they are not required.

Consume these artifacts in place. Do not refetch, recollect, rediscover, or regenerate `original/`, safe test fixtures, CVE inventories, dependent inventories, package scripts, ABI inventories, link-compatibility harnesses, downstream smoke harnesses, package-smoke projects, or prior validator artifacts. Do not use `make fetch-port-debs`. Do not edit validator tests, shared scripts, manifests, runner code, or tools.

## New Outputs

- Fixed packaging/install-surface files if the baseline showed package provenance failures.
- Refreshed `safe/dist/*.deb`, `validator/artifacts/debs/local/libtiff/*.deb`, `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`, `validator/artifacts/libtiff-safe/port/results/libtiff/*.json`, `validator/artifacts/libtiff-safe/port/logs/libtiff/*.log`, `validator/artifacts/libtiff-safe/port/casts/libtiff/*.cast`, and `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json` after any package fix.
- `validator-report.md` updated with package gate disposition, package/provenance failures found, fixes applied, and any fully justified waiver candidates.
- A git commit before yielding.

## File Changes

- Always: `validator-report.md`.
- Conditional packaging fixes: `safe/debian/control`, `safe/debian/*.install`, `safe/debian/rules`, `safe/CMakeLists.txt`, `safe/pkgconfig/libtiff-4.pc.in`, `safe/cmake/TiffConfig.cmake.in`, `safe/capi/libtiff-safe.map`, `safe/capi/libtiffxx-safe.map`, or package helper scripts.
- Do not modify validator runtime files: `validator/tests/libtiff`, `validator/tests/_shared`, `validator/repositories.yml`, `validator/test.sh`, or `validator/tools`.

Critical file guidance for this phase:

- Package/install fixes belong in the safe port, not validator runtime code.
- `scripts/lib/build_port_lock.py` should be consumed as-is unless the local lock generator itself is wrong for the current validator schema; if changed, verify with root validation hook tests.
- `safe/capi/libtiff-safe.map` and `safe/capi/libtiffxx-safe.map` should change only for public ABI export corrections.

## Implementation Details

Workflow-generation contract preserved for this implement block:

- Execute phases linearly. Do not generate `parallel_groups`.
- Preserve the source-plan generation boundary for downstream workflow generation: generate only `.plan/plan.md`; do not generate or edit `.plan/phases/*`, `.plan/workflow-structure.yaml`, or `workflow.yaml` from inside phase-level prompts because workflow-generation stages own those files.
- Use self-contained inline-only YAML. Do not use a top-level `include`.
- Do not use phase-level `prompt_file`, `workflow_file`, `workflow_dir`, `checks`, `source`, or any other YAML-source indirection.
- Do not generate `bounce_targets` lists. Each verifier has exactly one fixed `bounce_target`.
- Every verifier is an explicit top-level `check` phase, stays inside the implement block it verifies, and bounces only to `impl_package_provenance_waiver_gate`.
- Put verifier commands directly in the checker instructions; do not model tests, builds, proof generation, artifact parsing, or review commands as non-agentic phases.
- Consume phase 1 artifacts in place. Do not refetch, recollect, rediscover, or regenerate prepared artifacts from scratch.
- Do not refetch or move `validator/`; phase 1 is the only phase that may fetch or checkout the validator repository.

- Treat any `override_debs_installed != true`, missing canonical package, stale `port_commit`, wrong architecture, checksum mismatch, or non-empty `unported_original_packages` as a safe packaging bug unless validator original-mode evidence proves otherwise.
- For package fixes, make minimal safe-port changes and add a package/install regression where possible. Use `safe/scripts/check-packaged-install-surface.sh` with the explicit existing `validator/artifacts/libtiff-safe/package-smoke/` CMake/pkg-config projects for installed header, CMake, pkg-config, C++ facade, and tool smoke coverage.
- For suspected validator bugs, run original mode for the same validator commit and libtiff cases, capture logs under `validator/artifacts/libtiff-original/`, and document why the expectation is invalid. Do not edit validator testcases. A waiver requires original-package evidence, safe-package evidence, testcase id, validator commit, logs, result paths, and a concrete explanation in `validator-report.md`. Checkers may ignore only testcase ids listed on the machine-readable `Waived testcase ids:` line.
- Rebuild packages and rerun the full libtiff validator matrix after any safe packaging fix. The rerun must use the local README port flow: lay out local `.deb` files as `validator/artifacts/debs/local/libtiff/*.deb`, regenerate `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`, run `bash validator/test.sh --mode port --library libtiff`, and regenerate `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json` with casts.
- Before rebuilding `.deb` packages for a validator run, commit all `safe/` source, test, packaging, or script changes represented in those packages. The lock must record the safe-source commit actually tested.
- Update `validator-report.md` with the package gate result, any package hashes or provenance corrections, exact artifact paths, and waiver disposition.
- Commit before yielding with `impl_package_provenance_waiver_gate: fix libtiff package override provenance`, or create an empty/report-only phase commit when no package changes are required.

## Verification Phases

### `check_package_gate_tester`

- Phase ID: `check_package_gate_tester`
- Type: `check`
- Fixed `bounce_target`: `impl_package_provenance_waiver_gate`
- Purpose: Prove every testcase installed the local canonical packages, the lock matches on-disk `.deb` files, and any waiver candidate has original-mode evidence.
- Commands:

```bash
python3 - <<'PY'
import hashlib
import json
import re
import subprocess
from pathlib import Path

lock = json.loads(Path("validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json").read_text())
proof = json.loads(Path("validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json").read_text())
lib = lock["libraries"][0]
assert lib["unported_original_packages"] == [], lib
safe_commit = subprocess.check_output(["git", "log", "-1", "--format=%H", "--", "safe"], text=True).strip()
assert lib["commit"] == safe_commit, (lib["commit"], safe_commit)
assert proof["libraries"][0]["port_commit"] == safe_commit, (proof["libraries"][0], safe_commit)
expected = ["libtiff6", "libtiffxx6", "libtiff-dev", "libtiff-tools"]
assert [d["package"] for d in lib["debs"]] == expected, lib
for d in lib["debs"]:
    p = Path("validator/artifacts/debs/local/libtiff") / d["filename"]
    assert p.is_file(), p
    assert p.stat().st_size == d["size"], (p, d)
    assert hashlib.sha256(p.read_bytes()).hexdigest() == d["sha256"], p

bad = []
for path in sorted(Path("validator/artifacts/libtiff-safe/port/results/libtiff").glob("*.json")):
    if path.name == "summary.json":
        continue
    r = json.loads(path.read_text())
    if r.get("override_debs_installed") is not True:
        bad.append((r["testcase_id"], "override_debs_installed", r.get("override_debs_installed")))
    installed = [p["package"] for p in r.get("override_installed_packages", [])]
    if installed != expected:
        bad.append((r["testcase_id"], "installed", installed))
    if r.get("port_commit") != lib["commit"]:
        bad.append((r["testcase_id"], "port_commit", r.get("port_commit")))
assert not bad, bad[:20]

report = Path("validator-report.md").read_text()
for line in ("Validator commit:", "Safe source commit tested:", "Failures found:", "Waived testcase ids:"):
    assert re.search(rf"^{re.escape(line)}", report, re.MULTILINE), line
PY
safe/scripts/check-packaged-install-surface.sh \
  --dist-dir safe/dist \
  --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target \
  --cmake-project-no-target validator/artifacts/libtiff-safe/package-smoke/cmake-targetless \
  --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c \
  --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp \
  --input-tiff original/test/images/rgb-3c-8b.tiff
```

### `check_package_gate_senior`

- Phase ID: `check_package_gate_senior`
- Type: `check`
- Fixed `bounce_target`: `impl_package_provenance_waiver_gate`
- Purpose: Review that no validator runtime files were changed, package fixes belong to the safe port, and waiver documentation is specific enough to audit.
- Commands:

```bash
git show --stat --format=fuller HEAD
git -C validator diff -- tests/libtiff tests/_shared repositories.yml test.sh tools
test -z "$(git -C validator status --porcelain -- tests/libtiff tests/_shared repositories.yml test.sh tools)"
rg -n "Waived testcase ids:|Validator bug|original mode|override|unported|canonical|libtiff6|libtiffxx6|libtiff-dev|libtiff-tools" validator-report.md
```

## Success Criteria

- All canonical local packages are represented in the lock and installed for every validator testcase.
- Lock size and SHA-256 metadata match the `.deb` files on disk.
- The safe-source commit in the lock, proof, per-case results, and report is consistent.
- Any waiver candidate has original-mode evidence and is named on the machine-readable waiver line.
- Package-smoke checks pass with the existing package-smoke projects.
- Validator runtime files remain unchanged.

## Git Commit Requirement

The implementer must commit work to git before yielding. If the phase only updates the report or has no code change, commit the report change or create an empty phase commit naming `impl_package_provenance_waiver_gate`.
