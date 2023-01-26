import re
from dataclasses import dataclass
from typing import Any, NamedTuple


@dataclass
class DynamicPrompt:
    pragmas: dict[str, Any]
    python_code: str


@dataclass
class PromptFragment:
    text: str


@dataclass
class PromptExit:
    value: Any


# Bench prompt language is really just python with pragmas and lonely strings as prompt emits.
# This is transformed into python code that can be executed with some metadata.
PRAGMA_REGEX = re.compile(r"^pragma\((?P<value>.*)\)$", re.MULTILINE)
EMIT_REGEX = re.compile(r"^(?P<indent> *)\"(?P<value>.*?)\"$")
RETURN_REGEX = re.compile(r"^(?P<indent> *)return (?P<value>.*)$")
EMIT_VARIABLE_REGEX = re.compile(r"\{(?P<value>.*?)\}")
EMIT_HOLE_REGEX = re.compile(r"\[(?P<value>.*?)\]")

PromptHole = NamedTuple("Hole", [("name", str), ("type", str)])


def parse_bpl_hole(hole: str) -> PromptHole:
    name, type = hole.split(":", 1)
    return PromptHole(name, type.strip())


def parse_bpl(bpl: str) -> DynamicPrompt:
    """
    Transforms a bench prompt language statement into a python statement,
     noting pragmas, imputed emits and replaced returns.
    """
    lines = bpl.splitlines()
    python_lines = []
    lineno = 0
    pragmas_raw = {}
    encountered_non_pragma = False

    while lineno < len(lines):
        line = lines[lineno]
        lineno += 1

        # handle pragmas
        match = PRAGMA_REGEX.match(line)
        if match:
            if encountered_non_pragma:
                raise ValueError(f"pragmas must be at the top: {line}")
            pragma = match.group("value")
            for arg in pragma.split(","):
                key, value = arg.strip().split("=", 1)
                pragmas_raw[key] = value
            python_lines.append(f"# pragma: {pragma}")
            continue
        encountered_non_pragma = True

        # transform returns
        match = RETURN_REGEX.match(line)
        if match:
            indent = match.group("indent")
            value = match.group("value")
            python_lines.append(indent + f"yield PromptExit({value})")
            continue

        # transform emits
        match = EMIT_REGEX.match(line)
        if match:
            indent = match.group("indent")
            value = match.group("value")
            holes = [parse_bpl_hole(h) for h in EMIT_HOLE_REGEX.findall(value)]

            filled_hole_names = [hole.name for hole in holes]
            if len(filled_hole_names) != len(set(filled_hole_names)):
                raise ValueError(f"hole names must be unique: {line}")

            yield_str = f"yield PromptFragment(f{value})"
            if holes:
                python_line = indent + ", ".join(filled_hole_names) + f" = {yield_str}"
            else:
                python_line = indent + yield_str
            python_lines.append(python_line)
            continue

        # pass through anything else
        python_lines.append(line)

    pragmas = pragmas_raw
    python_code = "\n".join(python_lines)
    return DynamicPrompt(pragmas, python_code)
