from __future__ import annotations

from abc import ABC
from dataclasses import dataclass
from typing import Any, Dict, Iterator, List, Optional, Union

Record = Dict[str, Any]


class FeatureType:
    pass


@dataclass
class FeatureSpec:
    name: str
    description: str
    type: FeatureType


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


class Supplier(Function, ABC):
    def __call__(self, record: Record) -> Iterator[Record]:
        raise NotImplementedError


class Predicate(Function, ABC):
    @property
    def input_spec(self) -> Optional[RecordSpec]:
        raise NotImplementedError

    @property
    def output_spec(self) -> Optional[RecordSpec]:
        return None

    def __call__(self, record: Record, reference_record: Record) -> bool:
        raise NotImplementedError


@dataclass
class ModelRunner(Map, ABC):
    model: Model


@dataclass
class Lexicon(Dataset):
    pass


@dataclass
class Attack:
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


class Capability:
    sub_capabilities: List[Capability]
    related_tests: List[Test]
    related_augmentations: List[Map]
