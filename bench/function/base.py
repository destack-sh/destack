from __future__ import annotations

from abc import ABC
from dataclasses import dataclass
from typing import Any, Dict, Iterator, List, Optional, Union

Record = Dict[str, Any]


# Interface inspired by HF's Dataset object, allowing for column-based and streaming DS.
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
    name: str
    description: str
    input_spec: RecordSpec
    output_spec: RecordSpec


@dataclass
class DatasetSpec:
    name: str
    description: str
    record_spec: RecordSpec


RecordSpec = Dict[str, FeatureSpec]
ConfigSpec = Dict[str, Union[FeatureSpec, ModelSpec, DatasetSpec]]


@dataclass
class Dataset:
    spec: Dict[str, FeatureSpec]
    records: List[Record]


@dataclass
class DatasetView:
    dataset: Dataset


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


class Map(Function, ABC):
    @property
    def input_spec(self) -> RecordSpec:
        raise NotImplementedError

    @property
    def output_spec(self) -> RecordSpec:
        raise NotImplementedError

    def __call__(self, record: Record) -> Record:
        raise NotImplementedError


class ModelRunner(Map, ABC):
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


class BatchMap(Function, ABC):
    def __call__(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class Capability:
    sub_capabilities: List[Capability]
    related_tests: List[Test]
    related_maps: List[Map]


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
    supplier: Union[Supplier, DatasetView]
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
