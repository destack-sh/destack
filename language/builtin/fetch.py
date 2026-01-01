#!/usr/bin/env python3
import argparse
import os
import posixpath
import re
import urllib.request
from pathlib import Path

DEFAULT_TYPESCRIPT_VERSION = "5.9.3"
DEFAULT_NODE_TARGETS = [
    ("18", "18.19.130"),
    ("20", "20.19.27"),
    ("22", "22.19.3"),
    ("24", "24.10.4"),
]
DEFAULT_DENO_TARGETS = [("2.6", "2.6.3"), ("2.5", "2.5.6")]
DEFAULT_BUN_TARGETS = [("1.3", "1.3.5"), ("1.2", "1.2.23")]

REFERENCE_PATH_PATTERN = re.compile(
    r'^///\s*<reference\s+path=(?:"([^"]+)"|\'([^\']+)\')\s*/>\s*$',
    flags=re.M,
)

def resolve_reference_path(source_path: str, reference: str) -> str:
    """Resolve a reference path relative to the current source path."""
    # normalize path separators
    normalized_reference = reference.replace("\\", "/")
    if normalized_reference.startswith("/"):
        return normalized_reference.lstrip("/")

    # join with the source directory
    base_directory = posixpath.dirname(source_path)
    if base_directory:
        return posixpath.normpath(posixpath.join(base_directory, normalized_reference))

    return posixpath.normpath(normalized_reference)


def fetch_remote_text(url: str) -> str:
    """Fetch a url and normalize line endings."""
    # read remote content
    with urllib.request.urlopen(url) as response:
        data = response.read().decode("utf-8")

    return data.replace("\r\n", "\n")


def fetch_reference_tree(base_url: str, entry_path: str) -> str:
    """Fetch a file and its reference path dependencies."""
    # initialize traversal state
    seen_paths: set[str] = set()
    sections: list[str] = []

    def fetch(path: str) -> None:
        """Fetch a single file and its dependencies."""
        # skip already visited files
        if path in seen_paths:
            return

        # read the current file
        seen_paths.add(path)
        url = f"{base_url}/{path}"
        content = fetch_remote_text(url)

        # traverse referenced files
        reference_matches = REFERENCE_PATH_PATTERN.findall(content)
        for first_reference, second_reference in reference_matches:
            reference = first_reference or second_reference
            resolved_reference = resolve_reference_path(path, reference)
            fetch(resolved_reference)

        # remove reference directives and record content
        cleaned_content = REFERENCE_PATH_PATTERN.sub("", content)
        sections.append(cleaned_content.strip())

    fetch(entry_path)

    # join sections and verify references are resolved
    combined = "\n\n".join(section for section in sections if section)
    if REFERENCE_PATH_PATTERN.search(combined):
        raise RuntimeError("unresolved reference path in fetched types")

    return combined + "\n"


def write_text(destination_path: Path, content: str) -> None:
    """Write content to the destination path."""
    # ensure destination directory exists
    destination_path.parent.mkdir(parents=True, exist_ok=True)
    destination_path.write_text(content, encoding="utf-8")


def fetch_typescript_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript standard library sources."""
    # define the library fetch helper
    def fetch_typescript_library(source_name: str, destination_path: Path) -> None:
        """Fetch one TypeScript library file."""
        url = f"{base_url}/{source_name}"
        content = fetch_remote_text(url)
        write_text(destination_path, content)

    # fetch es5
    print("  - es5")
    fetch_typescript_library("lib.es5.d.ts", library_directory / "es" / "es5" / "index.d.ts")

    # enumerate grouped es libraries
    es_groups = {
        "es2015": [
            "core",
            "collection",
            "generator",
            "iterable",
            "promise",
            "proxy",
            "reflect",
            "symbol",
            "symbol.wellknown",
        ],
        "es2016": ["array.include", "intl", "full"],
        "es2017": ["arraybuffer", "date", "intl", "object", "sharedmemory", "string", "typedarrays", "full"],
        "es2018": ["asyncgenerator", "asynciterable", "intl", "promise", "regexp", "full"],
        "es2019": ["array", "intl", "object", "string", "symbol", "full"],
        "es2020": [
            "bigint",
            "date",
            "intl",
            "number",
            "promise",
            "sharedmemory",
            "string",
            "symbol.wellknown",
            "full",
        ],
        "es2021": ["intl", "promise", "string", "weakref", "full"],
        "es2022": ["array", "error", "intl", "object", "regexp", "string", "full"],
        "es2023": ["array", "collection", "intl", "full"],
        "es2024": [
            "arraybuffer",
            "collection",
            "object",
            "promise",
            "regexp",
            "sharedmemory",
            "string",
            "full",
        ],
        "esnext": [
            "array",
            "collection",
            "decorators",
            "disposable",
            "error",
            "float16",
            "intl",
            "iterator",
            "promise",
            "sharedmemory",
            "full",
        ],
    }

    # fetch grouped es libraries
    for group, files in es_groups.items():
        print(f"  - {group}")
        group_directory = library_directory / "es" / group
        fetch_typescript_library(f"lib.{group}.d.ts", group_directory / "index.d.ts")
        for file_name in files:
            fetch_typescript_library(f"lib.{group}.{file_name}.d.ts", group_directory / f"{file_name}.d.ts")

    # fetch decorators libraries
    print("  - decorators")
    fetch_typescript_library("lib.decorators.d.ts", library_directory / "es" / "decorators.d.ts")
    fetch_typescript_library("lib.decorators.legacy.d.ts", library_directory / "es" / "decorators.legacy.d.ts")

    # fetch dom libraries
    print("  - dom")
    fetch_typescript_library("lib.dom.d.ts", library_directory / "dom" / "index.d.ts")
    fetch_typescript_library("lib.dom.iterable.d.ts", library_directory / "dom" / "iterable.d.ts")
    fetch_typescript_library("lib.dom.asynciterable.d.ts", library_directory / "dom" / "asynciterable.d.ts")

    # fetch worker libraries
    print("  - worker")
    fetch_typescript_library("lib.webworker.d.ts", library_directory / "worker" / "index.d.ts")
    fetch_typescript_library("lib.webworker.iterable.d.ts", library_directory / "worker" / "iterable.d.ts")
    fetch_typescript_library("lib.webworker.asynciterable.d.ts", library_directory / "worker" / "asynciterable.d.ts")
    fetch_typescript_library("lib.webworker.importscripts.d.ts", library_directory / "worker" / "importscripts.d.ts")

    # fetch scripthost libraries
    print("  - scripthost")
    fetch_typescript_library("lib.scripthost.d.ts", library_directory / "scripthost" / "index.d.ts")


def parse_target_overrides(value: str, default: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Parse override targets from an environment variable."""
    # return defaults when not provided
    if not value:
        return default

    # split and validate override entries
    targets: list[tuple[str, str]] = []
    for item in value.split(","):
        stripped_item = item.strip()
        if not stripped_item:
            continue
        parts = stripped_item.split(":", 1)
        if len(parts) != 2 or not parts[0] or not parts[1]:
            raise ValueError(f"invalid target {item}")
        targets.append((parts[0], parts[1]))

    return targets


def format_targets(targets: list[tuple[str, str]]) -> str:
    """Format targets for display."""
    # render target pairs
    formatted = [f"{name}:{version}" for name, version in targets]
    return ",".join(formatted)


def fetch_node_libs(library_directory: Path) -> None:
    """Fetch the Node.js library definitions."""
    # resolve targets
    override_value = os.environ.get("NODE_TARGETS_OVERRIDE", "")
    targets = parse_target_overrides(override_value, DEFAULT_NODE_TARGETS)

    # fetch each target
    for library_version, types_version in targets:
        base_url = (
            os.environ.get("NODE_TYPES_BASE_URL")
            or os.environ.get("NODE_TYPES_URL")
            or f"https://unpkg.com/@types/node@{types_version}"
        )
        print(f"  - node.v{library_version} @types/node {types_version}")
        content = fetch_reference_tree(base_url, "index.d.ts")
        destination_path = library_directory / "node" / f"v{library_version}" / "index.d.ts"
        write_text(destination_path, content)


def fetch_deno_libs(library_directory: Path) -> None:
    """Fetch the Deno library definitions."""
    # resolve targets
    override_value = os.environ.get("DENO_TARGETS_OVERRIDE", "")
    targets = parse_target_overrides(override_value, DEFAULT_DENO_TARGETS)

    # fetch each target
    for library_version, deno_version in targets:
        base_url = (
            os.environ.get("DENO_TYPES_BASE_URL")
            or os.environ.get("DENO_TYPES_URL")
            or f"https://raw.githubusercontent.com/denoland/deno/v{deno_version}/cli/tsc/dts"
        )
        print(f"  - deno.v{library_version} {deno_version}")
        content = fetch_reference_tree(base_url, "lib.deno.ns.d.ts")
        destination_path = library_directory / "deno" / f"v{library_version}" / "index.d.ts"
        write_text(destination_path, content)


def fetch_bun_libs(library_directory: Path) -> None:
    """Fetch the Bun library definitions."""
    # resolve targets
    override_value = os.environ.get("BUN_TARGETS_OVERRIDE", "")
    targets = parse_target_overrides(override_value, DEFAULT_BUN_TARGETS)

    # fetch each target
    for library_version, bun_version in targets:
        url = os.environ.get(
            "BUN_TYPES_URL",
            f"https://raw.githubusercontent.com/oven-sh/bun/bun-v{bun_version}/packages/bun-types/bun.d.ts",
        )
        print(f"  - bun.v{library_version}")
        destination_path = library_directory / "bun" / f"v{library_version}" / "index.d.ts"
        write_text(destination_path, fetch_remote_text(url))


def parse_only_targets(value: str) -> set[str]:
    """Parse the only argument into target names."""
    # handle empty value
    if not value:
        return {"ts", "node", "deno", "bun"}

    # split target names
    targets = {name.strip() for name in value.split(",") if name.strip()}
    return targets


def print_versions(typescript_version: str) -> None:
    """Print the configured library versions."""
    # resolve node targets for display
    node_override = os.environ.get("NODE_TARGETS_OVERRIDE", "")
    node_targets = parse_target_overrides(node_override, DEFAULT_NODE_TARGETS)

    # resolve deno targets for display
    deno_override = os.environ.get("DENO_TARGETS_OVERRIDE", "")
    deno_targets = parse_target_overrides(deno_override, DEFAULT_DENO_TARGETS)

    # resolve bun targets for display
    bun_override = os.environ.get("BUN_TARGETS_OVERRIDE", "")
    bun_targets = parse_target_overrides(bun_override, DEFAULT_BUN_TARGETS)

    # emit version summary
    print(f"typescript lib version: {typescript_version}")
    print(f"node targets: {format_targets(node_targets)}")
    print(f"deno targets: {format_targets(deno_targets)}")
    print(f"bun targets: {format_targets(bun_targets)}")


def main() -> int:
    """Run the builtin library fetcher."""
    # parse command line arguments
    parser = argparse.ArgumentParser()
    parser.add_argument("--only", help="comma separated list: ts,node,deno,bun", default="")
    parser.add_argument("--print-versions", action="store_true")
    args = parser.parse_args()

    # resolve script directories
    script_directory = Path(__file__).resolve().parent
    library_directory = script_directory / "lib"

    # resolve typescript version and base url
    typescript_version = os.environ.get("TS_VERSION") or DEFAULT_TYPESCRIPT_VERSION
    typescript_base_url = os.environ.get("BASE_URL") or f"https://unpkg.com/typescript@{typescript_version}/lib"

    # print versions when requested
    if args.print_versions:
        print_versions(typescript_version)
        return 0

    # resolve target selection
    only_targets = parse_only_targets(args.only)

    # fetch selected libraries
    if "ts" in only_targets:
        fetch_typescript_libs(library_directory, typescript_base_url)
    if "node" in only_targets:
        fetch_node_libs(library_directory)
    if "deno" in only_targets:
        fetch_deno_libs(library_directory)
    if "bun" in only_targets:
        fetch_bun_libs(library_directory)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
