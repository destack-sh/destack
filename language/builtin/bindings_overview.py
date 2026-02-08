#!/usr/bin/env python3
"""Render a hierarchical overview of platform @binding declarations."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


FIELD_ORDER = ["effect", "replay", "payload", "scope", "blocking", "requires"]


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
    requires: list[str]


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


def parse_requires(block: str) -> list[str]:
    """Parse the requires array from one @binding block."""
    match = re.search(r"requires\s*:\s*\[([^\]]*)\]", block, re.DOTALL)
    if match is None:
        return []

    requires_body = match.group(1)
    requires = re.findall(r'"([^"]+)"', requires_body)
    return requires


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
                    requires=parse_requires(block),
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
        "requires": ",".join(record.requires) if record.requires else "-",
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


def main() -> int:
    """Run the binding overview command."""
    arguments = parse_arguments()
    root = Path(arguments.root)
    allowed_modules = set(arguments.module)
    use_color = (not arguments.no_color) and sys.stdout.isatty()

    if not root.exists():
        raise SystemExit(f"root does not exist: {root}")

    records = collect_records(root, allowed_modules)
    if arguments.json:
        output = json.dumps([asdict(record) for record in records], indent=2)
        print(output)
        return 0

    print(render_text(records, root, use_color), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
