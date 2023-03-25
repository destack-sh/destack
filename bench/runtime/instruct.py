import enum
import random
import uuid
from dataclasses import dataclass, field
from typing import Union

import structlog

from bench.language.type import (
    Code,
    Dataset,
    Expectation,
    InterpSymbol,
    Model,
    Record,
    Task,
    Type,
    TypeNode,
    TypeTag,
)
from bench.language.typer import fabricate_value
from bench.runtime.run import instantiate, run
from bench.runtime.type import Modality, TextGenerationSettings
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)

Expect = Union[Task, Code, Dataset, Expectation]


class InstructionType(enum.StrEnum):
    """The kind of instruction expressed in a symbol (or sub-symbol)."""

    TypeDefinition = "type_definition"
    TaskDefinition = "task_definition"
    ExpectationDefinition = "expectation_definition"
    SampleData = "sample_data"
    SampleCode = "sample_code"
    EvaluateCode = "evaluate_code"
    CheckCode = "check_code"


@dataclass(repr=False, slots=True)
class InstructionNode:
    type: InstructionType
    node: InterpSymbol | TypeNode
    id: uuid.UUID
    children: list["InstructionNode"] = field(default_factory=list)


@dataclass(repr=False, slots=True)
class InstructionTree:
    nodes: dict[uuid.UUID, InstructionNode]
    root: InstructionNode | None = None


def instruction_tree(symbol: InterpSymbol) -> InstructionTree:
    """Build a tree of instructions from a symbol and its referenced symbols (and sub-symbols)."""
    raise NotImplementedError


def source(func):
    return dataclass(repr=False, slots=True)(func)


def anonymous_dataset(type: Type, n_records: int = 0) -> Dataset:
    order_keys = generate_n_keys_between(None, None, n_records)
    records = [Record(order_key=order_key, data={}) for order_key in order_keys]
    return Dataset(
        name="", type=type, type_node=type, description="", records=records, language="jsonl"
    )


@source
class SampleSource:
    async def __call__(self) -> Dataset:
        raise NotImplementedError


@source
class SampleSourceDataset(SampleSource):
    """Samples the given dataset"""

    source_dataset: Dataset
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        target_dataset = anonymous_dataset(self.source_dataset.type, self.count)
        sample_indices = random.sample(range(len(self.source_dataset.records)), self.count)
        for i in sample_indices:
            target_dataset.records[i].data = self.source_dataset.records[i].data
        return target_dataset


@source
class SampleSourceFabricator(SampleSource):
    """
    Generates a dataset of the given type by fabricating values
    TODO @Feature: fabricated values are static, use directed probing strategy!
    """

    type: Type
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        target_dataset = anonymous_dataset(self.type, self.count)
        for i in range(self.count):
            target_dataset.records[i].data = fabricate_value(self.type)
        return target_dataset


@source
class SampleSourceGenerator(SampleSource):
    """Generates a dataset of the given type using a model"""

    type: Type
    model: Model
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        from bench.runtime.build import (  # prevent circular import
            TaskPlan,
            XEmitInput,
            XEmitOutput,
            XEmitSettings,
            XEmitSystem,
            XEmitTask,
            XEmitTypeExplanation,
            XEmitTypeSample,
            do_build_task_plan,
        )

        # manually build the task plan (later this will be in symbolx/bench (?))
        output_type = self.type.deepcopy(keep_id=False)
        output_type.name = "output"
        generation_task_type = Type(
            name="generate examples",
            tag=TypeTag.FUNCTION,
            children=[
                TypeNode(
                    name="input",
                    tag=TypeTag.STRUCT,
                    children=[TypeNode(name="count", tag=TypeTag.NUMBER)],
                ),
                TypeNode(name="output", tag=TypeTag.ARRAY, children=[output_type]),
            ],
        )
        generation_task = Task(
            name="generate examples",
            type=generation_task_type,
            type_node=generation_task_type,
            description="Generate diverse, useful and instructive examples of the given type",
        )
        plan = TaskPlan(task=generation_task, model=self.model, modality=Modality.GenerateText)
        plan.emit(
            XEmitSystem(),
            XEmitTask(task=generation_task),
            XEmitInput(input_type=generation_task.type.input),
            XEmitSettings(
                # TODO @Build: tune model sample generation settings (and adapt to model context size)
                base_settings=TextGenerationSettings(
                    temperature=0.8, max_tokens=2048, top_p=1.0
                ).__dict__
            ),
            XEmitTypeExplanation(
                type=self.type, type_label="Output", include_descriptions=True, recursive=True
            ),
            XEmitTypeSample(type=generation_task_type.output, type_label="Output"),
            XEmitOutput(output_type=generation_task_type.output, output_label="Output"),
        )
        implementation = await do_build_task_plan(plan)
        implementation.context[self.model.name] = self.model
        implementation_instance = instantiate(implementation)

        generated_samples = await run(implementation_instance, {"count": self.count})
        target_dataset = anonymous_dataset(self.type, self.count)
        for i, sample in enumerate(generated_samples):
            target_dataset.records[i].data = sample
        return target_dataset
