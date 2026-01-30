#!/usr/bin/env python3

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

# resolve paths
ROOT = Path(__file__).resolve().parent
LIB_ROOT = ROOT / "lib"
REGISTRY_PATH = LIB_ROOT / "registry.json"
OUT_DIR = ROOT / "src" / "libs" / "lib"


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


@dataclass(frozen=True)
class FileEntry:
    """A file manifest containing builtin libraries."""

    path: str
    libs: list[LibEntry]


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
        declared_symbols = as_str(raw["declaredSymbols"], f"libs.{name}.declaredSymbols")

    specifier_aliases = None
    if raw.get("specifierAliases"):
        specifier_aliases = as_str(raw["specifierAliases"], f"libs.{name}.specifierAliases")

    # finalize
    return LibEntry(
        name=name,
        ambient=ambient,
        sources=sources,
        dependencies=dependencies,
        declared_symbols=declared_symbols,
        specifier_aliases=specifier_aliases,
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


def parse_registry() -> tuple[
    dict[str, list[str]],
    dict[str, list[list[str]]],
    dict[str, SourceSet],
    list[FileEntry],
]:
    """Parse the full registry."""
    # load registry
    registry = load_json(REGISTRY_PATH)
    symbol_sets = parse_symbol_sets(registry.get("symbols", {}))
    alias_sets = parse_alias_sets(registry.get("aliases", {}))
    manifest_paths = as_str_list(registry.get("manifests"), "registry.manifests")

    # collect file entries and source sets
    files: list[FileEntry] = []
    source_sets: list[dict[str, SourceSet]] = []

    for manifest_path in manifest_paths:
        path = LIB_ROOT / manifest_path
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
        "native_hosted": "BuiltinRuntime::NativeHosted",
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


def read_sources(source_set: SourceSet) -> list[str]:
    """Resolve sources for a source set."""
    # return explicit file list
    if source_set.files is not None:
        return source_set.files

    # skip when no glob is configured
    if source_set.auto is None:
        return []

    # expand glob relative to the lib root
    sources = []
    for path in LIB_ROOT.glob(source_set.auto):
        relative = str(path.relative_to(LIB_ROOT)).replace("\\", "/")
        sources.append(f"lib/{relative}")
    return sorted(sources)


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


def render_symbols(symbol_sets: dict[str, list[str]], alias_sets: dict[str, list[list[str]]]) -> str:
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
            lines.append(f"    \"{symbol}\",")
        lines.append("];\n")
    # specifier alias sets
    for name in sorted(alias_sets.keys()):
        const_name = const_name_for_alias_set(name)
        lines.append(f"pub(crate) const {const_name}: &[(&str, &str)] = &[")
        for left, right in alias_sets[name]:
            lines.append(f"    (\"{left}\", \"{right}\"),")
        lines.append("];\n")
    return "\n".join(lines).rstrip() + "\n"


def render_mod(group: str, files: list[FileEntry]) -> str:
    """Render the module index for a group."""
    lines = [
        "// generated by builtin/generate.py",
        "// run `just generate-builtin-libs` to regenerate",
        "",
    ]
    # collect module names
    modules = []
    for file in files:
        parts = file.path.split("/")
        name = Path(parts[-1]).stem
        modules.append(name)

    # declare modules
    if group == "":
        lines.append("mod symbols;")
    for module in modules:
        lines.append(f"mod {module};")

    lines.append("")
    # write exports
    if group == "":
        lines.append("use super::source::BuiltinLib;")
        lines.append("")

    for module in modules:
        lines.append(f"pub use {module}::*;")

    # emit lib registry
    if group == "":
        lines.append("")
        lines.append("pub const LIBS: &[BuiltinLib] = &[")
        for file in files:
            for lib in file.libs:
                lines.append(f"    {const_name_for_lib(lib.name)},")
        lines.append("];\n")

    return "\n".join(lines).rstrip() + "\n"


def render_file(
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
            sources_list = read_sources(target_set)
        else:
            sources_list = lib.sources

        target_key = (
            *(target_set.runtimes if target_set else []),
            "|",
            *(target_set.outputs if target_set else []),
            "|",
            *(target_set.platforms if target_set else []),
        )

        if target_set and (target_set.runtimes or target_set.outputs or target_set.platforms):
            uses_targeted = True
        else:
            uses_untargeted = True

        if target_key not in target_entries:
            target_entries[target_key] = []
            target_labels[target_key] = target_set.name if target_set else None

        for source in sources_list:
            if source not in source_const_map:
                root, *rest = source.split("/")
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
            root, *rest = source.split("/")
            module_path = "/".join(rest[:-1])
            file_name = rest[-1]
            entries.append((const_name, root, module_path, file_name))
        source_blocks.append((label, runtimes, outputs, platforms, entries))

    # imports
    import_lines = ["use crate::libs::source::BuiltinLib;"]
    if uses_targeted:
        import_lines.append("use crate::libs::source::{BuiltinOutputFormat, BuiltinPlatform, BuiltinRuntime};")

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
        import_lines.append(f"use crate::libs::lib::symbols::{{{', '.join(symbol_imports)}}};")

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
            for const_name, root, module_path, file_name in entries:
                lines.append(
                    f"        ({const_name}, \"{root}\", \"{module_path}\", \"{file_name}\"),"
                )
            lines.append("    ]")
            lines.append(");\n")
        else:
            lines.append("builtin_lib_sources!(")
            lines.append("    [")
            for const_name, root, module_path, file_name in entries:
                lines.append(
                    f"        ({const_name}, \"{root}\", \"{module_path}\", \"{file_name}\"),"
                )
            lines.append("    ]")
            lines.append(");\n")

    # libs
    for lib in file.libs:
        const_name = const_name_for_lib(lib.name)
        constructor = "ambient_lib" if lib.ambient else "explicit_lib"

        sources_list: list[str]
        if isinstance(lib.sources, str):
            sources_list = read_sources(source_sets[lib.sources])
        else:
            sources_list = lib.sources

        source_consts = [source_const_map[source] for source in sources_list]

        lines.append(f"pub const {const_name}: BuiltinLib = BuiltinLib::{constructor}(")
        lines.append(f"    \"{lib.name}\",")
        lines.append("    &[")
        for source in source_consts:
            lines.append(f"        {source},")
        lines.append("    ],")
        lines.append("    &[")
        for dep in lib.dependencies:
            lines.append(f"        \"{dep}\",")
        lines.append("    ],")
        lines.append(")")

        if lib.specifier_aliases:
            alias_const = const_name_for_alias_set(lib.specifier_aliases)
            lines.append(f".with_specifier_aliases({alias_const})")
        if lib.declared_symbols:
            symbol_const = const_name_for_symbol_set(lib.declared_symbols)
            lines.append(f".with_declared_symbols({symbol_const})")

        if lib.specifier_aliases or lib.declared_symbols:
            lines.append(";")
        else:
            lines[-1] = lines[-1] + ";"
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def generate() -> None:
    """Generate the builtin lib sources."""
    symbol_sets, alias_sets, source_sets, files = parse_registry()

    # symbols
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "symbols.rs").write_text(render_symbols(symbol_sets, alias_sets), encoding="utf-8")

    # file modules
    for file in files:
        out_path = OUT_DIR / file.path
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(render_file(file, symbol_sets, alias_sets, source_sets), encoding="utf-8")

    # mod files
    grouped = group_by_parent(files)
    for group, entries in grouped.items():
        if group == "":
            mod_path = OUT_DIR / "mod.rs"
        else:
            mod_path = OUT_DIR / group / "mod.rs"
        mod_path.write_text(render_mod(group, entries), encoding="utf-8")


def main() -> None:
    """Run the generator."""
    generate()


if __name__ == "__main__":
    main()
