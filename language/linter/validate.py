#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

README_HEADER = "| Code | Rule | Source | Level | Status | Fixability | Description |"
CODE_PATTERN = re.compile(r"\bL([A-Z])([0-9]{3})\b")
CATEGORY_BY_LETTER = {
    "C": "Correctness",
    "U": "Suspicious",
    "S": "Security",
    "P": "Performance",
    "Y": "Style",
    "X": "Complexity",
    "R": "Restriction",
}
LETTER_BY_CATEGORY = {value: key for key, value in CATEGORY_BY_LETTER.items()}
VALID_LEVELS = {"AST", "DIR", "MIR"}
VALID_FIXABILITY = {"None", "Safe", "Unsafe", "Suggestion", "Always"}
ALLOWED_SUFFIXES = {".rs", ".md", ".ds", ".txt"}


@dataclass(frozen=True)
class RuleSpec:
    """Lint rule metadata parsed from README."""

    rule_id: str
    code: str
    category_letter: str
    level: str
    status: str
    fixability: str
    line: int


@dataclass(frozen=True)
class RuleFileSpec:
    """Lint rule metadata parsed from a rule file."""

    rule_id: str
    code: str
    category: str
    level: str
    fixable: str


def parse_readme(path: Path) -> tuple[dict[str, RuleSpec], list[str]]:
    """Parse lint rule metadata from the README tables."""
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

        if not in_table:
            continue

        if not line.strip().startswith("|"):
            in_table = False
            continue

        if not line.strip().startswith("| `L"):
            continue

        columns = split_markdown_row(line)
        if not columns:
            errors.append(f"{path}:{line_number}: failed to parse row")
            continue

        if len(columns) < 7:
            errors.append(f"{path}:{line_number}: expected 7 columns, got {len(columns)}")
            continue

        if len(columns) > 7:
            columns = columns[:6] + [" | ".join(columns[6:])]

        code = columns[0].strip("`")
        rule_id = columns[1].strip("`")
        level = columns[3].strip().upper()
        status = columns[4].strip()
        fixability = columns[5].strip()

        # validate code format and category prefix
        code_match = CODE_PATTERN.fullmatch(code)
        if not code_match:
            errors.append(f"{path}:{line_number}: invalid code '{code}'")
        else:
            code_letter = code_match.group(1)
            code_index = int(code_match.group(2))

            if category_letter and code_letter != category_letter:
                errors.append(
                    f"{path}:{line_number}: code '{code}' does not match category {category_letter}"
                )
            if code_index < 1:
                errors.append(
                    f"{path}:{line_number}: code '{code}' must be >= L{code_letter}001"
                )

        # ensure codes are unique
        if code in seen_codes:
            errors.append(f"{path}:{line_number}: duplicate code '{code}'")
        else:
            seen_codes.add(code)

        # ensure rule ids are unique
        if rule_id in rules_by_id:
            errors.append(f"{path}:{line_number}: duplicate rule id '{rule_id}'")
            continue

        if level not in VALID_LEVELS:
            errors.append(
                f"{path}:{line_number}: invalid level '{columns[3].strip()}', expected AST, DIR, or MIR"
            )

        if status not in {"", "✓"}:
            errors.append(
                f"{path}:{line_number}: invalid status '{status}', expected blank or ✓"
            )

        if fixability not in VALID_FIXABILITY:
            errors.append(
                f"{path}:{line_number}: invalid fixability '{fixability}', expected one of {sorted(VALID_FIXABILITY)}"
            )

        rules_by_id[rule_id] = RuleSpec(
            rule_id=rule_id,
            code=code,
            category_letter=category_letter or "?",
            level=level,
            status=status,
            fixability=fixability,
            line=line_number,
        )

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

    for char in content:
        if is_escaping:
            current.append(char)
            is_escaping = False
            continue

        if char == "\\":
            current.append(char)
            is_escaping = True
            continue

        if char == "|":
            columns.append("".join(current).strip())
            current = []
            continue

        current.append(char)

    columns.append("".join(current).strip())
    return columns


def parse_rule_file(path: Path) -> RuleFileSpec | None:
    """Parse lint metadata from a rule file."""
    text = path.read_text()

    # skip non lint rule modules
    if "declare_lint!" not in text:
        return None

    rule_id_match = re.search(r'\bid\s*=\s*"([^"]+)"', text)
    code_match = re.search(r'\bcode\s*=\s*"([^"]+)"', text)
    category_match = re.search(r"\bcategory\s*=\s*([A-Za-z]+)\b", text)
    level_match = re.search(r"\blevel\s*=\s*([A-Za-z]+)\b", text)
    fixable_match = re.search(r"\bfixable\s*=\s*([A-Za-z]+)\b", text)

    if not (rule_id_match and code_match and category_match and level_match and fixable_match):
        return None

    return RuleFileSpec(
        rule_id=rule_id_match.group(1),
        code=code_match.group(1),
        category=category_match.group(1),
        level=level_match.group(1).upper(),
        fixable=fixable_match.group(1),
    )


def validate_rule_files(
    rules_dir: Path, rules_by_id: dict[str, RuleSpec], readme_path: Path
) -> list[str]:
    """Validate rule files against README metadata."""
    errors: list[str] = []
    seen_rule_ids: set[str] = set()

    # check each rule file for matching metadata
    for path in sorted(rules_dir.rglob("*.rs")):
        spec = parse_rule_file(path)
        if spec is None:
            continue

        seen_rule_ids.add(spec.rule_id)
        readme_rule = rules_by_id.get(spec.rule_id)
        if not readme_rule:
            errors.append(f"{path}: rule id '{spec.rule_id}' is missing from README")
            continue

        if spec.code != readme_rule.code:
            errors.append(
                f"{path}: code '{spec.code}' does not match README '{readme_rule.code}'"
            )

        expected_category = LETTER_BY_CATEGORY.get(spec.category)
        if expected_category and expected_category != readme_rule.category_letter:
            errors.append(
                f"{path}: category '{spec.category}' does not match README category '{readme_rule.category_letter}'"
            )

        if spec.level != readme_rule.level:
            errors.append(
                f"{path}: level '{spec.level}' does not match README level '{readme_rule.level}'"
            )

        if readme_rule.status != "✓":
            errors.append(
                f"{path}: README row for '{spec.rule_id}' must have status '✓' ({readme_path}:{readme_rule.line})"
            )

        # validate fixability presence only: No => None, otherwise non None
        if spec.fixable == "No" and readme_rule.fixability != "None":
            errors.append(
                f"{path}: fixable=No requires README fixability None, got '{readme_rule.fixability}'"
            )
        if spec.fixable != "No" and readme_rule.fixability == "None":
            errors.append(
                f"{path}: fixable={spec.fixable} requires non None README fixability"
            )

    # any row marked implemented must exist on disk
    for rule_id, readme_rule in rules_by_id.items():
        if readme_rule.status == "✓" and rule_id not in seen_rule_ids:
            errors.append(
                f"{readme_path}:{readme_rule.line}: status is ✓ but rule '{rule_id}' has no implementation file"
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
    """Run lint metadata validation."""
    readme_path = Path(__file__).with_name("README.md")
    rules_dir = Path(__file__).parent / "src" / "rules"
    language_root = Path(__file__).resolve().parents[1]

    # parse README metadata
    rules_by_id, errors = parse_readme(readme_path)

    # validate rule files
    errors.extend(validate_rule_files(rules_dir, rules_by_id, readme_path))

    # scan for stale code references
    valid_codes = {spec.code for spec in rules_by_id.values()}
    errors.extend(find_unknown_codes(language_root, valid_codes))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("lint metadata validation ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
