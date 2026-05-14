#!/usr/bin/env python3
import argparse
import re
import shutil
import subprocess
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path


SOURCE_EXTENSIONS = {".ds", ".rs"}
UNSAFE_CANDIDATE_PATTERN = r"\bunsafe\b|@\s*unsafe\b"
RUST_UNSAFE_PATTERN = r"(?<!r#)\bunsafe\b"
ANSI_PATTERN = re.compile(r"\x1b\[[0-9;]*m")

GENERATED_PATHS = (
    "language/compiler/generate/native/cranelift/**",
    "language/compiler/generate/native/pulley/**",
    "language/compiler/generate/native/regalloc2/**",
    "language/compiler/generate/native/wasmtime-core/**",
)

FIXTURE_PATHS = (
    "language/test/fixtures/**",
)

IGNORED_PATHS = (
    ".git/**",
    "target/**",
    "**/target/**",
    "node_modules/**",
    "**/node_modules/**",
)

RUST_PATTERNS = {
    "attribute": re.compile(r"#\s*\[\s*unsafe\s*\("),
    "block": re.compile(r"\bunsafe\s*\{"),
    "extern": re.compile(r"\bunsafe\s+extern\b"),
    "function": re.compile(r"\bunsafe\s+fn\b"),
    "implementation": re.compile(r"\bunsafe\s+impl\b"),
    "trait": re.compile(r"\bunsafe\s+trait\b"),
}

DESTACK_PATTERNS = {
    "attribute": re.compile(r"@\s*unsafe\b"),
}


class Color:
    reset = "\033[0m"
    bold = "\033[1m"
    dim = "\033[2m"
    red = "\033[31m"
    yellow = "\033[33m"
    green = "\033[32m"
    cyan = "\033[36m"
    magenta = "\033[35m"
    blue = "\033[34m"


@dataclass(frozen=True)
class Audit:
    """Unsafe counts for one source file."""

    path: Path
    extension: str
    counts: Counter

    @property
    def total(self) -> int:
        """Return the total unsafe count."""

        return sum(self.counts.values())


@dataclass(frozen=True)
class Options:
    """Command line options."""

    root: Path
    include_generated: bool
    include_fixtures: bool
    depth: int
    file_count: int
    color: str


def parse_args() -> Options:
    """Parse command line arguments."""

    parser = argparse.ArgumentParser(description="Audit unsafe source usage.")
    parser.add_argument("--root", type=Path, default=None, help="Repository root to audit.")
    parser.add_argument("--include-generated", action="store_true", help="Include generated backend source.")
    parser.add_argument("--include-fixtures", action="store_true", help="Include test fixtures.")
    parser.add_argument("--depth", type=int, default=4, help="Directory depth for aggregate output.")
    parser.add_argument("--files", type=int, default=25, help="Number of hottest files to print.")
    parser.add_argument("--color", choices=["auto", "always", "never"], default="auto", help="Color output.")
    args = parser.parse_args()

    root = args.root.resolve() if args.root else repo_root()

    return Options(
        root=root,
        include_generated=args.include_generated,
        include_fixtures=args.include_fixtures,
        depth=args.depth,
        file_count=args.files,
        color=args.color,
    )


def repo_root() -> Path:
    """Return the repository root."""

    return Path(__file__).resolve().parents[2]


def candidate_files(options: Options) -> list[Path]:
    """Return source files that may contain unsafe usages."""

    if shutil.which("rg"):
        return ripgrep_candidate_files(options)

    return python_candidate_files(options)


def ripgrep_candidate_files(options: Options) -> list[Path]:
    """Return unsafe candidate files through ripgrep."""

    command = [
        "rg",
        "--files-with-matches",
        "--no-messages",
        "--glob",
        "*.rs",
        "--glob",
        "*.ds",
    ]

    for pattern in excluded_paths(options):
        command.extend(["--glob", f"!{pattern}"])

    command.append(UNSAFE_CANDIDATE_PATTERN)

    result = subprocess.run(command, cwd=options.root, capture_output=True, text=True, check=False)
    if result.returncode not in (0, 1):
        raise SystemExit(result.stderr.strip() or "rg failed")

    files = []
    for line in result.stdout.splitlines():
        path = (options.root / line).resolve()
        if path.suffix in SOURCE_EXTENSIONS:
            files.append(path)

    return sorted(files)


def python_candidate_files(options: Options) -> list[Path]:
    """Return unsafe candidate files without external tools."""

    files = []
    for path in options.root.rglob("*"):
        if not path.is_file() or path.suffix not in SOURCE_EXTENSIONS:
            continue

        relative = path.relative_to(options.root)
        if is_excluded(relative, options):
            continue

        text = path.read_text(encoding="utf-8")
        if re.search(UNSAFE_CANDIDATE_PATTERN, text):
            files.append(path)

    return sorted(files)


def excluded_paths(options: Options) -> tuple[str, ...]:
    """Return ripgrep exclusion globs."""

    paths = list(IGNORED_PATHS)
    if not options.include_generated:
        paths.extend(GENERATED_PATHS)

    if not options.include_fixtures:
        paths.extend(FIXTURE_PATHS)

    return tuple(paths)


def is_excluded(path: Path, options: Options) -> bool:
    """Return whether a relative path is excluded."""

    text = path.as_posix()
    if any(path.match(pattern) or text.startswith(pattern.removesuffix("/**")) for pattern in IGNORED_PATHS):
        return True

    if not options.include_generated and any(path.match(pattern) for pattern in GENERATED_PATHS):
        return True

    if not options.include_fixtures and any(path.match(pattern) for pattern in FIXTURE_PATHS):
        return True

    return False


def audit_file(path: Path) -> Audit:
    """Audit unsafe usages in one source file."""

    text = path.read_text(encoding="utf-8")
    text = mask_source(text)

    if path.suffix == ".rs":
        counts = audit_rust(text)
    else:
        counts = audit_destack(text)

    return Audit(path=path, extension=path.suffix, counts=counts)


def audit_rust(text: str) -> Counter:
    """Count Rust unsafe forms."""

    counts = Counter()
    for name, pattern in RUST_PATTERNS.items():
        counts[name] = len(pattern.findall(text))

    total = len(re.findall(RUST_UNSAFE_PATTERN, text))
    counted = sum(counts.values())
    if total > counted:
        counts["other"] = total - counted

    return counts


def audit_destack(text: str) -> Counter:
    """Count TS++ unsafe markers."""

    counts = Counter()
    for name, pattern in DESTACK_PATTERNS.items():
        counts[name] = len(pattern.findall(text))

    return counts


def mask_source(text: str) -> str:
    """Mask comments and strings while preserving byte positions roughly."""

    output = []
    cursor = 0
    state = "normal"
    block_depth = 0
    raw_hashes = ""

    while cursor < len(text):
        char = text[cursor]
        next_char = text[cursor + 1] if cursor + 1 < len(text) else ""

        if state == "line":
            cursor = mask_line_comment(text, cursor, output)
            state = "normal"
            continue

        if state == "block":
            cursor, block_depth, state = mask_block_comment(text, cursor, block_depth, output)
            continue

        if state == "string":
            cursor, state = mask_string(text, cursor, output)
            continue

        if state == "raw":
            cursor, state = mask_raw_string(text, cursor, raw_hashes, output)
            continue

        if char == "/" and next_char == "/":
            output.extend("  ")
            cursor += 2
            state = "line"
            continue

        if char == "/" and next_char == "*":
            output.extend("  ")
            cursor += 2
            block_depth = 1
            state = "block"
            continue

        raw = raw_string_start(text, cursor)
        if raw is not None:
            prefix_len, raw_hashes = raw
            output.extend(" " * prefix_len)
            cursor += prefix_len
            state = "raw"
            continue

        if char == '"':
            output.append(" ")
            cursor += 1
            state = "string"
            continue

        output.append(char)
        cursor += 1

    return "".join(output)


def mask_line_comment(text: str, cursor: int, output: list[str]) -> int:
    """Mask one line comment."""

    while cursor < len(text):
        char = text[cursor]
        output.append("\n" if char == "\n" else " ")
        cursor += 1
        if char == "\n":
            break

    return cursor


def mask_block_comment(text: str, cursor: int, depth: int, output: list[str]) -> tuple[int, int, str]:
    """Mask one block comment step."""

    char = text[cursor]
    next_char = text[cursor + 1] if cursor + 1 < len(text) else ""

    if char == "/" and next_char == "*":
        output.extend("  ")

        return cursor + 2, depth + 1, "block"

    if char == "*" and next_char == "/":
        output.extend("  ")
        depth -= 1
        state = "normal" if depth == 0 else "block"

        return cursor + 2, depth, state

    output.append("\n" if char == "\n" else " ")

    return cursor + 1, depth, "block"


def mask_string(text: str, cursor: int, output: list[str]) -> tuple[int, str]:
    """Mask one string step."""

    char = text[cursor]
    if char == "\\":
        output.extend("  ")

        return min(cursor + 2, len(text)), "string"

    output.append("\n" if char == "\n" else " ")
    state = "normal" if char == '"' else "string"

    return cursor + 1, state


def mask_raw_string(text: str, cursor: int, hashes: str, output: list[str]) -> tuple[int, str]:
    """Mask one raw string step."""

    terminator = '"' + hashes
    if text.startswith(terminator, cursor):
        output.extend(" " * len(terminator))

        return cursor + len(terminator), "normal"

    char = text[cursor]
    output.append("\n" if char == "\n" else " ")

    return cursor + 1, "raw"


def raw_string_start(text: str, cursor: int) -> tuple[int, str] | None:
    """Return raw string prefix length and hash suffix."""

    start = cursor
    if cursor < len(text) and text[cursor] == "b":
        cursor += 1

    if cursor >= len(text) or text[cursor] != "r":
        return None

    cursor += 1
    while cursor < len(text) and text[cursor] == "#":
        cursor += 1

    if cursor >= len(text) or text[cursor] != '"':
        return None

    hash_start = start + (2 if text[start] == "b" else 1)
    hashes = text[hash_start:cursor]

    return cursor - start + 1, hashes


def aggregate_by_extension(audits: list[Audit]) -> dict[str, Counter]:
    """Aggregate unsafe counts by file extension."""

    totals = defaultdict(Counter)
    for audit in audits:
        totals[audit.extension].update(audit.counts)

    return totals


def aggregate_by_module(audits: list[Audit], root: Path, depth: int) -> dict[tuple[Path, str], Counter]:
    """Aggregate unsafe counts by module path and file extension."""

    totals = defaultdict(Counter)
    for audit in audits:
        relative = audit.path.relative_to(root)
        module = Path(*relative.parent.parts[:depth])
        totals[(module, audit.extension)].update(audit.counts)

    return totals


def color_enabled(color: str) -> bool:
    """Return whether output should use ANSI colors."""

    if color == "always":
        return True

    if color == "never":
        return False

    return sys.stdout.isatty()


def paint(text: str, color: str, enabled: bool) -> str:
    """Apply ANSI color when enabled."""

    if not enabled:
        return text

    return f"{color}{text}{Color.reset}"


def paint_total(total: int, enabled: bool) -> str:
    """Color one total cell by size."""

    if total >= 100:
        color = Color.red
    elif total >= 25:
        color = Color.yellow
    else:
        color = Color.green

    return paint(f"{total:>5}", color, enabled)


def format_counts(counts: Counter) -> str:
    """Format nonzero unsafe form counts."""

    return " ".join(f"{name}={counts[name]}" for name in sorted(counts) if counts[name])


def plain_len(text: str) -> int:
    """Return visible string length."""

    return len(ANSI_PATTERN.sub("", text))


def print_table(title: str, headers: list[str], rows: list[list[str]], colors: bool) -> None:
    """Print one aligned table."""

    if not rows:
        return

    widths = column_widths(headers, rows)

    print()
    print(paint(title, Color.bold + Color.cyan, colors))
    print_table_row(headers, widths, colors, header=True)
    print(paint("─┼─".join("─" * width for width in widths), Color.dim, colors))

    for row in rows:
        print_table_row(row, widths, colors, header=False)


def column_widths(headers: list[str], rows: list[list[str]]) -> list[int]:
    """Return visible column widths."""

    widths = [len(header) for header in headers]
    for row in rows:
        for index, cell in enumerate(row):
            widths[index] = max(widths[index], plain_len(cell))

    return widths


def print_table_row(row: list[str], widths: list[int], colors: bool, header: bool) -> None:
    """Print one table row."""

    cells = []
    for index, cell in enumerate(row):
        padding = " " * (widths[index] - plain_len(cell))
        value = cell + padding
        if header:
            value = paint(value, Color.bold, colors)
        cells.append(value)

    print(" │ ".join(cells))


def total_counts(audits: list[Audit]) -> Counter:
    """Return total unsafe counts."""

    counts = Counter()
    for audit in audits:
        counts.update(audit.counts)

    return counts


def print_report(audits: list[Audit], options: Options) -> None:
    """Print the unsafe audit report."""

    colors = color_enabled(options.color)
    counts = total_counts(audits)
    total = sum(counts.values())

    print(paint("unsafe audit", Color.bold + Color.magenta, colors))
    print(f"{paint('root', Color.bold, colors)}   {paint(str(options.root), Color.dim, colors)}")
    print(f"{paint('total', Color.bold, colors)}  {paint_total(total, colors).strip()}  {format_counts(counts)}")

    print_extension_table(audits, colors)
    print_module_table(audits, options, colors)
    print_file_table(audits, options, colors)


def print_extension_table(audits: list[Audit], colors: bool) -> None:
    """Print unsafe counts by file extension."""

    rows = []
    for extension, counts in sorted(aggregate_by_extension(audits).items(), key=sort_counted_item):
        rows.append([paint_total(sum(counts.values()), colors), paint(extension, Color.blue, colors), format_counts(counts)])

    print_table("by file type", ["total", "type", "forms"], rows, colors)


def print_module_table(audits: list[Audit], options: Options, colors: bool) -> None:
    """Print unsafe counts by module and file extension."""

    rows = []
    for (module, extension), counts in sorted(aggregate_by_module(audits, options.root, options.depth).items(), key=sort_module_item):
        rows.append([paint_total(sum(counts.values()), colors), str(module), paint(extension, Color.blue, colors), format_counts(counts)])

    print_table("by module and file type", ["total", "module", "type", "forms"], rows, colors)


def print_file_table(audits: list[Audit], options: Options, colors: bool) -> None:
    """Print the files with the highest unsafe counts."""

    rows = []
    for audit in sorted(audits, key=lambda item: (-item.total, str(item.path)))[: options.file_count]:
        relative = audit.path.relative_to(options.root)
        rows.append([paint_total(audit.total, colors), paint(audit.extension, Color.blue, colors), str(relative), format_counts(audit.counts)])

    print_table("hottest files", ["total", "type", "file", "forms"], rows, colors)


def sort_counted_item(item: tuple[str, Counter]) -> tuple[int, str]:
    """Sort aggregate rows by count descending."""

    name, counts = item

    return -sum(counts.values()), name


def sort_module_item(item: tuple[tuple[Path, str], Counter]) -> tuple[int, str, str]:
    """Sort module rows by count descending."""

    (module, extension), counts = item

    return -sum(counts.values()), str(module), extension


def main() -> int:
    """Run the unsafe audit."""

    options = parse_args()
    audits = [audit_file(path) for path in candidate_files(options)]
    audits = [audit for audit in audits if audit.total]
    print_report(audits, options)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
