from __future__ import annotations

import asyncio
import itertools
import json
import re
import typing
from dataclasses import asdict, dataclass, is_dataclass
from typing import Any, Optional

import structlog

from bench.language.reconstruct import render_statement
from bench.language.type import (
    Data,
    Expectation,
    InterpSymbol,
    LiteralValue,
    StatementModifier,
    Task,
    Type,
    TypeTag,
    XBlock,
    XBlockContent,
    XKind,
    XSource,
)
from bench.language.typer import check_type
from bench.runtime.inference import SETTINGS_CLS_BY_MODALITY, Modality
from bench.runtime.instance import (
    AsyncCodeInstance,
    ModelInstance,
    Session,
    TaskInstance,
    TypeInstance,
)
from bench.runtime.instruct import (
    InstructionOp,
    SampleDatasetRandom,
    SampleSource,
    fabricate_value,
    instruction_tree_from_symbol,
)
from bench.runtime.model import TextGenerationSettings

logger = structlog.get_logger(__name__)


class XGenerationError(ValueError):
    pass


@dataclass(repr=False)
class XEmit:
    """Generate X blocks for models with dynamic code to manage dynamic values."""

    async def __call__(self) -> XBlock | DynamicXBlock | list[XBlock | DynamicXBlock]:
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


async def build_task_implementation(
    task: TaskInstance, model: ModelInstance, session: Session
) -> AsyncCodeInstance:
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
    x = XBuilder(task=task, model=model, modality=Modality.GenerateText)
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
        XTypeSample(type=task.type, type_label="Output", is_output=True),
    )
    if task.type.inputs:
        x.emit(XInput(type=task.type))
    x.emit(
        # TODO @Broken: adjust & tune generation settings
        XEmitSettings(TextGenerationSettings(temperature=0.5, max_tokens=512, top_p=1.0)),
        XOutputText(type=task.type, type_label=f"output for task {task.name}"),
    )

    return await x.build(task, model, session)


class XBuilder:
    """Build a structured X prompt."""

    def __init__(self, task: TaskInstance, model: ModelInstance, modality: Modality):
        self.task = task
        self.model = model
        self.modality = modality
        self.emits: list[XEmit] = []

    def emit(self, *emits: XEmit):
        self.emits.extend(emits)

    async def build(
        self, task: TaskInstance, model: ModelInstance, session: Session
    ) -> AsyncCodeInstance:
        emissions = await asyncio.gather(*[emit() for emit in self.emits])
        emissions = list(
            itertools.chain.from_iterable([e] if not isinstance(e, list) else e for e in emissions)
        )

        xblocks = []
        dynamic_inputs: dict[int, XInputHandler] = {}
        output_handler: XOutputHandler | None = None
        # reduce to single settings since that's what most models support right now
        settings: Any | None = None
        for x in emissions:
            if isinstance(x, DynamicXBlock):
                if x.xblock.kind == XKind.Input:
                    dynamic_inputs[len(xblocks)] = x.handler
                    xblocks.append(x.xblock)
                elif x.xblock.kind == XKind.Output:
                    if output_handler:
                        raise RuntimeError("cannot have multiple output handlers")
                    output_handler = x.handler
                    # not added to xblocks since it's not a real xblock
                else:
                    raise RuntimeError(f"cannot have dynamic x block of kind {x.xblock.kind}")
            elif x.kind == XKind.Settings:
                if settings:
                    raise RuntimeError("cannot have multiple settings")
                settings_cls = SETTINGS_CLS_BY_MODALITY[self.modality]
                settings = settings_cls(**x.value)
            else:
                xblocks.append(x)

        async def _invoke_task(
            *args, retries: int = None, cache: bool = None, **kwargs
        ) -> dict[str, LiteralValue]:
            # TODO @Feature: should definitely retry on invalid output with error message
            inputs = {**kwargs}  # combine inputs from args/kwargs
            for input_t, input in zip(self.task.type.inputs, args):
                inputs[input_t.name] = input
            # copy x blocks to impute dynamic inputs
            xblocks_copy = [xblock.copy() for xblock in xblocks]
            # apply dynamic inputs
            for i, impute in dynamic_inputs.items():
                impute(xblocks_copy[i], inputs)
            try:
                outputs = await model.inference(self.modality, xblocks_copy, settings)
            except Exception as e:
                raise XGenerationError("model backend failed") from e
            return output_handler(outputs)

        _invoke_task.__name__ = self.task.name
        return AsyncCodeInstance(
            id=task.id,
            task=task,
            name=self.task.name,
            type=self.task.type,
            type_nodes=self.task.type_nodes,
            session=session,
            tracer=session.tracer,
            code_callable=_invoke_task,
            transform=None,
            tag=TypeTag.FUNCTION,
        )


@dataclass(repr=False)
class XSystem(XEmit):
    """Emits the system message about general expectations for JSON."""

    message: str = (
        "You are a precise and concise assistant."
        " Perform the given tasks following the instructions to the letter."
        " Output valid JSON as dictated by the type schema."
    )

    async def __call__(self) -> XBlock:
        return xstatic(self.message, XSource.System)


@xemit
class XTask(XEmit):
    """Emits the task exactly as written"""

    task: Task
    task_label: str = None
    include_description: bool = True

    async def __call__(self) -> XBlock:
        text = f"Task {self.task_label or self.task.name}:"
        if self.include_description:
            text += f" {self.task.description}"
        return xstatic(text, XSource.Developer)


@xemit
class XExpectations(XEmit):
    """Emits the expectation exactly as written"""

    task_label: str
    expectations: list[Expectation]

    async def __call__(self) -> XBlock:
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

    async def __call__(self) -> XBlock:
        dataset = await self.source()
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

    async def __call__(self) -> XBlock:
        bench_lines = []
        for node in self.type.walk(include_references=True):
            if node.reference is not None:
                continue  # skip the link
            if node.tag in (TypeTag.STRUCT, TypeTag.FUNCTION, TypeTag.ENUM, TypeTag.UNION):
                line = render_statement(node.source, include_content=node.tag != TypeTag.FUNCTION)
                bench_lines.append(line)
        bench_str = "\n\n".join(bench_lines)
        schema_str = f"Type schemas to adhere to:\n{bench_str}".strip()
        return xstatic(schema_str, XSource.Developer)


@xemit
class XTypeSample(XEmit):
    """Emits a single sample output of the given type (default to fabricate)"""

    type: Type
    type_label: Optional[str]
    is_output: bool

    async def __call__(self) -> list[XBlock]:
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

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
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
        # escape/try to parse the output if needed (handles trivial model confusions)
        value = output.strip()
        if not value.startswith("{"):
            # sometimes the model prefixes the output with some explanation, find the { ... }
            value = re.compile(r"\{.*}", re.DOTALL).search(value)
            if value:
                value = value.group(0)
            else:
                raise XGenerationError(f"output does not contain JSON object: {output}")

        # escape strings with multiline content
        # these aren't technically valid JSON, but they're very useful for model output
        def sub_multiline_str(match):
            # replace line breaks with \n escape sequence
            modified_string = match.group(1).replace("\n", "\\n").replace("\r", "")
            return f'"{modified_string}"'

        value = re.compile(r'"(.*?)(?<!\\)"', re.DOTALL).sub(sub_multiline_str, value)

        try:
            ret = json.loads(value)
            check_type(ret, self.type, is_output=True)
            return ret
        except Exception as e:
            raise XGenerationError(f"output is invalid for {self.type}: {e}") from e

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        output_keys = ", ".join(t.name for t in self.type.outputs)
        output_request = xstatic(
            f"Generate {self.type_label} given the inputs and instructions - a JSON object with keys [{output_keys}], starting with {{",
            XSource.System,
        )
        output = xoutput(None, path=self.path)
        return [output_request, DynamicXBlock(output, self.parse_output)]


@xemit
class XEmitSettings(XEmit):
    settings: Any

    async def __call__(self) -> XBlock:
        return xsettings(self.settings)

    @property
    def sources(self) -> list[InterpSymbol]:
        return []
