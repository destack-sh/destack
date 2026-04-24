#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from typing import Any
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[5]
FIXTURE_ROOT = REPO_ROOT / "language/test/fixtures/formatter"
REFERENCE_REPO = Path.home() / "symbol/oxc"
REFERENCE_TARGET_DIR = REFERENCE_REPO / "target-no-napi"
DEFAULT_REFERENCE_BINARY = REFERENCE_TARGET_DIR / "debug/oxfmt"
DEFAULT_LINE_WIDTH = 100
DEFAULT_INDENT_WIDTH = 4

LOCAL_OR_EXTENDED_PATTERNS: tuple[tuple[str, str], ...] = (
    (r"\bcomptime\b", "comptime"),
    (r"\bextension\b", "extension"),
    (r"\bstruct\b", "struct"),
    (r"\bnewtype\b", "newtype"),
    (r"\bmatch\b", "match"),
    (r"\bif let\b", "if-let"),
    (r"\bawait\?", "await-maybe"),
    (r"\bloop\s*\{", "loop"),
    (r"\btry\s+[A-Za-z_(]", "try-expression"),
    (r"\bas comptime\b", "as-comptime"),
    (r"[+*/%-][|%]=", "extended-assignment-operator"),
    (r"\bbreak\s*:[_$A-Za-z]", "colon-label"),
    (r"\bcontinue\s*:[_$A-Za-z]", "colon-label"),
    (r"<[^>\n]*:\s*[_$A-Za-z]", "colon-type-constraint"),
    (r"^\s*@[_$A-Za-z][\w$]*(?:\([^)]*\))?\s*\n(?:@[_$A-Za-z][\w$]*(?:\([^)]*\))?\s*\n)*(?:async\s+)?function\b", "function-decorator"),
    (r"=\s*&[_$A-Za-z]", "reference-expression"),
    (r"=\s*&readonly\s+[_$A-Za-z]", "readonly-reference-expression"),
    (r"=\s*\^[_$A-Za-z]", "owned-reference-expression"),
    (r"=\s*\*[_$A-Za-z]", "pointer-expression"),
    (r"type\s+[_$A-Za-z][\w$]*\s*=\s*&[_$A-Za-z]", "reference-type"),
    (r"type\s+[_$A-Za-z][\w$]*\s*=\s*&readonly\s+[_$A-Za-z]", "readonly-reference-type"),
    (r"type\s+[_$A-Za-z][\w$]*\s*=\s*\^[_$A-Za-z]", "owned-reference-type"),
    (r"type\s+[_$A-Za-z][\w$]*\s*=\s*\([^)]*,[^)]*\)", "paren-tuple-type"),
    (r":\s*@[_$A-Za-z]", "decorated-type-annotation"),
    (r"[(,]\s*@[_$A-Za-z]", "annotated-parameter"),
    (r"(?<!\.)\bwhere\b", "where-clause"),
)

REFERENCE_INVALID_INPUT_PATTERNS: tuple[str, ...] = (
    "must be last in a parameter list",
    "This syntax is reserved in files with the .mts or .cts extension",
    "Missing initializer in const declaration",
    "Cannot assign to this expression",
    "cannot be followed by a property access",
    "only allowed within async functions",
    "cannot be used with",
)

REVIEWED_LOCAL_CASES: frozenset[str] = frozenset(
    {
        "transform/declarations/classes.md::Decorators::decorated field",
        "transform/declarations/classes.md::Decorators::decorated method",
        "transform/declarations/types-advanced.md::Ownership Types::borrowed reference type",
        "transform/expressions/match.md::Match Expressions::match with annotated arm",
        "transform/expressions/match.md::Switch Expressions::basic switch expression",
        "transform/expressions/match.md::Switch Expressions::switch with default case",
        "transform/expressions/match.md::Switch Expressions::switch with string patterns",
        "transform/expressions/match.md::Syntax Preservation::switch preserves switch keyword at statement level",
        "transform/imports/specifiers.md::basic sorting::alphabetical order",
        "transform/imports/specifiers.md::basic sorting::with default import",
        "transform/imports/specifiers.md::basic sorting::natural sort order",
        "transform/imports/specifiers.md::type imports::type before value",
        "transform/imports/specifiers.md::type imports::mixed with default",
        "transform/imports/specifiers.md::exports::export specifier sorting",
        "transform/imports/specifiers.md::exports::export with type specifiers",
        "transform/imports/specifiers.md::aliases::sort by alias name",
        "transform/imports/statements.md::group ordering::builtin imports first",
        "transform/imports/statements.md::group ordering::multiple runtime protocols",
        "transform/imports/statements.md::group ordering::packages before relative",
        "transform/imports/statements.md::group ordering::aliases before relative",
        "transform/imports/statements.md::alphabetical within groups::packages sorted",
        "transform/imports/statements.md::alphabetical within groups::relative imports sorted",
        "transform/imports/statements.md::side-effect imports::side-effects stay at top",
        "transform/imports/statements.md::scoped packages::scoped packages as packages",
        "transform/imports/statements.md::full example::comprehensive import sorting",
        "smoke/js-idempotent-annotations",
    }
)


@dataclass
class FixtureCase:
    kind: str
    name: str
    source_path: Path
    section_name: str | None
    case_title: str | None
    input_text: str
    expected_text: str
    line_width: int
    indent_width: int
    options: dict[str, str]
    file_candidates: list[str]
    reference_options: dict[str, Any]


@dataclass
class AuditResult:
    case: FixtureCase
    outcome: str
    surface: str
    markers: list[str]
    file_name: str | None
    actual: str | None
    detail: str | None


def normalize_output(text: str) -> str:
    lines = [line.rstrip() for line in text.splitlines()]
    result = "\n".join(lines)
    if result and not result.endswith("\n"):
        result += "\n"
    return result


def parse_fence_info(info: str) -> tuple[str | None, str | None, dict[str, str], bool]:
    info = info.strip()
    if not info:
        return None, None, {}, False

    parts = info.split()
    first = parts[0]
    language, _, file_name = first.partition(":")
    file_name = file_name or None
    options: dict[str, str] = {}
    is_expected = False

    for part in parts[1:]:
        if part == "expected":
            is_expected = True
            continue

        key, separator, value = part.partition("=")
        if separator:
            options[key] = value

    return language, file_name, options, is_expected


def parse_bool_option(value: str | bool | None) -> bool | None:
    if isinstance(value, bool):
        return value

    if value is None:
        return None

    if value == "true":
        return True
    if value == "false":
        return False

    return None


def build_reference_options(
    *,
    line_width: int,
    indent_width: int,
    options: dict[str, str | int | bool],
) -> dict[str, Any]:
    reference_options: dict[str, Any] = {
        "printWidth": line_width,
        "tabWidth": indent_width,
    }

    quote_style = options.get("quote-style")
    if quote_style == "single":
        reference_options["singleQuote"] = True
    elif quote_style == "double":
        reference_options["singleQuote"] = False

    quote_props = options.get("quote-props")
    if quote_props in {"as-needed", "consistent", "preserve"}:
        reference_options["quoteProps"] = quote_props

    bracket_same_line = parse_bool_option(options.get("bracket-same-line"))
    if bracket_same_line is not None:
        reference_options["bracketSameLine"] = bracket_same_line

    single_attribute_per_line = parse_bool_option(options.get("single-attribute-per-line"))
    if single_attribute_per_line is not None:
        reference_options["singleAttributePerLine"] = single_attribute_per_line

    return reference_options


def detect_local_or_extended_markers(case: FixtureCase) -> list[str]:
    markers: list[str] = []
    source_name = case.source_path.relative_to(FIXTURE_ROOT).as_posix()

    # path level local surfaces
    if "/declarations/extensions.md" in source_name:
        markers.append("extensions-fixture")
    if "/declarations/structs.md" in source_name:
        markers.append("structs-fixture")
    if "/literals/structs.md" in source_name:
        markers.append("struct-literal-fixture")
    if "/declarations/newtypes.md" in source_name:
        markers.append("newtypes-fixture")
    if "/expressions/comptime.md" in source_name:
        markers.append("comptime-fixture")
    if "/expressions/match.md" in source_name or "/statements/match.md" in source_name:
        markers.append("match-fixture")
    if "/literals/tuples.md" in source_name:
        markers.append("tuple-fixture")

    # option level local surfaces
    if case.options.get("organize-imports") == "on":
        markers.append("organize-imports")

    # case level local surfaces
    if case.case_title == "if as expression":
        markers.append("if-expression")
    if case.case_title == "annotated for loop":
        markers.append("decorated-statement")
    if case.case_title in {
        "await inside maybe preserves parentheses",
        "unary inside maybe preserves parentheses",
        "postfix inside maybe needs no extra parentheses",
        "call inside maybe needs no parentheses",
        "binary inside maybe preserves parentheses",
        "ternary inside maybe preserves parentheses",
    }:
        markers.append("maybe-operator")
    if case.case_title in {
        "wrapping operators stay spaced",
        "saturating operators stay spaced",
    }:
        markers.append("extended-binary-operator")
    if case.case_title == "member assignment through as assertion target":
        markers.append("assertion-target-assignment")
    if case.case_title in {
        "chain with instantiation stays inline",
        "chain with instantiation breaks cleanly",
    }:
        markers.append("instantiation-member-chain")
    if "/declarations/classes.md" in source_name and case.case_title in {
        "decorated field",
        "decorated method",
    }:
        markers.append("body-annotation-policy")
    if "/declarations/enums.md" in source_name and case.section_name == "Decorators":
        markers.append("enum-decorators")

    # syntax markers
    for pattern, marker in LOCAL_OR_EXTENDED_PATTERNS:
        if re.search(pattern, case.input_text):
            markers.append(marker)

    # enum members use commas in shared TS, our fixtures do not
    if re.search(r"\benum\b[^{]*\{[^}]*;[^}]*\}", case.input_text, re.DOTALL):
        markers.append("enum-semicolons")

    return sorted(set(markers))


def is_reference_invalid_input(detail: str) -> bool:
    return any(pattern in detail for pattern in REFERENCE_INVALID_INPUT_PATTERNS)


def ensure_reference_binary(reference_binary: Path) -> bool:
    if reference_binary.exists():
        return True

    build_environment = dict(os.environ)
    build_environment["CARGO_TARGET_DIR"] = str(REFERENCE_TARGET_DIR)
    result = subprocess.run(
        [
            "cargo",
            "build",
            "-q",
            "-p",
            "oxfmt",
            "--no-default-features",
        ],
        cwd=REFERENCE_REPO,
        env=build_environment,
        capture_output=True,
        text=True,
    )

    return result.returncode == 0 and reference_binary.exists()


def explicit_reference_candidates(
    language: str | None,
    file_name: str | None,
    content: str,
) -> list[str]:
    if file_name is not None:
        return [file_name]

    if language in {"js", "javascript"}:
        return ["reference.js"]
    if language == "jsx":
        return ["reference.jsx"]
    if language in {"ts", "typescript"}:
        return ["reference.ts"]
    if language == "tsx":
        return ["reference.tsx"]

    # ds fixtures often still contain shared JS or TS syntax, so try the
    # plausible reference extensions before classifying the case as local only
    if language == "ds" or language is None:
        if "<" in content and ">" in content:
            return ["reference.tsx", "reference.jsx", "reference.ts", "reference.js"]

        return ["reference.ts", "reference.js", "reference.tsx", "reference.jsx"]

    return []


def run_reference_formatter(
    reference_binary: Path,
    input_text: str,
    file_name: str,
    reference_options: dict[str, Any],
) -> tuple[bool, str | None, str | None]:
    with tempfile.TemporaryDirectory() as temporary_directory:
        temporary_root = Path(temporary_directory)
        input_path = temporary_root / file_name
        config_path = temporary_root / ".oxfmtrc.json"

        input_path.parent.mkdir(parents=True, exist_ok=True)
        input_path.write_text(input_text)
        config_path.write_text(json.dumps(reference_options))

        result = subprocess.run(
            [
                str(reference_binary),
                "--config",
                str(config_path),
                str(input_path),
            ],
            capture_output=True,
            text=True,
        )
        output = input_path.read_text()

    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or None
        return False, None, detail

    return True, normalize_output(output), None


def parse_transform_cases() -> list[FixtureCase]:
    cases: list[FixtureCase] = []

    for path in sorted((FIXTURE_ROOT / "transform").rglob("*.md")):
        lines = path.read_text().splitlines()
        current_section = ""
        current_case = ""
        current_blocks: list[tuple[str, str]] = []
        current_paragraphs: list[str] = []
        in_fence = False
        fence_info = ""
        fence_lines: list[str] = []

        def flush_case() -> None:
            nonlocal current_blocks, current_paragraphs
            if not current_case:
                current_blocks = []
                current_paragraphs = []
                return

            input_block: tuple[str, str] | None = None
            expected_block: tuple[str, str] | None = None

            for info, content in current_blocks:
                language, file_name, options, is_expected = parse_fence_info(info)
                if is_expected and expected_block is None:
                    expected_block = (info, content)
                    continue

                if not is_expected and input_block is None:
                    input_block = (info, content)

            if input_block is None or expected_block is None:
                current_blocks = []
                current_paragraphs = []
                return

            input_language, input_file_name, input_options, _ = parse_fence_info(input_block[0])
            line_width = int(input_options.get("line-width", DEFAULT_LINE_WIDTH))
            indent_width = int(input_options.get("indent-width", DEFAULT_INDENT_WIDTH))
            file_candidates = explicit_reference_candidates(
                input_language,
                input_file_name,
                input_block[1],
            )

            relative_name = path.relative_to(FIXTURE_ROOT).as_posix()
            case_name = f"{relative_name}::{current_section}::{current_case}"

            cases.append(
                FixtureCase(
                    kind="transform",
                    name=case_name,
                    source_path=path,
                    section_name=current_section,
                    case_title=current_case,
                    input_text=input_block[1],
                    expected_text=normalize_output(expected_block[1]),
                    line_width=line_width,
                    indent_width=indent_width,
                    options=input_options,
                    file_candidates=file_candidates,
                    reference_options=build_reference_options(
                        line_width=line_width,
                        indent_width=indent_width,
                        options=input_options,
                    ),
                )
            )

            current_blocks = []
            current_paragraphs = []

        for line in lines:
            if line.startswith("```"):
                if not in_fence:
                    in_fence = True
                    fence_info = line[3:].strip()
                    fence_lines = []
                else:
                    in_fence = False
                    current_blocks.append((fence_info, "\n".join(fence_lines) + "\n"))
                continue

            if in_fence:
                fence_lines.append(line)
                continue

            if line.startswith("## "):
                flush_case()
                current_section = line[3:].strip()
                current_case = ""
                continue

            if line.startswith("### "):
                flush_case()
                current_case = line[4:].strip()
                continue

        flush_case()

    return cases


def parse_smoke_cases() -> list[FixtureCase]:
    cases: list[FixtureCase] = []

    for input_path in sorted((FIXTURE_ROOT / "smoke").rglob("input.*")):
        extension = input_path.suffix
        if extension not in {".js", ".jsx", ".ts", ".tsx", ".mts", ".cts"}:
            continue

        directory = input_path.parent
        expected_path = directory / f"expected{extension}"
        expected_text = (
            expected_path.read_text() if expected_path.exists() else input_path.read_text()
        )

        line_width = DEFAULT_LINE_WIDTH
        indent_width = DEFAULT_INDENT_WIDTH
        options_path = directory / "formatter.toml"
        if options_path.exists():
            options = tomllib.loads(options_path.read_text())
            line_width = int(options.get("line-width", line_width))
            indent_width = int(options.get("indent-width", indent_width))
        else:
            options = {}

        relative_name = directory.relative_to(FIXTURE_ROOT).as_posix()
        cases.append(
            FixtureCase(
                kind="smoke",
                name=relative_name,
                source_path=input_path,
                section_name=None,
                case_title=None,
                input_text=input_path.read_text(),
                expected_text=normalize_output(expected_text),
                line_width=line_width,
                indent_width=indent_width,
                options=options,
                file_candidates=[f"reference{extension}"],
                reference_options=build_reference_options(
                    line_width=line_width,
                    indent_width=indent_width,
                    options=options,
                ),
            )
        )

    return cases


def parse_roundtrip_cases() -> list[FixtureCase]:
    cases: list[FixtureCase] = []

    for path in sorted((FIXTURE_ROOT / "roundtrip").iterdir()):
        if not path.is_file():
            continue

        extension = path.suffix
        if extension not in {".js", ".jsx", ".ts", ".tsx", ".ds"} and not path.name.endswith(".d.ts"):
            continue

        content = path.read_text()
        file_candidates: list[str]

        if path.name.endswith(".d.ts"):
            file_candidates = ["reference.d.ts"]
        elif extension == ".ds":
            file_candidates = explicit_reference_candidates("ds", None, content)
        else:
            file_candidates = [f"reference{extension}"]

        cases.append(
            FixtureCase(
                kind="roundtrip",
                name=path.relative_to(FIXTURE_ROOT).as_posix(),
                source_path=path,
                section_name=None,
                case_title=None,
                input_text=content,
                expected_text=normalize_output(content),
                line_width=DEFAULT_LINE_WIDTH,
                indent_width=DEFAULT_INDENT_WIDTH,
                options={},
                file_candidates=file_candidates,
                reference_options=build_reference_options(
                    line_width=DEFAULT_LINE_WIDTH,
                    indent_width=DEFAULT_INDENT_WIDTH,
                    options={},
                ),
            )
        )

    return cases


def audit_case(reference_binary: Path, case: FixtureCase) -> AuditResult:
    markers = detect_local_or_extended_markers(case)
    surface = "local_or_extended" if markers else "shared_candidate"
    candidate_failures: list[str] = []

    if not case.file_candidates:
        return AuditResult(
            case,
            "unsupported_fixture",
            surface,
            markers,
            None,
            None,
            "no reference candidates",
        )

    for file_name in case.file_candidates:
        is_supported, actual, detail = run_reference_formatter(
            reference_binary,
            case.input_text,
            file_name,
            case.reference_options,
        )

        if not is_supported:
            if detail is not None:
                candidate_failures.append(f"{file_name}: {detail}")
            continue

        expected = case.expected_text
        if actual == expected:
            return AuditResult(case, "exact_match", surface, markers, file_name, actual, None)

        if markers:
            if case.name in REVIEWED_LOCAL_CASES:
                return AuditResult(
                    case,
                    "local_or_extended_surface",
                    surface,
                    markers,
                    file_name,
                    actual,
                    "reviewed local truth",
                )

            return AuditResult(case, "local_needs_rule", surface, markers, file_name, actual, None)

        return AuditResult(case, "mismatch", surface, markers, file_name, actual, None)

    failure_detail = None
    if candidate_failures:
        failure_detail = "\n".join(candidate_failures)

    if markers:
        return AuditResult(
            case,
            "local_or_extended_surface",
            surface,
            markers,
            None,
            None,
            failure_detail or "reference formatter could not parse local or extended syntax surface",
        )

    if failure_detail is not None and is_reference_invalid_input(failure_detail):
        return AuditResult(
            case,
            "reference_invalid_input",
            surface,
            markers,
            None,
            None,
            failure_detail,
        )

    return AuditResult(
        case,
        "reference_parse_failure",
        surface,
        markers,
        None,
        None,
        failure_detail or "reference formatter could not parse any candidate extension",
    )


def replace_transform_expected_block(
    path: Path,
    section_name: str,
    case_title: str,
    expected_text: str,
) -> None:
    lines = path.read_text().splitlines()
    current_section = ""
    current_case = ""
    in_fence = False

    for index, line in enumerate(lines):
        if line.startswith("```"):
            if not in_fence:
                in_fence = True
                language, _, _, is_expected = parse_fence_info(line[3:].strip())
                if (
                    current_section == section_name
                    and current_case == case_title
                    and is_expected
                    and language is not None
                ):
                    end_index = index + 1
                    while end_index < len(lines) and lines[end_index] != "```":
                        end_index += 1

                    replacement = expected_text.rstrip("\n").splitlines()
                    lines[index + 1 : end_index] = replacement
                    path.write_text("\n".join(lines) + "\n")
                    return
            else:
                in_fence = False

            continue

        if in_fence:
            continue

        if line.startswith("## "):
            current_section = line[3:].strip()
            current_case = ""
            continue

        if line.startswith("### "):
            current_case = line[4:].strip()

    raise ValueError(f"failed to find expected block for {path}::{section_name}::{case_title}")


def write_shared_mismatches(results: list[AuditResult]) -> int:
    updated_case_count = 0

    for result in results:
        if result.outcome != "mismatch" or result.surface != "shared_candidate":
            continue

        if result.actual is None:
            continue

        case = result.case

        if case.kind == "transform":
            if case.section_name is None or case.case_title is None:
                raise ValueError(f"transform case metadata missing for {case.name}")

            replace_transform_expected_block(
                case.source_path,
                case.section_name,
                case.case_title,
                result.actual,
            )
            updated_case_count += 1
            continue

        if case.kind == "smoke":
            expected_path = case.source_path.parent / f"expected{case.source_path.suffix}"
            expected_path.write_text(result.actual)
            updated_case_count += 1
            continue

        if case.kind == "roundtrip":
            case.source_path.write_text(result.actual)
            updated_case_count += 1
            continue

        raise ValueError(f"unsupported fixture kind: {case.kind}")

    return updated_case_count


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--reference-binary",
        type=Path,
        default=DEFAULT_REFERENCE_BINARY,
        help="path to the reference formatter example binary",
    )
    parser.add_argument(
        "--kind",
        choices=["all", "transform", "smoke", "roundtrip"],
        default="all",
        help="fixture group to audit",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=25,
        help="maximum mismatches to print in detail",
    )
    parser.add_argument(
        "--write-shared-mismatches",
        action="store_true",
        help="rewrite shared syntax mismatch fixtures to the reference output",
    )
    args = parser.parse_args()

    if not ensure_reference_binary(args.reference_binary):
        print(f"missing reference formatter binary: {args.reference_binary}", file=sys.stderr)
        return 2

    cases: list[FixtureCase] = []
    if args.kind in {"all", "transform"}:
        cases.extend(parse_transform_cases())
    if args.kind in {"all", "smoke"}:
        cases.extend(parse_smoke_cases())
    if args.kind in {"all", "roundtrip"}:
        cases.extend(parse_roundtrip_cases())

    results = [audit_case(args.reference_binary, case) for case in cases]

    if args.write_shared_mismatches:
        updated_case_count = write_shared_mismatches(results)
        print(f"updated_shared_mismatch_cases={updated_case_count}")
        return 0

    by_outcome: dict[str, int] = {}
    by_kind: dict[str, int] = {}
    by_surface: dict[str, int] = {}
    for result in results:
        by_outcome[result.outcome] = by_outcome.get(result.outcome, 0) + 1
        by_kind[result.case.kind] = by_kind.get(result.case.kind, 0) + 1
        by_surface[result.surface] = by_surface.get(result.surface, 0) + 1

    print("fixture audit summary")
    print(f"cases={len(results)}")
    print("kinds=" + ", ".join(f"{kind}:{count}" for kind, count in sorted(by_kind.items())))
    print("surfaces=" + ", ".join(f"{surface}:{count}" for surface, count in sorted(by_surface.items())))
    print(
        "outcomes="
        + ", ".join(f"{outcome}:{count}" for outcome, count in sorted(by_outcome.items()))
    )

    printed = 0
    for result in results:
        if result.outcome not in {
            "mismatch",
            "local_needs_rule",
            "reference_invalid_input",
            "reference_parse_failure",
            "unsupported_fixture",
        }:
            continue
        if printed >= args.limit:
            break

        printed += 1
        print()
        print(f"[{result.outcome}] {result.case.name}")
        print(f"  source={result.case.source_path}")
        print(f"  surface={result.surface}")
        if result.markers:
            print(f"  markers={', '.join(result.markers)}")
        if result.file_name is not None:
            print(f"  reference={result.file_name}")
        if result.detail is not None:
            print(f"  detail={result.detail}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
