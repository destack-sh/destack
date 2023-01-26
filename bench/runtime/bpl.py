import re
from dataclasses import dataclass
from typing import Any, AsyncGenerator, NamedTuple, Union

from bench.language import TypeNode
from bench.runtime.type import DynamicPrompt, PromptSettings

UNSET = object()


@dataclass(frozen=True, slots=True)
class PromptStatic:
    content: str


@dataclass(frozen=True, slots=True)
class PromptVariable:
    name: str
    value: Union[Any, UNSET]
    type: Union[str, TypeNode, None]


@dataclass(slots=True)
class PromptHole:
    name: str
    type: Union[str, TypeNode, None]
    # what comes after this hole (None means not that)
    next_static: Union[str, None] = None
    next_variable: Union[str, None] = None


@dataclass(frozen=True, slots=True)
class PromptExit:
    value: Union[Any, None, UNSET]


PromptFragment = PromptStatic | PromptVariable | PromptHole
PromptPart = PromptFragment | PromptExit
SourcePromptPart = NamedTuple("SourcePromptPart", [("indent", str), ("part", PromptPart)])


def render_part(part: PromptPart) -> str:
    """Renders a prompt part to a string that evaluates to the same part."""
    if isinstance(part, PromptStatic):
        return f'PromptStatic("{part.content}")'
    elif isinstance(part, PromptVariable):
        return f'PromptVariable("{part.name}", None, {part.type})'
    elif isinstance(part, PromptHole):
        next_static_str = f'"{part.next_static}"' if part.next_static else "None"
        next_variable_str = f'"{part.next_variable}"' if part.next_variable else "None"
        return f'PromptHole("{part.name}", {part.type}, {next_static_str}, {next_variable_str})'
    elif isinstance(part, PromptExit):
        return f"PromptExit({part.value})"
    raise ValueError(f"unexpected part type: {part}")


# 'builtins' required in context to execute the generated python code
REQUIRED_BUILTINS = {
    "PromptStatic": PromptStatic,
    "PromptVariable": PromptVariable,
    "PromptHole": PromptHole,
    "PromptExit": PromptExit,
}

# Bench prompt language is really just python with pragmas and lonely strings as prompt emits.
# This is transformed into python code that can be executed with some metadata.
# pragmas like pragma(n=1, z=Banana())
PRAGMA_REGEX = re.compile(r"^pragma\((?P<value>.*)\)$", re.MULTILINE)
# returns like return or return 5
RETURN_REGEX = re.compile(r"^(?P<indent> *)return ?(?P<value>.*)$")
# emits like "hello" or "hello {name}" or "hello [name: string]"
EMIT_REGEX = re.compile(r"^(?P<indent>\s*)\"(?P<value>.*?)\"\s*$")
# emit variables like {name} (classic f-string)
EMIT_VARIABLE_REGEX = re.compile(r"\{(?P<value>.*?)(: (?P<type>.*?))?}")
# emit holes like [name: string]
EMIT_HOLE_REGEX = re.compile(r"\[(?P<name>.*?)(: (?P<type>.*?))?]")


def split_fragment(fragment: str) -> list[PromptFragment]:
    """Splits an emitted fragment into its content and hole parts."""
    pos = 0
    while pos < len(fragment):
        hole_match = EMIT_HOLE_REGEX.search(fragment, pos)
        if hole_match:
            if pos != hole_match.start():
                yield PromptStatic(fragment[pos : hole_match.start()])
            yield PromptHole(hole_match.group("name"), hole_match.group("type"))
            pos = hole_match.end()
            continue

        variable_match = EMIT_VARIABLE_REGEX.search(fragment, pos)
        if variable_match:
            if pos != variable_match.start():
                yield PromptStatic(fragment[pos : variable_match.start()])
            yield PromptVariable(variable_match.group("value"), UNSET, variable_match.group("type"))
            pos = variable_match.end()
            continue

        break  # nothing matched
    if pos < len(fragment):
        yield PromptStatic(fragment[pos:])


def parse_bpl(bpl: str, context: dict[str, Any]) -> DynamicPrompt:
    """
    Transforms a bench prompt language statement into a python statement,
     noting pragmas, imputed emits and replaced returns.
    """
    lines = bpl.splitlines()
    parsed: list[Union[str, SourcePromptPart]] = []
    pragmas = {}

    for line in lines:
        # handle pragmas
        match = PRAGMA_REGEX.match(line)
        if match:
            pragma = match.group("value")
            for arg in pragma.split(","):
                key, value = arg.strip().split("=", 1)
                if key in pragmas:
                    raise ValueError(f"duplicate pragma: {key} at {line}")
                pragmas[key] = eval(value, context)
            parsed.append(f"# pragma: {pragma}")
            continue

        # transform returns
        match = RETURN_REGEX.match(line)
        if match:
            indent = match.group("indent")
            value = match.group("value") or UNSET
            parsed.append(SourcePromptPart(indent, PromptExit(value)))
            continue

        # transform emits
        match = EMIT_REGEX.match(line)
        if match:
            indent = match.group("indent")
            value = match.group("value")
            for fragment in split_fragment(value):
                parsed.append(SourcePromptPart(indent, fragment))
            continue

        # pass through anything else
        parsed.append(line)

    # second pass to determine what terminates holes
    for i, part in enumerate(parsed):
        if isinstance(part, str):
            continue
        if not isinstance(part.part, PromptHole):
            continue
        # set 'next' static or variable if immediately following
        next_part = parsed[i + 1] if i + 1 < len(parsed) else None
        if isinstance(next_part, str):
            continue  # source line
        elif isinstance(next_part.part, PromptStatic):
            part.part.next_static = next_part.part.content
        elif isinstance(next_part.part, SourcePromptPart):
            part.part.next_variable = next_part.part.name

    # render source lines
    parsed_lines = []
    for part in parsed:
        if isinstance(part, str):
            parsed_lines.append(part)
        else:
            parsed_lines.append(part.indent + "yield " + render_part(part.part))
    python_code = "\n".join(parsed_lines)
    return DynamicPrompt(python_code, None)  # TODO @Incomplete: set settings


class InferenceContext:
    def __init__(self, settings: PromptSettings):
        self.settings = settings
        self.parts: list[str] = []
        self.running_length = 0

    def append(self, part: str):
        self.parts.append(part)
        self.running_length += len(part)

    @property
    def current_prompt(self) -> str:
        return "".join(self.parts)


async def run_bpl(
    generator: AsyncGenerator[PromptFragment | PromptExit, None], settings: PromptSettings
) -> Any:
    """Runs inference on the given BPL-based generator."""
    ctx = InferenceContext(settings)
    async for part in generator:
        if isinstance(part, PromptExit):
            return part.value
        elif isinstance(part, PromptStatic):
            ctx.append(part.content)
        elif isinstance(part, PromptVariable):
            value_str = str(part.value)
            # TODO @Incomplete: cast to part.type representation
            ctx.append(value_str)
        elif isinstance(part, PromptHole):
            # get completion that satisfies this hole
            # terminate if full valid value for hole and/or separator token
            pass  # TODO @Incomplete: fill prompt hole

    completion = await settings.model.handle.complete(ctx.current_prompt)
