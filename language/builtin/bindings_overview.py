#!/usr/bin/env python3
"""Render a hierarchical overview of platform @binding declarations."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


FIELD_ORDER = [
    "effect",
    "replay",
    "payload",
    "scope",
    "blocking",
    "capabilities",
    "platforms",
]

PLATFORM_ENUM_PATTERN = re.compile(
    r"pub enum Platform \{(.*?)\n\}",
    re.DOTALL,
)
ENUM_VARIANT_PATTERN = re.compile(r"^\s*([A-Za-z][A-Za-z0-9]*)\s*,", re.MULTILINE)


def load_canonical_platform_tags() -> set[str]:
    """Load canonical platform tags from workspace target enum variants."""
    root = Path(__file__).resolve().parents[1]
    target_config_path = root / "workspace" / "src" / "config" / "target.rs"
    source = target_config_path.read_text(encoding="utf-8")
    match = PLATFORM_ENUM_PATTERN.search(source)
    if match is None:
        raise RuntimeError("missing Platform enum in workspace target config")

    variants = ENUM_VARIANT_PATTERN.findall(match.group(1))
    if not variants:
        raise RuntimeError("missing Platform variants in workspace target config")

    return {variant.lower() for variant in variants}


CANONICAL_PLATFORM_TAGS = load_canonical_platform_tags()
PLATFORM_SELECTOR_TAGS = {"unix", "bsd"}

SEGMENT_PATTERN = re.compile(r"^[a-z][a-zA-Z0-9]*$")


@dataclass
class BindingRecord:
    """One extracted binding declaration."""

    module: str
    relative_file: str
    line: int
    binding_id: str
    function_name: str
    summary_line: str
    detail_line: str | None
    effect: str | None
    replay: str | None
    payload: str | None
    scope: str | None
    blocking: str | None
    capabilities: list[str]
    platforms: list[str]


def parse_arguments() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Render platform binding metadata in a hierarchical view."
    )
    parser.add_argument(
        "--root",
        default="language/builtin/lib/platform",
        help="binding source root directory",
    )
    parser.add_argument(
        "--module",
        action="append",
        default=[],
        help="restrict to one module directory, repeatable",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="emit json instead of text",
    )
    parser.add_argument(
        "--no-color",
        action="store_true",
        help="disable ansi colors",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate binding metadata and exit nonzero on violations",
    )
    return parser.parse_args()


def find_binding_blocks(source: str) -> list[tuple[int, int]]:
    """Find byte ranges for each @binding(...) block."""
    blocks: list[tuple[int, int]] = []
    cursor = 0

    while True:
        start = source.find("@binding(", cursor)
        if start == -1:
            break

        depth = 0
        index = start
        while index < len(source):
            character = source[index]
            if character == "(":
                depth += 1
            elif character == ")":
                depth -= 1
                if depth == 0:
                    blocks.append((start, index + 1))
                    cursor = index + 1
                    break
            index += 1
        else:
            raise ValueError("unterminated @binding(...) block")

    return blocks


def parse_binding_id(block: str) -> str:
    """Parse the binding id from one @binding block."""
    match = re.search(r'@binding\(\s*"([^"]+)"', block)
    if match is None:
        raise ValueError("missing binding id")
    return match.group(1)


def parse_option_value(block: str, key: str) -> str | None:
    """Parse one string option value by key."""
    match = re.search(rf"{key}\s*:\s*\"([^\"]+)\"", block)
    if match is None:
        return None
    return match.group(1)


def parse_capabilities(block: str) -> list[str]:
    """Parse the capabilities array from one @binding block."""
    match = re.search(r"capabilities\s*:\s*\[([^\]]*)\]", block, re.DOTALL)
    if match is None:
        return []

    capabilities_body = match.group(1)
    capabilities = re.findall(r'"([^"]+)"', capabilities_body)
    return capabilities


def parse_platforms(block: str) -> list[str]:
    """Parse the platforms array from one @binding block."""
    match = re.search(r"platforms\s*:\s*\[([^\]]*)\]", block, re.DOTALL)
    if match is None:
        return []

    platforms_body = match.group(1)
    platforms = re.findall(r'"([^"]+)"', platforms_body)
    return platforms


def parse_function_name(source: str, block_end: int) -> str:
    """Parse the function name that follows one @binding block."""
    tail = source[block_end : block_end + 500]
    match = re.search(r"export\s+function\s+([A-Za-z0-9_]+)\s*\(", tail)
    if match is None:
        return "<unknown>"
    return match.group(1)


def parse_doc_lines(lines: list[str], binding_line_index: int) -> tuple[str, str | None]:
    """Parse summary and top detail line from docs above one binding."""
    # skip immediate blank lines above the binding
    cursor = binding_line_index - 1
    while cursor >= 0 and lines[cursor].strip() == "":
        cursor -= 1

    # collect doc block lines
    documentation_lines: list[str] = []
    while cursor >= 0:
        stripped = lines[cursor].strip()
        if stripped.startswith("///"):
            documentation_lines.append(stripped[3:].lstrip())
            cursor -= 1
            continue
        if stripped == "":
            documentation_lines.append("")
            cursor -= 1
            continue
        break

    # normalize source order
    documentation_lines.reverse()
    if not documentation_lines:
        return ("<missing summary>", None)

    # find first non-heading text line
    summary_index = -1
    for index, line in enumerate(documentation_lines):
        line = line.strip()
        if line == "" or line.startswith("#"):
            continue
        summary_index = index
        break

    if summary_index == -1:
        return ("<missing summary>", None)

    summary_line = documentation_lines[summary_index].strip()

    # find first body line after summary paragraph
    detail_line: str | None = None
    seen_separator = False
    for line in documentation_lines[summary_index + 1 :]:
        stripped = line.strip()
        if stripped == "":
            seen_separator = True
            continue
        if stripped.startswith("#"):
            continue
        if seen_separator:
            detail_line = stripped
            break

    return (summary_line, detail_line)


def collect_records(root: Path, allowed_modules: set[str]) -> list[BindingRecord]:
    """Collect binding records from all .ds files under root."""
    records: list[BindingRecord] = []
    for path in sorted(root.rglob("*.ds")):
        relative = path.relative_to(root)
        if not relative.parts:
            continue

        module = relative.parts[0]
        if allowed_modules and module not in allowed_modules:
            continue

        source = path.read_text(encoding="utf-8")
        source_lines = source.splitlines()
        for start, end in find_binding_blocks(source):
            block = source[start:end]
            binding_id = parse_binding_id(block)
            function_name = parse_function_name(source, end)
            line = source.count("\n", 0, start) + 1
            summary_line, detail_line = parse_doc_lines(source_lines, line - 1)

            records.append(
                BindingRecord(
                    module=module,
                    relative_file=str(relative),
                    line=line,
                    binding_id=binding_id,
                    function_name=function_name,
                    summary_line=summary_line,
                    detail_line=detail_line,
                    effect=parse_option_value(block, "effect"),
                    replay=parse_option_value(block, "replay"),
                    payload=parse_option_value(block, "payload"),
                    scope=parse_option_value(block, "scope"),
                    blocking=parse_option_value(block, "blocking"),
                    capabilities=parse_capabilities(block),
                    platforms=parse_platforms(block),
                )
            )

    return records


def render_record_metadata(record: BindingRecord) -> str:
    """Render one record metadata fragment."""
    metadata: dict[str, str] = {
        "effect": record.effect or "-",
        "replay": record.replay or "-",
        "payload": record.payload or "-",
        "scope": record.scope or "-",
        "blocking": record.blocking or "-",
        "capabilities": ",".join(record.capabilities) if record.capabilities else "-",
        "platforms": ",".join(record.platforms) if record.platforms else "-",
    }
    return " ".join(f"{field}={metadata[field]}" for field in FIELD_ORDER)


def colorize(text: str, code: str, use_color: bool) -> str:
    """Apply one ansi style sequence when colors are enabled."""
    if not use_color:
        return text
    return f"\x1b[{code}m{text}\x1b[0m"


def render_text(records: list[BindingRecord], root: Path, use_color: bool) -> str:
    """Render all records as a hierarchical text report."""
    lines: list[str] = []
    lines.append(colorize("Platform Binding Overview", "1;36", use_color))
    lines.append(f"root: {root}")
    lines.append(f"bindings: {len(records)}")

    modules: dict[str, list[BindingRecord]] = {}
    for record in records:
        modules.setdefault(record.module, []).append(record)

    for module in sorted(modules.keys()):
        module_records = modules[module]
        lines.append("")
        lines.append(colorize(f"[{module}] ({len(module_records)})", "1;34", use_color))

        files: dict[str, list[BindingRecord]] = {}
        for record in module_records:
            files.setdefault(record.relative_file, []).append(record)

        for relative_file in sorted(files.keys()):
            file_records = sorted(files[relative_file], key=lambda item: item.line)
            lines.append(
                f"  {colorize(relative_file, '35', use_color)} "
                f"{colorize(f'[{len(file_records)}]', '2', use_color)}"
            )

            for record in file_records:
                metadata = render_record_metadata(record)
                binding_text = colorize(record.binding_id, "32", use_color)
                function_text = colorize(record.function_name, "33", use_color)
                location_text = colorize(f"L{record.line}", "2", use_color)
                lines.append(
                    f"    - {binding_text} -> {function_text} ({location_text})"
                )
                lines.append(f"      {record.summary_line}")
                if record.detail_line is not None:
                    lines.append(f"      {record.detail_line}")
                lines.append(f"      {colorize(metadata, '2', use_color)}")

    return "\n".join(lines) + "\n"


def validate_records(records: list[BindingRecord]) -> list[str]:
    """Validate binding records against canonical surface rules."""
    issues: list[str] = []

    # detect duplicate binding ids
    seen_ids: dict[str, BindingRecord] = {}
    for record in records:
        previous = seen_ids.get(record.binding_id)
        if previous is not None:
            issues.append(
                "duplicate binding id "
                f"{record.binding_id}: {previous.relative_file}:{previous.line} "
                f"and {record.relative_file}:{record.line}"
            )
        else:
            seen_ids[record.binding_id] = record

    for record in records:
        # binding id shape
        segments = record.binding_id.split(".")
        if len(segments) < 4:
            issues.append(
                f"{record.relative_file}:{record.line}: binding id must have >=4 segments: "
                f"{record.binding_id}"
            )
            continue
        if segments[0] != "destack":
            issues.append(
                f"{record.relative_file}:{record.line}: binding id must start with 'destack': "
                f"{record.binding_id}"
            )
        if segments[1] != record.module:
            issues.append(
                f"{record.relative_file}:{record.line}: binding module segment mismatch: "
                f"{record.binding_id} vs module {record.module}"
            )
        for segment in segments:
            if not SEGMENT_PATTERN.match(segment):
                issues.append(
                    f"{record.relative_file}:{record.line}: invalid binding id segment '{segment}' "
                    f"in {record.binding_id}"
                )

        # capability shape
        for capability in record.capabilities:
            capability_segments = capability.split(".")
            if len(capability_segments) < 2:
                issues.append(
                    f"{record.relative_file}:{record.line}: capability must have >=2 segments: "
                    f"{capability}"
                )
                continue
            for segment in capability_segments:
                if not SEGMENT_PATTERN.match(segment):
                    issues.append(
                        f"{record.relative_file}:{record.line}: invalid capability segment "
                        f"'{segment}' in {capability}"
                    )

        # platform tags
        for platform in record.platforms:
            if (
                platform not in CANONICAL_PLATFORM_TAGS
                and platform not in PLATFORM_SELECTOR_TAGS
            ):
                issues.append(
                    f"{record.relative_file}:{record.line}: unsupported platform tag '{platform}' "
                    f"in {record.binding_id}"
                )

    return issues


def main() -> int:
    """Run the binding overview command."""
    arguments = parse_arguments()
    root = Path(arguments.root)
    allowed_modules = set(arguments.module)
    use_color = (not arguments.no_color) and sys.stdout.isatty()

    if not root.exists():
        raise SystemExit(f"root does not exist: {root}")

    records = collect_records(root, allowed_modules)
    if arguments.check:
        issues = validate_records(records)
        if issues:
            print("Binding metadata validation failed:")
            for issue in issues:
                print(f" - {issue}")
            return 1
        print("Binding metadata validation passed.")
        return 0

    if arguments.json:
        output = json.dumps([asdict(record) for record in records], indent=2)
        print(output)
        return 0

    print(render_text(records, root, use_color), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
