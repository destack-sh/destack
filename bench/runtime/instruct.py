import random
from dataclasses import dataclass, field
from typing import Union, cast

import structlog

from bench.language.type import Code, Dataset, Expectation, InterpSymbol, Record, Task, Type
from bench.language.typer import fabricate_value
from bench.runtime.run import instantiate, run
from bench.runtime.type import CodeInstance
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)

Expect = Union[Task, Code, Dataset, Expectation]


def gather_expectations(symbol: Type | Expectation | Task) -> list[Expect]:
    expects = []
    if isinstance(symbol, Expectation):
        expects.append(symbol)
    if isinstance(symbol, (Type, Task, Expectation)):
        for child in symbol.expectations:
            if not isinstance(child, Expectation):
                expects.append(child)
            expects.extend(gather_expectations(child))
    return expects


def instruction_source(func):
    # just forward to dataclass(repr=False, slots=True)
    return dataclass(repr=False, slots=True)(func)


def anonymous_dataset(type: Type, n_records: int = 0) -> Dataset:
    order_keys = generate_n_keys_between(None, None, n_records)
    records = [Record(order_key=order_key, data={}) for order_key in order_keys]
    return Dataset(
        name="", type=type, type_node=type, description="", records=records, language="jsonl"
    )


@instruction_source
class InstructionSource:
    async def __call__(self) -> InterpSymbol:
        raise NotImplementedError

    @property
    def target_symbol(self) -> Dataset:
        raise NotImplementedError


@instruction_source
class InstructionSourceSampleDataset(InstructionSource):
    """Samples the given dataset"""

    dataset: Dataset
    count: int
    seed: int
    target_dataset: Dataset = field(init=False)

    def __post_init__(self):
        self.target_dataset = self.target_dataset or anonymous_dataset(self.dataset.type)

    async def __call__(self) -> Dataset:
        sample_indices = random.sample(range(len(self.dataset.records)), self.count)
        for index in sample_indices:
            source = self.dataset.records[index]
            target = Record(order_key=source.order_key, data=source.data)
            self.target_dataset.records.append(target)
        return self.target_dataset

    @property
    def target_symbol(self) -> Dataset:
        return self.target_dataset


@instruction_source
class InstructionSourceSampleCode(InstructionSource):
    """Runs the code to create samples (for like/unlike)"""

    code: Code
    count: int
    seed: int
    target_dataset: Dataset = field(init=False)

    def __post_init__(self):
        self.target_dataset = self.target_dataset or anonymous_dataset(self.code.type)

    async def __call__(self) -> Dataset:
        code_instance = cast(CodeInstance, instantiate(self.code))
        input_samples = await InstructionSourceGenerate(
            type=self.code.type.input, count=self.count, seed=self.seed
        )()
        for sample in input_samples.records:
            output_sample = await run(code_instance, sample.data)
            combined_sample = Record(
                order_key=sample.order_key, data=dict(**sample.data, output=output_sample)
            )
            self.target_dataset.records.append(combined_sample)
        return self.target_dataset

    @property
    def target_symbol(self) -> Dataset:
        return self.target_dataset


@instruction_source
class InstructionSourceGenerate(InstructionSource):
    """Generates a dataset of the given type and count"""

    type: Type
    count: int
    seed: int
    target_dataset: Dataset = field(default=None)

    def __post_init__(self):
        self.target_dataset = self.target_dataset or anonymous_dataset(self.type)

    async def __call__(self) -> Dataset:
        order_keys = generate_n_keys_between(None, None, self.count)
        for i in range(self.count):
            # TODO @Incomplete: generate samples
            data = fabricate_value(self.type)
            record = Record(order_key=order_keys[i], data=data)
            self.target_dataset.records.append(record)
        return self.target_dataset

    @property
    def target_symbol(self) -> Dataset:
        return self.target_dataset
