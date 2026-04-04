#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import statistics
import subprocess
import sys
import tempfile
import time


def repo_root() -> pathlib.Path:
    return pathlib.Path(__file__).resolve().parents[2]


def resolve_repo_path(root: pathlib.Path, raw: str) -> pathlib.Path:
    path = pathlib.Path(raw)
    return path if path.is_absolute() else root / path


def load_manifest(path: pathlib.Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def run_command(
    cmd: list[str],
    *,
    env: dict[str, str] | None = None,
    cwd: pathlib.Path | None = None,
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        cmd,
        cwd=str(cwd) if cwd is not None else None,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def tool_env(build_dir: pathlib.Path) -> dict[str, str]:
    env = os.environ.copy()
    library_parts = [str(build_dir), str(build_dir / "libtiff")]
    if env.get("LD_LIBRARY_PATH"):
        library_parts.append(env["LD_LIBRARY_PATH"])
    env["LD_LIBRARY_PATH"] = ":".join(library_parts)
    return env


def configure_safe_build(
    root: pathlib.Path,
    manifest: dict,
    jobs: int,
    build_dir_override: pathlib.Path | None = None,
) -> pathlib.Path:
    safe_build = manifest["safe_build"]
    source_dir = resolve_repo_path(root, safe_build["source_dir"])
    build_dir = (
        build_dir_override
        if build_dir_override is not None
        else resolve_repo_path(root, safe_build["build_dir"])
    )
    cmake_args = ["cmake", "-S", str(source_dir), "-B", str(build_dir), *safe_build["cmake_args"]]
    subprocess.run(cmake_args, cwd=str(root), check=True)
    subprocess.run(
        ["cmake", "--build", str(build_dir), "--parallel", str(jobs)],
        cwd=str(root),
        check=True,
    )
    return build_dir


def output_path_for_run(run_dir: pathlib.Path, workload: dict) -> pathlib.Path | None:
    output = workload["output"]
    if output["kind"] == "stdout":
        return None
    return run_dir / f"output{output['extension']}"


def command_for_workload(
    tools_dir: pathlib.Path,
    workload: dict,
    input_path: pathlib.Path,
    output_path: pathlib.Path | None,
) -> list[str]:
    replacements = {
        "{input}": str(input_path),
        "{output}": str(output_path) if output_path is not None else "",
    }
    args = [replacements.get(arg, arg) for arg in workload["args"]]
    return [str(tools_dir / workload["tool"]), *args]


def hash_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def validate_stdout(cp: subprocess.CompletedProcess[bytes], validator: dict) -> tuple[bool, str]:
    needle = validator["needle"].encode("utf-8")
    if not cp.stdout:
        return False, "stdout was empty"
    if needle not in cp.stdout:
        return False, f"stdout did not contain {validator['needle']!r}"
    return True, "stdout contained expected marker"


def validate_tiff_output(
    original_build_dir: pathlib.Path,
    output_path: pathlib.Path | None,
) -> tuple[bool, str]:
    if output_path is None or not output_path.is_file():
        return False, "missing TIFF output"
    if output_path.stat().st_size == 0:
        return False, "TIFF output was empty"
    validator_cmd = [
        str(original_build_dir / "tools" / "tiffinfo"),
        "-D",
        str(output_path),
    ]
    cp = run_command(validator_cmd, env=tool_env(original_build_dir))
    if cp.returncode != 0:
        return False, f"original tiffinfo -D failed with exit code {cp.returncode}"
    return True, "original tiffinfo -D accepted the output"


def validate_pdf_output(output_path: pathlib.Path | None) -> tuple[bool, str]:
    if output_path is None or not output_path.is_file():
        return False, "missing PDF output"
    if output_path.stat().st_size == 0:
        return False, "PDF output was empty"
    qpdf = shutil.which("qpdf")
    if qpdf is not None:
        cp = run_command([qpdf, "--check", str(output_path)])
        if cp.returncode != 0:
            return False, f"qpdf --check failed with exit code {cp.returncode}"
        return True, "qpdf --check accepted the output"
    pdfinfo = shutil.which("pdfinfo")
    if pdfinfo is not None:
        cp = run_command([pdfinfo, str(output_path)])
        if cp.returncode != 0:
            return False, f"pdfinfo failed with exit code {cp.returncode}"
        return True, "pdfinfo accepted the output"
    with output_path.open("rb") as handle:
        header = handle.read(5)
    if header != b"%PDF-":
        return False, "PDF output did not start with %PDF-"
    return True, "PDF header looked valid"


def validate_run(
    *,
    original_build_dir: pathlib.Path,
    workload: dict,
    output_path: pathlib.Path | None,
    cp: subprocess.CompletedProcess[bytes],
) -> tuple[bool, str]:
    if cp.returncode != 0:
        return False, f"command failed with exit code {cp.returncode}"
    validator = workload["validator"]
    kind = validator["kind"]
    if kind == "stdout_contains":
        return validate_stdout(cp, validator)
    if kind == "tiffinfo":
        return validate_tiff_output(original_build_dir, output_path)
    if kind == "qpdf":
        return validate_pdf_output(output_path)
    raise ValueError(f"unsupported validator kind: {kind}")


def writes_output_file(workload: dict) -> bool:
    return workload["output"]["kind"] in {"file", "stdout_file"}


def run_single_measurement(
    *,
    root: pathlib.Path,
    build_dir: pathlib.Path,
    original_build_dir: pathlib.Path,
    workload: dict,
    impl_name: str,
    run_dir: pathlib.Path,
) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    input_path = resolve_repo_path(root, workload["input"])
    output_path = output_path_for_run(run_dir, workload)
    command = command_for_workload(build_dir / "tools", workload, input_path, output_path)
    start = time.perf_counter()
    cp = run_command(command, env=tool_env(build_dir), cwd=run_dir)
    duration = time.perf_counter() - start
    if workload["output"]["kind"] == "stdout_file" and output_path is not None:
        output_path.write_bytes(cp.stdout)
    stdout_path = run_dir / "stdout.bin"
    stderr_path = run_dir / "stderr.bin"
    stdout_path.write_bytes(cp.stdout)
    stderr_path.write_bytes(cp.stderr)
    valid, validation_message = validate_run(
        original_build_dir=original_build_dir,
        workload=workload,
        output_path=output_path,
        cp=cp,
    )
    if writes_output_file(workload):
        payload = output_path.read_bytes() if output_path and output_path.exists() else b""
        payload_kind = "file"
    elif workload["output"]["kind"] == "stdout":
        payload = cp.stdout
        payload_kind = "stdout"
    else:
        raise ValueError(f"unsupported output kind: {workload['output']['kind']}")
    return {
        "implementation": impl_name,
        "command": command,
        "duration_seconds": duration,
        "returncode": cp.returncode,
        "valid": valid,
        "validation_message": validation_message,
        "payload_kind": payload_kind,
        "payload_sha256": hash_bytes(payload) if payload else None,
        "payload_size": len(payload),
        "stdout_path": str(stdout_path),
        "stderr_path": str(stderr_path),
        "output_path": str(output_path) if output_path is not None else None,
    }


def median_from_runs(runs: list[dict]) -> float | None:
    durations = [run["duration_seconds"] for run in runs if run["valid"]]
    if len(durations) != len(runs):
        return None
    return statistics.median(durations)


def evaluate_workload(
    *,
    root: pathlib.Path,
    original_build_dir: pathlib.Path,
    safe_build_dir: pathlib.Path,
    workload: dict,
    warmup_runs: int,
    timed_runs: int,
    temp_root: pathlib.Path,
) -> dict:
    result = {"name": workload["name"], "tool": workload["tool"], "implementations": {}}
    impls = {
        "original": original_build_dir,
        "safe": safe_build_dir,
    }
    for impl_name, build_dir in impls.items():
        impl_dir = temp_root / workload["name"] / impl_name
        warmups = []
        timed = []
        for index in range(warmup_runs):
            warmups.append(
                run_single_measurement(
                    root=root,
                    build_dir=build_dir,
                    original_build_dir=original_build_dir,
                    workload=workload,
                    impl_name=impl_name,
                    run_dir=impl_dir / f"warmup-{index + 1:02d}",
                )
            )
        for index in range(timed_runs):
            timed.append(
                run_single_measurement(
                    root=root,
                    build_dir=build_dir,
                    original_build_dir=original_build_dir,
                    workload=workload,
                    impl_name=impl_name,
                    run_dir=impl_dir / f"timed-{index + 1:02d}",
                )
            )
        result["implementations"][impl_name] = {
            "warmups": warmups,
            "warmups_valid": all(run["valid"] for run in warmups),
            "timed_runs": timed,
            "median_seconds": median_from_runs(timed),
        }
    original_median = result["implementations"]["original"]["median_seconds"]
    safe_median = result["implementations"]["safe"]["median_seconds"]
    warmups_ok = all(
        implementation["warmups_valid"] for implementation in result["implementations"].values()
    )
    if not warmups_ok or original_median is None or safe_median is None or original_median == 0:
        result["slowdown_ratio"] = None
        result["passed"] = False
    else:
        result["slowdown_ratio"] = safe_median / original_median
        result["passed"] = True
    return result


def suite_passes(results: list[dict], thresholds: dict) -> tuple[bool, float | None]:
    ratios = []
    suite_ok = True
    for result in results:
        ratio = result["slowdown_ratio"]
        result["workload_threshold"] = thresholds["max_workload_slowdown"]
        if ratio is None:
            result["passed"] = False
            suite_ok = False
            continue
        if ratio > thresholds["max_workload_slowdown"]:
            result["passed"] = False
            suite_ok = False
        ratios.append(ratio)
    suite_median = statistics.median(ratios) if ratios else None
    if suite_median is None or suite_median > thresholds["max_suite_median_slowdown"]:
        suite_ok = False
    return suite_ok, suite_median


def summarize(results: list[dict], thresholds: dict, suite_median: float | None, output_path: pathlib.Path) -> None:
    for result in results:
        ratio = result["slowdown_ratio"]
        ratio_display = "invalid" if ratio is None else f"{ratio:.3f}x"
        original = result["implementations"]["original"]["median_seconds"]
        safe = result["implementations"]["safe"]["median_seconds"]
        original_display = "invalid" if original is None else f"{original:.6f}s"
        safe_display = "invalid" if safe is None else f"{safe:.6f}s"
        status = "PASS" if result["passed"] else "FAIL"
        print(
            f"{status} {result['name']}: original={original_display} safe={safe_display} slowdown={ratio_display}"
        )
    suite_display = "invalid" if suite_median is None else f"{suite_median:.3f}x"
    print(
        "Suite median slowdown: "
        f"{suite_display} (threshold {thresholds['max_suite_median_slowdown']:.3f}x)"
    )
    print(f"Latest report: {output_path}")


def parse_args() -> argparse.Namespace:
    root = repo_root()
    default_manifest = root / "safe" / "perf" / "workloads.json"
    default_output = root / "safe" / "perf" / "latest.json"
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=pathlib.Path, default=None)
    parser.add_argument("--workloads", type=pathlib.Path, default=None)
    parser.add_argument("--output", type=pathlib.Path, default=default_output)
    parser.add_argument("--original-build", type=pathlib.Path, default=None)
    parser.add_argument("--safe-build", type=pathlib.Path, default=None)
    parser.add_argument("--max-slowdown", type=float, default=None)
    parser.add_argument("--suite-median-max", type=float, default=None)
    parser.add_argument("--warmup-runs", type=int, default=None)
    parser.add_argument("--timed-runs", type=int, default=None)
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 1)
    parser.add_argument("--keep-temp", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    parser.set_defaults(default_manifest=default_manifest)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = repo_root()
    manifest_path = resolve_repo_path(
        root,
        str(args.workloads or args.manifest or args.default_manifest),
    )
    manifest = load_manifest(manifest_path)
    thresholds = dict(manifest["thresholds"])
    if args.max_slowdown is not None:
        thresholds["max_workload_slowdown"] = args.max_slowdown
    if args.suite_median_max is not None:
        thresholds["max_suite_median_slowdown"] = args.suite_median_max
    warmup_runs = args.warmup_runs if args.warmup_runs is not None else manifest["warmup_runs"]
    timed_runs = args.timed_runs if args.timed_runs is not None else manifest["timed_runs"]
    original_build_dir = (
        resolve_repo_path(root, str(args.original_build))
        if args.original_build is not None
        else resolve_repo_path(root, manifest["baseline"]["build_dir"])
    )
    safe_build_dir = (
        resolve_repo_path(root, str(args.safe_build))
        if args.safe_build is not None
        else resolve_repo_path(root, manifest["safe_build"]["build_dir"])
    )

    if not args.skip_build:
        safe_build_dir = configure_safe_build(
            root,
            manifest,
            args.jobs,
            build_dir_override=safe_build_dir,
        )

    temp_dir_obj = tempfile.TemporaryDirectory(prefix="libtiff-perf-")
    temp_root = pathlib.Path(temp_dir_obj.name)
    try:
        results = [
            evaluate_workload(
                root=root,
                original_build_dir=original_build_dir,
                safe_build_dir=safe_build_dir,
                workload=workload,
                warmup_runs=warmup_runs,
                timed_runs=timed_runs,
                temp_root=temp_root,
            )
            for workload in manifest["workloads"]
        ]
        suite_ok, suite_median = suite_passes(results, thresholds)
        report = {
            "version": manifest["version"],
            "manifest": str(manifest_path),
            "safe_build_dir": str(safe_build_dir),
            "original_build_dir": str(original_build_dir),
            "warmup_runs": warmup_runs,
            "timed_runs": timed_runs,
            "thresholds": thresholds,
            "temp_root": str(temp_root),
            "suite_median_slowdown": suite_median,
            "suite_passed": suite_ok,
            "results": results,
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        summarize(results, thresholds, suite_median, args.output)
        return 0 if suite_ok else 1
    finally:
        if args.keep_temp:
            print(f"Preserved temp data at {temp_root}", file=sys.stderr)
            temp_dir_obj.detach()
        else:
            temp_dir_obj.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
