import os
import re
import shutil
import subprocess
import sys
import traceback
from collections.abc import Iterable, Mapping, Sequence
from contextlib import contextmanager
from typing import Literal

# ansi color codes
COLORS = {
    "reset": "\x1b[0m",
    "bold": "\x1b[1m",
    "dim": "\x1b[2m",
    # foreground colors
    "black": "\x1b[30m",
    "red": "\x1b[31m",
    "green": "\x1b[32m",
    "yellow": "\x1b[33m",
    "blue": "\x1b[34m",
    "magenta": "\x1b[35m",
    "cyan": "\x1b[36m",
    "white": "\x1b[37m",
    "gray": "\x1b[90m",
    # bright foreground colors
    "bright_red": "\x1b[91m",
    "bright_green": "\x1b[92m",
    "bright_yellow": "\x1b[93m",
    "bright_blue": "\x1b[94m",
    "bright_magenta": "\x1b[95m",
    "bright_cyan": "\x1b[96m",
    "bright_white": "\x1b[97m",
}

Color = Literal[
    "reset",
    "bold",
    "dim",
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "gray",
    "bright_red",
    "bright_green",
    "bright_yellow",
    "bright_blue",
    "bright_magenta",
    "bright_cyan",
    "bright_white",
]


def color(text: str, *styles: Color) -> str:
    """Apply color to text."""
    codes = "".join(COLORS[s] for s in styles)
    return f"{codes}{text}{COLORS['reset']}"


_CAPTURE_STACK: list[list[str]] = []


@contextmanager
def capture_output():
    """Capture all console output produced by console helpers."""
    buf: list[str] = []
    _CAPTURE_STACK.append(buf)
    try:
        yield buf
    finally:
        _CAPTURE_STACK.pop()


def page(text: str) -> None:
    """Send text to a pager if available, preserving ANSI colors."""
    pager = os.environ.get("PAGER")
    cmd: list[str] | None = None
    if pager:
        cmd = [pager]
        # add -R for less if detectable in command string
        if os.path.basename(pager) == "less" or "less" in pager:
            cmd.append("-R")
    else:
        less = shutil.which("less")
        if less:
            cmd = [less, "-R"]

    if cmd is None:
        # fallback: just print
        sys.stdout.write(text)
        if text and not text.endswith("\n"):
            sys.stdout.write("\n")
        sys.stdout.flush()
        return

    try:
        subprocess.run(cmd, input=text, text=True, check=False)
    except Exception:
        # fallback on error
        sys.stdout.write(text)
        if text and not text.endswith("\n"):
            sys.stdout.write("\n")
        sys.stdout.flush()


def print(text: str, *styles: Color) -> None:
    """Print colored text to stdout."""
    line = color(text, *styles) + "\n"
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(line)
    else:
        sys.stdout.write(line)
        sys.stdout.flush()


def write(text: str, *styles: Color) -> None:
    """Print text without newline."""
    out = color(text, *styles)
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(out)
    else:
        sys.stdout.write(out)
        sys.stdout.flush()


def error(text: str) -> None:
    """Print error to stderr."""
    line = color(text, "red") + "\n"
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(line)
    else:
        sys.stderr.write(line)
        sys.stderr.flush()


def warn(text: str) -> None:
    """Print warning."""
    line = color(text, "yellow") + "\n"
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(line)
    else:
        sys.stdout.write(line)
        sys.stdout.flush()


def success(text: str) -> None:
    """Print success message."""
    line = color(text, "green") + "\n"
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(line)
    else:
        sys.stdout.write(line)
        sys.stdout.flush()


def info(text: str) -> None:
    """Print info message."""
    line = color(text, "cyan") + "\n"
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(line)
    else:
        sys.stdout.write(line)
        sys.stdout.flush()


def clear() -> None:
    """Clear the terminal."""
    sys.stdout.write("\x1b[2J\x1b[0f")
    sys.stdout.flush()


def move_up(lines: int = 1) -> None:
    """Move cursor up n lines."""
    sys.stdout.write(f"\x1b[{lines}A")
    sys.stdout.flush()


def clear_line() -> None:
    """Clear the current line."""
    sys.stdout.write("\x1b[2K\r")
    sys.stdout.flush()


def stacktrace(exc: BaseException, show_locals: bool = False) -> None:
    """Render a formatted stacktrace for an exception."""
    # get the traceback information
    tb = traceback.TracebackException.from_exception(exc, capture_locals=show_locals)

    # print exception header
    exc_type = type(exc).__name__
    exc_msg = str(exc)

    sys.stdout.write("\n")
    sys.stdout.write(color("╭─ ", "red") + color(f"{exc_type}", "bright_red", "bold"))
    if exc_msg:
        sys.stdout.write(color(": ", "red") + color(exc_msg, "bright_white"))
    sys.stdout.write("\n")

    # print the stack frames
    for frame in tb.stack:
        # file info
        filename = frame.filename
        lineno = frame.lineno
        function = frame.name

        # shorten long file paths to just show the relevant parts
        if "/site-packages/" in filename:
            # external library
            parts = filename.split("/site-packages/")
            short_filename = "📦 " + parts[-1]
            file_color = "dim"
        elif "/venv/" in filename or "/.venv/" in filename:
            # virtual environment
            parts = filename.split("/venv/")[-1].split("/.venv/")[-1]
            short_filename = "🐍 " + parts
            file_color = "dim"
        else:
            # project file
            import os

            try:
                short_filename = os.path.relpath(filename)
            except ValueError:
                short_filename = filename
            file_color = "cyan"

        # print frame header
        sys.stdout.write(color("│\n├─ ", "red"))
        sys.stdout.write(color(f"{short_filename}", file_color))
        sys.stdout.write(color(":", "gray"))
        sys.stdout.write(color(f"{lineno}", "yellow"))
        sys.stdout.write(color(" in ", "gray"))
        sys.stdout.write(color(f"{function}", "bright_blue", "bold"))
        sys.stdout.write("\n")

        # print the code line if available
        if frame.line:
            code_line = frame.line.strip()
            sys.stdout.write(color("│  ", "red"))
            # simple syntax highlighting
            import keyword

            words = []
            for word in code_line.split():
                if keyword.iskeyword(word.rstrip("()[]{}:,.")):
                    words.append(color(word, "magenta"))
                elif word.startswith('"') or word.startswith("'"):
                    words.append(color(word, "green"))
                elif word[0].isdigit() or word in ["True", "False", "None"]:
                    words.append(color(word, "yellow"))
                else:
                    words.append(word)
            sys.stdout.write(" ".join(words))
            sys.stdout.write("\n")

        # print locals if requested
        if show_locals and hasattr(frame, "locals") and frame.locals:
            for var_name, var_value in frame.locals.items():
                if not var_name.startswith("__"):
                    sys.stdout.write(color("│    ", "red"))
                    sys.stdout.write(color(f"{var_name}", "gray"))
                    sys.stdout.write(color(" = ", "gray"))
                    try:
                        # truncate long values
                        value_str = repr(var_value)
                        if len(value_str) > 60:
                            value_str = value_str[:57] + "..."
                        sys.stdout.write(color(value_str, "dim"))
                    except Exception:
                        sys.stdout.write(color("<error getting value>", "dim"))
                    sys.stdout.write("\n")

    # print footer
    sys.stdout.write(color("╰", "red"))
    sys.stdout.write(color("─" * 50, "red"))
    sys.stdout.write("\n\n")
    sys.stdout.flush()


def box(content: str, padding: int = 1) -> str:
    """Create a simple box around content."""
    lines = content.split("\n")
    max_length = max(len(line) for line in lines)
    width = max_length + padding * 2

    result = []

    # top border
    result.append(f"┌{'─' * width}┐")

    # content with padding
    for line in lines:
        padded_line = line.ljust(max_length)
        result.append(f"│{' ' * padding}{padded_line}{' ' * padding}│")

    # bottom border
    result.append(f"└{'─' * width}┘")

    return "\n".join(result)


def header(text: str, char: str = "=") -> str:
    """Create a section header."""
    line = char * len(text)
    return f"{line}\n{text}\n{line}"


ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")


def _strip_ansi(text: str) -> str:
    return ANSI_RE.sub("", text)


def _visible_len(text: str) -> int:
    return len(_strip_ansi(text))


def _pad_cell(cell: str, width: int) -> str:
    pad = max(0, width - _visible_len(cell))
    return cell + (" " * pad)


def table(
    rows: Sequence[Mapping[str, object]] | Sequence[Sequence[object]],
    headers: Sequence[str] | None = None,
    padding: int = 2,
    right_align_numeric: bool = True,
    secondary_headers: Sequence[str] | None = None,
    separators_on_change: Sequence[str] | None = None,
) -> str:
    """Create a formatted table.

    Accepts rows as list of dicts (preferred) or list of lists.
    - Dict rows: column order is taken from headers if provided, otherwise
      from the first row's key order, then union of subsequent keys.
    - Numbers in data cells are right-aligned if `right_align_numeric` is True.
    - Multi-line cells are supported and rendered with proper padding.
    """
    if not rows and not headers:
        return ""

    # normalize rows to matrix + derive headers if dict-based
    use_dict = bool(rows) and isinstance(rows[0], Mapping)  # type: ignore[index]

    if use_dict:
        dict_rows: Sequence[Mapping[str, object]] = rows  # type: ignore[assignment]
        if headers is None:
            ordered_keys: list[str] = []
            seen: set[str] = set()
            for r in dict_rows:
                for k in r:
                    if k not in seen:
                        seen.add(k)
                        ordered_keys.append(str(k))
            headers = ordered_keys
        matrix: list[list[str]] = [[str(r.get(h, "")) for h in headers] for r in dict_rows]
    else:
        matrix = [[str(c) for c in row] for row in rows]  # type: ignore[list-item]
        # if headers provided, ensure col count matches
        if headers is None and matrix:
            headers = [str(i + 1) for i in range(len(matrix[0]))]

    assert headers is not None

    # helpers for multi-line cells and numeric detection
    def _max_visible_line_len(cell: str) -> int:
        return max((_visible_len(line) for line in cell.splitlines()), default=0)

    def _is_numeric_like(cell: str) -> bool:
        txt = _strip_ansi(cell).strip()
        if not txt:
            return False
        # allow thousands separators
        txt = txt.replace(",", "")
        # strip common unit suffixes once for detection (bytes and time)
        unit_match = re.fullmatch(r"(.*?)(?:B|KB|MB|GB|TB|s|ms|us|μs|ns|%)$", txt)
        core = unit_match.group(1) if unit_match else txt
        core = core.strip()
        # ranges and prefixed comparisons (e.g., 1..10, <5, =3)
        if re.fullmatch(r"[<=>]?\d+(?:\.\d+)?(?:\.\.)?[<=>]?\d*(?:\.\d+)?", core):
            return True
        return bool(re.fullmatch(r"-?\d+(?:\.\d+)?", core))

    # compute column widths using max line length per cell
    num_cols = len(headers)
    col_widths = [0] * num_cols
    for i in range(num_cols):
        # include header in width
        col_widths[i] = max(col_widths[i], _visible_len(str(headers[i])))
        if secondary_headers is not None and i < len(secondary_headers):
            col_widths[i] = max(col_widths[i], _visible_len(str(secondary_headers[i])))
        for row in matrix:
            if i < len(row):
                col_widths[i] = max(col_widths[i], _max_visible_line_len(row[i]))

    # determine alignment per column
    right_align_cols: set[int] = set()
    if right_align_numeric and matrix:
        for i in range(num_cols):
            # if all data cells in this column are numeric-like → right align
            if matrix and all(
                (i < len(row) and (_is_numeric_like(row[i]) or _strip_ansi(row[i]).strip() == ""))
                for row in matrix
            ):
                right_align_cols.add(i)

    # renderer for one physical row possibly spanning multiple lines
    def _render_physical_rows(cells: list[str]) -> list[str]:
        split_cells = [c.splitlines() if c else [""] for c in cells]
        height = max(len(sc) for sc in split_cells)
        lines: list[str] = []
        for line_idx in range(height):
            parts: list[str] = []
            for col_idx, sc in enumerate(split_cells):
                line_text = sc[line_idx] if line_idx < len(sc) else ""
                width = col_widths[col_idx]
                if col_idx in right_align_cols:
                    pad = max(0, width - _visible_len(line_text))
                    parts.append((" " * pad) + line_text)
                else:
                    parts.append(_pad_cell(line_text, width))
            lines.append(" " * padding + (" " * padding).join(parts))
        return lines

    out_lines: list[str] = []

    # header
    header_cells = [str(h) for h in headers]
    out_lines.extend(_render_physical_rows(header_cells))

    # secondary header (dim)
    if secondary_headers is not None:
        sec_cells = [
            str(secondary_headers[i]) if i < len(secondary_headers) else "" for i in range(num_cols)
        ]
        sec_cells = [color(c, "dim") if c else c for c in sec_cells]
        out_lines.extend(_render_physical_rows(sec_cells))

    # separator
    sep_row = ["─" * w for w in col_widths]
    out_lines.append(" " * padding + (" " * padding).join(sep_row))

    # data rows
    monitor_indices: list[int] = []
    if separators_on_change:
        name_to_idx = {str(h): i for i, h in enumerate(headers)}
        for name in separators_on_change:
            idx = name_to_idx.get(str(name))
            if idx is not None:
                monitor_indices.append(idx)
    prev_keys: list[str] | None = None
    for row in matrix:
        # insert a separator if any monitored key changed
        if monitor_indices and prev_keys is not None:
            cur_keys = []
            for i in monitor_indices:
                if i < len(row):
                    lines = _strip_ansi(row[i]).splitlines()
                    cur_keys.append(lines[0] if lines else "")
                else:
                    cur_keys.append("")
            if cur_keys != prev_keys:
                sep_row = ["─" * w for w in col_widths]
                out_lines.append(" " * padding + (" " * padding).join(sep_row))
            prev_keys = cur_keys
        elif monitor_indices and prev_keys is None:
            prev_keys = []
            for i in monitor_indices:
                if i < len(row):
                    lines = _strip_ansi(row[i]).splitlines()
                    prev_keys.append(lines[0] if lines else "")
                else:
                    prev_keys.append("")

        cells = [row[i] if i < len(row) else "" for i in range(num_cols)]
        out_lines.extend(_render_physical_rows(cells))

    return "\n".join(out_lines)


def kv(mapping: Mapping[str, object], padding: int = 2) -> str:
    """Render a key/value mapping as a two-column table."""
    rows: list[Mapping[str, object]] = []
    for k, v in mapping.items():
        rows.append({"Key": str(k), "Value": str(v)})
    return table(rows, headers=["Key", "Value"], padding=padding)


def print_kv(mapping: Mapping[str, object], padding: int = 2) -> None:
    """Print a key/value mapping as a two-column table."""
    sys.stdout.write(kv(mapping, padding) + "\n")
    sys.stdout.flush()


def humanize_bytes(num_bytes: int, decimals: int = 2) -> str:
    """Humanize a byte count to B, KB, MB, GB, TB."""
    units = ["B", "KB", "MB", "GB", "TB"]
    value = float(num_bytes)
    idx = 0
    while value >= 1024.0 and idx < len(units) - 1:
        value /= 1024.0
        idx += 1
    if idx == 0:
        return f"{int(value)}{units[idx]}"
    return f"{value:.{decimals}f}{units[idx]}"


def humanize_count(n: int) -> str:
    """Humanize an integer count with thousands separators."""
    return f"{n:_}"


def humanize_count_text(text: str) -> str:
    """Humanize numeric tokens in count expressions like '<1000', '1..2000', '=3000'."""
    s = _strip_ansi(text)
    s = s.replace(",", "")

    def _hum(match: re.Match[str]) -> str:
        token = match.group(0)
        if token in ("..", "<", ">", "=", "-"):
            return token
        try:
            return humanize_count(int(token))
        except ValueError:
            return token

    # replace integers while keeping separators
    return re.sub(r"\d+|\.\.|<|>|=|-", _hum, s)


def humanize_duration(seconds: float) -> str:
    """Humanize a duration in seconds into s/ms/μs/ns."""
    if seconds >= 1.0:
        return f"{seconds:.2f}s"
    ms = seconds * 1_000.0
    if ms >= 1.0:
        return f"{ms:.2f}ms"
    us = seconds * 1_000_000.0
    if us >= 1.0:
        return f"{us:.2f}μs"
    ns = seconds * 1_000_000_000.0
    return f"{ns:.2f}ns"


def prompt(text: str, default: str | None = None) -> str:
    """Display a prompt and get user input."""
    prompt_text = color(text, "cyan")
    if default:
        prompt_text += color(f" [{default}]", "dim")
    prompt_text += ": "

    try:
        user_input = input(prompt_text).strip()
        if not user_input and default:
            return default
        return user_input
    except (KeyboardInterrupt, EOFError):
        print("")  # new line after ^C
        return ""


def paginate(text: str, page_size: int = 20) -> None:
    """Display text with pagination."""
    lines = text.split("\n")
    total_lines = len(lines)

    if total_lines <= page_size:
        # no pagination needed
        for line in lines:
            sys.stdout.write(line + "\n")
        return

    page_num = 0
    while page_num * page_size < total_lines:
        start = page_num * page_size
        end = min(start + page_size, total_lines)

        # display current page
        for i in range(start, end):
            sys.stdout.write(lines[i] + "\n")

        # check if more pages
        if end < total_lines:
            remaining = total_lines - end
            write(f"\n{color('--- More ---', 'dim')} ")
            write(color(f"({remaining} lines remaining) ", "dim"))
            write(color("[Enter/Space: next, q: quit] ", "yellow"))

            try:
                response = input().strip().lower()
                if response in ["q", "quit"]:
                    break
                # move cursor up and clear the prompt line
                move_up(1)
                clear_line()
            except (KeyboardInterrupt, EOFError):
                print("")  # new line
                break
        else:
            break

        page_num += 1


def section(title: str, content: str | None = None) -> None:
    """Print a section with title."""
    lines: list[str] = []
    lines.append("\n")
    lines.append(color("━" * 60, "dim") + "\n")
    lines.append(color(title, "cyan", "bold") + "\n")
    if content:
        lines.append(color("━" * 60, "dim") + "\n")
        lines.append(content + "\n")
    else:
        lines.append(color("━" * 60, "dim") + "\n")
    out = "".join(lines)
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(out)
    else:
        sys.stdout.write(out)
        sys.stdout.flush()


def render_tree(
    root_id: str,
    children_by_id: dict[str, list[str]],
    label_by_id: dict[str, str],
    highlight_id: str | None = None,
) -> str:
    """
    Render a simple ASCII tree from a parent→children mapping.

    The ids must be keys of `label_by_id`. Children lists define order.
    `highlight_id` will be rendered with stronger emphasis.
    """

    def style_label(node_id: str) -> str:
        node_label = label_by_id.get(node_id, node_id)
        if node_id == highlight_id:
            return color(node_label, "bright_white", "bold")
        return color(node_label, "white")

    lines: list[str] = []

    def walk(node_id: str, prefix: str, is_last: bool) -> None:
        connector = "└─ " if is_last else "├─ "
        if prefix:
            lines.append(prefix + connector + style_label(node_id))
        else:
            lines.append(style_label(node_id))

        children = children_by_id.get(node_id, [])
        # ensure deterministic order by visible label
        children = sorted(children, key=lambda cid: _strip_ansi(label_by_id.get(cid, cid)))
        if not children:
            return

        next_prefix = prefix + ("   " if is_last else "│  ")
        for idx, child_id in enumerate(children):
            walk(child_id, next_prefix, idx == len(children) - 1)

    # root line
    walk(root_id, "", True)
    return "\n".join(lines)


def print_tree(
    root_id: str,
    children_by_id: dict[str, list[str]],
    label_by_id: dict[str, str],
    highlight_id: str | None = None,
) -> None:
    """Print a tree rendered by `render_tree`."""
    out = render_tree(root_id, children_by_id, label_by_id, highlight_id) + "\n"
    if _CAPTURE_STACK:
        _CAPTURE_STACK[-1].append(out)
    else:
        sys.stdout.write(out)
        sys.stdout.flush()


def paginate_if_needed(render: Iterable[str] | str, page_size: int = 25) -> None:
    # deprecated: direct print is preferred in manual
    text = "".join(render) if not isinstance(render, str) else render
    if text and not text.endswith("\n"):
        text += "\n"
    sys.stdout.write(text)
    sys.stdout.flush()
