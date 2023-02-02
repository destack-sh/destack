import ast
import enum
import json
import re
import uuid
from dataclasses import dataclass
from typing import Any, AsyncGenerator, NamedTuple, Union

from bench.language import TypeNode
from bench.language.lex import lex_string
from bench.language.parse import TokenParser, impute_type_reference, parse_type_node_inline
from bench.language.type import InterpSymbol, TypeTag
from bench.language.typer import check_type
from bench.runtime.inference import Inference
from bench.runtime.type import (
    DecoderSettings,
    DynamicPrompt,
    FinishReason,
    ModelInstance,
    PromptSettings,
    TextGeneration,
    TypeInstance,
)

UNSET = object()


@dataclass(frozen=True, slots=True)
class PromptConstant:
    content: str


@dataclass(frozen=True, slots=True)
class PromptVariable:
    name: str
    value: Union[Any, UNSET]
    type: Union[str, TypeNode, None]


@dataclass(slots=True)  # not frozen because we need to set the next_* fields
class PromptHole:
    name: str
    type: Union[str, TypeNode, None]
    # what comes after this hole
    next_exit: bool = False
    next_constant_content: Union[str, None] = None
    next_variable: Union[str, None] = None


@dataclass(frozen=True, slots=True)
class PromptPragmaZoneEnter:
    id: int
    variables: dict[str, Any]
    temperature: Union[float, None] = None
    max_tokens: Union[int, None] = None


@dataclass(frozen=True, slots=True)
class PromptPragmaZoneExit:
    id: int


@dataclass(frozen=True, slots=True)
class PromptExit:
    value: Union[Any, None, UNSET]


PromptFragment = PromptConstant | PromptVariable | PromptHole
PromptPart = PromptFragment | PromptExit
SourcePromptPart = NamedTuple("SourcePromptPart", [("indent", str), ("part", PromptPart)])


def render_part(part: PromptPart) -> str:
    # TODO @Architecture: rendering bpl into strings to parse back seems unnecessary and limited
    #  Why don't we store BPL as a simple AST? We already have type information here.
    #  Does it need to be trivially serializable somewhere?
    """Renders a prompt part to a string that evaluates to the same part."""

    def _render_type(type: str) -> str | None:
        # transform type into a recoverable representation
        # :PrimitiveTypeMap
        if type == "string":
            return "str"
        elif type == "number":
            return "float"
        elif type == "boolean":
            return "bool"
        elif type == "null":
            return "None"
        elif isinstance(type, str):
            return f'_parse_type_inline("{part.type}", source_context)'
        else:
            return None

    if isinstance(part, PromptConstant):
        return f'PromptConstant(content="{part.content}")'
    elif isinstance(part, PromptVariable):
        return (
            f"PromptVariable("
            f'name="{part.name}", '
            f"value={part.name}, "
            f"type={_render_type(part.type)}"
            f")"
        )
    elif isinstance(part, PromptHole):
        next_constant_content_str = (
            f'"{part.next_constant_content}"' if part.next_constant_content else "None"
        )
        next_variable_str = f'"{part.next_variable}"' if part.next_variable else "None"
        return (
            f"PromptHole("
            f'name="{part.name}", '
            f"type={_render_type(part.type)}, "
            f"next_constant_content={next_constant_content_str}, "
            f"next_variable={next_variable_str}, "
            f"next_exit={part.next_exit}"
            f")"
        )
    elif isinstance(part, PromptExit):
        return f"PromptExit(value={part.value})"
    elif isinstance(part, PromptPragmaZoneEnter):
        return (
            f"PromptPragmaZoneEnter("
            f"id={part.id}, "
            f"max_tokens={part.max_tokens}, "
            f"temperature={part.temperature}, "
        )
    elif isinstance(part, PromptPragmaZoneExit):
        return f"PromptPragmaZoneExit(id={part.id})"
    raise ValueError(f"unexpected part type: {part}")


# Bench prompt language is really just python with pragmas and lonely strings as prompt emits.
# This is transformed into python code that can be executed with some metadata.
# That said, using only regexes for parsing is quick and dirty, should use ast.parse later.

# pragmas like pragma(n=1, z=Banana())
PRAGMA_REGEX = re.compile(r"^pragma\((?P<value>.*)\)$", re.MULTILINE)
# pragma zone begin like pragma_zone
PRAGMA_ZONE_BEGIN_REGEX = re.compile(r"^(?P<indent> *)pragma_zone\((?P<value>.*)\)$", re.MULTILINE)
# pragma zone end like pragma_zone_end
PRAGMA_ZONE_END_REGEX = re.compile(r"^(?P<indent> *)pragma_zone_end\(\)$")
# returns like return or return 5
RETURN_REGEX = re.compile(r"^(?P<indent> *)return ?(?P<value>.*)$")
# emits like "hello" or "hello {name}" or "hello [name: string]"
EMIT_REGEX = re.compile(r"^(?P<indent>\s*)\"(?P<value>.*?)\"\s*$")
# emit variables like |{name}| (classic f-string)
EMIT_VARIABLE_REGEX = re.compile(r"\|\{(?P<value>.*?)(: (?P<type>.*?))?}\|")
# emit holes like |[name: string]|
EMIT_HOLE_REGEX = re.compile(r"\|\[(?P<name>\w+?)(: (?P<type>.+?))?]\|")


def split_fragment(fragment: str) -> list[PromptFragment]:
    """Splits an emitted fragment into its content and hole parts."""
    pos = 0
    while pos < len(fragment):
        hole_match = EMIT_HOLE_REGEX.search(fragment, pos)
        if hole_match:
            if pos != hole_match.start():
                yield PromptConstant(fragment[pos : hole_match.start()])
            name = hole_match.group("name")
            type = hole_match.group("type")
            yield PromptHole(name, type)
            pos = hole_match.end()
            continue

        variable_match = EMIT_VARIABLE_REGEX.search(fragment, pos)
        if variable_match:
            if pos != variable_match.start():
                yield PromptConstant(fragment[pos : variable_match.start()])
            value = variable_match.group("value")
            type = variable_match.group("type")
            yield PromptVariable(value, UNSET, type)
            pos = variable_match.end()
            continue

        break  # nothing matched
    if pos < len(fragment):
        s = fragment[pos:]
        yield PromptConstant(s)


def parse_bpl(bpl: str, context: dict[str, Any]) -> DynamicPrompt:
    """
    Transforms a bench prompt language statement into a python code string
    that will produce an iterator of prompt parts (i.e. "dynamic prompt").
    """
    # try parsing as python first to ensure it's valid
    try:
        ast.parse(bpl)
    except SyntaxError as e:
        raise ValueError(f"invalid python: {e}")

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
        # TODO @Accuracy: support pragma zones for granular decoder control
        match = PRAGMA_ZONE_BEGIN_REGEX.match(line)
        if match:
            raise NotImplementedError("pragma zones not implemented")
        match = PRAGMA_ZONE_END_REGEX.match(line)
        if match:
            raise NotImplementedError("pragma zones not implemented")

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

    # determine what terminates holes
    for i, source_p in enumerate(parsed):
        if isinstance(source_p, str):
            continue
        if not isinstance(source_p.part, PromptHole):
            continue
        # set 'next' constant or variable if immediately following
        next_part = parsed[i + 1] if i + 1 < len(parsed) else None
        if isinstance(next_part, str) or next_part is None:
            continue  # source line or end
        elif isinstance(next_part.part, PromptConstant):
            source_p.part.next_constant_content = next_part.part.content
            # accumulate constant content until we hit something else
            for next_next_part in parsed[i + 2 :]:
                if not isinstance(next_next_part, str) and isinstance(
                    next_next_part.part, PromptConstant
                ):
                    source_p.part.next_constant_content += next_next_part.part.content
        elif isinstance(next_part.part, SourcePromptPart):
            source_p.part.next_variable = next_part.part.name
        elif isinstance(next_part.part, PromptExit) or next_part.part is None:
            source_p.part.next_exit = True

    # render source lines
    parsed_lines = []
    for source_p in parsed:
        if isinstance(source_p, str):
            parsed_lines.append(source_p)
        elif isinstance(source_p.part, PromptHole):
            parsed_lines.append(
                source_p.indent + f"{source_p.part.name} = yield " + render_part(source_p.part)
            )
        else:
            parsed_lines.append(source_p.indent + "yield " + render_part(source_p.part))
    python_code = "\n".join(parsed_lines)

    # parse settings from pragmas
    try:
        temperature = float(pragmas["temperature"])
        max_tokens = int(pragmas["max_tokens"])
        model = pragmas["model"]
        stop = pragmas.get("stop", [])
        settings = PromptSettings(
            model=model, temperature=temperature, max_tokens=max_tokens, stop=stop
        )
    except (KeyError, ValueError) as e:
        raise ValueError(f"invalid pragma settings: {e}")

    # parse settings from pragmas
    return DynamicPrompt(python_code, settings)


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
        return f"id={self.id}, parts={len(self.parts)}, length={self.running_length}, settings={self.settings}"

    def __repr__(self):
        return f"<InferenceContext {str(self)}>"

    @property
    def model(self) -> ModelInstance:
        return self.settings.model

    def append(self, part: str):
        self.parts.append(part)
        self.running_length += len(part)

    @property
    def current_block(self) -> str:
        return "".join(self.parts)

    @property
    def remaining_tokens(self) -> int:
        return self.settings.max_tokens - self.running_length

    async def generate(self, step: DecoderSettings) -> str:
        if self.inference is None:
            raise RuntimeError("inference context is closed")
        prefix = self.current_block

        # trim last token if it's a space (not sure if this is the right place)
        ends_in_space = prefix.endswith(" ")
        if ends_in_space:
            prefix = prefix[:-1]
        generation = await self.inference.generate(prefix, step)
        if generation.finish_reason == FinishReason.MAX_TOKENS:
            raise GenerationError(GenerationErrorType.OUT_OF_TOKENS)

        # trim space from generation as well
        # TODO @Cleanup: mangling space for generation messes with generation tokens & logits
        if ends_in_space and generation.text.startswith(" "):
            generation.text = generation.text[1:]
        self.generated_parts[self.running_length] = generation
        self.append(generation.text)
        return generation.text

    def close(self):
        self.inference.end()
        self.inference = None


class GenerationErrorType(enum.Enum):
    INTERNAL = 0, "internal error"
    OUT_OF_TOKENS = 1, "ran out of tokens"
    INVALID_VARIABLE = 2, "invalid variable"
    INVALID_HOLE = 3, "invalid hole"

    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message


class GenerationError(ValueError):
    def __init__(
        self,
        _t: GenerationErrorType,
        part: PromptPart | None = None,
        message_detail: str | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(_t.message, cause)
        self.type = _t
        self.part = part
        self.cause = cause
        self.message_detail = message_detail


async def run_bpl_controlled(
    prompt: AsyncGenerator[PromptPart, None], ctx: InferenceContext
) -> Any:
    """Runs inference on a BPL dynamic prompt, filling holes step-by-step in controlled generation."""

    part = await prompt.asend(None)  # start iteration
    while True:
        if ctx.remaining_tokens <= 0:
            raise GenerationError(GenerationErrorType.OUT_OF_TOKENS, part)

        send_back = None
        if isinstance(part, PromptExit):
            return part.value
        elif isinstance(part, PromptConstant):
            ctx.append(part.content)
        elif isinstance(part, PromptVariable):
            ctx.append(render_variable_repr(part))
        elif isinstance(part, PromptHole):
            # generate to satisfy this hole
            settings = get_hole_decode_settings(
                ctx.remaining_tokens, ctx.settings.temperature, part
            )
            value = await ctx.generate(settings)
            # transform value to target type
            send_back = parse_hole_repr(part, value)
        else:
            raise RuntimeError(f"unexpected prompt part: {part}")

        # send back to prompt and continue
        try:
            part = await prompt.asend(send_back)
        except StopAsyncIteration:
            break


async def run_bpl_speculative(
    prompt: AsyncGenerator[PromptPart, None], ctx: InferenceContext
) -> Any:
    """
    Runs inference on a BPL dynamic prompt, filling ahead once a hole is encountered (speculative).
    Useful for inference where we don't own the decoder loop (like with hosted models)
     as we usually get to fill many holes in one call with minor backtracking to correct.
    """

    # forward mode: build prefix until we hit a hole
    first_unfilled_hole: PromptHole | None = None
    async for part in prompt:
        if isinstance(part, PromptExit):
            return part.value
        elif isinstance(part, PromptConstant):
            ctx.append(part.content)
        elif isinstance(part, PromptVariable):
            ctx.append(render_variable_repr(part))
        elif isinstance(part, PromptHole):
            first_unfilled_hole = part
            break  # time to generate
        else:
            raise RuntimeError(f"unexpected prompt part: {part}")
    if first_unfilled_hole is None:
        return None  # nothing to do

    if ctx.remaining_tokens <= 0:
        raise GenerationError(GenerationErrorType.OUT_OF_TOKENS, first_unfilled_hole)
    # generate to satisfy this (and potentially future) holes
    generated = await ctx.generate(
        DecoderSettings(
            temperature=ctx.settings.temperature,
            max_tokens=ctx.remaining_tokens,
            stop=None,  # no stopping in uncontrolled mode
        )
    )

    # backward mode: match generated text to holes
    # (emulate how controlled step-wise forward would have behaved)
    pos = 0
    part = first_unfilled_hole  # resume where we left off
    while pos < ctx.running_length:
        send_back = None
        if isinstance(part, PromptExit):
            return part.value
        elif isinstance(part, (PromptConstant, PromptVariable)):
            # match constant or variable, rewind speculation if mismatch
            if isinstance(part, PromptConstant):
                expected = part.content
            else:
                expected = render_variable_repr(part)
            actual = generated[pos : pos + len(expected)]
            if actual != expected:
                # TODO @Incomplete: unwind, correct and proceed with forward mode
                raise GenerationError(
                    GenerationErrorType.INVALID_HOLE,
                    part,
                    f"expected constant {expected!r} but got {actual!r} at {pos}",
                )
            pos += len(expected)
        elif isinstance(part, PromptHole):
            # 'decode' from generated text like a decoder would for this hole
            decode = get_hole_decode_settings(len(generated) - pos, ctx.settings.temperature, part)
            actual = generated[pos : pos + decode.max_tokens]
            if decode.stop:  # stop at the first stop of the hole
                min_stop = min(actual.find(s) for s in decode.stop)
                if min_stop >= 0:
                    actual = actual[:min_stop]
            # transform value to target type
            send_back = parse_hole_repr(part, actual)
            pos += len(actual)
        else:
            raise RuntimeError(f"unexpected prompt part: {part}")

        # send back to prompt and continue
        try:
            part = await prompt.asend(send_back)
        except StopAsyncIteration:
            break


def get_hole_decode_settings(
    max_tokens: int, base_temperature: float, part: PromptHole
) -> DecoderSettings:
    stop = []
    if isinstance(part.type, TypeNode) and part.type.tag == TypeTag.UNION:
        # special case to set max length if all union members are string literals
        all_string_literals = all(
            t.tag == TypeTag.LITERAL and isinstance(t.value, str) for t in part.type.children
        )
        if all_string_literals:
            max_tokens = max(len(t.value) for t in part.type.children)
    if part.next_constant_content:
        # TODO @Robustness: limit stop length of next constant content in hole to..?
        stop.append(part.next_constant_content)
    elif part.next_variable:
        raise NotImplementedError  # can't handle?
    return DecoderSettings(
        temperature=base_temperature,
        max_tokens=max_tokens,
        stop=stop,
    )


def parse_hole_repr(part: PromptHole, value: str) -> Any:
    """Parses a string representation of a hole value into its target type."""
    if isinstance(part.type, type):
        value = part.type(value)
    elif isinstance(part.type, TypeNode):
        # note that we don't actually transform the value here, we just validate it
        try:
            # :JsonHole
            value = json.loads(value)
            check_type(value, part.type)  # raises our TypeError
        except (ValueError, TypeError) as e:
            raise GenerationError(GenerationErrorType.INVALID_HOLE, part, cause=e)
    else:
        raise RuntimeError(f"invalid target type: {part}")
    return value


def render_variable_repr(part: PromptVariable) -> str:
    """Renders a variable to a string representation for the prompt (pre-tokenization)."""
    value = part.value
    if part.type is not None:
        # represent target type appropriately
        if isinstance(part.type, type):
            value = part.type(value)
        else:
            raise ValueError(f"invalid target type: {part}")
    # :JsonHole
    value = json.dumps(value, indent=2)
    return value


def _parse_type_inline(type_str: str, source_context: dict[str, InterpSymbol]) -> TypeNode:
    """Resolves (rather naively) a type str using source context"""
    type_node = parse_type_node_inline(TokenParser(lex_string(type_str)), name=None)

    # resolve references in source context
    for node in type_node.walk():
        if isinstance(node.reference, str):
            resolved = source_context.get(node.reference)
            if not isinstance(resolved, TypeInstance):
                # this shouldn't happen
                raise ValueError(f"unknown type reference: {node.reference}")
            node.reference = resolved

    # impute references in-place
    for node in type_node.walk():
        impute_type_reference(node)

    return type_node


# 'builtins' required in context to execute the generated python code
BPL_BUILTINS = {
    "PromptConstant": PromptConstant,
    "PromptVariable": PromptVariable,
    "PromptHole": PromptHole,
    "PromptExit": PromptExit,
    "PromptPragmaZoneEnter": PromptPragmaZoneEnter,
    "PromptPragmaZoneExit": PromptPragmaZoneExit,
    "_parse_type_inline": _parse_type_inline,
    "json": json,
}
