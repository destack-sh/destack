import sys
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
