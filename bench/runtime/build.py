from __future__ import annotations

import enum
import json
import re
import typing
from copy import deepcopy
from dataclasses import asdict, dataclass, is_dataclass
from json import JSONDecodeError
from typing import Any, Optional
from uuid import UUID

import structlog

from bench.language.reconstruct import render_statement
from bench.language.type import (
    Build,
    Data,
    Expectation,
    InterpSymbol,
    StatementModifier,
    Task,
    Type,
    TypeTag,
    XBlock,
    XBlockContent,
    XKind,
    XSource,
)
from bench.language.typer import check_type, map_value
from bench.runtime.inference import SETTINGS_CLS_BY_MODALITY, Modality
from bench.runtime.instruct import (
    InstructionOp,
    SampleDatasetRandom,
    SampleSource,
    fabricate_value,
    instruction_tree_from_symbol,
)
from bench.runtime.models import TextGenerationSettings
from bench.utils.utils import DotDict

logger = structlog.get_logger(__name__)

if typing.TYPE_CHECKING:
    from bench.runtime.instance import ModelInstance, Session, TaskInstance, TypeInstance
    from bench.runtime.interp import InterpModule


class XGenerationErrorType(enum.StrEnum):
    TIMEOUT = "timeout"
    INVALID_JSON = "invalid_json"
    INVALID_TYPE = "invalid_type"
    UNKNOWN = "unknown"


class XGenerationError(ValueError):
    def __init__(self, type: XGenerationErrorType, message: str, path: str = None):
        super().__init__(message)
        self.type = type
        self.path = path


@dataclass(repr=False)
class XEmit:
    """Generate X blocks for models with dynamic code to manage dynamic values."""

    def __call__(self) -> XBlock | DynamicXBlock | list[XBlock | DynamicXBlock]:
        raise NotImplementedError


def xemit(func):
    # just forward to dataclass(repr=False, slots=True)
    return dataclass(repr=False, slots=True)(func)


ValueT = typing.TypeVar("ValueT", bound=typing.Any)


def xsettings(
    value: ValueT, source: XSource = XSource.System, path: str = None
) -> XBlockContent[ValueT]:
    if is_dataclass(value):
        value = asdict(value)
    return XBlockContent(kind=XKind.Settings, source=source, value=value, path=path)


def xstatic(
    value: ValueT, source: XSource = XSource.Developer, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Static, source=source, value=value, path=path)


def xinput(
    value: ValueT, source: XSource = XSource.User, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Input, source=source, value=value, path=path)


def xoutput(
    value: ValueT, source: XSource = XSource.Model, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Output, source=source, value=value, path=path)


XInputHandler = typing.Callable[[XBlock, typing.Any], None]
XOutputHandler = typing.Callable[[Any], typing.Any]


@dataclass
class DynamicXBlock:
    xblock: XBlockContent
    handler: XInputHandler | XOutputHandler


def get_default_builds(interp: InterpModule) -> list[Build]:
    return [
        Build(
            name="balanced",
            models=[
                interp.symbol("openai.std.text.gpt3"),
                interp.symbol("anthropic.std.text.claude-instant"),
            ],
        ),
    ]


def build_task_implementation(
    task: TaskInstance, model: ModelInstance, session: Session
) -> XPrompt:
    """Build the implementation for a task using some model."""
    # TODO @Broken: consider context length in X prompt planning/building
    if not task.outputs:
        raise RuntimeError(f"cannot build task {task} without output")
    instruction, tree = instruction_tree_from_symbol(task)
    expectations: list[Expectation] = [
        i.node for i in instruction.walk() if i.op == InstructionOp.ExpectationDefinition
    ]
    data_samples: list[Data] = [
        i.node for i in instruction.walk() if i.op == InstructionOp.SampleData
    ]
    x = XPrompt(task=task, model=model, modality=Modality.GenerateText, session=session)
    x.emit(
        XSystem(),
        XTypeSchema(
            type=task.type,
            type_label="Output",
            recursive=True,
        ),
    )
    if expectations:
        x.emit(XExpectations(task_label=task.name, expectations=expectations))
    for dataset in data_samples:
        if len(dataset) > 0:
            x.emit(
                XSamples(
                    source=SampleDatasetRandom(dataset, count=3, seed=0),
                    task_label=task.name,
                    positive=dataset.modifier == StatementModifier.LIKE,
                )
            )
    x.emit(
        XTask(task=task),
        XTypeFabricatedSample(type=task.type, type_label="Output", is_output=True),
    )
    if task.type.inputs:
        x.emit(XInput(type=task.type))
    x.emit(
        # TODO @Broken: adjust & tune generation settings
        XEmitSettings(TextGenerationSettings(temperature=0.5, max_tokens=512, top_p=1.0)),
        XOutputText(type=task.type, type_label=f"output for task {task.name}"),
    )

    return x


class XPrompt:
    """Build a structured X prompt."""

    def __init__(
        self, task: TaskInstance, model: ModelInstance, modality: Modality, session: Session
    ):
        self.task = task
        self.model = model
        self.modality = modality
        self.session = session
        # the actual prompt
        self.blocks: list[XBlock] = []
        self.input_handlers: dict[int, XInputHandler] = {}
        self.output_handler: XOutputHandler | None = None
        self.settings: Any | None = None

    def __str__(self):
        return f"{self.model.fqn} {self.modality} ({len(self.blocks)})"

    def __repr__(self):
        return f"<XPrompt {self}>"

    def copy(self) -> XPrompt:
        x = XPrompt(self.task, self.model, self.modality, self.session)
        x.blocks = [b.copy() for b in self.blocks]
        x.input_handlers = {**self.input_handlers}
        x.output_handler = self.output_handler
        x.settings = deepcopy(self.settings)
        return x

    def emit(self, *emits: XEmit):
        blocks = []
        for emit in emits:
            x = emit()
            if isinstance(x, list):
                blocks.extend(x)
            else:
                blocks.append(x)
        for x in blocks:
            if isinstance(x, DynamicXBlock):
                if x.xblock.kind == XKind.Input:
                    self.input_handlers[len(self.blocks)] = x.handler
                    self.blocks.append(x.xblock)
                elif x.xblock.kind == XKind.Output:
                    if self.output_handler:
                        raise RuntimeError("cannot have multiple output handlers")
                    self.output_handler = x.handler
                    # not added to blocks since it's not a real xblock
                else:
                    raise RuntimeError(f"cannot have dynamic x block of kind {x.xblock.kind}")
            elif x.kind == XKind.Settings:
                if self.settings:
                    raise RuntimeError("cannot have multiple settings")
                settings_cls = SETTINGS_CLS_BY_MODALITY[self.modality]
                self.settings = settings_cls(**x.value)
            else:
                self.blocks.append(x)
        return self

    async def __call__(self, *args, cache: bool = None, timeout: float = None, **kwargs) -> Any:
        inputs = {**kwargs}  # combine inputs from args/kwargs
        for input_t, input in zip(self.task.type.inputs, args):
            inputs[input_t.name] = input
        # copy x blocks to impute dynamic inputs
        blocks_copy = [xblock.copy() for xblock in self.blocks]
        # apply dynamic inputs
        for i, impute in self.input_handlers.items():
            impute(blocks_copy[i], inputs)
        try:
            outputs = await self.model.inference(
                self.modality, blocks_copy, self.settings, cache=cache, timeout=timeout
            )
        except TimeoutError as e:
            raise XGenerationError(XGenerationErrorType.TIMEOUT, "model backend timed out") from e
        except Exception as e:
            raise XGenerationError(XGenerationErrorType.UNKNOWN, "model backend failed") from e
        return self.output_handler(outputs)


@dataclass(repr=False)
class XSystem(XEmit):
    """Emits the system message about general expectations for JSON."""

    message: str = (
        "You are a precise and concise assistant."
        " Perform the given tasks following the instructions to the letter."
        " If the task is underspecified or ambiguous, guess without asking."
        " Output valid JSON as dictated by the type schema."
    )

    def __call__(self) -> XBlock:
        return xstatic(self.message, XSource.System)


@xemit
class XTask(XEmit):
    """Emits the task exactly as written"""

    task: Task
    task_label: str = None
    include_description: bool = True

    def __call__(self) -> XBlock:
        text = f"Task {self.task_label or self.task.name}:"
        if self.include_description:
            text += f" {self.task.description}"
        return xstatic(text, XSource.Developer)


@xemit
class XExpectations(XEmit):
    """Emits the expectation exactly as written"""

    task_label: str
    expectations: list[Expectation]

    def __call__(self) -> XBlock:
        expectation_strs = [
            f" - {expectation.name}: {expectation.description}" for expectation in self.expectations
        ]
        return xstatic(
            f"For task {self.task_label}, you must consider:\n" + "\n".join(expectation_strs),
            XSource.Developer,
        )


@xemit
class XSamples(XEmit):
    """Emits fewshot examples in a specific format"""

    source: SampleSource
    task_label: str
    positive: bool

    def __call__(self) -> XBlock:
        dataset = self.source()
        if len(dataset) == 0:
            raise RuntimeError(f"expected at least one sample for {self.task.name}")
        if self.positive:
            preamble = f"Good examples of {self.task_label}"
        else:
            preamble = f"Bad examples of {self.task_label} (don't do this!)"
        data_str = "\n".join(json.dumps(record.data, sort_keys=True) for record in dataset.records)
        return xstatic(f"{preamble}:\n{data_str}", XSource.Developer)


@xemit
class XTypeSchema(XEmit):
    """Emits the type exactly as written"""

    type: Type
    type_label: Optional[str]
    recursive: bool

    def __call__(self) -> XBlock:
        bench_lines = []
        seen_types: set[UUID] = set()  # TODO @Cleanup: seen types dedup shouldn't be needed
        for node in self.type.walk(include_references=True):
            if node.id in seen_types:
                continue
            seen_types.add(node.id)
            if node.reference is not None:
                continue  # skip the link
            if node.tag in (TypeTag.STRUCT, TypeTag.FUNCTION, TypeTag.ENUM, TypeTag.UNION):
                line = render_statement(node.source, include_content=node.tag != TypeTag.FUNCTION)
                bench_lines.append(line)
        bench_str = "\n\n".join(bench_lines)
        schema_str = f"Type schemas you must adhere to. Do not invent new fields or options. ? = optional:\n{bench_str}".strip()
        return xstatic(schema_str, XSource.Developer)


@xemit
class XTypeFabricatedSample(XEmit):
    """Emits a single sample output of the given type (default to fabricate)"""

    type: Type
    type_label: Optional[str]
    is_output: bool

    def __call__(self) -> list[XBlock]:
        fabricated_sample = fabricate_value(self.type, is_output=self.is_output)
        sample_declaration = xstatic(
            f"Example {self.type_label or self.type.name} with fabricated values:",
            XSource.System,
        )
        sample = xstatic(json.dumps(fabricated_sample, sort_keys=True), XSource.Developer)
        return [sample_declaration, sample]


@xemit
class XInput(XEmit):
    """Emits the code to input the given type"""

    type: Type
    type_label: str = "Input"
    path: str = ""

    def impute_input(self, input: XBlock, value: Any) -> None:
        input.value = json.dumps(value, sort_keys=True)

    def __call__(self) -> list[XBlock | DynamicXBlock]:
        input_declaration = xstatic(f"{self.type_label}:", XSource.System)
        input = xinput(None, path=self.path)
        return [input_declaration, DynamicXBlock(input, self.impute_input)]


@xemit
class XOutputText(XEmit):
    """Emits the code to request and read generated output of the given type"""

    type: TypeInstance
    type_label: str = "Output"
    path: str = ""

    def parse_output(self, output: str):
        from bench.runtime.instance import instantiate_py_value_flat

        # escape/try to parse the output if needed (handles trivial model confusions)
        value = output.strip()
        if not value.startswith("{"):
            # sometimes the model prefixes the output with some explanation, find the { ... }
            value = re.compile(r"\{.*}", re.DOTALL).search(value)
            if value:
                value = value.group(0)
            else:
                raise XGenerationError(
                    XGenerationErrorType.INVALID_JSON,
                    f"output does not contain JSON object: {output}",
                )

        # escape strings with multiline content
        # these aren't technically valid JSON, but they're very useful for model output
        def sub_multiline_str(match):
            # replace line breaks with \n escape sequence
            modified_string = match.group(1).replace("\n", "\\n").replace("\r", "")
            return f'"{modified_string}"'

        value = re.compile(r'"(.*?)(?<!\\)"', re.DOTALL).sub(sub_multiline_str, value)

        try:
            ret = json.loads(value)
            ret = map_value(
                ret,
                self.type,
                map_v=instantiate_py_value_flat,
                is_output=True,
                ignore_outer_map=True,
            )
            check_type(ret, self.type, is_output=True)
            ret = DotDict(**ret)  # behave like a typed dict
            return ret
        except Exception as e:
            if isinstance(e, JSONDecodeError):
                error_type = XGenerationErrorType.INVALID_JSON
            elif isinstance(e, TypeError):
                error_type = XGenerationErrorType.INVALID_TYPE
            else:
                error_type = XGenerationErrorType.UNKNOWN
            raise XGenerationError(
                type=error_type, message=f"output is invalid for {self.type}: {e}", path=None
            ) from e

    def __call__(self) -> list[XBlock | DynamicXBlock]:
        output_keys = ", ".join(t.name for t in self.type.outputs)
        output_request = xstatic(
            f"Generate {self.type_label} given the inputs and instructions - a JSON object with keys [{output_keys}], starting with {{",
            XSource.System,
        )
        output = xoutput(None, path=self.path)
        return [output_request, DynamicXBlock(output, self.parse_output)]


@xemit
class XConsiderError(XEmit):
    """Emits a note about an error that occured previously"""

    error: XGenerationError

    def __call__(self) -> XBlock:
        error_str = str(self.error)
        # remove (source=...) from error message
        error_str = re.sub(r"\(source=.+\)", "", error_str)
        return xstatic(f"Note: please avoid mistakes like this: {error_str}", XSource.System)


@xemit
class XEmitSettings(XEmit):
    settings: Any

    def __call__(self) -> XBlock:
        return xsettings(self.settings)

    @property
    def sources(self) -> list[InterpSymbol]:
        return []
