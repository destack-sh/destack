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
    Record,
    Task,
    Type,
    TypeNode,
)
from bench.language.typer import fabricate_value
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

    @property
    def target_symbol(self) -> Dataset:
        return self.target_dataset


@source
class SampleSourceModelGenerator(SampleSource):
    """Generates a dataset of the given type and count"""

    type: Type
    count: int
    seed: int

    async def __call__(self) -> Dataset:
        # TODO @Broken: generate with model
        target_dataset = anonymous_dataset(self.type, self.count)
        for i in range(self.count):
            target_dataset.records[i].data = fabricate_value(self.type)
        return target_dataset

    @property
    def target_symbol(self) -> Dataset:
        return self.target_dataset
