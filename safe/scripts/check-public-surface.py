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

DEFAULT_C_HEADERS = [
    ORIGINAL_ROOT / "libtiff" / "tiffio.h",
]

DEFAULT_CXX_HEADERS = [
    ORIGINAL_ROOT / "libtiff" / "tiffio.hxx",
]

DEFAULT_CONFIG_HEADERS = [
    ORIGINAL_ROOT / "build" / "libtiff" / "tif_config.h",
    ORIGINAL_ROOT / "build" / "libtiff" / "tiffconf.h",
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

DEFAULT_LIBRARY_CONFIGS = [
    {
        "name": "libtiff",
        "soname": "libtiff.so.6",
        "safe_map": SAFE_ROOT / "capi" / "libtiff-safe.map",
        "upstream_map": ORIGINAL_ROOT / "libtiff" / "libtiff.map",
        "debian_symbols": ORIGINAL_ROOT / "debian" / "libtiff6.symbols",
        "observed_dso": SAFE_ROOT / "build" / "libtiff" / "libtiff.so.6",
        "header_names": None,
    },
    {
        "name": "libtiffxx",
        "soname": "libtiffxx.so.6",
        "safe_map": SAFE_ROOT / "capi" / "libtiffxx-safe.map",
        "upstream_map": ORIGINAL_ROOT / "libtiff" / "libtiffxx.map",
        "debian_symbols": ORIGINAL_ROOT / "debian" / "libtiffxx6.symbols",
        "observed_dso": SAFE_ROOT / "build" / "libtiff" / "libtiffxx.so.6",
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


def make_header_targets(
    c_headers: List[Path],
    cxx_headers: List[Path],
    config_headers: List[Path],
) -> List[Dict[str, object]]:
    targets = []
    config_include_dirs = []
    for header in config_headers:
        parent = header.parent
        if parent not in config_include_dirs:
            config_include_dirs.append(parent)
    for source in c_headers:
        targets.append(
            {
                "language": "c",
                "compiler": CC,
                "source": source,
                "include_dirs": [source.parent, *config_include_dirs],
            }
        )
    for source in cxx_headers:
        targets.append(
            {
                "language": "c++",
                "compiler": CXX,
                "source": source,
                "include_dirs": [source.parent, *config_include_dirs],
            }
        )
    return targets


def default_source_config() -> Dict[str, object]:
    return {
        "c_headers": list(DEFAULT_C_HEADERS),
        "cxx_headers": list(DEFAULT_CXX_HEADERS),
        "config_headers": list(DEFAULT_CONFIG_HEADERS),
        "libraries": [dict(item) for item in DEFAULT_LIBRARY_CONFIGS],
    }


def library_key_for_path(path: Path) -> str:
    name = path.name
    if "tiffxx" in name:
        return "libtiffxx.so.6"
    return "libtiff.so.6"


def default_live_library_paths() -> Dict[str, Path]:
    return {
        "libtiff.so.6": SAFE_ROOT / "build" / "libtiff" / "libtiff.so.6",
        "libtiffxx.so.6": SAFE_ROOT / "build" / "libtiff" / "libtiffxx.so.6",
    }


def default_original_library_paths() -> Dict[str, Path]:
    return {
        "libtiff.so.6": ORIGINAL_ROOT / "build" / "libtiff" / "libtiff.so.6.0.1",
        "libtiffxx.so.6": ORIGINAL_ROOT / "build" / "libtiff" / "libtiffxx.so.6.0.1",
    }


def apply_source_overrides(args: argparse.Namespace) -> Dict[str, object]:
    config = default_source_config()
    by_soname = {item["soname"]: item for item in config["libraries"]}

    if args.c_headers:
        config["c_headers"] = [Path(path) for path in args.c_headers]
    if args.cxx_headers:
        config["cxx_headers"] = [Path(path) for path in args.cxx_headers]
    if args.config_headers:
        config["config_headers"] = [Path(path) for path in args.config_headers]

    for map_path_str in args.maps:
        map_path = Path(map_path_str)
        library = by_soname[library_key_for_path(map_path)]
        field = "safe_map" if map_path.name.endswith("-safe.map") else "upstream_map"
        library[field] = map_path

    for symbols_path_str in args.debian_symbols:
        symbols_path = Path(symbols_path_str)
        library = by_soname[library_key_for_path(symbols_path)]
        library["debian_symbols"] = symbols_path

    for library_path_str in args.libraries:
        library_path = Path(library_path_str)
        library = by_soname[library_key_for_path(library_path)]
        library["observed_dso"] = library_path

    return config


def apply_live_library_overrides(
    args: argparse.Namespace,
) -> Tuple[Dict[str, Path], Dict[str, Path]]:
    live_libraries = default_live_library_paths()
    original_libraries = default_original_library_paths()

    for library_path_str in args.libraries:
        library_path = Path(library_path_str)
        live_libraries[library_key_for_path(library_path)] = library_path

    for library_path_str in args.original_libraries:
        library_path = Path(library_path_str)
        original_libraries[library_key_for_path(library_path)] = library_path

    return live_libraries, original_libraries


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
    include_dirs = [Path(path) for path in target["include_dirs"]]
    include_args = []
    for include_dir in include_dirs:
        include_args.extend(["-I", str(include_dir)])
    deps_args = [compiler, "-M", *include_args, "-x", str(target["language"]), str(source)]
    preprocess_args = [
        compiler,
        "-E",
        "-dD",
        "-P",
        *include_args,
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
            dep_path = dep.resolve()
        else:
            dep_path = (REPO_ROOT / dep).resolve()
        if dep_path.is_file() and REPO_ROOT in dep_path.parents:
            repo_deps.append(dep_path)
    unique_deps = sorted({dep.resolve() for dep in repo_deps})
    return preprocessed, unique_deps, preprocess_args


def parse_header_symbols(public_header_targets: List[Dict[str, object]]) -> Tuple[Dict[str, Dict[str, object]], Dict[str, object]]:
    pattern = re.compile(HEADER_REGEX, re.S)
    symbols: Dict[str, Dict[str, object]] = {}
    commands = []
    consumed_paths = []
    for target in public_header_targets:
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


def collect_inventory(source_config: Dict[str, object]) -> Tuple[Dict[str, object], Dict[str, object], List[str]]:
    header_targets = make_header_targets(
        source_config["c_headers"],
        source_config["cxx_headers"],
        source_config["config_headers"],
    )
    header_symbols, header_metadata = parse_header_symbols(header_targets)

    config_snapshots = {
        repo_relative(path): parse_config_header(path)
        for path in source_config["config_headers"]
    }

    library_records = []
    consumed_source_paths = set(header_metadata["consumed_paths"])
    consumed_source_paths.update(repo_relative(path) for path in source_config["config_headers"])

    for library in source_config["libraries"]:
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
                repo_relative(Path(library["safe_map"])) for library in source_config["libraries"]
            )
            + sorted(repo_relative(Path(library["upstream_map"])) for library in source_config["libraries"]),
            "debian_symbol_inputs": sorted(
                repo_relative(Path(library["debian_symbols"])) for library in source_config["libraries"]
            ),
            "observed_export_inputs": sorted(
                repo_relative(Path(library["observed_dso"])) for library in source_config["libraries"]
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


def load_json(path: Path) -> Dict[str, object]:
    return json.loads(path.read_text())


def parse_symbol_version_assertion(raw_value: str) -> Tuple[str, str]:
    for separator in ("=", ":", "@"):
        if separator in raw_value:
            symbol, version = raw_value.split(separator, 1)
            return symbol, version
    raise ValueError(f"expected SYMBOL=VERSION for --must-record-version, got: {raw_value}")


def inventory_matches_symbol(symbol: Dict[str, object], requested_name: str) -> bool:
    return symbol["name"] == requested_name or symbol.get("base_name") == requested_name


def export_matches_symbol(export_entry: Dict[str, object], requested_name: str) -> bool:
    return requested_name in {
        export_entry["link_name"],
        export_entry["demangled_name"],
        export_entry["base_name"],
    }


def run_assertions(
    inventory_data: Dict[str, object],
    linux_exclusions_path: Path,
    must_contain: List[str],
    must_record_version: List[str],
    must_record_linux_exclusion: List[str],
) -> int:
    failures = []
    symbols = [
        symbol
        for library in inventory_data.get("libraries", [])
        for symbol in library.get("symbols", [])
    ]
    linux_exclusions = set()
    if linux_exclusions_path.exists():
        linux_exclusions = {
            line.strip()
            for line in linux_exclusions_path.read_text().splitlines()
            if line.strip()
        }

    for requested_name in must_contain:
        if not any(inventory_matches_symbol(symbol, requested_name) for symbol in symbols):
            failures.append(f"missing required inventory entry: {requested_name}")

    for raw_assertion in must_record_version:
        requested_name, requested_version = parse_symbol_version_assertion(raw_assertion)
        if not any(
            inventory_matches_symbol(symbol, requested_name)
            and symbol.get("required_version_node") == requested_version
            for symbol in symbols
        ):
            failures.append(
                f"inventory does not record {requested_name} with version node {requested_version}"
            )

    for requested_name in must_record_linux_exclusion:
        if not any(
            inventory_matches_symbol(symbol, requested_name)
            and symbol.get("linux_excluded")
            for symbol in symbols
        ) or requested_name not in linux_exclusions:
            failures.append(
                f"inventory does not record Linux exclusion for {requested_name}"
            )

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1
    return 0


def summarize_symbol_pairs(pairs: Iterable[Tuple[str, str]]) -> str:
    return ", ".join(f"{name}@{version}" for name, version in sorted(pairs))


def run_live_export_assertions(
    live_library_paths: Dict[str, Path],
    original_library_paths: Dict[str, Path],
    must_not_export: List[str],
    check_versioned_symbols: bool,
) -> int:
    failures = []
    live_exports_by_library: Dict[str, Dict[str, object]] = {}

    required_libraries = set()
    if must_not_export:
        required_libraries.update(live_library_paths.keys())
    if check_versioned_symbols:
        required_libraries.update(live_library_paths.keys())

    for soname in sorted(required_libraries):
        live_path = live_library_paths.get(soname)
        if not live_path or not live_path.exists():
            failures.append(
                f"missing live library for export assertions: {display_path(live_path or Path(soname))}"
            )
            continue
        live_exports_by_library[soname] = parse_observed_exports(live_path)

    for requested_name in must_not_export:
        for soname, observed in live_exports_by_library.items():
            for export_entry in observed["exports"].values():
                if export_matches_symbol(export_entry, requested_name):
                    failures.append(
                        f"{requested_name} unexpectedly exported from {soname} via {display_path(live_library_paths[soname])}"
                    )
                    break

    if check_versioned_symbols:
        for soname in sorted(live_library_paths.keys()):
            live_path = live_library_paths[soname]
            original_path = original_library_paths.get(soname)
            if soname not in live_exports_by_library:
                continue
            if not original_path or not original_path.exists():
                failures.append(
                    f"missing original library for versioned symbol comparison: {display_path(original_path or Path(soname))}"
                )
                continue

            live_observed = live_exports_by_library[soname]
            original_observed = parse_observed_exports(original_path)
            live_pairs = {
                (name, entry["version"])
                for name, entry in live_observed["exports"].items()
                if entry["version"]
            }
            original_pairs = {
                (name, entry["version"])
                for name, entry in original_observed["exports"].items()
                if entry["version"]
            }
            missing_pairs = original_pairs - live_pairs
            extra_pairs = live_pairs - original_pairs
            if missing_pairs:
                failures.append(
                    f"{soname} is missing versioned exports present in {display_path(original_path)}: {summarize_symbol_pairs(missing_pairs)}"
                )
            if soname == "libtiffxx.so.6" and extra_pairs:
                failures.append(
                    f"{soname} exports extra versioned symbols not present in {display_path(original_path)}: {summarize_symbol_pairs(extra_pairs)}"
                )

            live_version_nodes = sorted(set(live_observed["version_nodes"]))
            original_version_nodes = sorted(set(original_observed["version_nodes"]))
            if live_version_nodes != original_version_nodes:
                failures.append(
                    f"{soname} version nodes differ from {display_path(original_path)}: "
                    f"live={','.join(live_version_nodes)} original={','.join(original_version_nodes)}"
                )

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1
    return 0


def flatten_cli_values(raw_values: List[object]) -> List[str]:
    flattened: List[str] = []
    for value in raw_values:
        if isinstance(value, list):
            flattened.extend(str(item) for item in value)
        else:
            flattened.append(str(value))
    return flattened


def platform_matches(requested_platform: str, platform: Dict[str, str]) -> bool:
    requested = requested_platform.strip().lower()
    if not requested:
        return True

    triple = str(platform.get("triple", "")).lower()
    system = str(platform.get("system", "")).lower()
    distribution = str(platform.get("distribution_baseline", "")).lower()

    if requested in {triple, system, distribution}:
        return True
    if requested == "linux" and "linux" in triple:
        return True
    if requested in set(filter(None, re.split(r"[-_]", triple))):
        return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
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
        "--validate-existing-inventory",
        action="store_true",
        help="Verifier alias for non-mutating validation mode.",
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
        "--linux-exclusions",
        "--platform-excluded-path",
        dest="platform_excluded_path",
        default=str(LINUX_EXCLUDED_PATH),
        help="Path to the Linux platform exclusion text file.",
    )
    parser.add_argument(
        "--platform",
        dest="platform",
        default=None,
        help="Verifier compatibility flag for the target platform tuple.",
    )
    parser.add_argument(
        "--c-header",
        dest="c_headers",
        action="append",
        default=[],
        help="Override the C public headers used for collection.",
    )
    parser.add_argument(
        "--cxx-header",
        "--cpp-header",
        dest="cxx_headers",
        action="append",
        default=[],
        help="Override the C++ public headers used for collection.",
    )
    parser.add_argument(
        "--config-header",
        "--tif-config",
        "--tiffconf",
        dest="config_headers",
        action="append",
        default=[],
        help="Override generated config headers used for collection.",
    )
    parser.add_argument(
        "--map",
        dest="maps",
        action="append",
        default=[],
        help="Override a safe or upstream version-script input.",
    )
    parser.add_argument(
        "--xx-map",
        dest="maps",
        action="append",
        help="Override the libtiffxx version-script input.",
    )
    parser.add_argument(
        "--library",
        dest="libraries",
        action="append",
        default=[],
        help="Override an observed shared-library input.",
    )
    parser.add_argument(
        "--library-xx",
        dest="libraries",
        action="append",
        help="Override the observed libtiffxx shared-library input.",
    )
    parser.add_argument(
        "--original-library",
        dest="original_libraries",
        action="append",
        default=[],
        help="Verifier compatibility flag for the upstream/reference libtiff shared library.",
    )
    parser.add_argument(
        "--original-library-xx",
        dest="original_libraries",
        action="append",
        help="Verifier compatibility flag for the upstream/reference libtiffxx shared library.",
    )
    parser.add_argument(
        "--debian-symbols",
        "--symbols",
        "--xx-symbols",
        dest="debian_symbols",
        action="append",
        default=[],
        help="Override a Debian symbols input file.",
    )
    parser.add_argument(
        "--must-contain",
        action="append",
        nargs="+",
        default=[],
        help="Assert that the inventory contains the specified symbol names or base names.",
    )
    parser.add_argument(
        "--must-export",
        action="append",
        nargs="+",
        default=[],
        help="Verifier compatibility alias for --must-contain.",
    )
    parser.add_argument(
        "--must-not-export",
        action="append",
        nargs="+",
        default=[],
        help="Assert that the current safe shared libraries do not export the specified symbol names.",
    )
    parser.add_argument(
        "--must-record-version",
        action="append",
        nargs="+",
        default=[],
        help="Assert that the listed SYMBOL=VERSION assertions are recorded in the inventory.",
    )
    parser.add_argument(
        "--must-export-version",
        action="append",
        nargs="+",
        default=[],
        help="Verifier compatibility alias for --must-record-version.",
    )
    parser.add_argument(
        "--must-record-linux-exclusion",
        action="append",
        nargs="+",
        default=[],
        help="Assert that the inventory and Linux exclusions file record the listed symbols.",
    )
    parser.add_argument(
        "--must-export-linux-exclusion",
        action="append",
        nargs="+",
        default=[],
        help="Verifier compatibility alias for --must-record-linux-exclusion.",
    )
    parser.add_argument(
        "--check-versioned-symbols",
        action="store_true",
        help="Compare the safe versioned dynamic symbol sets against the original DSOs.",
    )
    args = parser.parse_args()

    must_contain = flatten_cli_values(args.must_contain) + flatten_cli_values(
        args.must_export
    )
    must_record_version = flatten_cli_values(
        args.must_record_version
    ) + flatten_cli_values(args.must_export_version)
    must_record_linux_exclusion = flatten_cli_values(
        args.must_record_linux_exclusion
    ) + flatten_cli_values(args.must_export_linux_exclusion)
    must_not_export = flatten_cli_values(args.must_not_export)

    source_config = apply_source_overrides(args)
    live_library_paths, original_library_paths = apply_live_library_overrides(args)
    inventory, manifest, platform_excluded_lines = collect_inventory(source_config)

    inventory_text = json.dumps(inventory, indent=2, sort_keys=True) + "\n"
    manifest_text = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    excluded_text = "".join(f"{name}\n" for name in platform_excluded_lines)

    if args.platform and not platform_matches(args.platform, inventory["platform"]):
        print(
            f"requested platform {args.platform!r} does not match collected platform "
            f"{inventory['platform']['triple']!r}",
            file=sys.stderr,
        )
        return 1

    inventory_path = Path(args.inventory_path)
    inputs_path = Path(args.inputs_path)
    exclusions_path = Path(args.platform_excluded_path)

    explicit_generate = args.mode == "generate"
    validate_mode = args.validate or args.check or args.mode in {"validate", "check"}
    validate_mode = validate_mode or args.validate_existing_inventory
    assertion_mode = bool(
        must_contain
        or must_record_version
        or must_record_linux_exclusion
        or must_not_export
        or args.check_versioned_symbols
    )
    if assertion_mode and not explicit_generate:
        validate_mode = True
    if validate_mode:
        result = validate_outputs(
            inventory_text,
            manifest_text,
            excluded_text,
            inventory_path,
            inputs_path,
            exclusions_path,
        )
        if result != 0:
            return result
        inventory_data = load_json(inventory_path)
        result = run_assertions(
            inventory_data,
            exclusions_path,
            must_contain,
            must_record_version,
            must_record_linux_exclusion,
        )
        if result != 0:
            return result
        return run_live_export_assertions(
            live_library_paths,
            original_library_paths,
            must_not_export,
            args.check_versioned_symbols,
        )

    write_if_changed(inventory_path, inventory_text)
    write_if_changed(inputs_path, manifest_text)
    write_if_changed(exclusions_path, excluded_text)
    result = run_assertions(
        inventory,
        exclusions_path,
        must_contain,
        must_record_version,
        must_record_linux_exclusion,
    )
    if result != 0:
        return result
    return run_live_export_assertions(
        live_library_paths,
        original_library_paths,
        must_not_export,
        args.check_versioned_symbols,
    )


if __name__ == "__main__":
    sys.exit(main())
