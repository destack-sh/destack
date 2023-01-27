import re
import uuid
from dataclasses import dataclass
from typing import Any, AsyncGenerator, NamedTuple, Union

from bench.language import TypeNode
from bench.runtime.inference import Inference
from bench.runtime.type import (
    DecoderStepSettings,
    DynamicPrompt,
    ModelInstance,
    PromptSettings,
    TextGeneration,
)

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
    # what comes after this hole
    next_exit: bool = False
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
        return f'PromptStatic(content="{part.content}")'
    elif isinstance(part, PromptVariable):
        return f'PromptVariable(name="{part.name}", value=None, type={part.type})'
    elif isinstance(part, PromptHole):
        next_static_str = f'"{part.next_static}"' if part.next_static else "None"
        next_variable_str = f'"{part.next_variable}"' if part.next_variable else "None"
        return (
            f"PromptHole("
            f'name="{part.name}", '
            f"type={part.type}, "
            f"next_static={next_static_str}, "
            f"next_variable={next_variable_str}, "
            f"next_exit={part.next_exit}"
            f")"
        )
    elif isinstance(part, PromptExit):
        return f"PromptExit(value={part.value})"
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

    # fuse adjacent static parts
    parsed = _fuse_adjacent_static(parsed)

    # determine what terminates holes
    for i, source_p in enumerate(parsed):
        if isinstance(source_p, str):
            continue
        if not isinstance(source_p.part, PromptHole):
            continue
        # set 'next' static or variable if immediately following
        next_part = parsed[i + 1] if i + 1 < len(parsed) else None
        if isinstance(next_part, str):
            continue  # source line
        elif isinstance(next_part.part, PromptStatic):
            source_p.part.next_static = next_part.part.content
        elif isinstance(next_part.part, SourcePromptPart):
            source_p.part.next_variable = next_part.part.name
        elif isinstance(next_part.part, PromptExit) or next_part.part is None:
            source_p.part.next_exit = True

    # render source lines
    parsed_lines = []
    for source_p in parsed:
        if isinstance(source_p, str):
            parsed_lines.append(source_p)
        else:
            parsed_lines.append(source_p.indent + "yield " + render_part(source_p.part))
    python_code = "\n".join(parsed_lines)
    return DynamicPrompt(python_code, None)  # TODO @Incomplete: set settings


def _fuse_adjacent_static(
    parts: list[Union[str, SourcePromptPart]]
) -> list[Union[str, SourcePromptPart]]:
    """Fuses any list of adjacent static prompt parts together in order."""
    fused = []
    for source_p in parts:
        if isinstance(source_p, str):
            fused.append(source_p)
        elif isinstance(source_p.part, PromptStatic):
            if (
                fused
                and isinstance(fused[-1], SourcePromptPart)
                and isinstance(fused[-1].part, PromptStatic)
            ):
                fused_content = fused[-1].part.content + source_p.part.content
                fused[-1] = SourcePromptPart(fused[-1].indent, PromptStatic(fused_content))
            else:
                fused.append(source_p)
        else:
            fused.append(source_p)
    parts = fused
    return parts


class InferenceContext:
    """Append-only context for decoder inference."""

    def __init__(self, settings: PromptSettings, inference: Inference):
        self.settings = settings
        self.inference: Inference | None = inference
        self.parts: list[str] = []
        self.running_length = 0
        self.generated_parts: dict[int, TextGeneration] = {}
        self.id = uuid.uuid4()

    def __str__(self):
        return self.id

    def __repr__(self):
        return f"<InferenceContext {self.id}>"

    @property
    def model(self) -> ModelInstance:
        return self.settings.model

    def append(self, part: str):
        self.parts.append(part)
        self.running_length += len(part)

    @property
    def current_prompt(self) -> str:
        return "".join(self.parts)

    async def generate(self, step: DecoderStepSettings) -> str:
        if self.inference is None:
            raise RuntimeError("inference context is closed")
        prefix = self.current_prompt
        generation = await self.inference.generate(prefix, step)
        self.generated_parts[self.running_length] = generation
        self.append(generation.text)
        return generation.text

    def close(self):
        self.inference.end()
        self.inference = None


async def run_bpl_stepwise(prompt: AsyncGenerator[PromptPart, None], ctx: InferenceContext) -> Any:
    """Runs inference on a BPL-generated prompt, filling holes step-by-step."""
    async for part in prompt:
        if isinstance(part, PromptExit):
            return part.value
        elif isinstance(part, PromptStatic):
            ctx.append(part.content)
        elif isinstance(part, PromptVariable):
            ctx.append(str(part.value))  # TODO @Incomplete: cast to part.type representation
        elif isinstance(part, PromptHole):
            # get completion that satisfies this hole
            # terminate if full valid value for hole and/or separator token
            if part.next_static:
                stop = [part.next_static]
            elif part.next_variable:
                raise NotImplementedError
            elif part.next_exit:
                stop = None
            # TODO @Incomplete: fill prompt hole
            step_settings = DecoderStepSettings()
            next_generation = await ctx.generate(step_settings)


async def run_bpl_backtracking(
    prompt: AsyncGenerator[PromptPart, None], ctx: InferenceContext
) -> Any:
    """Runs inference on a BPL-generated prompt, filling holes in one go and backtracking the parser."""
    last_hole: PromptHole | None = None
    async for part in prompt:
        if isinstance(part, PromptExit):
            return part.value
        elif isinstance(part, PromptStatic):
            ctx.append(part.content)
        elif isinstance(part, PromptVariable):
            ctx.append(str(part.value))  # TODO @Incomplete: cast to part.type representation
        elif isinstance(part, PromptHole):
            last_hole = part
            break  # time to generate
    if last_hole is None:
        return None  # nothing to do

    # generate the prompt completion
    raise NotImplementedError  # TODO @Incomplete: generate prompt completion
