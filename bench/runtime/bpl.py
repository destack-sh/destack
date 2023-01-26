import re
from dataclasses import dataclass
from functools import cached_property
from typing import Any


@dataclass
class PromptHole:
    name: str
    type: str
    start: int
    end: int


@dataclass
class DynamicPrompt:
    pragmas: dict[str, Any]
    python_code: str


@dataclass
class PromptFragment:
    content: str

    @cached_property
    def holes(self) -> list[PromptHole]:
        holes = []
        for hole_match in EMIT_HOLE_REGEX.finditer(self.content):
            hole = PromptHole(
                hole_match.group("name"),
                hole_match.group("type"),
                hole_match.start(),
                hole_match.end(),
            )
            holes.append(hole)
        return holes


@dataclass
class PromptExit:
    value: Any


# Bench prompt language is really just python with pragmas and lonely strings as prompt emits.
# This is transformed into python code that can be executed with some metadata.
# pragmas like pragma(n=1, z=Banana())
PRAGMA_REGEX = re.compile(r"^pragma\((?P<value>.*)\)$", re.MULTILINE)
# returns like return or return 5
RETURN_REGEX = re.compile(r"^(?P<indent> *)return ?(?P<value>.*)$")
# emits like "hello" or "hello {name}" or "hello [name: string]"
EMIT_REGEX = re.compile(r"^(?P<indent>\s*)(?P<value>\".*?\")\s*$")
# emit variables like {name} (classic f-string)
EMIT_VARIABLE_REGEX = re.compile(r"\{(?P<value>.*?)\}")
# emit holes like [name: string]
EMIT_HOLE_REGEX = re.compile(r"\[(?P<name>.*?)(: (?P<type>.*?))?]")


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
            value = match.group("value") or "None"
            python_lines.append(indent + f"yield PromptExit({value})")
            continue

        # transform emits
        match = EMIT_REGEX.match(line)
        if match:
            indent = match.group("indent")
            value = match.group("value")
            holes = PromptFragment(value).holes
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
