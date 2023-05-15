from __future__ import annotations

import asyncio
import enum
import json
from dataclasses import dataclass, field
from typing import Any, Optional

import structlog

from bench.language.type import (
    Code,
    Data,
    Expectation,
    InterpSymbol,
    Model,
    StatementModifier,
    Task,
    Type,
    TypeFlag,
    TypeNode,
    TypeTag,
    XBlock,
    XSource,
)
from bench.runtime.inference import Modality
from bench.runtime.instruct import (
    InstructionOp,
    SampleDatasetRandom,
    SampleSource,
    fabricate_value,
    instruction_tree_from_symbol,
)
from bench.runtime.model import TextGenerationSettings
from bench.runtime.x import DynamicXBlock, XBuilder, xinput, xoutput, xsettings, xstatic

logger = structlog.get_logger(__name__)


class BuildErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    RUN = 1, "Error running user code"
    CONFIG = 2, "Invalid configuration"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class BuildError(ValueError):
    def __init__(
        self,
        _t: BuildErrorType,
        symbol: Optional[InterpSymbol],
        cause: Optional[Exception] = None,
    ):
        self.type = _t
        self.symbol = symbol
        self.cause = cause
        super().__init__(self.type.description)


@dataclass(repr=False)
class XEmit:
    """Generate X blocks for models with dynamic code to manage dynamic values."""

    async def __call__(self) -> XBlock | DynamicXBlock | list[XBlock | DynamicXBlock]:
        raise NotImplementedError


def xemit(func):
    # just forward to dataclass(repr=False, slots=True)
    return dataclass(repr=False, slots=True)(func)


@dataclass(repr=False)
class TaskPlan:
    task: Task
    model: Model
    modality: Modality
    base_settings: Optional[dict[str, Any]] = None
    sources: list[SampleSource] = field(default_factory=dict)
    emits: list[XEmit] = field(default_factory=list)

    def __str__(self):
        return f"task={self.task}, model={self.model}, modality={self.modality}, sources={len(self.sources)}, targets={len(self.emits)}"

    def __repr__(self):
        return f"<TaskPlan {self}>"

    def source(self, source: SampleSource):
        self.sources.append(source)

    def emit(self, *target: XEmit | list[XEmit]):
        self.emits.extend(target)


@dataclass(repr=False)
class BuildPlan:
    id: int
    models: list[Model]
    # finetunes: list[Finetune] (soon)
    task_plans: list[TaskPlan] = field(default_factory=list)

    def __str__(self):
        return f"models={self.models}, task_plans={self.task_plans}"

    def __repr__(self):
        return f"<BuildPlan {self}>"


async def build_task_implementation(task: Task, model: Model) -> Code:
    """Build the implementation for a task using some model."""
    # TODO @Broken: consider context length in X prompt planning/building
    if not task.outputs:
        # cannot build task without output, should be caught before this
        raise BuildError(BuildErrorType.CONFIG)
    instruction, tree = instruction_tree_from_symbol(task)
    expectations: list[Expectation] = [
        i.node for i in instruction.walk() if i.op == InstructionOp.ExpectationDefinition
    ]
    data_samples: list[Data] = [
        i.node for i in instruction.walk() if i.op == InstructionOp.SampleData
    ]
    # clarity output label as task completion if we don't have a structured output
    plan = TaskPlan(task=task, model=model, modality=Modality.GenerateText)
    # map output type
    output_label = "Output"
    # this is obviously hacky and suboptimal and will be replaced
    plan.emit(
        XEmitSystem(),
        XEmitTypeExplanation(
            type=task.type,
            type_label=output_label,
            include_descriptions=True,
            recursive=True,
        ),
    )
    if expectations:
        plan.emit(XEmitExpectations(task_label=task.name, expectations=expectations))
    for dataset in data_samples:
        if len(dataset) > 0:
            plan.emit(
                XEmitSamples(
                    source=SampleDatasetRandom(dataset, count=3, seed=0),
                    task_label=task.name,
                    positive=dataset.modifier == StatementModifier.LIKE,
                )
            )
    plan.emit(
        XEmitTask(task=task),
        XEmitTypeSample(type=task.type, type_label=output_label),
    )
    if task.type.inputs:
        plan.emit(XEmitInput())
    plan.emit(
        # TODO @Broken: adjust & tune generation settings
        XEmitSettings(TextGenerationSettings(temperature=0.5, max_tokens=512, top_p=1.0)),
        XEmitOutput(type=task.type, type_label=f"Output for task {task.name}"),
    )

    return await build_task_plan(plan)


async def build_task_plan(plan: TaskPlan) -> Code:
    xbuilder = XBuilder(
        name=plan.task.name,
        type=plan.task.type,
        model=plan.model,
        modality=plan.modality,
    )
    emissions = await asyncio.gather(*[emit() for emit in plan.emits])
    for emit in emissions:
        if isinstance(emit, list):
            xbuilder.extend(emit)
        else:
            xbuilder.append(emit)
    return xbuilder.to_symbol()


@dataclass(repr=False)
class XEmitSystem(XEmit):
    """Emits the system message about general expectations for JSON."""

    message: str = (
        "You are a precise and helpful assistant."
        " Perform the given tasks following the instructions to produce outputs."
        " Only output valid JSON (literal, array or object) as per the type schemas."
    )

    async def __call__(self) -> XBlock:
        return xstatic(self.message, XSource.System)


@xemit
class XEmitTask(XEmit):
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
class XEmitExpectations(XEmit):
    """Emits the expectation exactly as written"""

    task_label: str
    expectations: list[Expectation]

    async def __call__(self) -> XBlock:
        expectation_strs = [
            f" - {expectation.name}: {expectation.description}" for expectation in self.expectations
        ]
        return xstatic(
            f"For task {self.task_label}, consider:\n" + "\n".join(expectation_strs),
            XSource.Developer,
        )


@xemit
class XEmitSamples(XEmit):
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
class XEmitTypeExplanation(XEmit):
    """Emits the type exactly as written"""

    type: Type
    type_label: Optional[str]
    include_descriptions: bool
    recursive: bool

    async def __call__(self) -> XBlock:
        unexplained_types = [(self.type_label or self.type.name, self.type)]

        def _render_description(d: str):
            return " # " + d if d and self.include_descriptions else ""

        def _render_simple_type(t: TypeNode):
            if t.flags & TypeFlag.IsNullable:
                return _render_simple_type(t.type_nodes[0]) + "?"
            return t.reference.name if t.reference else t.tag.value

        el_strs = []
        while unexplained_types:
            label, type = unexplained_types.pop()
            if type.tag == TypeTag.ENUM:
                el_str = f"\n{label} enum:{_render_description(type.description)}\n"
                for choice in type.type_nodes:
                    el_str += f"- {choice.name}{_render_description(choice.description)}\n"
            elif type.tag == TypeTag.STRUCT:
                el_str = f"\n{label} struct:{_render_description(type.description)}\n"
                for child in type.type_nodes:
                    el_str += f"- {child.name}: {_render_simple_type(child)}{_render_description(child.description)}\n"
            elif type.flags & TypeFlag.IsArray:
                el_str = f"\n{label} array of {_render_simple_type(type)}{_render_description(type.description)}\n"
            else:
                el_str = f"{label}: {_render_simple_type(type)} {_render_description(type.description)}\n"
            el_strs.append(el_str)

            if self.recursive:
                for child in type.type_nodes or []:
                    if isinstance(child.reference, TypeNode):
                        unexplained_types.append((child.reference.name, child.reference))
                    elif child.tag in (TypeTag.STRUCT, TypeTag.ENUM, TypeTag.UNION):
                        unexplained_types.append((child.name, child))

        el_str = f"Schemas:\n{''.join(el_strs)}".strip()
        return xstatic(el_str, XSource.Developer)


@xemit
class XEmitTypeSample(XEmit):
    """Emits a single sample of the given type (default to fabricate)"""

    type: Type
    type_label: Optional[str]
    value: Any = None

    async def __call__(self) -> list[XBlock]:
        fabricated_sample = self.value or fabricate_value(self.type)
        sample_declaration = xstatic(
            f"Example {self.type_label or self.type.name}:",
            XSource.System,
        )
        sample = xstatic(json.dumps(fabricated_sample, sort_keys=True), XSource.Developer)
        return [sample_declaration, sample]


@xemit
class XEmitInput(XEmit):
    """Emits the code to input the given type"""

    type_label: str = "Input"
    path: str = ""

    @staticmethod
    def impute_input(input: XBlock, value: Any):
        import json

        input.value = json.dumps(value, sort_keys=True)

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        input_declaration = xstatic(f"{self.type_label}:", XSource.System)
        input = xinput(None, path=self.path)
        return [input_declaration, DynamicXBlock(input, self.impute_input)]


@xemit
class XEmitOutput(XEmit):
    """Emits the code to request and read generated output of the given type"""

    type: Type
    type_label: str = "Output"
    path: str = ""

    @staticmethod
    def parse_output(output: XBlock):
        import json
        import re

        # escape/try to parse the output if needed (handles trivial model confusions)
        value = output.value.strip()
        if not value.startswith("{"):
            # sometimes the model prefixes the output with some explanation, find the { ... }
            value = re.compile(r"\{.*?}", re.DOTALL).search(value)
            if value:
                value = value.group(0)
            else:
                raise ValueError(f"model generated invalid X output: {output.value}")

        # escape strings with multiline content
        # these aren't technically valid JSON, but they're very useful for models
        def sub_multiline_str(match):
            # replace line breaks with \n escape sequence
            modified_string = match.group(1).replace("\n", "\\n").replace("\r", "")
            return f'"{modified_string}"'

        value = re.compile(r'"(.*?)(?<!\\)"', re.DOTALL).sub(sub_multiline_str, value)

        try:
            ret = json.loads(value)
            if output.path:
                ret = ret[output.path]
            return ret
        except Exception as e:
            raise ValueError(f"model generated invalid X output: {e}") from e

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        output_keys = ", ".join(t.name for t in self.type.outputs)
        output_request = xstatic(
            f"{self.type_label} - JSON object with keys [{output_keys}], start with {{",
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
