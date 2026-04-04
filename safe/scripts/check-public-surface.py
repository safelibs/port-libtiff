#!/usr/bin/env python3

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Tuple


REPO_ROOT = Path(__file__).resolve().parents[2]
SAFE_ROOT = REPO_ROOT / "safe"
ORIGINAL_ROOT = REPO_ROOT / "original"

INVENTORY_PATH = SAFE_ROOT / "abi" / "public-surface.json"
INPUTS_PATH = SAFE_ROOT / "abi" / "public-surface.inputs.json"
LINUX_EXCLUDED_PATH = SAFE_ROOT / "abi" / "platform-excluded-linux.txt"

CC = shutil.which("gcc") or shutil.which("cc")
CXX = shutil.which("g++") or shutil.which("c++")

HEADER_REGEX = (
    r"extern\s+[^;{]*?\b("
    r"TIFF[A-Za-z0-9_]+|"
    r"_TIFF[A-Za-z0-9_]+|"
    r"LogL(?:16|10)(?:toY|fromY)|"
    r"LogLuv(?:24|32)(?:toXYZ|fromXYZ)|"
    r"XYZtoRGB24|"
    r"uv_(?:decode|encode)|"
    r"TIFFStreamOpen"
    r")\s*\("
)

PUBLIC_HEADER_TARGETS = [
    {
        "language": "c",
        "compiler": CC,
        "source": SAFE_ROOT / "include" / "tiffio.h",
        "include_dir": SAFE_ROOT / "include",
    },
    {
        "language": "c++",
        "compiler": CXX,
        "source": SAFE_ROOT / "include" / "tiffio.hxx",
        "include_dir": SAFE_ROOT / "include",
    },
]

CONFIG_HEADERS = [
    SAFE_ROOT / "include" / "tif_config.h",
    SAFE_ROOT / "include" / "tiffconf.h",
    SAFE_ROOT / "include" / "tiff.h",
    SAFE_ROOT / "include" / "tiffio.h",
    SAFE_ROOT / "include" / "tiffio.hxx",
    SAFE_ROOT / "include" / "tiffvers.h",
]

LINUX_EXCLUDED_SYMBOLS = {
    "TIFFOpenW": {
        "library": "libtiff.so.6",
        "reason": "Declaration is gated behind __WIN32__ in the public header set.",
    },
    "TIFFOpenWExt": {
        "library": "libtiff.so.6",
        "reason": "Declaration is gated behind __WIN32__ in the public header set.",
    },
}

LIBRARIES = [
    {
        "name": "libtiff",
        "soname": "libtiff.so.6",
        "safe_map": SAFE_ROOT / "capi" / "libtiff-safe.map",
        "upstream_map": ORIGINAL_ROOT / "libtiff" / "libtiff.map",
        "debian_symbols": ORIGINAL_ROOT / "debian" / "libtiff6.symbols",
        "observed_dso": ORIGINAL_ROOT / "build" / "libtiff" / "libtiff.so.6.0.1",
        "header_names": None,
    },
    {
        "name": "libtiffxx",
        "soname": "libtiffxx.so.6",
        "safe_map": SAFE_ROOT / "capi" / "libtiffxx-safe.map",
        "upstream_map": ORIGINAL_ROOT / "libtiff" / "libtiffxx.map",
        "debian_symbols": ORIGINAL_ROOT / "debian" / "libtiffxx6.symbols",
        "observed_dso": ORIGINAL_ROOT / "build" / "libtiff" / "libtiffxx.so.6.0.1",
        "header_names": {"TIFFStreamOpen"},
    },
]


def run_command(args: List[str], cwd: Path | None = None) -> str:
    completed = subprocess.run(
        args,
        cwd=str(cwd) if cwd else None,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout


def repo_relative(path: Path) -> str:
    return path.resolve().relative_to(REPO_ROOT).as_posix()


def display_path(path: Path) -> str:
    try:
        return repo_relative(path)
    except Exception:
        return str(path)


def normalize_command_arg(arg: str) -> str:
    path_arg = Path(arg)
    if path_arg.is_absolute() and path_arg.exists() and REPO_ROOT in path_arg.resolve().parents:
        return repo_relative(path_arg)
    if os.path.isabs(arg) and os.path.basename(arg) in {"gcc", "g++", "cc", "c++"}:
        return os.path.basename(arg)
    return arg


def sha256_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalize_text(text: str) -> str:
    return text.replace("\r\n", "\n")


def write_if_changed(path: Path, text: str) -> None:
    existing = path.read_text() if path.exists() else None
    if existing != text:
        path.write_text(text)


def diff_text(name: str, expected: str, actual: str) -> str:
    diff = difflib.unified_diff(
        actual.splitlines(),
        expected.splitlines(),
        fromfile=f"{name} (checked-in)",
        tofile=f"{name} (expected)",
        lineterm="",
    )
    return "\n".join(diff)


def preprocess_header(target: Dict[str, object]) -> Tuple[str, List[Path], List[str]]:
    compiler = target["compiler"]
    if not compiler:
        raise RuntimeError(f"missing compiler for {target['language']} header pass")
    source = Path(target["source"])
    include_dir = Path(target["include_dir"])
    deps_args = [compiler, "-M", "-I", str(include_dir), "-x", str(target["language"]), str(source)]
    preprocess_args = [
        compiler,
        "-E",
        "-dD",
        "-P",
        "-I",
        str(include_dir),
        "-x",
        str(target["language"]),
        str(source),
    ]
    deps_output = run_command(deps_args)
    preprocessed = run_command(preprocess_args)
    repo_deps = []
    tokens = deps_output.replace("\\\n", " ").split()
    for token in tokens[1:]:
        dep = Path(token)
        if dep.is_absolute():
            continue
        dep_path = (REPO_ROOT / dep).resolve()
        if dep_path.is_file() and REPO_ROOT in dep_path.parents:
            repo_deps.append(dep_path)
    unique_deps = sorted({dep.resolve() for dep in repo_deps})
    return preprocessed, unique_deps, preprocess_args


def parse_header_symbols() -> Tuple[Dict[str, Dict[str, object]], Dict[str, object]]:
    pattern = re.compile(HEADER_REGEX, re.S)
    symbols: Dict[str, Dict[str, object]] = {}
    commands = []
    consumed_paths = []
    for target in PUBLIC_HEADER_TARGETS:
        text, deps, command = preprocess_header(target)
        commands.append(
            {
                "language": target["language"],
                "args": [normalize_command_arg(arg) for arg in command],
            }
        )
        consumed_paths.extend(deps)
        source_rel = repo_relative(Path(target["source"]))
        for match in pattern.finditer(text):
            name = match.group(1)
            entry = symbols.setdefault(
                name,
                {
                    "base_name": name,
                    "header_sources": [],
                },
            )
            if source_rel not in entry["header_sources"]:
                entry["header_sources"].append(source_rel)
    for entry in symbols.values():
        entry["header_sources"].sort()
    metadata = {
        "commands": commands,
        "consumed_paths": sorted({repo_relative(path) for path in consumed_paths}),
    }
    return symbols, metadata


def parse_config_header(path: Path) -> Dict[str, str]:
    defines: Dict[str, str] = {}
    for line in path.read_text().splitlines():
        if not line.startswith("#define "):
            continue
        parts = line.split(None, 2)
        if len(parts) == 2:
            _, name = parts
            value = ""
        else:
            _, name, value = parts
        defines[name] = value
    return defines


def parse_version_script(path: Path) -> Dict[str, object]:
    current_version = None
    current_scope = None
    symbol_versions: Dict[str, str] = {}
    version_nodes: List[str] = []
    wildcard_global = False

    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        match = re.match(r"([A-Za-z0-9_.]+)\s*\{", line)
        if match:
            current_version = match.group(1)
            current_scope = "global"
            version_nodes.append(current_version)
            continue
        if line == "global:":
            current_scope = "global"
            continue
        if line == "local:":
            current_scope = "local"
            continue
        if line.startswith("}"):
            current_version = None
            current_scope = None
            continue
        if line == "*;":
            if current_scope == "global":
                wildcard_global = True
            continue
        if current_scope == "global" and current_version and line.endswith(";"):
            symbol_versions[line[:-1]] = current_version

    return {
        "path": repo_relative(path),
        "version_nodes": version_nodes,
        "symbol_versions": symbol_versions,
        "wildcard_global": wildcard_global,
    }


def parse_debian_symbols(path: Path) -> Dict[str, object]:
    version_nodes: Dict[str, str] = {}
    c_symbols: Dict[str, Dict[str, str]] = {}
    cpp_symbols: Dict[str, Dict[str, str]] = {}

    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("libtiff"):
            continue

        cpp_match = re.match(r'^\(c\+\+\)"(.+?)@([A-Za-z0-9_.]+)"\s+(.+)$', line)
        if cpp_match:
            cpp_symbols[cpp_match.group(1)] = {
                "version": cpp_match.group(2),
                "package_min_version": cpp_match.group(3),
            }
            continue

        c_match = re.match(r"^([A-Za-z0-9_]+)@([A-Za-z0-9_.]+)\s+(.+)$", line)
        if not c_match:
            continue
        symbol_name = c_match.group(1)
        version = c_match.group(2)
        package_min_version = c_match.group(3)
        if symbol_name == version:
            version_nodes[symbol_name] = package_min_version
        else:
            c_symbols[symbol_name] = {
                "version": version,
                "package_min_version": package_min_version,
            }

    return {
        "path": repo_relative(path),
        "version_nodes": version_nodes,
        "c_symbols": c_symbols,
        "cpp_symbols": cpp_symbols,
    }


def demangle_names(names: Iterable[str]) -> Dict[str, str]:
    unique_names = sorted(set(names))
    if not unique_names:
        return {}
    try:
        output = run_command(["c++filt", *unique_names])
        demangled = output.splitlines()
        if len(demangled) != len(unique_names):
            raise RuntimeError("c++filt output length mismatch")
        return dict(zip(unique_names, demangled))
    except Exception:
        return {name: name for name in unique_names}


def parse_observed_exports(path: Path) -> Dict[str, object]:
    raw_output = run_command(["nm", "-D", "--defined-only", str(path)])
    lines = [line for line in raw_output.splitlines() if line.strip()]
    raw_names = []
    parsed = []
    version_nodes = []
    for line in lines:
        parts = line.split()
        if len(parts) != 3:
            continue
        _, symbol_type, symbol_version = parts
        if symbol_type == "A":
            version_nodes.append(symbol_version)
            continue
        if "@@" in symbol_version:
            raw_name, version = symbol_version.rsplit("@@", 1)
        elif "@" in symbol_version:
            raw_name, version = symbol_version.rsplit("@", 1)
        else:
            raw_name, version = symbol_version, ""
        raw_names.append(raw_name)
        parsed.append(
            {
                "link_name": raw_name,
                "version": version,
                "nm_type": symbol_type,
            }
        )

    demangled = demangle_names(raw_names)
    exports = {}
    for entry in parsed:
        link_name = entry["link_name"]
        pretty = demangled.get(link_name, link_name)
        exports[link_name] = {
            "link_name": link_name,
            "demangled_name": pretty,
            "base_name": pretty.split("(", 1)[0],
            "version": entry["version"],
            "binding": "weak" if entry["nm_type"] in {"W", "V"} else "global",
            "observed": True,
        }

    return {
        "path": repo_relative(path),
        "version_nodes": version_nodes,
        "exports": exports,
    }


def build_libtiff_symbol_records(
    library: Dict[str, object],
    header_symbols: Dict[str, Dict[str, object]],
    safe_map: Dict[str, object],
    upstream_map: Dict[str, object],
    debian_symbols: Dict[str, object],
    observed_exports: Dict[str, object],
) -> List[Dict[str, object]]:
    candidate_names = set()
    header_names = {
        name
        for name in header_symbols
        if name != "TIFFStreamOpen"
    }
    candidate_names.update(header_names)
    candidate_names.update(safe_map["symbol_versions"].keys())
    candidate_names.update(upstream_map["symbol_versions"].keys())
    candidate_names.update(debian_symbols["c_symbols"].keys())
    candidate_names.update(observed_exports["exports"].keys())

    records = []
    for name in sorted(candidate_names):
        header_entry = header_symbols.get(name)
        observed_entry = observed_exports["exports"].get(name)
        debian_entry = debian_symbols["c_symbols"].get(name)
        safe_version = safe_map["symbol_versions"].get(name)
        upstream_version = upstream_map["symbol_versions"].get(name)
        observed_version = observed_entry["version"] if observed_entry else None
        debian_version = debian_entry["version"] if debian_entry else None
        linux_excluded = name in LINUX_EXCLUDED_SYMBOLS
        required_version = safe_version or observed_version or debian_version or upstream_version
        if not required_version:
            raise RuntimeError(f"unable to determine version node for {name}")

        mismatch_flags = []
        if linux_excluded:
            mismatch_flags.append("linux_excluded")
        if header_entry and not observed_entry:
            mismatch_flags.append("missing_from_original_linux_dso")
        if header_entry and not upstream_version:
            mismatch_flags.append("not_in_upstream_version_script")
        if not header_entry and (safe_version or observed_entry or debian_entry):
            mismatch_flags.append("not_in_public_headers")
        if observed_entry and not debian_entry and not header_entry:
            mismatch_flags.append("observed_only")

        records.append(
            {
                "name": name,
                "base_name": name,
                "owning_library": library["soname"],
                "required_version_node": required_version,
                "linux_required": not linux_excluded,
                "linux_excluded": linux_excluded,
                "demangled_name": name,
                "binding": observed_entry["binding"] if observed_entry else "global",
                "source_provenance": {
                    "public_header": bool(header_entry),
                    "safe_version_script": bool(safe_version),
                    "upstream_version_script": bool(upstream_version),
                    "debian_symbols": bool(debian_entry),
                    "observed_export": bool(observed_entry),
                },
                "provenance_files": {
                    "public_header": header_entry["header_sources"] if header_entry else [],
                    "safe_version_script": [safe_map["path"]] if safe_version else [],
                    "upstream_version_script": [upstream_map["path"]] if upstream_version else [],
                    "debian_symbols": [debian_symbols["path"]] if debian_entry else [],
                    "observed_export": [observed_exports["path"]] if observed_entry else [],
                },
                "mismatch_flags": mismatch_flags,
                "notes": [LINUX_EXCLUDED_SYMBOLS[name]["reason"]] if linux_excluded else [],
            }
        )
    return records


def build_libtiffxx_symbol_records(
    library: Dict[str, object],
    header_symbols: Dict[str, Dict[str, object]],
    safe_map: Dict[str, object],
    upstream_map: Dict[str, object],
    debian_symbols: Dict[str, object],
    observed_exports: Dict[str, object],
) -> List[Dict[str, object]]:
    candidate_names = set()
    candidate_names.update(safe_map["symbol_versions"].keys())
    candidate_names.update(observed_exports["exports"].keys())

    debian_by_demangled = debian_symbols["cpp_symbols"]
    records = []
    for link_name in sorted(candidate_names):
        observed_entry = observed_exports["exports"].get(link_name)
        safe_version = safe_map["symbol_versions"].get(link_name)
        observed_version = observed_entry["version"] if observed_entry else None
        required_version = safe_version or observed_version
        if not required_version:
            raise RuntimeError(f"unable to determine version node for {link_name}")
        demangled_name = observed_entry["demangled_name"] if observed_entry else link_name
        base_name = observed_entry["base_name"] if observed_entry else link_name
        header_entry = header_symbols.get(base_name)
        debian_entry = debian_by_demangled.get(demangled_name)

        mismatch_flags = []
        if not header_entry and (safe_version or observed_entry):
            mismatch_flags.append("not_in_public_headers")
        if observed_entry and not debian_entry and not header_entry:
            mismatch_flags.append("observed_only")

        records.append(
            {
                "name": link_name,
                "base_name": base_name,
                "owning_library": library["soname"],
                "required_version_node": required_version,
                "linux_required": True,
                "linux_excluded": False,
                "demangled_name": demangled_name,
                "binding": observed_entry["binding"] if observed_entry else "global",
                "source_provenance": {
                    "public_header": bool(header_entry),
                    "safe_version_script": bool(safe_version),
                    "upstream_version_script": False,
                    "debian_symbols": bool(debian_entry),
                    "observed_export": bool(observed_entry),
                },
                "provenance_files": {
                    "public_header": header_entry["header_sources"] if header_entry else [],
                    "safe_version_script": [safe_map["path"]] if safe_version else [],
                    "upstream_version_script": [upstream_map["path"]] if upstream_map["wildcard_global"] else [],
                    "debian_symbols": [debian_symbols["path"]] if debian_entry else [],
                    "observed_export": [observed_exports["path"]] if observed_entry else [],
                },
                "mismatch_flags": mismatch_flags,
                "notes": [],
            }
        )
    return records


def collect_inventory() -> Tuple[Dict[str, object], Dict[str, object], List[str]]:
    header_symbols, header_metadata = parse_header_symbols()

    config_snapshots = {
        repo_relative(path): parse_config_header(path)
        for path in CONFIG_HEADERS
    }

    library_records = []
    consumed_source_paths = set(header_metadata["consumed_paths"])
    consumed_source_paths.update(repo_relative(path) for path in CONFIG_HEADERS)

    for library in LIBRARIES:
        safe_map = parse_version_script(Path(library["safe_map"]))
        upstream_map = parse_version_script(Path(library["upstream_map"]))
        debian_symbols = parse_debian_symbols(Path(library["debian_symbols"]))
        observed_exports = parse_observed_exports(Path(library["observed_dso"]))

        consumed_source_paths.update(
            [
                safe_map["path"],
                upstream_map["path"],
                debian_symbols["path"],
                observed_exports["path"],
            ]
        )

        if library["soname"] == "libtiff.so.6":
            symbols = build_libtiff_symbol_records(
                library,
                header_symbols,
                safe_map,
                upstream_map,
                debian_symbols,
                observed_exports,
            )
        else:
            symbols = build_libtiffxx_symbol_records(
                library,
                header_symbols,
                safe_map,
                upstream_map,
                debian_symbols,
                observed_exports,
            )

        library_records.append(
            {
                "name": library["name"],
                "soname": library["soname"],
                "safe_version_script": {
                    "path": safe_map["path"],
                    "version_nodes": safe_map["version_nodes"],
                },
                "upstream_version_script": {
                    "path": upstream_map["path"],
                    "version_nodes": upstream_map["version_nodes"],
                    "wildcard_global": upstream_map["wildcard_global"],
                },
                "debian_symbols": {
                    "path": debian_symbols["path"],
                    "version_nodes": debian_symbols["version_nodes"],
                },
                "observed_exports": {
                    "path": observed_exports["path"],
                    "version_nodes": observed_exports["version_nodes"],
                },
                "symbols": symbols,
            }
        )

    triple = run_command([CC or "gcc", "-dumpmachine"]).strip()
    platform = {
        "triple": triple,
        "system": run_command(["uname", "-s"]).strip(),
        "distribution_baseline": "ubuntu-24.04",
    }

    inventory = {
        "schema_version": 1,
        "platform": platform,
        "collector": {
            "header_symbol_regex": HEADER_REGEX,
            "header_passes": header_metadata["commands"],
            "linux_excluded_symbols": sorted(LINUX_EXCLUDED_SYMBOLS.keys()),
        },
        "config_snapshots": config_snapshots,
        "libraries": sorted(library_records, key=lambda item: item["soname"]),
        "platform_exclusions": {
            "linux": sorted(LINUX_EXCLUDED_SYMBOLS.keys()),
        },
    }

    manifest = {
        "schema_version": 1,
        "target_platform": platform,
        "collector_options": {
            "header_symbol_regex": HEADER_REGEX,
            "header_passes": header_metadata["commands"],
            "linux_excluded_symbols": sorted(LINUX_EXCLUDED_SYMBOLS.keys()),
            "version_script_inputs": sorted(
                repo_relative(Path(library["safe_map"])) for library in LIBRARIES
            )
            + sorted(repo_relative(Path(library["upstream_map"])) for library in LIBRARIES),
            "debian_symbol_inputs": sorted(
                repo_relative(Path(library["debian_symbols"])) for library in LIBRARIES
            ),
            "observed_export_inputs": sorted(
                repo_relative(Path(library["observed_dso"])) for library in LIBRARIES
            ),
        },
        "sources": [
            {
                "path": path,
                "sha256": sha256_digest(REPO_ROOT / path),
            }
            for path in sorted(consumed_source_paths)
        ],
    }

    platform_excluded_lines = sorted(LINUX_EXCLUDED_SYMBOLS.keys())
    return inventory, manifest, platform_excluded_lines


def validate_outputs(
    expected_inventory: str,
    expected_manifest: str,
    expected_exclusions: str,
    inventory_path: Path,
    inputs_path: Path,
    exclusions_path: Path,
) -> int:
    failures = []

    actual_inventory = inventory_path.read_text() if inventory_path.exists() else ""
    if actual_inventory != expected_inventory:
        failures.append(diff_text(display_path(inventory_path), expected_inventory, actual_inventory))

    actual_manifest = inputs_path.read_text() if inputs_path.exists() else ""
    if actual_manifest != expected_manifest:
        failures.append(diff_text(display_path(inputs_path), expected_manifest, actual_manifest))

    actual_exclusions = exclusions_path.read_text() if exclusions_path.exists() else ""
    if actual_exclusions != expected_exclusions:
        failures.append(diff_text(display_path(exclusions_path), expected_exclusions, actual_exclusions))

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "mode",
        nargs="?",
        choices=("generate", "validate", "check"),
        help="Generate outputs or validate the checked-in files without mutating them.",
    )
    parser.add_argument(
        "--validate",
        action="store_true",
        help="Check the checked-in inventory and manifest without mutating them.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Alias for --validate.",
    )
    parser.add_argument(
        "--inventory",
        "--inventory-path",
        dest="inventory_path",
        default=str(INVENTORY_PATH),
        help="Path to the public-surface inventory JSON.",
    )
    parser.add_argument(
        "--inputs",
        "--input-manifest",
        "--inputs-path",
        "--input-manifest-path",
        dest="inputs_path",
        default=str(INPUTS_PATH),
        help="Path to the public-surface input manifest JSON.",
    )
    parser.add_argument(
        "--platform-excluded",
        "--platform-excluded-linux",
        "--platform-excluded-path",
        dest="platform_excluded_path",
        default=str(LINUX_EXCLUDED_PATH),
        help="Path to the Linux platform exclusion text file.",
    )
    args = parser.parse_args()

    inventory, manifest, platform_excluded_lines = collect_inventory()

    inventory_text = json.dumps(inventory, indent=2, sort_keys=True) + "\n"
    manifest_text = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    excluded_text = "".join(f"{name}\n" for name in platform_excluded_lines)

    inventory_path = Path(args.inventory_path)
    inputs_path = Path(args.inputs_path)
    exclusions_path = Path(args.platform_excluded_path)

    validate_mode = args.validate or args.check or args.mode in {"validate", "check"}
    if validate_mode:
        return validate_outputs(
            inventory_text,
            manifest_text,
            excluded_text,
            inventory_path,
            inputs_path,
            exclusions_path,
        )

    write_if_changed(inventory_path, inventory_text)
    write_if_changed(inputs_path, manifest_text)
    write_if_changed(exclusions_path, excluded_text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
