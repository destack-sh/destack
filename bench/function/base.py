from __future__ import annotations

from abc import ABC
from dataclasses import dataclass
from typing import Any, Dict, Iterator, List, Optional, Tuple, Union

Record = Dict[str, Any]


# Interface inspired by Ray's Dataset and HF's Dataset
#  need to allow for column-based and streaming dataset processing.
class RecordBatch:
    def __getitem__(self, index: Union[int, str]) -> Union[Record, RecordBatch]:
        raise NotImplementedError


class FieldType:
    pass


@dataclass
class FeatureSpec:
    name: str
    description: str
    type: FieldType


@dataclass
class ModelSpec:
    input_spec: RecordSpec
    output_spec: RecordSpec


@dataclass
class DatasetSpec:
    record_spec: RecordSpec


RecordSpec = Dict[str, FeatureSpec]
ConfigSpec = Dict[str, Union[FeatureSpec, ModelSpec, DatasetSpec]]


@dataclass
class Dataset:
    spec: Dict[str, FeatureSpec]
    records: RecordBatch  # virtual, this will just be a link


@dataclass
class DatasetSlice:  # or DatasetFacet
    dataset: Dataset
    filter: Union[Tuple[int], Predicate]


@dataclass
class Model:
    input_spec: RecordSpec
    output_spec: RecordSpec
    model_files: Dict[str, bytes]


class Function(ABC):
    # could auto-infer this from function constructor in some cases
    config_spec: ConfigSpec

    @property
    def input_spec(self) -> Optional[RecordSpec]:
        raise NotImplementedError

    @property
    def output_spec(self) -> Optional[RecordSpec]:
        raise NotImplementedError


class Transform(Function, ABC):
    @property
    def input_spec(self) -> RecordSpec:
        raise NotImplementedError

    @property
    def output_spec(self) -> RecordSpec:
        raise NotImplementedError

    def __call__(self, record: Record) -> Record:
        raise NotImplementedError


class BatchTransform(Function, ABC):
    def __call__(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class ModelRunner(Transform, ABC):
    model: Model


class Supplier(Function, ABC):
    def __call__(self) -> Iterator[Record]:
        raise NotImplementedError


class Multiplier(Function, ABC):
    def __call__(self, record: Record) -> Iterator[Record]:
        raise NotImplementedError


class Predicate(Function, ABC):
    @property
    def output_spec(self) -> Optional[RecordSpec]:
        return None

    def __call__(self, record: Record) -> bool:
        raise NotImplementedError


class BiPredicate(Function, ABC):
    @property
    def output_spec(self) -> Optional[RecordSpec]:
        return None

    def __call__(self, record: Record, reference_record: Record) -> bool:
        raise NotImplementedError


class Reducer(Function, ABC):
    def __call__(self, records: RecordBatch) -> Record:
        raise NotImplementedError


class Capability:
    sub_capabilities: List[Capability]
    related_tests: List[Test]
    related_transforms: List[Transform]


class Metric:
    data: Record


@dataclass
class Run:
    inputs: RecordBatch
    outputs: RecordBatch
    metrics: List[Metric]


@dataclass
class Attack:
    attack_method: Any  # ?
    tests: List[Test]


@dataclass
class AttackRun:
    attack: Attack
    test_runs: List[TestRun]


@dataclass
class Test:
    supplier: Union[Supplier, DatasetSlice]
    model: Model
    evaluator: Predicate
    expressed_capabilities: List[Capability]


@dataclass
class TestRun:
    test: Test
    run: Run


@dataclass
class TestSuite:
    tests: List[Test]


@dataclass
class TestSuiteRun:
    test_runs: List[TestRun]
