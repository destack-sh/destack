#!/usr/bin/env python3

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

# resolve paths
ROOT = Path(__file__).resolve().parent
LANGUAGE_ROOT = ROOT / "language"
LIBRARY_ROOT = ROOT / "library"
LANGUAGE_REGISTRY_PATH = LANGUAGE_ROOT / "registry.json"
LIBRARY_REGISTRY_PATH = LIBRARY_ROOT / "registry.json"
LANGUAGE_OUT_DIR = ROOT / "src" / "libs" / "language"
LIBRARY_OUT_DIR = ROOT / "src" / "libs" / "library"
PLATFORM_DOC_DIRECTORIES = ["fs", "net", "process", "time", "timer"]
PLATFORM_DOC_SECTIONS = ["# Platform", "# Errors", "# Security", "# Replay"]
VERSIONED_LIB_PATTERN = re.compile(
    r"^(?P<base>.+)\.v(?P<version>[0-9][0-9A-Za-z._-]*)$"
)
LEGACY_FEATURE_BY_FAMILY = {
    "bun": "legacy-bun",
    "deno": "legacy-deno",
    "node": "legacy-node",
    "undici_types": "legacy-undici-types",
}


@dataclass(frozen=True)
class SourceSet:
    """A group of source files with shared target metadata."""

    name: str
    files: list[str] | None
    auto: str | None
    runtimes: list[str]
    outputs: list[str]
    platforms: list[str]


@dataclass(frozen=True)
class LibEntry:
    """A single builtin library definition."""

    name: str
    ambient: bool
    sources: str | list[str]
    dependencies: list[str]
    declared_symbols: str | None
    specifier_aliases: str | None
    types_package_names: list[str]
    latest_from: str | None


@dataclass(frozen=True)
class FileEntry:
    """A file manifest containing builtin libraries."""

    path: str
    libs: list[LibEntry]


@dataclass(frozen=True)
class LatestFileVariant:
    """A generated latest only file variant."""

    file: FileEntry
    latest_file: FileEntry
    latest_versioned_lib_names: set[str]


@dataclass(frozen=True)
class RegistryGroup:
    """One generated builtin family."""

    kind: str
    root: Path
    registry_path: Path
    out_dir: Path


def load_json(path: Path) -> dict:
    """Load a JSON file from disk."""
    # read json content
    return json.loads(path.read_text(encoding="utf-8"))


def as_str(value: object, context: str) -> str:
    """Coerce a value to a string."""
    # accept strings as is
    if isinstance(value, str):
        return value
    # reject unexpected types
    raise ValueError(f"expected string for {context}")


def as_bool(value: object, context: str) -> bool:
    """Coerce a value to a bool."""
    # accept booleans as is
    if isinstance(value, bool):
        return value
    # reject unexpected types
    raise ValueError(f"expected bool for {context}")


def as_str_list(value: object, context: str) -> list[str]:
    """Coerce a value to a list of strings."""
    # validate list shape
    if not isinstance(value, list):
        raise ValueError(f"expected list for {context}")
    # validate element types
    if not all(isinstance(item, str) for item in value):
        raise ValueError(f"expected string list for {context}")
    return value


def as_alias_list(value: object, context: str) -> list[list[str]]:
    """Coerce a value to a list of two element string pairs."""
    # validate list shape
    if not isinstance(value, list):
        raise ValueError(f"expected list for {context}")

    # validate alias entries
    aliases: list[list[str]] = []
    for item in value:
        is_pair = isinstance(item, list) and len(item) == 2
        if not is_pair or not all(isinstance(entry, str) for entry in item):
            raise ValueError(f"expected [string, string] for {context}")
        aliases.append(item)
    return aliases


def parse_symbol_sets(raw: dict) -> dict[str, list[str]]:
    """Parse symbol set definitions."""
    # parse symbol sets
    symbol_sets: dict[str, list[str]] = {}
    for name, value in raw.items():
        symbol_sets[name] = as_str_list(value, f"symbols.{name}")
    return symbol_sets


def parse_alias_sets(raw: dict) -> dict[str, list[list[str]]]:
    """Parse alias set definitions."""
    # parse alias sets
    alias_sets: dict[str, list[list[str]]] = {}
    for name, value in raw.items():
        alias_sets[name] = as_alias_list(value, f"aliases.{name}")
    return alias_sets


def parse_source_set(name: str, raw: dict) -> SourceSet:
    """Parse a source set entry."""
    # initialize fields
    files = None
    auto = None
    runtimes: list[str] = []
    outputs: list[str] = []
    platforms: list[str] = []

    # parse fields
    if "files" in raw:
        files = as_str_list(raw["files"], f"source_sets.{name}.files")
    if "auto" in raw:
        auto = as_str(raw["auto"], f"source_sets.{name}.auto")
    if "runtimes" in raw:
        runtimes = as_str_list(raw["runtimes"], f"source_sets.{name}.runtimes")
    if "outputs" in raw:
        outputs = as_str_list(raw["outputs"], f"source_sets.{name}.outputs")
    if "platforms" in raw:
        platforms = as_str_list(raw["platforms"], f"source_sets.{name}.platforms")

    # validate required fields
    if files is None and auto is None:
        raise ValueError(f"source_sets.{name} must define files or auto")

    # finalize
    return SourceSet(
        name=name,
        files=files,
        auto=auto,
        runtimes=runtimes,
        outputs=outputs,
        platforms=platforms,
    )


def parse_lib_entry(raw: dict) -> LibEntry:
    """Parse a library manifest entry."""
    # parse required fields
    name = as_str(raw.get("name"), "libs.name")
    if "sources" not in raw:
        raise ValueError(f"lib {name} must define sources")

    # parse sources
    sources_raw = raw["sources"]
    sources: str | list[str]
    if isinstance(sources_raw, list):
        sources = as_str_list(sources_raw, f"libs.{name}.sources")
    else:
        sources = as_str(sources_raw, f"libs.{name}.sources")

    # parse optional fields
    ambient = True
    if "ambient" in raw:
        ambient = as_bool(raw["ambient"], f"libs.{name}.ambient")

    dependencies = []
    if raw.get("dependencies"):
        dependencies = as_str_list(raw["dependencies"], f"libs.{name}.dependencies")

    declared_symbols = None
    if raw.get("declaredSymbols"):
        declared_symbols = as_str(
            raw["declaredSymbols"], f"libs.{name}.declaredSymbols"
        )

    latest_from = None
    if raw.get("latestFrom"):
        latest_from = as_str(raw["latestFrom"], f"libs.{name}.latestFrom")

    specifier_aliases = None
    if raw.get("specifierAliases"):
        specifier_aliases = as_str(
            raw["specifierAliases"], f"libs.{name}.specifierAliases"
        )

    types_package_names: list[str] = []
    if raw.get("typesPackageNames"):
        types_package_names = as_str_list(
            raw["typesPackageNames"], f"libs.{name}.typesPackageNames"
        )

    # finalize
    return LibEntry(
        name=name,
        ambient=ambient,
        sources=sources,
        dependencies=dependencies,
        declared_symbols=declared_symbols,
        specifier_aliases=specifier_aliases,
        types_package_names=types_package_names,
        latest_from=latest_from,
    )


def parse_manifest(path: Path) -> tuple[FileEntry, dict[str, SourceSet]]:
    """Parse a manifest file."""
    # load manifest json
    raw = load_json(path)

    # parse output path
    output_path = as_str(raw.get("path"), f"manifest {path} path")

    # parse source sets
    source_sets: dict[str, SourceSet] = {}
    for name, value in raw.get("sourceSets", {}).items():
        if not isinstance(value, dict):
            raise ValueError(f"sourceSets.{name} must be a table")
        source_sets[name] = parse_source_set(name, value)

    # parse libs
    libs_raw = raw.get("libs")
    if not isinstance(libs_raw, list):
        raise ValueError(f"manifest {path} libs must be a list")

    libs = [parse_lib_entry(entry) for entry in libs_raw]

    # finalize
    return FileEntry(path=output_path, libs=libs), source_sets


def merge_source_sets(sets: list[dict[str, SourceSet]]) -> dict[str, SourceSet]:
    """Merge source set definitions across manifests."""
    # merge unique definitions
    merged: dict[str, SourceSet] = {}
    for source_set_map in sets:
        for name, source_set in source_set_map.items():
            if name in merged and merged[name] != source_set:
                raise ValueError(f"source_sets.{name} defined multiple times")
            merged[name] = source_set
    return merged


def parse_registry(registry_path: Path, root: Path) -> tuple[
    dict[str, list[str]],
    dict[str, list[list[str]]],
    dict[str, SourceSet],
    list[FileEntry],
]:
    """Parse the full registry."""
    # load registry
    registry = load_json(registry_path)
    symbol_sets = parse_symbol_sets(registry.get("symbols", {}))
    alias_sets = parse_alias_sets(registry.get("aliases", {}))
    manifest_paths = as_str_list(registry.get("manifests"), "registry.manifests")

    # collect file entries and source sets
    files: list[FileEntry] = []
    source_sets: list[dict[str, SourceSet]] = []

    for manifest_path in manifest_paths:
        path = root / manifest_path
        file_entry, manifest_sets = parse_manifest(path)
        files.append(file_entry)
        source_sets.append(manifest_sets)

    # finalize combined view
    merged_source_sets = merge_source_sets(source_sets)

    return symbol_sets, alias_sets, merged_source_sets, files


def normalize_const_fragment(name: str) -> str:
    """Normalize a name segment for constant identifiers."""
    # normalize separators and punctuation
    return name.replace("/", "_").replace("-", "_").replace(".", "_")


def const_name_for_source(module_path: str, file_name: str) -> str:
    """Build a const name for a source file."""
    # combine path segments
    name = f"{module_path}/{file_name}" if module_path else file_name
    name = normalize_const_fragment(name)
    return f"LIB_{name.upper()}"


def const_name_for_symbol_set(name: str) -> str:
    """Build a const name for a symbol set."""
    return f"{normalize_const_fragment(name).upper()}_DECLARED_SYMBOLS"


def const_name_for_alias_set(name: str) -> str:
    """Build a const name for an alias set."""
    return f"{normalize_const_fragment(name).upper()}_SPECIFIER_ALIASES"


def const_name_for_lib(name: str) -> str:
    """Build a const name for a library."""
    return f"LIB_{normalize_const_fragment(name).upper()}"


def runtime_variants(values: Iterable[str]) -> list[str]:
    """Map runtime strings to enum variants."""
    # map runtime identifiers
    mapping = {
        "browser": "BuiltinRuntime::Browser",
        "node": "BuiltinRuntime::Node",
        "deno": "BuiltinRuntime::Deno",
        "bun": "BuiltinRuntime::Bun",
        "worker": "BuiltinRuntime::Worker",
        "wasm_js": "BuiltinRuntime::WasmJs",
        "wasm_wasi": "BuiltinRuntime::WasmWasi",
        "native_managed": "BuiltinRuntime::NativeManaged",
        "native_freestanding": "BuiltinRuntime::NativeFreestanding",
        "native_embedded": "BuiltinRuntime::NativeEmbedded",
    }
    return [mapping[value] for value in values]


def output_variants(values: Iterable[str]) -> list[str]:
    """Map output strings to enum variants."""
    # map output identifiers
    mapping = {
        "js": "BuiltinOutputFormat::Js",
        "ts": "BuiltinOutputFormat::Ts",
        "wasm": "BuiltinOutputFormat::Wasm",
        "native": "BuiltinOutputFormat::Native",
    }
    return [mapping[value] for value in values]


def platform_variants(values: Iterable[str]) -> list[str]:
    """Map platform strings to enum variants."""
    # map platform identifiers
    mapping = {
        "web": "BuiltinPlatform::Web",
        "windows": "BuiltinPlatform::Windows",
        "macos": "BuiltinPlatform::MacOS",
        "linux": "BuiltinPlatform::Linux",
        "ios": "BuiltinPlatform::IOS",
        "android": "BuiltinPlatform::Android",
        "wasi": "BuiltinPlatform::Wasi",
        "bare_metal": "BuiltinPlatform::BareMetal",
        "universal": "BuiltinPlatform::Universal",
    }
    return [mapping[value] for value in values]


def read_sources(root: Path, source_set: SourceSet) -> list[str]:
    """Resolve sources for a source set."""
    # return explicit file list
    if source_set.files is not None:
        return source_set.files

    # skip when no glob is configured
    if source_set.auto is None:
        return []

    # expand glob relative to the lib root
    sources = []
    for path in root.glob(source_set.auto):
        relative = str(path.relative_to(root)).replace("\\", "/")
        sources.append(f"{root.name}/{relative}")
    return sorted(sources)


def reference_libs_for_sources(root: Path, sources: list[str]) -> list[str]:
    """Collect reference lib directives for a set of builtin source files."""
    references: list[str] = []
    seen: set[str] = set()

    for source in sources:
        if not source.endswith(".d.ts"):
            continue

        source_path = root / source.removeprefix(f"{root.name}/")
        content = source_path.read_text(encoding="utf-8")
        for reference in reference_libs_from_content(content):
            if reference not in seen:
                seen.add(reference)
                references.append(reference)

    return references


def reference_libs_from_content(content: str) -> list[str]:
    """Collect reference lib directives from one source file."""
    references: list[str] = []

    for line in content.splitlines():
        stripped = line.lstrip()
        if not stripped.startswith("///"):
            continue

        directive = stripped.removeprefix("///").lstrip()
        if not directive.startswith("<reference"):
            continue

        reference = parse_reference_lib(directive)
        if reference is not None:
            references.append(reference)

    return references


def parse_reference_lib(directive: str) -> str | None:
    """Parse one `lib=` attribute from a reference directive."""
    parts = directive.split()
    head = next(iter(parts), None)
    if head is None or not head.startswith("<reference"):
        return None

    for part in parts:
        if not part.startswith("lib="):
            continue

        value = part[len("lib=") :].lstrip()
        if value.startswith('"'):
            quote = '"'
            value = value[1:]
        elif value.startswith("'"):
            quote = "'"
            value = value[1:]
        else:
            continue

        end = value.find(quote)
        if end == -1:
            return None

        return value[:end]

    return None


def group_by_parent(files: list[FileEntry]) -> dict[str, list[FileEntry]]:
    """Group file entries by their top level directory."""
    # group by parent directory
    grouped: dict[str, list[FileEntry]] = {}
    for file in files:
        parts = file.path.split("/")
        if len(parts) == 1:
            group = ""
        else:
            group = parts[0]
        grouped.setdefault(group, []).append(file)
    return grouped


def find_binding_doc_errors(path: Path) -> list[str]:
    """Find missing binding documentation sections in a platform file."""
    # read file content
    lines = path.read_text(encoding="utf-8").splitlines()
    errors: list[str] = []

    # scan for binding decorators
    for index, line in enumerate(lines):
        if "@binding(" not in line:
            continue

        # collect adjacent documentation lines
        documentation_lines: list[str] = []
        cursor = index - 1
        while cursor >= 0:
            stripped_line = lines[cursor].strip()

            # include documentation comments
            if stripped_line.startswith("///"):
                documentation_lines.append(stripped_line)
                cursor -= 1
                continue

            # include blank separators inside docs
            if stripped_line == "":
                documentation_lines.append(stripped_line)
                cursor -= 1
                continue

            break

        # reverse to source order
        documentation_lines.reverse()
        if not documentation_lines:
            errors.append(f"{path}:{index + 1}: missing binding documentation")
            continue

        # join docs for section checks
        joined = "\n".join(documentation_lines)
        for section in PLATFORM_DOC_SECTIONS:
            if section not in joined:
                errors.append(
                    f"{path}:{index + 1}: missing `{section}` in binding documentation"
                )

    return errors


def validate_platform_binding_docs() -> None:
    """Validate platform binding docs for required sections."""
    # collect files in configured directories
    all_errors: list[str] = []
    for directory_name in PLATFORM_DOC_DIRECTORIES:
        directory = LIBRARY_ROOT / "platform" / directory_name
        for path in sorted(directory.glob("*.ds")):
            all_errors.extend(find_binding_doc_errors(path))

    # fail with the full error list
    if all_errors:
        formatted_errors = "\n".join(all_errors)
        raise ValueError(
            f"platform binding documentation validation failed:\n{formatted_errors}"
        )


def render_symbols(
    symbol_sets: dict[str, list[str]], alias_sets: dict[str, list[list[str]]]
) -> str:
    """Render the generated symbols module."""
    lines = [
        "// generated by builtin/generate.py",
        "// run `just generate-builtin-libs` to regenerate",
        "",
        "#![allow(dead_code)]",
        "",
    ]
    # declared symbol sets
    for name in sorted(symbol_sets.keys()):
        const_name = const_name_for_symbol_set(name)
        lines.append(f"pub(crate) const {const_name}: &[&str] = &[")
        for symbol in symbol_sets[name]:
            lines.append(f'    "{symbol}",')
        lines.append("];\n")
    # specifier alias sets
    for name in sorted(alias_sets.keys()):
        const_name = const_name_for_alias_set(name)
        lines.append(f"pub(crate) const {const_name}: &[(&str, &str)] = &[")
        for left, right in alias_sets[name]:
            lines.append(f'    ("{left}", "{right}"),')
        lines.append("];\n")
    return "\n".join(lines).rstrip() + "\n"


def module_name_for_path(path: str) -> str:
    """Get the module name for a generated file path."""
    # extract the file stem
    return Path(path).stem


def family_name_for_path(path: str) -> str:
    """Get the lib family name for a generated file path."""
    # derive the top level family for feature gates
    if "/" in path:
        return path.split("/", maxsplit=1)[0]
    return Path(path).stem


def feature_name_for_family(family: str) -> str:
    """Build the cargo feature name for a lib family."""
    # normalize family names for feature identifiers
    return f"lib-{family.replace('_', '-')}"


def legacy_mode_condition_for_family(family: str) -> str:
    """Build the feature condition for all versions mode."""
    # include explicit all versions mode
    conditions = ['feature = "versions-all"']

    # include family specific legacy override when defined
    legacy_feature = LEGACY_FEATURE_BY_FAMILY.get(family)
    if legacy_feature:
        conditions.append(f'feature = "{legacy_feature}"')

    if len(conditions) == 1:
        return conditions[0]

    return f"any({', '.join(conditions)})"


def cfg_all(*conditions: str) -> str:
    """Join conditions with cfg all syntax."""
    # collapse single item conditions
    if len(conditions) == 1:
        return conditions[0]

    return f"all({', '.join(conditions)})"


def latest_mode_condition_for_family(family: str) -> str:
    """Build the feature condition for latest only mode."""
    # require explicit latest mode
    conditions = ['feature = "versions-latest"', 'not(feature = "versions-all")']

    # disable latest mode when legacy is explicitly enabled
    legacy_feature = LEGACY_FEATURE_BY_FAMILY.get(family)
    if legacy_feature:
        conditions.append(f'not(feature = "{legacy_feature}")')

    return cfg_all(*conditions)


def versioned_name_parts(name: str) -> tuple[str, str] | None:
    """Split a versioned lib name into base and version."""
    # parse .v* suffixed names
    match = VERSIONED_LIB_PATTERN.match(name)
    if not match:
        return None

    return match.group("base"), match.group("version")


def version_key(version: str) -> tuple[tuple[int, int | str], ...]:
    """Build a sortable key for semanticish version strings."""
    # split dotted parts and rank numeric tokens first
    tokens: list[tuple[int, int | str]] = []
    for token in version.split("."):
        if token.isdigit():
            tokens.append((0, int(token)))
        else:
            tokens.append((1, token))

    return tuple(tokens)


def latest_file_variant(file: FileEntry) -> LatestFileVariant | None:
    """Build a latest only variant for files with legacy versioned libs."""
    # index libs by name for explicit latest overrides
    libs_by_name: dict[str, LibEntry] = {lib.name: lib for lib in file.libs}

    # track latest versioned lib per base name
    latest_by_base: dict[str, tuple[tuple[int, int | str], str]] = {}
    for lib in file.libs:
        parts = versioned_name_parts(lib.name)
        if parts is None:
            continue

        base, version = parts
        key = version_key(version)
        current = latest_by_base.get(base)
        if current is None or key > current[0]:
            latest_by_base[base] = (key, lib.name)

    # skip files without legacy version variants
    if not latest_by_base:
        return None

    # keep unversioned aliases and latest explicit versions
    latest_versioned_lib_names = {name for _, name in latest_by_base.values()}
    filtered_libs: list[LibEntry] = []
    for lib in file.libs:
        parts = versioned_name_parts(lib.name)

        # keep non versioned names
        if parts is None:
            filtered_libs.append(lib)
            continue

        # keep the latest explicit version per base
        if lib.name in latest_versioned_lib_names:
            filtered_libs.append(lib)

    has_dropped_legacy = len(filtered_libs) != len(file.libs)

    # rewrite unversioned aliases that pin explicit latest sources
    has_latest_override = False
    latest_libs: list[LibEntry] = []
    for lib in filtered_libs:
        if lib.latest_from is None:
            latest_libs.append(lib)
            continue

        latest_target = libs_by_name.get(lib.latest_from)
        if latest_target is None:
            raise ValueError(
                f"missing latestFrom target for {lib.name}: {lib.latest_from}"
            )

        latest_libs.append(
            LibEntry(
                name=lib.name,
                ambient=lib.ambient,
                sources=latest_target.sources,
                dependencies=lib.dependencies,
                declared_symbols=lib.declared_symbols,
                specifier_aliases=lib.specifier_aliases,
                types_package_names=lib.types_package_names,
                latest_from=lib.latest_from,
            )
        )
        has_latest_override = True

    # skip files that do not change in latest mode
    if not has_dropped_legacy and not has_latest_override:
        return None

    latest_file = FileEntry(path=file.path, libs=latest_libs)

    return LatestFileVariant(
        file=file,
        latest_file=latest_file,
        latest_versioned_lib_names=latest_versioned_lib_names,
    )


def render_mod(
    libs_const_name: str,
    group: str,
    files: list[FileEntry],
    *,
    all_lib_files: list[FileEntry] | None = None,
    child_groups: list[str] | None = None,
    latest_variants_by_path: dict[str, LatestFileVariant] | None = None,
) -> str:
    """Render the module index for a group."""
    lines = [
        "// generated by builtin/generate.py",
        "// run `just generate-builtin-libs` to regenerate",
        "",
    ]

    # collect module names
    modules: list[tuple[FileEntry, str]] = []
    for file in files:
        modules.append((file, module_name_for_path(file.path)))

    # declare modules
    if group == "":
        lines.append("mod symbols;")

    for file, module in modules:
        if group != "":
            lines.append(f"mod {module};")
            continue

        family = family_name_for_path(file.path)
        feature_name = feature_name_for_family(family)
        feature_condition = f'feature = "{feature_name}"'

        latest_variant = None
        if latest_variants_by_path is not None:
            latest_variant = latest_variants_by_path.get(file.path)

        if latest_variant is None:
            lines.append(f"#[cfg({feature_condition})]")
            lines.append(f"mod {module};")
            continue

        legacy_mode_condition = legacy_mode_condition_for_family(family)
        all_mode_condition = cfg_all(feature_condition, legacy_mode_condition)
        latest_mode_condition = cfg_all(
            feature_condition, latest_mode_condition_for_family(family)
        )

        lines.append(f"#[cfg({all_mode_condition})]")
        lines.append(f"mod {module};")
        lines.append(f"#[cfg({latest_mode_condition})]")
        lines.append(f"mod {module}_latest;")

    if group == "":
        for child_group in child_groups or []:
            feature_name = feature_name_for_family(child_group)
            feature_condition = f'feature = "{feature_name}"'
            lines.append(f"#[cfg({feature_condition})]")
            lines.append(f"mod {child_group};")

    lines.append("")

    # write exports
    if group == "":
        lines.append("use super::source::BuiltinLibrary;")
        lines.append("")

    for file, module in modules:
        if group != "":
            lines.append(f"pub use {module}::*;")
            continue

        family = family_name_for_path(file.path)
        feature_name = feature_name_for_family(family)
        feature_condition = f'feature = "{feature_name}"'

        latest_variant = None
        if latest_variants_by_path is not None:
            latest_variant = latest_variants_by_path.get(file.path)

        if latest_variant is None:
            lines.append(f"#[cfg({feature_condition})]")
            lines.append(f"pub use {module}::*;")
            continue

        legacy_mode_condition = legacy_mode_condition_for_family(family)
        all_mode_condition = cfg_all(feature_condition, legacy_mode_condition)
        latest_mode_condition = cfg_all(
            feature_condition, latest_mode_condition_for_family(family)
        )

        lines.append(f"#[cfg({all_mode_condition})]")
        lines.append(f"pub use {module}::*;")
        lines.append(f"#[cfg({latest_mode_condition})]")
        lines.append(f"pub use {module}_latest::*;")

    if group == "":
        for child_group in child_groups or []:
            feature_name = feature_name_for_family(child_group)
            feature_condition = f'feature = "{feature_name}"'
            lines.append(f"#[cfg({feature_condition})]")
            lines.append(f"pub use {child_group}::*;")

    # emit lib registry
    if group == "":
        lib_files = all_lib_files if all_lib_files is not None else files

        lines.append("")
        lines.append(f"pub const {libs_const_name}: &[BuiltinLibrary] = &[")
        for file in lib_files:
            family = family_name_for_path(file.path)
            feature_name = feature_name_for_family(family)
            feature_condition = f'feature = "{feature_name}"'

            latest_variant = None
            if latest_variants_by_path is not None:
                latest_variant = latest_variants_by_path.get(file.path)

            latest_versioned_lib_names: set[str] = set()
            family_mode_condition = feature_condition
            legacy_mode_condition = ""
            if latest_variant is not None:
                latest_versioned_lib_names = latest_variant.latest_versioned_lib_names
                legacy_mode_condition = legacy_mode_condition_for_family(family)
                latest_mode_condition = latest_mode_condition_for_family(family)
                family_mode_condition = cfg_all(
                    feature_condition,
                    f"any({legacy_mode_condition}, {latest_mode_condition})",
                )

            for lib in file.libs:
                condition = family_mode_condition

                parts = versioned_name_parts(lib.name)
                if (
                    latest_variant is not None
                    and parts is not None
                    and lib.name not in latest_versioned_lib_names
                ):
                    condition = cfg_all(feature_condition, legacy_mode_condition)
                lines.append(f"    #[cfg({condition})]")
                lines.append(f"    {const_name_for_lib(lib.name)},")
        lines.append("];\n")

    return "\n".join(lines).rstrip() + "\n"


def render_file(
    kind: str,
    module_name: str,
    root: Path,
    file: FileEntry,
    symbol_sets: dict[str, list[str]],
    alias_sets: dict[str, list[list[str]]],
    source_sets: dict[str, SourceSet],
) -> str:
    """Render a single generated lib module."""
    lines = [
        "// generated by builtin/generate.py",
        "// run `just generate-builtin-libs` to regenerate",
        "",
    ]

    # collect symbol and alias usage
    needs_symbols: set[str] = set()
    needs_aliases: set[str] = set()
    uses_targeted = False
    uses_untargeted = False

    # resolve sources and build const map
    source_const_map: dict[str, str] = {}
    source_blocks: list[
        tuple[
            str | None,
            list[str],
            list[str],
            list[str],
            list[tuple[str, str, str, str]],
        ]
    ] = []
    target_entries: dict[tuple[str, ...], list[str]] = {}
    target_labels: dict[tuple[str, ...], str | None] = {}

    # collect sources per target profile
    for lib in file.libs:
        if lib.declared_symbols:
            needs_symbols.add(lib.declared_symbols)
        if lib.specifier_aliases:
            needs_aliases.add(lib.specifier_aliases)

        sources_list: list[str]
        target_set: SourceSet | None = None
        if isinstance(lib.sources, str):
            target_set = source_sets[lib.sources]
            sources_list = read_sources(root, target_set)
        else:
            sources_list = lib.sources

        target_key = (
            *(target_set.runtimes if target_set else []),
            "|",
            *(target_set.outputs if target_set else []),
            "|",
            *(target_set.platforms if target_set else []),
        )

        if target_set and (
            target_set.runtimes or target_set.outputs or target_set.platforms
        ):
            uses_targeted = True
        else:
            uses_untargeted = True

        if target_key not in target_entries:
            target_entries[target_key] = []
            target_labels[target_key] = target_set.name if target_set else None

        for source in sources_list:
            if source not in source_const_map:
                source_root, *rest = source.split("/")
                module_path = "/".join(rest[:-1])
                file_name = rest[-1]
                const_name = const_name_for_source(
                    module_path.replace("/", "_"),
                    file_name,
                )
                source_const_map[source] = const_name
            if source not in target_entries[target_key]:
                target_entries[target_key].append(source)

    # validate referenced symbol sets
    missing_symbols = sorted(name for name in needs_symbols if name not in symbol_sets)
    if missing_symbols:
        raise ValueError(f"missing declared_symbols: {', '.join(missing_symbols)}")

    # validate referenced alias sets
    missing_aliases = sorted(name for name in needs_aliases if name not in alias_sets)
    if missing_aliases:
        raise ValueError(f"missing specifier_aliases: {', '.join(missing_aliases)}")

    # materialize source blocks
    for target_key, sources in target_entries.items():
        label = target_labels[target_key]
        runtimes: list[str] = []
        outputs: list[str] = []
        platforms: list[str] = []
        if label:
            target_set = source_sets[label]
            runtimes = target_set.runtimes
            outputs = target_set.outputs
            platforms = target_set.platforms
        entries = []
        for source in sources:
            const_name = source_const_map[source]
            source_root, *rest = source.split("/")
            module_path = "/".join(rest[:-1])
            file_name = rest[-1]
            entries.append((const_name, source_root, module_path, file_name))
        source_blocks.append((label, runtimes, outputs, platforms, entries))

    # imports
    import_lines = ["use crate::libs::source::BuiltinLibrary;"]
    if uses_targeted:
        import_lines.append(
            "use crate::libs::source::{BuiltinOutputFormat, BuiltinPlatform, BuiltinRuntime};"
        )

    macro_imports = []
    if uses_untargeted:
        macro_imports.append("builtin_lib_sources")
    if uses_targeted:
        macro_imports.append("builtin_lib_sources_targeted")
    if macro_imports:
        import_lines.append(f"use crate::{{{', '.join(macro_imports)}}};")

    # import symbol and alias sets
    if needs_symbols or needs_aliases:
        symbol_imports = []
        for name in sorted(needs_symbols):
            symbol_imports.append(const_name_for_symbol_set(name))
        for name in sorted(needs_aliases):
            symbol_imports.append(const_name_for_alias_set(name))
        import_lines.append(
            f"use crate::libs::{module_name}::symbols::{{{', '.join(symbol_imports)}}};"
        )

    lines.extend(import_lines)
    lines.append("")

    # target consts and source blocks
    for set_name, runtimes, outputs, platforms, entries in source_blocks:
        if set_name and (runtimes or outputs or platforms):
            prefix = set_name.upper()
            lines.append(f"const {prefix}_RUNTIMES: &[BuiltinRuntime] = &[")
            for runtime in runtime_variants(runtimes):
                lines.append(f"    {runtime},")
            lines.append("];\n")

            lines.append(f"const {prefix}_OUTPUTS: &[BuiltinOutputFormat] = &[")
            for output in output_variants(outputs):
                lines.append(f"    {output},")
            lines.append("];\n")

            lines.append(f"const {prefix}_PLATFORMS: &[BuiltinPlatform] = &[")
            for platform in platform_variants(platforms):
                lines.append(f"    {platform},")
            lines.append("];\n")

            lines.append("builtin_lib_sources_targeted!(")
            lines.append(f"    {prefix}_RUNTIMES,")
            lines.append(f"    {prefix}_OUTPUTS,")
            lines.append(f"    {prefix}_PLATFORMS,")
            lines.append("    [")
            for const_name, source_root, module_path, file_name in entries:
                lines.append(
                    f'        ({const_name}, "{source_root}", "{module_path}", "{file_name}"),'
                )
            lines.append("    ]")
            lines.append(");\n")
        else:
            lines.append("builtin_lib_sources!(")
            lines.append("    [")
            for const_name, source_root, module_path, file_name in entries:
                lines.append(
                    f'        ({const_name}, "{source_root}", "{module_path}", "{file_name}"),'
                )
            lines.append("    ]")
            lines.append(");\n")

    # libs
    for lib in file.libs:
        const_name = const_name_for_lib(lib.name)
        constructor = "language" if kind == "language" else "library"
        mode = "ambient" if lib.ambient else "explicit"

        sources_list: list[str]
        if isinstance(lib.sources, str):
            sources_list = read_sources(root, source_sets[lib.sources])
        else:
            sources_list = lib.sources

        source_consts = [source_const_map[source] for source in sources_list]
        reference_libs = reference_libs_for_sources(root, sources_list)

        lines.append(f"pub const {const_name}: BuiltinLibrary = BuiltinLibrary::{constructor}(")
        lines.append(f'    "{lib.name}",')
        lines.append("    &[")
        for source in source_consts:
            lines.append(f"        {source},")
        lines.append("    ],")
        lines.append("    &[")
        for dep in lib.dependencies:
            lines.append(f'        "{dep}",')
        lines.append("    ],")
        lines.append(")")
        lines.append(f".{mode}()")

        if lib.specifier_aliases:
            alias_const = const_name_for_alias_set(lib.specifier_aliases)
            lines.append(f".with_specifier_aliases({alias_const})")
        if reference_libs:
            names = ", ".join(f'"{name}"' for name in reference_libs)
            lines.append(f".with_reference_libs(&[{names}])")
        if lib.types_package_names:
            names = ", ".join(f'"{name}"' for name in lib.types_package_names)
            lines.append(f".with_types_package_names(&[{names}])")
        if lib.declared_symbols:
            symbol_const = const_name_for_symbol_set(lib.declared_symbols)
            lines.append(f".with_declared_symbols({symbol_const})")

        if (
            lib.specifier_aliases
            or reference_libs
            or lib.types_package_names
            or lib.declared_symbols
        ):
            lines.append(";")
        else:
            lines[-1] = lines[-1] + ";"
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def generate_group(group: RegistryGroup) -> None:
    """Generate one builtin family."""
    symbol_sets, alias_sets, source_sets, files = parse_registry(
        group.registry_path, group.root
    )

    # derive latest only variants for versioned runtime libs
    latest_variants: list[LatestFileVariant] = []
    latest_variants_by_path: dict[str, LatestFileVariant] = {}
    for file in files:
        variant = latest_file_variant(file)
        if variant is None:
            continue

        latest_variants.append(variant)
        latest_variants_by_path[file.path] = variant

    # symbols
    group.out_dir.mkdir(parents=True, exist_ok=True)
    (group.out_dir / "symbols.rs").write_text(
        render_symbols(symbol_sets, alias_sets), encoding="utf-8"
    )

    # file modules
    for file in files:
        out_path = group.out_dir / file.path
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(
            render_file(
                group.kind,
                group.root.name,
                group.root,
                file,
                symbol_sets,
                alias_sets,
                source_sets,
            ),
            encoding="utf-8",
        )

    # latest only file modules
    expected_latest_paths: set[Path] = set()
    for variant in latest_variants:
        out_path = group.out_dir / variant.file.path
        latest_path = out_path.with_name(f"{out_path.stem}_latest.rs")
        latest_path.parent.mkdir(parents=True, exist_ok=True)
        latest_path.write_text(
            render_file(
                group.kind,
                group.root.name,
                group.root,
                variant.latest_file,
                symbol_sets,
                alias_sets,
                source_sets,
            ),
            encoding="utf-8",
        )
        expected_latest_paths.add(latest_path.resolve())

    # clean stale latest files no longer generated
    for path in group.out_dir.rglob("*_latest.rs"):
        if path.resolve() in expected_latest_paths:
            continue

        path.unlink()

    # mod files
    grouped = group_by_parent(files)
    libs_const_name = f"{group.kind.upper()}_LIBS"
    for subgroup, entries in grouped.items():
        if subgroup == "":
            child_groups = sorted(name for name in grouped.keys() if name)
            rendered_mod = render_mod(
                libs_const_name,
                subgroup,
                entries,
                all_lib_files=files,
                child_groups=child_groups,
                latest_variants_by_path=latest_variants_by_path,
            )
        else:
            rendered_mod = render_mod(libs_const_name, subgroup, entries)

        if subgroup == "":
            mod_path = group.out_dir / "mod.rs"
        else:
            mod_path = group.out_dir / subgroup / "mod.rs"
        mod_path.write_text(rendered_mod, encoding="utf-8")


def generate() -> None:
    """Generate the builtin lib sources."""
    validate_platform_binding_docs()

    groups = [
        RegistryGroup(
            kind="language",
            root=LANGUAGE_ROOT,
            registry_path=LANGUAGE_REGISTRY_PATH,
            out_dir=LANGUAGE_OUT_DIR,
        ),
        RegistryGroup(
            kind="library",
            root=LIBRARY_ROOT,
            registry_path=LIBRARY_REGISTRY_PATH,
            out_dir=LIBRARY_OUT_DIR,
        ),
    ]

    for group in groups:
        generate_group(group)


def main() -> None:
    """Run the generator."""
    generate()


if __name__ == "__main__":
    main()
