#!/usr/bin/env python3

"""Sync transform fixture expected blocks with oxfmt output."""

from __future__ import annotations

import argparse
import re
import subprocess
import tempfile
from pathlib import Path


FENCE_PATTERN = re.compile(r"(?ms)^```([^\n]*)\n(.*?)\n```[ \t]*$")
SOURCE_LANGUAGES = {"js", "jsx", "ts", "tsx", "ds"}
FORMATTED_OUTPUT_PATTERN = re.compile(
    r"--- Formatted Code ---\n(.*?)\n--- End Formatted Code ---",
    re.S,
)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Update transform fixture expected blocks from oxfmt",
    )
    parser.add_argument(
        "--fixtures-root",
        type=Path,
        default=Path("language/test/fixtures/formatter/transform"),
        help="root directory containing transform markdown fixtures",
    )
    parser.add_argument(
        "--oxfmt-bin",
        type=Path,
        default=Path.home() / "symbol/oxc/target/debug/examples/formatter",
        help="path to oxfmt formatter example binary",
    )
    parser.add_argument(
        "--indent-width",
        type=int,
        default=4,
        help="indent width for expected output normalization",
    )
    parser.add_argument(
        "--line-width",
        type=int,
        default=100,
        help="print width passed to oxfmt when fixture blocks do not override it",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report changes without writing files",
    )
    return parser.parse_args()


def default_filename_for_language(language: str) -> str:
    if language in {"js"}:
        return "main.js"
    if language in {"jsx"}:
        return "main.jsx"
    if language in {"ts", "ds"}:
        return "main.ts"
    if language in {"tsx"}:
        return "main.tsx"
    return "main.ts"


def source_block_metadata(source_header: str) -> tuple[str, str, int | None] | None:
    parts = source_header.strip().split()
    if not parts:
        return None

    language_and_name = parts[0]
    if ":" in language_and_name:
        language, filename = language_and_name.split(":", 1)
    else:
        language = language_and_name
        filename = default_filename_for_language(language)
    if language not in SOURCE_LANGUAGES:
        return None

    line_width = None
    for part in parts[1:]:
        if part.startswith("line-width="):
            value = part[len("line-width=") :]
            if value.isdigit():
                line_width = int(value)
            break

    return language, filename, line_width


def source_block_should_skip(source_header: str) -> bool:
    return "organize-imports=" in source_header


def expected_header_language(expected_header: str) -> str | None:
    parts = expected_header.strip().split()
    if len(parts) != 2 or parts[1] != "expected":
        return None
    return parts[0]


def normalize_indent(text: str, indent_width: int) -> str:
    if indent_width == 2:
        return text

    lines = text.split("\n")
    normalized_lines: list[str] = []

    for line in lines:
        stripped = line.lstrip(" ")
        leading_spaces = len(line) - len(stripped)

        if leading_spaces == 0:
            normalized_lines.append(line)
            continue

        if leading_spaces % 2 != 0:
            normalized_lines.append(line)
            continue

        indent_level = leading_spaces // 2
        normalized_lines.append(" " * (indent_level * indent_width) + stripped)

    return "\n".join(normalized_lines)


def run_oxfmt(
    oxfmt_bin: Path,
    source_text: str,
    filename: str,
    line_width: int | None,
    default_line_width: int | None,
) -> str | None:
    if filename.endswith(".ds"):
        filename = filename[:-3] + ".ts"

    suffix = Path(filename).suffix
    if not suffix:
        return None

    with tempfile.TemporaryDirectory(prefix="destack-oxfmt-") as directory:
        source_path = Path(directory) / f"main{suffix}"
        source_path.write_text(source_text + "\n", encoding="utf-8")

        command = [str(oxfmt_bin)]
        effective_line_width = line_width
        if effective_line_width is None:
            effective_line_width = default_line_width
        if effective_line_width is not None:
            command.extend(["--print-width", str(effective_line_width)])
        command.append(str(source_path))

        result = subprocess.run(
            command,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            return None

        output = result.stdout
        if "Parsed with Errors." in output:
            return None

        match = FORMATTED_OUTPUT_PATTERN.search(output)
        if match is None:
            return None

        formatted = match.group(1)
        if formatted.endswith("\n"):
            formatted = formatted[:-1]

        return formatted


def update_fixture_file(
    fixture_path: Path,
    oxfmt_bin: Path,
    indent_width: int,
    default_line_width: int | None,
    dry_run: bool,
) -> tuple[int, int]:
    content = fixture_path.read_text(encoding="utf-8")
    matches = list(FENCE_PATTERN.finditer(content))
    replacements: list[tuple[int, int, str]] = []

    updated_cases = 0
    skipped_cases = 0

    for index, source_match in enumerate(matches[:-1]):
        source_header = source_match.group(1)
        if source_block_should_skip(source_header):
            continue

        source_metadata = source_block_metadata(source_header)
        if source_metadata is None:
            continue

        source_language, source_filename, source_line_width = source_metadata
        expected_match = matches[index + 1]
        expected_header = expected_match.group(1)
        expected_language = expected_header_language(expected_header)
        if expected_language is None or expected_language != source_language:
            continue

        source_body = source_match.group(2)
        expected_body = expected_match.group(2)

        formatted_body = run_oxfmt(
            oxfmt_bin=oxfmt_bin,
            source_text=source_body,
            filename=source_filename,
            line_width=source_line_width,
            default_line_width=default_line_width,
        )
        if formatted_body is None:
            skipped_cases += 1
            continue

        normalized_body = normalize_indent(formatted_body, indent_width)
        if normalized_body == expected_body:
            continue

        expected_body_start, expected_body_end = expected_match.span(2)
        replacements.append((expected_body_start, expected_body_end, normalized_body))
        updated_cases += 1

    if not replacements:
        return (updated_cases, skipped_cases)

    next_content = content
    for start, end, replacement in sorted(replacements, key=lambda item: item[0], reverse=True):
        next_content = next_content[:start] + replacement + next_content[end:]

    if not dry_run:
        fixture_path.write_text(next_content, encoding="utf-8")

    return (updated_cases, skipped_cases)


def main() -> int:
    arguments = parse_arguments()

    if not arguments.oxfmt_bin.exists():
        print(f"error: oxfmt binary not found at {arguments.oxfmt_bin}")
        return 1

    fixture_files = sorted(arguments.fixtures_root.rglob("*.md"))
    total_updated_cases = 0
    total_skipped_cases = 0
    files_with_changes = 0

    for fixture_path in fixture_files:
        updated_cases, skipped_cases = update_fixture_file(
            fixture_path=fixture_path,
            oxfmt_bin=arguments.oxfmt_bin,
            indent_width=arguments.indent_width,
            default_line_width=arguments.line_width,
            dry_run=arguments.dry_run,
        )

        total_updated_cases += updated_cases
        total_skipped_cases += skipped_cases

        if updated_cases > 0:
            files_with_changes += 1
            print(f"updated {fixture_path}: {updated_cases} case(s)")

    print(
        "summary: "
        f"{files_with_changes} files changed, "
        f"{total_updated_cases} cases updated, "
        f"{total_skipped_cases} cases skipped",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
