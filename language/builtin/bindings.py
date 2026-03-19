#!/usr/bin/env python3
"""Render inventories of platform bindings and exported `.ds` API surface."""

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
    "affinity",
    "capabilities",
    "platforms",
]

PLATFORM_ENUM_PATTERN = re.compile(
    r"pub enum Platform \{(.*?)\n\}",
    re.DOTALL,
)
ENUM_VARIANT_PATTERN = re.compile(r"^\s*([A-Za-z][A-Za-z0-9]*)\s*,", re.MULTILINE)
STRUCT_PATTERN = re.compile(r"^\s*export\s+struct\s+([A-Za-z][A-Za-z0-9]*)\s*\{")
ENUM_PATTERN = re.compile(r"^\s*export\s+enum\s+([A-Za-z][A-Za-z0-9]*)\s*\{")
NEWTYPE_PATTERN = re.compile(r"^\s*export\s+newtype\s+([A-Za-z][A-Za-z0-9]*)\s*=\s*(.*)")
CONST_PATTERN = re.compile(
    r"^\s*export\s+const\s+([A-Za-z][A-Za-z0-9_]*)\s*:\s*(.*?)\s*=\s*(.*);\s*$"
)
FUNCTION_PATTERN = re.compile(r"^\s*export\s+function\s+([A-Za-z][A-Za-z0-9_]*)\s*\(")
FIELD_PATTERN = re.compile(r"^\s*([A-Za-z][A-Za-z0-9]*)\s*:\s*(.*?)\s*;\s*$")
VARIANT_PATTERN = re.compile(r"^\s*([A-Za-z][A-Za-z0-9]*)\s*=\s*(.*?),\s*$")
UNION_MEMBER_PATTERN = re.compile(r"^\s*\|\s*(.*?)\s*$")
ARRAY_MEMBER_PATTERN = re.compile(r"^\s*([A-Za-z][A-Za-z0-9_]*)\s*,\s*$")
SEGMENT_PATTERN = re.compile(r"^[a-z][a-zA-Z0-9]*$")


def load_canonical_platform_tags() -> set[str]:
    """Load canonical platform tags from workspace target enum variants."""
    root = Path(__file__).resolve().parents[1]
    target_config_path = (
        root / "workspace" / "src" / "config" / "target" / "execution.rs"
    )
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
    affinity: str | None
    capabilities: list[str]
    platforms: list[str]


@dataclass
class ApiMemberRecord:
    """One exported API member nested inside another item."""

    kind: str
    name: str
    line: int
    type_text: str | None


@dataclass
class ApiRecord:
    """One exported API item."""

    module: str
    relative_file: str
    line: int
    kind: str
    name: str
    summary_line: str
    detail_line: str | None
    signature: str | None
    members: list[ApiMemberRecord]


def parse_arguments() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Render platform binding metadata and exported API surface."
    )
    parser.add_argument(
        "--root",
        default="language/builtin/library/platform",
        help="source root directory",
    )
    parser.add_argument(
        "--module",
        action="append",
        default=[],
        help="restrict to one module directory, repeatable",
    )
    parser.add_argument(
        "--catalog",
        choices=["bindings", "api", "all"],
        default="bindings",
        help="which catalog to render",
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
        help="validate metadata and exit nonzero on violations",
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


def parse_string_array(block: str, key: str) -> list[str]:
    """Parse one string array option from one @binding block."""
    match = re.search(rf"{key}\s*:\s*\[([^\]]*)\]", block, re.DOTALL)
    if match is None:
        return []

    body = match.group(1)
    return re.findall(r'"([^"]+)"', body)


def parse_function_name(source: str, block_end: int) -> str:
    """Parse the function name that follows one @binding block."""
    tail = source[block_end : block_end + 500]
    match = re.search(r"export\s+function\s+([A-Za-z0-9_]+)\s*\(", tail)
    if match is None:
        return "<unknown>"
    return match.group(1)


def parse_doc_lines(lines: list[str], binding_line_index: int) -> tuple[str, str | None]:
    """Parse summary and top detail line from docs above one declaration."""
    cursor = binding_line_index - 1
    while cursor >= 0 and lines[cursor].strip() == "":
        cursor -= 1

    # skip attached attribute blocks above functions
    while cursor >= 0:
        stripped = lines[cursor].strip()
        if stripped.startswith("///"):
            break
        if stripped.startswith("@"):
            cursor -= 1
            continue
        if stripped in {"})", "}", "{", "],", "]", ")", "("}:
            cursor -= 1
            continue
        if stripped.endswith("{"):
            cursor -= 1
            continue
        if stripped.startswith(
            (
                "effect:",
                "replay:",
                "payload:",
                "scope:",
                "blocking:",
                "affinity:",
                "capabilities:",
                "platforms:",
                "simulation:",
            )
        ):
            cursor -= 1
            continue
        break

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

    documentation_lines.reverse()
    if not documentation_lines:
        return ("<missing summary>", None)

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
                    affinity=parse_option_value(block, "affinity"),
                    capabilities=parse_string_array(block, "capabilities"),
                    platforms=parse_string_array(block, "platforms"),
                )
            )

    return records


def collect_brace_block(lines: list[str], start_index: int) -> tuple[list[str], int]:
    """Collect one brace-delimited declaration block."""
    block_lines: list[str] = []
    depth = 0

    for index in range(start_index, len(lines)):
        line = lines[index]
        block_lines.append(line)
        depth += line.count("{")
        depth -= line.count("}")
        if depth == 0 and "}" in line:
            return (block_lines, index)

    raise ValueError(f"unterminated brace block at line {start_index + 1}")


def collect_statement(lines: list[str], start_index: int) -> tuple[list[str], int]:
    """Collect one semicolon-terminated declaration."""
    statement_lines: list[str] = []

    for index in range(start_index, len(lines)):
        line = lines[index]
        statement_lines.append(line)
        if ";" in line:
            return (statement_lines, index)

    raise ValueError(f"unterminated statement at line {start_index + 1}")


def compact_signature(lines: list[str]) -> str:
    """Compact one multiline declaration into one signature string."""
    parts = [line.strip() for line in lines if line.strip() != ""]
    return " ".join(parts)


def parse_struct_members(block_lines: list[str], start_line: int) -> list[ApiMemberRecord]:
    """Parse members from one struct declaration."""
    members: list[ApiMemberRecord] = []

    for offset, line in enumerate(block_lines[1:], start=1):
        match = FIELD_PATTERN.match(line)
        if match is None:
            continue

        members.append(
            ApiMemberRecord(
                kind="field",
                name=match.group(1),
                line=start_line + offset,
                type_text=match.group(2).strip(),
            )
        )

    return members


def parse_enum_members(block_lines: list[str], start_line: int) -> list[ApiMemberRecord]:
    """Parse members from one enum declaration."""
    members: list[ApiMemberRecord] = []

    for offset, line in enumerate(block_lines[1:], start=1):
        match = VARIANT_PATTERN.match(line)
        if match is None:
            continue

        members.append(
            ApiMemberRecord(
                kind="variant",
                name=match.group(1),
                line=start_line + offset,
                type_text=match.group(2).strip(),
            )
        )

    return members


def parse_newtype_members(statement_lines: list[str], start_line: int) -> list[ApiMemberRecord]:
    """Parse members from one newtype declaration."""
    members: list[ApiMemberRecord] = []
    for offset, line in enumerate(statement_lines, start=0):
        union_match = UNION_MEMBER_PATTERN.match(line)
        if union_match is not None:
            member_text = union_match.group(1).rstrip(";").strip()
            members.append(
                ApiMemberRecord(
                    kind="union_member",
                    name=member_text,
                    line=start_line + offset,
                    type_text=None,
                )
            )
            continue

        array_match = ARRAY_MEMBER_PATTERN.match(line)
        if array_match is not None and offset > 0:
            members.append(
                ApiMemberRecord(
                    kind="array_member",
                    name=array_match.group(1),
                    line=start_line + offset,
                    type_text=None,
                )
            )

    return members


def parse_function_signature(statement_lines: list[str]) -> str:
    """Parse one function declaration into one compact signature."""
    return compact_signature(statement_lines)


def collect_api_records(root: Path, allowed_modules: set[str]) -> list[ApiRecord]:
    """Collect exported API records from all .ds files under root."""
    records: list[ApiRecord] = []

    for path in sorted(root.rglob("*.ds")):
        relative = path.relative_to(root)
        if not relative.parts:
            continue

        module = relative.parts[0]
        if allowed_modules and module not in allowed_modules:
            continue

        lines = path.read_text(encoding="utf-8").splitlines()
        index = 0
        while index < len(lines):
            line = lines[index]

            struct_match = STRUCT_PATTERN.match(line)
            if struct_match is not None:
                block_lines, end_index = collect_brace_block(lines, index)
                summary_line, detail_line = parse_doc_lines(lines, index)
                records.append(
                    ApiRecord(
                        module=module,
                        relative_file=str(relative),
                        line=index + 1,
                        kind="struct",
                        name=struct_match.group(1),
                        summary_line=summary_line,
                        detail_line=detail_line,
                        signature=compact_signature([block_lines[0]]),
                        members=parse_struct_members(block_lines, index + 1),
                    )
                )
                index = end_index + 1
                continue

            enum_match = ENUM_PATTERN.match(line)
            if enum_match is not None:
                block_lines, end_index = collect_brace_block(lines, index)
                summary_line, detail_line = parse_doc_lines(lines, index)
                records.append(
                    ApiRecord(
                        module=module,
                        relative_file=str(relative),
                        line=index + 1,
                        kind="enum",
                        name=enum_match.group(1),
                        summary_line=summary_line,
                        detail_line=detail_line,
                        signature=compact_signature([block_lines[0]]),
                        members=parse_enum_members(block_lines, index + 1),
                    )
                )
                index = end_index + 1
                continue

            newtype_match = NEWTYPE_PATTERN.match(line)
            if newtype_match is not None:
                statement_lines, end_index = collect_statement(lines, index)
                summary_line, detail_line = parse_doc_lines(lines, index)
                records.append(
                    ApiRecord(
                        module=module,
                        relative_file=str(relative),
                        line=index + 1,
                        kind="newtype",
                        name=newtype_match.group(1),
                        summary_line=summary_line,
                        detail_line=detail_line,
                        signature=compact_signature(statement_lines),
                        members=parse_newtype_members(statement_lines, index + 1),
                    )
                )
                index = end_index + 1
                continue

            const_match = CONST_PATTERN.match(line)
            if const_match is not None:
                summary_line, detail_line = parse_doc_lines(lines, index)
                records.append(
                    ApiRecord(
                        module=module,
                        relative_file=str(relative),
                        line=index + 1,
                        kind="const",
                        name=const_match.group(1),
                        summary_line=summary_line,
                        detail_line=detail_line,
                        signature=compact_signature([line]),
                        members=[],
                    )
                )
                index += 1
                continue

            function_match = FUNCTION_PATTERN.match(line)
            if function_match is not None:
                statement_lines, end_index = collect_statement(lines, index)
                summary_line, detail_line = parse_doc_lines(lines, index)
                records.append(
                    ApiRecord(
                        module=module,
                        relative_file=str(relative),
                        line=index + 1,
                        kind="function",
                        name=function_match.group(1),
                        summary_line=summary_line,
                        detail_line=detail_line,
                        signature=parse_function_signature(statement_lines),
                        members=[],
                    )
                )
                index = end_index + 1
                continue

            index += 1

    return records


def colorize(text: str, code: str, use_color: bool) -> str:
    """Apply one ansi style sequence when colors are enabled."""
    if not use_color:
        return text
    return f"\x1b[{code}m{text}\x1b[0m"


def render_record_metadata(record: BindingRecord) -> str:
    """Render one binding metadata fragment."""
    metadata: dict[str, str] = {
        "effect": record.effect or "-",
        "replay": record.replay or "-",
        "payload": record.payload or "-",
        "scope": record.scope or "-",
        "blocking": record.blocking or "-",
        "affinity": record.affinity or "-",
        "capabilities": ",".join(record.capabilities) if record.capabilities else "-",
        "platforms": ",".join(record.platforms) if record.platforms else "-",
    }
    return " ".join(f"{field}={metadata[field]}" for field in FIELD_ORDER)


def render_text(records: list[BindingRecord], root: Path, use_color: bool) -> str:
    """Render binding records as a hierarchical text report."""
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


def render_api_text(records: list[ApiRecord], root: Path, use_color: bool) -> str:
    """Render exported API records as a hierarchical text report."""
    lines: list[str] = []
    lines.append(colorize("Platform API Surface", "1;36", use_color))
    lines.append(f"root: {root}")
    lines.append(f"items: {len(records)}")

    modules: dict[str, list[ApiRecord]] = {}
    for record in records:
        modules.setdefault(record.module, []).append(record)

    for module in sorted(modules.keys()):
        module_records = modules[module]
        lines.append("")
        lines.append(colorize(f"[{module}] ({len(module_records)})", "1;34", use_color))

        files: dict[str, list[ApiRecord]] = {}
        for record in module_records:
            files.setdefault(record.relative_file, []).append(record)

        for relative_file in sorted(files.keys()):
            file_records = sorted(files[relative_file], key=lambda item: item.line)
            lines.append(
                f"  {colorize(relative_file, '35', use_color)} "
                f"{colorize(f'[{len(file_records)}]', '2', use_color)}"
            )

            for record in file_records:
                kind_text = colorize(record.kind, "32", use_color)
                name_text = colorize(record.name, "33", use_color)
                location_text = colorize(f"L{record.line}", "2", use_color)
                member_count_text = colorize(
                    f"[{len(record.members)} members]",
                    "2",
                    use_color,
                )
                lines.append(
                    f"    - {kind_text} {name_text} ({location_text}) {member_count_text}"
                )
                lines.append(f"      {record.summary_line}")
                if record.detail_line is not None:
                    lines.append(f"      {record.detail_line}")
                if record.signature is not None:
                    lines.append(f"      {colorize(record.signature, '2', use_color)}")

    return "\n".join(lines) + "\n"


def validate_binding_records(records: list[BindingRecord]) -> list[str]:
    """Validate binding records against canonical surface rules."""
    issues: list[str] = []

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

        for platform in record.platforms:
            if (
                platform not in CANONICAL_PLATFORM_TAGS
                and platform not in PLATFORM_SELECTOR_TAGS
            ):
                issues.append(
                    f"{record.relative_file}:{record.line}: unsupported platform tag '{platform}' "
                    f"in {record.binding_id}"
                )

        if record.affinity is None:
            issues.append(
                f"{record.relative_file}:{record.line}: missing affinity classification in "
                f"{record.binding_id}"
            )
        elif record.affinity not in {"any", "eventLoop", "owner", "processMain"}:
            issues.append(
                f"{record.relative_file}:{record.line}: unsupported affinity "
                f"'{record.affinity}' in {record.binding_id}"
            )

        if record.summary_line == "<missing summary>":
            issues.append(
                f"{record.relative_file}:{record.line}: missing binding documentation for "
                f"{record.binding_id}"
            )

    return issues


def validate_api_records(records: list[ApiRecord]) -> list[str]:
    """Validate exported API records against basic surface rules."""
    issues: list[str] = []

    seen_items: dict[tuple[str, str, str], ApiRecord] = {}
    for record in records:
        key = (record.module, record.kind, record.name)
        previous = seen_items.get(key)
        if previous is not None:
            issues.append(
                "duplicate exported item "
                f"{record.kind} {record.name}: {previous.relative_file}:{previous.line} "
                f"and {record.relative_file}:{record.line}"
            )
        else:
            seen_items[key] = record

        if record.summary_line == "<missing summary>":
            issues.append(
                f"{record.relative_file}:{record.line}: missing documentation for "
                f"{record.kind} {record.name}"
            )

        if record.kind in {"struct", "enum"} and not record.members:
            issues.append(
                f"{record.relative_file}:{record.line}: no parsed members for "
                f"{record.kind} {record.name}"
            )

        if record.kind == "newtype" and record.signature is None:
            issues.append(
                f"{record.relative_file}:{record.line}: missing signature for newtype "
                f"{record.name}"
            )

        if record.kind == "function" and record.signature is None:
            issues.append(
                f"{record.relative_file}:{record.line}: missing signature for function "
                f"{record.name}"
            )

    return issues


def main() -> int:
    """Run the inventory command."""
    arguments = parse_arguments()
    root = Path(arguments.root)
    allowed_modules = set(arguments.module)
    use_color = (not arguments.no_color) and sys.stdout.isatty()

    if not root.exists():
        raise SystemExit(f"root does not exist: {root}")

    binding_records: list[BindingRecord] = []
    api_records: list[ApiRecord] = []

    if arguments.catalog in {"bindings", "all"}:
        binding_records = collect_records(root, allowed_modules)

    if arguments.catalog in {"api", "all"}:
        api_records = collect_api_records(root, allowed_modules)

    if arguments.check:
        issues: list[str] = []
        if binding_records:
            issues.extend(validate_binding_records(binding_records))
        if api_records:
            issues.extend(validate_api_records(api_records))

        if issues:
            print("Platform surface validation failed:")
            for issue in issues:
                print(f" - {issue}")
            return 1

        print("Platform surface validation passed.")
        return 0

    if arguments.json:
        if arguments.catalog == "bindings":
            print(json.dumps([asdict(record) for record in binding_records], indent=2))
            return 0

        if arguments.catalog == "api":
            print(json.dumps([asdict(record) for record in api_records], indent=2))
            return 0

        print(
            json.dumps(
                {
                    "bindings": [asdict(record) for record in binding_records],
                    "api": [asdict(record) for record in api_records],
                },
                indent=2,
            )
        )
        return 0

    if arguments.catalog == "bindings":
        print(render_text(binding_records, root, use_color), end="")
        return 0

    if arguments.catalog == "api":
        print(render_api_text(api_records, root, use_color), end="")
        return 0

    print(render_text(binding_records, root, use_color), end="")
    print()
    print(render_api_text(api_records, root, use_color), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
