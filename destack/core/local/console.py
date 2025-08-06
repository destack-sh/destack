import sys
import traceback
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


def print(text: str, *styles: Color) -> None:
    """Print colored text to stdout."""
    sys.stdout.write(color(text, *styles) + "\n")
    sys.stdout.flush()


def write(text: str, *styles: Color) -> None:
    """Print text without newline."""
    sys.stdout.write(color(text, *styles))
    sys.stdout.flush()


def error(text: str) -> None:
    """Print error to stderr."""
    sys.stderr.write(color(text, "red") + "\n")
    sys.stderr.flush()


def warn(text: str) -> None:
    """Print warning."""
    sys.stdout.write(color(text, "yellow") + "\n")
    sys.stdout.flush()


def success(text: str) -> None:
    """Print success message."""
    sys.stdout.write(color(text, "green") + "\n")
    sys.stdout.flush()


def info(text: str) -> None:
    """Print info message."""
    sys.stdout.write(color(text, "cyan") + "\n")
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


def table(rows: list[list[str]], headers: list[str] | None = None, padding: int = 2) -> str:
    """Create a formatted table."""
    if not rows and not headers:
        return ""

    # calculate column widths
    all_rows = [headers] if headers else []
    all_rows.extend(rows)

    if not all_rows:
        return ""

    num_cols = len(all_rows[0])
    col_widths = [0] * num_cols

    for row in all_rows:
        for i, cell in enumerate(row[:num_cols]):
            col_widths[i] = max(col_widths[i], len(str(cell)))

    result = []

    # add headers if provided
    if headers:
        header_row = []
        for i, header in enumerate(headers):
            header_row.append(str(header).ljust(col_widths[i]))
        result.append(" " * padding + (" " * padding).join(header_row))

        # add separator
        sep_row = []
        for width in col_widths:
            sep_row.append("─" * width)
        result.append(" " * padding + (" " * padding).join(sep_row))

    # add data rows
    for row in rows:
        data_row = []
        for i, cell in enumerate(row[:num_cols]):
            data_row.append(str(cell).ljust(col_widths[i]))
        result.append(" " * padding + (" " * padding).join(data_row))

    return "\n".join(result)


def prompt(text: str, default: str | None = None) -> str:
    """Display a prompt and get user input."""
    prompt_text = color(text, "cyan")
    if default:
        prompt_text += color(f" [{default}]", "dim")
    prompt_text += ": "

    write(prompt_text)
    try:
        user_input = input().strip()
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
    print("")
    print(color("━" * 60, "dim"))
    print(color(title, "cyan", "bold"))
    if content:
        print(color("━" * 60, "dim"))
        sys.stdout.write(content + "\n")
    else:
        print(color("━" * 60, "dim"))
