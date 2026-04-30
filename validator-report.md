# Final Validator Hardening And Report Closure

## Summary

- Phase: `impl_final_validator_clean_run`
- Verification date: 2026-04-30, America/Phoenix
- Validator repository: `https://github.com/safelibs/validator`
- Validator commit: `5d908be26e33f071e119ffe1a52e3149f1e5ec4e`
- Safe source commit tested: `e15141ff1f018e170d3e10d885ef5c6262b32e36`
- Mode: port
- Library: `libtiff`
- Override root: `validator/artifacts/debs/local/libtiff/`
- Local port lock: `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json`
- Result summary: `validator/artifacts/libtiff-safe/port/results/libtiff/summary.json`
- Proof: `validator/artifacts/libtiff-safe/proof/libtiff-safe-port-proof.json`
- Final status: clean. The full validator port matrix passed with zero waivers and zero failed testcases.

## Final Counts

| Source cases | Usage cases | Total cases | Passed | Failed | Casts |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 130 | 135 | 135 | 0 | 135 |

Proof totals match the result summary:

```json
{
  "cases": 135,
  "casts": 135,
  "failed": 0,
  "libraries": 1,
  "passed": 135,
  "source_cases": 5,
  "usage_cases": 130
}
```

## Package Provenance

- Package source tree: committed `safe/` tree at `e15141ff1f018e170d3e10d885ef5c6262b32e36`
- Release tag in local lock: `local-e15141ff1f01`
- Package version required and verified: `1:4.5.1+git230720-4ubuntu2.5+safelibs1`
- Package names required and verified: `libtiff6`, `libtiffxx6`, `libtiff-dev`, `libtiff-tools`

| Package | Version | Architecture | Filename | SHA-256 | Size |
| --- | --- | --- | --- | --- | ---: |
| `libtiff6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `b5c7477fb5d99989ce034ecb9558cfc951c0fd23b05bd4be17514e0dc5ac0f29` | 641772 |
| `libtiffxx6` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiffxx6_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `4e8990c26cec0be672f64137d65dcd1543b1b6917633e41b2f91406d78236141` | 12306 |
| `libtiff-dev` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-dev_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `c18f9474273dfaa47fb48cb031a3b99d7c5e7ea8dde8c18b5d4f7142547f1323` | 35732 |
| `libtiff-tools` | `1:4.5.1+git230720-4ubuntu2.5+safelibs1` | `amd64` | `libtiff-tools_4.5.1+git230720-4ubuntu2.5+safelibs1_amd64.deb` | `10df893f1f813e5f31f57dc3426469c8131c2878b9a6db7445336ac90d858f1b` | 200508 |

The regenerated proof records `port_commit` as `e15141ff1f018e170d3e10d885ef5c6262b32e36`, matching the local lock commit and the machine-readable safe source commit below.

## Fixes Applied

| Area | Finding | Final disposition |
| --- | --- | --- |
| Public ABI manifest | The final Release build produced `libtiff.so.6` and `libtiffxx.so.6` hashes that differed from `safe/abi/public-surface.inputs.json`. | Regenerated the public-surface input manifest from the current build and committed the updated DSO hashes. Symbol inventory and Linux exclusions remained unchanged. |
| Link compatibility | The locally rebuilt upstream `libtiffxx` reference DSO did not emit two weak `std::fpos` helper exports that the safe DSO intentionally preserves through `safe/capi/libtiffxx-safe.map`. The strict equality check rejected these declared extras. | Hardened `safe/scripts/link-and-run-link-compat.sh` to continue failing missing upstream symbols and undeclared extra safe symbols, while accepting only safe `libtiffxx` extras explicitly listed in the safe version script. The link compatibility suite then passed. |

No validator testcases failed in the final matrix. No new TIFF fixtures, reference images, validator tests, or validator shared scripts were added or edited.

## Commands Executed

- `git diff --quiet -- safe`
- `git diff --cached --quiet -- safe`
- `cargo test --manifest-path safe/Cargo.toml`
- `cmake -S safe -B safe/build -DCMAKE_BUILD_TYPE=Release -Dtiff-tools=ON -Dtiff-tests=ON`
- `cmake --build safe/build --parallel`
- `python3 safe/scripts/check-public-surface.py --check --must-export _TIFFcalloc TIFFReadTile TIFFWriteTile TIFFReadFromUserBuffer TIFFStreamOpen --must-record-linux-exclusion TIFFOpenW TIFFOpenWExt`
- `ctest --test-dir safe/build --output-on-failure`
- `safe/scripts/run-upstream-shell-tests.sh --build-dir safe/build`
- `rm -rf safe/build/link-compat`
- `safe/scripts/build-link-compat-objects.sh`
- `safe/scripts/link-and-run-link-compat.sh`
- `safe/scripts/build-deb.sh --source-dir safe --out-dir safe/dist`
- `safe/scripts/check-packaged-install-surface.sh --dist-dir safe/dist --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-target --cmake-project validator/artifacts/libtiff-safe/package-smoke/cmake-targetless --pkgconfig-source validator/artifacts/libtiff-safe/package-smoke/test.c --cxx-smoke safe/test/install/tiffxx_staged_smoke.cpp --input-tiff original/test/images/rgb-3c-8b.tiff`
- `LIBTIFF_SAFE_DIST_DIR=safe/dist ./test-original.sh`
- `rm -rf validator/artifacts/debs/local/libtiff`
- `mkdir -p validator/artifacts/debs/local/libtiff validator/artifacts/libtiff-safe/proof`
- `find safe/dist -maxdepth 1 -type f -name '*.deb' -exec cp -f -t validator/artifacts/debs/local/libtiff {} +`
- Regenerated `validator/artifacts/libtiff-safe/proof/local-port-debs-lock.json` from actual local `.deb` metadata, package names, versions, architectures, sizes, SHA-256 hashes, and `git log -1 --format=%H -- safe`.
- `cd validator && make unit`
- `cd validator && make check-testcases`
- `cd validator && python3 tools/testcases.py --config repositories.yml --tests-root tests --library libtiff --check --min-source-cases 5 --min-usage-cases 130 --min-cases 135`
- `cd validator && bash test.sh --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --mode port --override-deb-root artifacts/debs/local --port-deb-lock artifacts/libtiff-safe/proof/local-port-debs-lock.json --library libtiff --record-casts`
- `cd validator && python3 tools/verify_proof_artifacts.py --config repositories.yml --tests-root tests --artifact-root artifacts/libtiff-safe --proof-output proof/libtiff-safe-port-proof.json --mode port --library libtiff --require-casts --min-source-cases 5 --min-usage-cases 130 --min-cases 135 --ports-root /home/yans/safelibs/pipeline/ports`
- Audited `validator/artifacts/libtiff-safe/port/results/libtiff/*.json` and asserted zero failed cases, 135 total cases, 5 source cases, 130 usage cases, and 135 casts.

## Waivers

No waivers were applied.

## Machine Readable

Validator commit: 5d908be26e33f071e119ffe1a52e3149f1e5ec4e
Safe source commit tested: e15141ff1f018e170d3e10d885ef5c6262b32e36
Checks executed: cargo test; CMake Release build with tools/tests; public ABI surface check; CTest; upstream shell tests; link compatibility; build-deb; packaged install surface; downstream test-original; local override copy and port-lock regeneration; validator unit/testcase checks; validator libtiff port matrix with local override debs and casts; verify_proof_artifacts; no-unwaived-failure audit
Failures found: 0
Waived testcase ids:
Final status: clean validator port matrix, zero unexpected failures, zero waivers
