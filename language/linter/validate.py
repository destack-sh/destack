#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

README_HEADER = "| Code | Rule | Source | Level | Ready | Status | Fixability | Description |"
CODE_PATTERN = re.compile(r"\bL[A-Z][0-9]{3}\b")
CATEGORY_BY_LETTER = {
    "C": "Correctness",
    "U": "Suspicious",
    "S": "Security",
    "P": "Performance",
    "Y": "Style",
    "X": "Complexity",
    "R": "Restriction",
}
ALLOWED_SUFFIXES = {".rs", ".md", ".ds", ".txt"}


@dataclass(frozen=True)
class RuleSpec:
    """Lint rule metadata parsed from README.

    Attributes:
        rule_id: The lint rule id.
        code: The lint code.
        category_letter: The category letter for this rule.
        line: The README line number for this rule.
    """

    rule_id: str
    code: str
    category_letter: str
    line: int


def parse_readme(path: Path) -> tuple[dict[str, RuleSpec], list[str]]:
    """Parse lint rule metadata from the README table."""
    lines = path.read_text().splitlines()
    errors: list[str] = []
    rules_by_id: dict[str, RuleSpec] = {}
    seen_codes: set[str] = set()
    category_letter: str | None = None
    in_table = False
    pending_separator = False

    # scan README tables for lint rows
    for line_number, line in enumerate(lines, start=1):
        heading_match = re.match(r"^##\s+.+\(([A-Z])\)\s*$", line)
        if heading_match:
            category_letter = heading_match.group(1)
            in_table = False
            pending_separator = False
            continue

        if category_letter and line.strip() == README_HEADER:
            in_table = True
            pending_separator = True
            continue

        if pending_separator and line.strip().startswith("|"):
            pending_separator = False
            continue

        if in_table:
            if line.strip().startswith("| `"):
                columns = split_markdown_row(line)
                if not columns:
                    errors.append(f"{path}:{line_number}: failed to parse row")
                    continue

                if len(columns) < 8:
                    errors.append(
                        f"{path}:{line_number}: expected 8 columns, got {len(columns)}"
                    )
                    continue

                if len(columns) > 8:
                    columns = columns[:7] + [" | ".join(columns[7:])]

                code = columns[0].strip("`")
                rule_id = columns[1].strip("`")

                # validate code format and category prefix
                if not CODE_PATTERN.fullmatch(code):
                    errors.append(f"{path}:{line_number}: invalid code '{code}'")
                elif category_letter and code[1] != category_letter:
                    errors.append(
                        f"{path}:{line_number}: code '{code}' does not match category {category_letter}"
                    )
                else:
                    try:
                        code_index = int(code[2:])
                    except ValueError:
                        code_index = -1
                    if code_index < 1:
                        errors.append(
                            f"{path}:{line_number}: code '{code}' must be >= {code[1]}001"
                        )

                # ensure codes are unique and sequential
                if code in seen_codes:
                    errors.append(f"{path}:{line_number}: duplicate code '{code}'")
                else:
                    seen_codes.add(code)

                # ensure rule ids are unique
                if rule_id in rules_by_id:
                    errors.append(f"{path}:{line_number}: duplicate rule id '{rule_id}'")
                else:
                    rules_by_id[rule_id] = RuleSpec(
                        rule_id=rule_id,
                        code=code,
                        category_letter=category_letter or "?",
                        line=line_number,
                    )
                continue

            if not line.strip().startswith("|"):
                in_table = False

    return rules_by_id, errors


def split_markdown_row(line: str) -> list[str] | None:
    """Split a markdown table row into columns."""
    if "|" not in line:
        return None

    # split on unescaped pipes
    content = line.strip().strip("|")
    columns: list[str] = []
    current: list[str] = []
    is_escaping = False

    for ch in content:
        if is_escaping:
            current.append(ch)
            is_escaping = False
            continue

        if ch == "\\":
            current.append(ch)
            is_escaping = True
            continue

        if ch == "|":
            columns.append("".join(current).strip())
            current = []
            continue

        current.append(ch)

    columns.append("".join(current).strip())
    return columns


def parse_rule_file(path: Path) -> tuple[str | None, str | None, str | None]:
    """Parse rule metadata from a rule file."""
    text = path.read_text()

    # skip non lint rule modules
    if "declare_lint!" not in text:
        return None, None, None

    # extract lint attribute values
    rule_id_match = re.search(r'\bid\s*=\s*"([^"]+)"', text)
    code_match = re.search(r'\bcode\s*=\s*"([^"]+)"', text)
    category_match = re.search(r"\bcategory\s*=\s*([A-Za-z]+)\b", text)

    rule_id = rule_id_match.group(1) if rule_id_match else None
    code = code_match.group(1) if code_match else None
    category = category_match.group(1) if category_match else None
    return rule_id, code, category


def validate_rule_files(
    rules_dir: Path, rules_by_id: dict[str, RuleSpec]
) -> list[str]:
    """Validate rule files against README metadata."""
    errors: list[str] = []

    # check each rule file for matching metadata
    for path in sorted(rules_dir.rglob("*.rs")):
        rule_id, code, category = parse_rule_file(path)
        if rule_id is None and code is None and category is None:
            continue

        if not rule_id or not code or not category:
            errors.append(f"{path}: missing id, code, or category")
            continue

        spec = rules_by_id.get(rule_id)
        if not spec:
            errors.append(f"{path}: rule id '{rule_id}' is missing from README")
            continue

        if code != spec.code:
            errors.append(
                f"{path}: code '{code}' does not match README '{spec.code}'"
            )

        expected_category = CATEGORY_BY_LETTER.get(spec.category_letter)
        if expected_category and category != expected_category:
            errors.append(
                f"{path}: category '{category}' does not match README '{expected_category}'"
            )

    return errors


def find_unknown_codes(root: Path, valid_codes: set[str]) -> list[str]:
    """Report lint codes referenced outside the README table."""
    errors: list[str] = []

    # scan codebase for lint codes not present in README
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue

        if {"target", "node_modules"} & set(path.parts):
            continue

        if path.suffix not in ALLOWED_SUFFIXES:
            continue

        text = path.read_text(errors="ignore")
        matches = {match.group(0) for match in CODE_PATTERN.finditer(text)}
        unknown = sorted(code for code in matches if code not in valid_codes)
        if unknown:
            errors.append(f"{path}: unknown codes {', '.join(unknown)}")

    return errors


def main() -> int:
    """Run lint code validation."""
    readme_path = Path(__file__).with_name("README.md")
    rules_dir = Path(__file__).parent / "src" / "rules"
    language_root = Path(__file__).resolve().parents[1]

    # parse README metadata
    rules_by_id, errors = parse_readme(readme_path)

    # validate rule files
    errors.extend(validate_rule_files(rules_dir, rules_by_id))

    # scan for stale code references
    valid_codes = {spec.code for spec in rules_by_id.values()}
    errors.extend(find_unknown_codes(language_root, valid_codes))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("lint code validation ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
