#!/usr/bin/env python3
import argparse
import json
import os
import posixpath
import re
import urllib.request
from pathlib import Path

# default target versions
DEFAULT_TYPESCRIPT_VERSION = "5.9.3"
DEFAULT_UNDICI_TYPES_TARGETS = [
    ("v5", "5.26.4"),
    ("v6", "6.21.0"),
    ("v7", "7.16.0"),
]
DEFAULT_NODE_TARGETS = [
    ("18", "18.19.130"),
    ("20", "20.19.27"),
    ("22", "22.19.3"),
    ("24", "24.10.4"),
]
DEFAULT_DENO_TARGETS = [("2.6", "2.6.3"), ("2.5", "2.5.6")]
DEFAULT_BUN_TARGETS = [("1.3", "1.3.5"), ("1.2", "1.2.23")]

# reference path parsing
REFERENCE_PATH_PATTERN = re.compile(
    r'^///\s*<reference\s+path=(?:"([^"]+)"|\'([^\']+)\')\s*/>\s*$',
    flags=re.M,
)

# reference lib overrides
TYPESCRIPT_REFERENCE_OVERRIDES = {
    # typescript dom and webworker need injected reference libs
    "lib.dom.d.ts": ["es2015", "es2018.asynciterable", "es2020"],
    "lib.webworker.d.ts": ["es2015", "es2018.asynciterable", "es2020"],
}

# deno file mapping
DENO_LIB_FILES = [
    ("lib.deno.ns.d.ts", ["deno", "deno.ns"]),
    ("lib.deno.shared_globals.d.ts", ["deno.shared_globals"]),
    ("lib.deno.unstable.d.ts", ["deno.unstable"]),
    ("lib.deno.window.d.ts", ["deno.window"]),
    ("lib.deno.worker.d.ts", ["deno.worker"]),
    ("lib.deno_broadcast_channel.d.ts", ["deno.broadcast_channel"]),
    ("lib.deno_cache.d.ts", ["deno.cache"]),
    ("lib.deno_canvas.d.ts", ["deno.canvas"]),
    ("lib.deno_console.d.ts", ["deno.console"]),
    ("lib.deno_crypto.d.ts", ["deno.crypto"]),
    ("lib.deno_fetch.d.ts", ["deno.fetch"]),
    ("lib.deno_net.d.ts", ["deno.net"]),
    ("lib.deno_url.d.ts", ["deno.url"]),
    ("lib.deno_web.d.ts", ["deno.web"]),
    ("lib.deno_webgpu.d.ts", ["deno.webgpu"]),
    ("lib.deno_websocket.d.ts", ["deno.websocket"]),
    ("lib.deno_webstorage.d.ts", ["deno.webstorage"]),
]


def _resolve_reference_path(source_path: str, reference: str) -> str:
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


def _fetch_remote_text(url: str) -> str:
    """Fetch a URL and normalize line endings."""
    # read remote content
    with urllib.request.urlopen(url) as response:
        data = response.read().decode("utf-8")

    return data.replace("\r\n", "\n")


def _fetch_remote_json(url: str) -> object:
    """Fetch a URL and parse JSON content."""
    # ensure github api requests include a user agent
    request = urllib.request.Request(url, headers={"User-Agent": "destack-builtin-fetch"})
    with urllib.request.urlopen(request) as response:
        return json.load(response)


def _fetch_reference_tree(base_url: str, entry_path: str) -> str:
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
        content = _fetch_remote_text(url)

        # traverse referenced files
        reference_matches = REFERENCE_PATH_PATTERN.findall(content)
        for first_reference, second_reference in reference_matches:
            reference = first_reference or second_reference
            resolved_reference = _resolve_reference_path(path, reference)
            fetch(resolved_reference)

        # remove reference directives and record content
        cleaned_content = REFERENCE_PATH_PATTERN.sub("", content)
        sections.append(cleaned_content.strip())

    # fetch the entry file
    fetch(entry_path)

    # join sections and verify references are resolved
    combined = "\n\n".join(section for section in sections if section)
    if REFERENCE_PATH_PATTERN.search(combined):
        raise RuntimeError("unresolved reference path in fetched types")

    return combined + "\n"


def _inject_reference_libs(content: str, libs: list[str]) -> str:
    """Inject reference lib directives into content."""
    # return early when no extra libs are needed
    if not libs:
        return content

    # scan for existing references
    existing = set(
        match.group(1)
        for match in re.finditer(r'^\s*///\s*<reference\s+lib="([^"]+)"\s*/>\s*$', content, re.M)
    )
    missing = [lib for lib in libs if lib not in existing]
    if not missing:
        return content

    # build inserted lines
    insert_lines = [f'/// <reference lib="{lib}" />' for lib in missing]

    # insert after the header references
    lines = content.splitlines()
    insert_after = None
    seen_reference = False
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("/// <reference"):
            insert_after = index
            seen_reference = True
            continue
        if seen_reference:
            if stripped == "":
                continue
            break

    if insert_after is None:
        return "\n".join(insert_lines + lines) + ("\n" if content.endswith("\n") else "")

    updated_lines = lines[: insert_after + 1] + insert_lines + lines[insert_after + 1 :]
    return "\n".join(updated_lines) + ("\n" if content.endswith("\n") else "")


def _fetch_git_tree_paths(tree_url: str, prefix: str, suffix: str) -> list[str]:
    """Fetch file paths from a git tree listing."""
    # load tree listing
    payload = _fetch_remote_json(tree_url)
    tree = payload.get("tree", [])

    # collect matching paths
    paths: list[str] = []
    for entry in tree:
        # skip non file entries
        if entry.get("type") != "blob":
            continue

        # skip entries without a path
        path = entry.get("path")
        if not path:
            continue

        # skip entries outside the prefix
        if not path.startswith(prefix):
            continue

        # skip entries without the suffix
        if not path.endswith(suffix):
            continue

        # record matching entries
        relative_path = path[len(prefix) :]
        if relative_path:
            paths.append(relative_path)

    # ensure a stable order
    paths.sort()
    if not paths:
        raise RuntimeError(f"git tree listing returned no {suffix} files")

    return paths


def _write_text(destination_path: Path, content: str) -> None:
    """Write content to the destination path."""
    # ensure destination directory exists
    destination_path.parent.mkdir(parents=True, exist_ok=True)

    # write content to disk
    destination_path.write_text(content, encoding="utf-8")


def _parse_target_overrides(value: str, default: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Parse override targets from an environment variable."""
    # return defaults when not provided
    if not value:
        return default

    # split and validate override entries
    targets: list[tuple[str, str]] = []
    for item in value.split(","):
        # skip empty entries
        stripped_item = item.strip()
        if not stripped_item:
            continue

        # split name and version
        parts = stripped_item.split(":", 1)
        if len(parts) != 2 or not parts[0] or not parts[1]:
            raise ValueError(f"invalid target {item}")

        # record valid entries
        targets.append((parts[0], parts[1]))

    return targets


def _format_targets(targets: list[tuple[str, str]]) -> str:
    """Format targets for display."""
    # render target pairs
    formatted = [f"{name}:{version}" for name, version in targets]

    return ",".join(formatted)


def _parse_only_targets(value: str) -> set[str]:
    """Parse the only argument into target names."""
    # handle empty value
    if not value:
        return {"ts", "undici-types", "node", "deno", "bun"}

    # split target names
    targets = {name.strip() for name in value.split(",") if name.strip()}

    return targets


def _write_lib_sources(
    library_directory: Path,
    targets: list[tuple[str, str]],
    sources: dict[str, list[str]],
    output_name: str,
    const_prefix: str,
    path_template: str,
) -> None:
    """Write a builtin lib sources list into a rust definition file."""
    # locate the rust lib directory
    source_directory = library_directory.parent / "src" / "libs" / "lib"
    destination_path = source_directory / output_name

    # build the source list file
    lines: list[str] = []
    for library_version, _bun_version in targets:
        # build the version header
        version_id = library_version.replace(".", "_")
        const_name = f"{const_prefix}_V{version_id}_SOURCES"
        base_path = path_template.format(version=library_version)
        lines.append(f"pub const {const_name}: &[BuiltinLibrarySource] = &[")

        # record source entries for the version
        for relative_path in sources.get(library_version, []):
            # resolve the source path pieces
            name = posixpath.basename(relative_path)
            directory = posixpath.dirname(relative_path)
            if directory and directory != ".":
                lib_path = f"{base_path}/{directory}"
            else:
                lib_path = base_path

            # emit the source entry
            lines.append("    BuiltinLibrarySource::new(")
            lines.append('        "library",')
            lines.append(f'        "{lib_path}",')
            lines.append(f'        "{name}",')
            lines.append(
                f'        include_str!(concat!("../../../library/{lib_path}/{name}")),'
            )
            lines.append("    ),")
        lines.append("];")
        lines.append("")

    # write the generated file
    content = "\n".join(lines).rstrip() + "\n"
    _write_text(destination_path, content)


def _fetch_typescript_library(base_url: str, source_name: str, destination_path: Path) -> None:
    """Fetch one TypeScript library file."""
    # build the source url
    url = f"{base_url}/{source_name}"

    # fetch and write the source
    content = _fetch_remote_text(url)
    overrides = TYPESCRIPT_REFERENCE_OVERRIDES.get(source_name, [])
    content = _inject_reference_libs(content, overrides)
    _write_text(destination_path, content)


def _fetch_typescript_es_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript ES library sources."""
    # fetch es5
    print("  - es5")
    _fetch_typescript_library(
        base_url,
        "lib.es5.d.ts",
        library_directory / "es" / "es5" / "index.d.ts",
    )

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
        "es2017": [
            "arraybuffer",
            "date",
            "intl",
            "object",
            "sharedmemory",
            "string",
            "typedarrays",
            "full",
        ],
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
        # emit the group label
        print(f"  - {group}")

        # fetch the group index
        group_directory = library_directory / "es" / group
        _fetch_typescript_library(
            base_url,
            f"lib.{group}.d.ts",
            group_directory / "index.d.ts",
        )

        # fetch the group files
        for file_name in files:
            _fetch_typescript_library(
                base_url,
                f"lib.{group}.{file_name}.d.ts",
                group_directory / f"{file_name}.d.ts",
            )


def _fetch_typescript_decorators_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript decorators libraries."""
    # emit the decorators label
    print("  - decorators")

    # fetch decorator libraries
    _fetch_typescript_library(
        base_url,
        "lib.decorators.d.ts",
        library_directory / "es" / "decorators.d.ts",
    )
    _fetch_typescript_library(
        base_url,
        "lib.decorators.legacy.d.ts",
        library_directory / "es" / "decorators.legacy.d.ts",
    )


def _fetch_typescript_dom_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript DOM libraries."""
    # emit the dom label
    print("  - dom")

    # fetch dom libraries
    _fetch_typescript_library(
        base_url,
        "lib.dom.d.ts",
        library_directory / "dom" / "index.d.ts",
    )
    _fetch_typescript_library(
        base_url,
        "lib.dom.iterable.d.ts",
        library_directory / "dom" / "iterable.d.ts",
    )
    _fetch_typescript_library(
        base_url,
        "lib.dom.asynciterable.d.ts",
        library_directory / "dom" / "asynciterable.d.ts",
    )


def _fetch_typescript_worker_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript web worker libraries."""
    # emit the worker label
    print("  - worker")

    # fetch worker libraries
    _fetch_typescript_library(
        base_url,
        "lib.webworker.d.ts",
        library_directory / "worker" / "index.d.ts",
    )
    _fetch_typescript_library(
        base_url,
        "lib.webworker.iterable.d.ts",
        library_directory / "worker" / "iterable.d.ts",
    )
    _fetch_typescript_library(
        base_url,
        "lib.webworker.asynciterable.d.ts",
        library_directory / "worker" / "asynciterable.d.ts",
    )
    _fetch_typescript_library(
        base_url,
        "lib.webworker.importscripts.d.ts",
        library_directory / "worker" / "importscripts.d.ts",
    )


def _fetch_typescript_scripthost_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript scripthost libraries."""
    # emit the scripthost label
    print("  - scripthost")

    # fetch scripthost libraries
    _fetch_typescript_library(
        base_url,
        "lib.scripthost.d.ts",
        library_directory / "scripthost" / "index.d.ts",
    )


def fetch_typescript_libs(library_directory: Path, base_url: str) -> None:
    """Fetch the TypeScript standard library sources."""
    # fetch standard library groups
    _fetch_typescript_es_libs(library_directory, base_url)
    _fetch_typescript_decorators_libs(library_directory, base_url)
    _fetch_typescript_dom_libs(library_directory, base_url)
    _fetch_typescript_worker_libs(library_directory, base_url)
    _fetch_typescript_scripthost_libs(library_directory, base_url)


def fetch_undici_types(library_directory: Path) -> None:
    """Fetch the undici-types package definitions."""
    # resolve targets
    override_value = os.environ.get("UNDICI_TYPES_TARGETS_OVERRIDE", "")
    targets = _parse_target_overrides(override_value, DEFAULT_UNDICI_TYPES_TARGETS)

    # fetch each target
    for target_name, version in targets:
        # resolve the base url
        base_url = (
            os.environ.get("UNDICI_TYPES_BASE_URL")
            or os.environ.get("UNDICI_TYPES_URL")
            or f"https://unpkg.com/undici-types@{version}"
        ).rstrip("/")
        meta_url = f"{base_url}/?meta"

        # fetch package metadata
        print(f"  - undici-types.{target_name} {version}")
        meta = _fetch_remote_json(meta_url)

        # write definition files
        for entry in meta.get("files", []):
            # skip non definition entries
            path = entry.get("path")
            if not path or not path.endswith(".d.ts"):
                continue

            # write the definition file
            relative_path = path.lstrip("/")
            content = _fetch_remote_text(f"{base_url}/{relative_path}")
            destination_path = (
                library_directory / "undici-types" / target_name / relative_path
            )
            _write_text(destination_path, content)


def fetch_node_libs(library_directory: Path) -> None:
    """Fetch the Node.js library definitions."""
    # resolve targets
    override_value = os.environ.get("NODE_TARGETS_OVERRIDE", "")
    targets = _parse_target_overrides(override_value, DEFAULT_NODE_TARGETS)

    # fetch each target
    for library_version, types_version in targets:
        # resolve the base url
        base_url = (
            os.environ.get("NODE_TYPES_BASE_URL")
            or os.environ.get("NODE_TYPES_URL")
            or f"https://unpkg.com/@types/node@{types_version}"
        )

        # fetch node definitions
        print(f"  - node.v{library_version} @types/node {types_version}")
        content = _fetch_reference_tree(base_url, "index.d.ts")
        destination_path = library_directory / "node" / f"v{library_version}" / "index.d.ts"
        _write_text(destination_path, content)


def fetch_deno_libs(library_directory: Path) -> None:
    """Fetch the Deno library definitions."""
    # resolve targets
    override_value = os.environ.get("DENO_TARGETS_OVERRIDE", "")
    targets = _parse_target_overrides(override_value, DEFAULT_DENO_TARGETS)

    # fetch each target
    for library_version, deno_version in targets:
        # resolve the base url
        base_url = (
            os.environ.get("DENO_TYPES_BASE_URL")
            or os.environ.get("DENO_TYPES_URL")
            or f"https://raw.githubusercontent.com/denoland/deno/v{deno_version}/cli/tsc/dts"
        )

        # fetch deno definitions
        print(f"  - deno.v{library_version} {deno_version}")
        for source_path, local_names in DENO_LIB_FILES:
            content = _fetch_reference_tree(base_url, source_path)
            for local_name in local_names:
                destination_path = (
                    library_directory / local_name / f"v{library_version}" / "index.d.ts"
                )
                _write_text(destination_path, content)


def fetch_bun_libs(library_directory: Path) -> None:
    """Fetch the Bun library definitions."""
    # resolve targets
    override_value = os.environ.get("BUN_TARGETS_OVERRIDE", "")
    targets = _parse_target_overrides(override_value, DEFAULT_BUN_TARGETS)

    # fetch each target
    for library_version, bun_version in targets:
        # resolve the base url
        base_url = (
            os.environ.get("BUN_TYPES_BASE_URL")
            or os.environ.get("BUN_TYPES_URL")
            or f"https://raw.githubusercontent.com/oven-sh/bun/bun-v{bun_version}/packages/bun-types"
        )

        # fetch bun definitions with reference paths
        print(f"  - bun.v{library_version} {bun_version}")
        content = _fetch_reference_tree(base_url, "index.d.ts")
        destination_path = library_directory / "bun" / f"v{library_version}" / "index.d.ts"
        _write_text(destination_path, content)

        # fetch vendor files for expect type
        vendor_files = [
            "vendor/expect-type/index.d.ts",
            "vendor/expect-type/branding.d.ts",
            "vendor/expect-type/messages.d.ts",
            "vendor/expect-type/overloads.d.ts",
            "vendor/expect-type/utils.d.ts",
        ]
        for vendor_file in vendor_files:
            vendor_content = _fetch_remote_text(f"{base_url}/{vendor_file}")
            vendor_destination = library_directory / "bun" / f"v{library_version}" / vendor_file
            _write_text(vendor_destination, vendor_content)


def _print_versions(typescript_version: str) -> None:
    """Print the configured library versions."""
    # resolve undici targets for display
    undici_override = os.environ.get("UNDICI_TYPES_TARGETS_OVERRIDE", "")
    undici_targets = _parse_target_overrides(undici_override, DEFAULT_UNDICI_TYPES_TARGETS)

    # resolve node targets for display
    node_override = os.environ.get("NODE_TARGETS_OVERRIDE", "")
    node_targets = _parse_target_overrides(node_override, DEFAULT_NODE_TARGETS)

    # resolve deno targets for display
    deno_override = os.environ.get("DENO_TARGETS_OVERRIDE", "")
    deno_targets = _parse_target_overrides(deno_override, DEFAULT_DENO_TARGETS)

    # resolve bun targets for display
    bun_override = os.environ.get("BUN_TARGETS_OVERRIDE", "")
    bun_targets = _parse_target_overrides(bun_override, DEFAULT_BUN_TARGETS)

    # emit version summary
    print(f"typescript lib version: {typescript_version}")
    print(f"undici-types targets: {_format_targets(undici_targets)}")
    print(f"node targets: {_format_targets(node_targets)}")
    print(f"deno targets: {_format_targets(deno_targets)}")
    print(f"bun targets: {_format_targets(bun_targets)}")


def main() -> int:
    """Run the builtin library fetcher."""
    # parse command line arguments
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--only",
        help="comma separated list: ts,undici-types,node,deno,bun",
        default="",
    )
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
        _print_versions(typescript_version)
        return 0

    # resolve target selection
    only_targets = _parse_only_targets(args.only)

    # fetch selected libraries
    if "ts" in only_targets:
        fetch_typescript_libs(library_directory, typescript_base_url)
    if "undici-types" in only_targets:
        fetch_undici_types(library_directory)
    if "node" in only_targets:
        fetch_node_libs(library_directory)
    if "deno" in only_targets:
        fetch_deno_libs(library_directory)
    if "bun" in only_targets:
        fetch_bun_libs(library_directory)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
