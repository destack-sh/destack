from __future__ import annotations

from abc import ABC
from typing import Iterator, Optional

from bench.models.record import Record, RecordBatch
from bench.models.spec import ConfigSpec, RecordSpec


class FunctionBase(ABC):
    # could auto-infer this from function constructor in some cases via inspect
    config_spec: ConfigSpec

    @property
    def input_spec(self) -> Optional[RecordSpec]:
        raise NotImplementedError

    @property
    def output_spec(self) -> Optional[RecordSpec]:
        raise NotImplementedError


class Transform(FunctionBase, ABC):
    @property
    def input_spec(self) -> RecordSpec:
        raise NotImplementedError

    @property
    def output_spec(self) -> RecordSpec:
        raise NotImplementedError

    def __call__(self, record: Record) -> Record:
        raise NotImplementedError


class BatchTransform(FunctionBase, ABC):
    def __call__(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class Producer(FunctionBase, ABC):
    def __call__(self) -> Iterator[Record]:
        raise NotImplementedError


class Multiplier(FunctionBase, ABC):
    def __call__(self, record: Record) -> Iterator[Record]:
        raise NotImplementedError


class Predicate(FunctionBase, ABC):
    @property
    def output_spec(self) -> Optional[RecordSpec]:
        return None

    def __call__(self, record: Record) -> bool:
        raise NotImplementedError


class BiPredicate(FunctionBase, ABC):
    @property
    def output_spec(self) -> Optional[RecordSpec]:
        return None

    def __call__(self, record: Record, reference_record: Record) -> bool:
        raise NotImplementedError


class Reducer(FunctionBase, ABC):
    def __call__(self, records: RecordBatch) -> Record:
        raise NotImplementedError
