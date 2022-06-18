from __future__ import annotations

import abc
import typing
from typing import Any, Dict, Iterator, List, Union

from bench.utils.spec import FieldType

Record = Union[FieldType, Dict[str, FieldType]]


def is_record(obj: Any) -> bool:
    return isinstance(obj, dict)


class RecordBatch(abc.ABC):
    """
    An ordered list of Records for unified Record batch processing.
    """

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> list[FieldType]:
        ...

    def __getitem__(
        self, index: Union[int, slice, str]
    ) -> Union[Record, RecordBatch, list[FieldType]]:
        raise NotImplementedError

    def __iter__(self) -> Iterator[Record]:
        raise NotImplementedError

    def __len__(self) -> int:
        raise NotImplementedError


class RecordList(RecordBatch):
    """
    An immutable list-backed implementation of a RecordBatch.
    """

    def __init__(self, records: List[Record]):
        self._records = records

    def __str__(self) -> str:
        return str(self._records)

    def __repr__(self) -> str:
        return repr(self._records)

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> list[FieldType]:
        ...

    def __getitem__(
        self, index: Union[int, slice, str]
    ) -> Union[Record, RecordBatch, list[FieldType]]:
        # TODO @Performance: improve RecordList __getitem__.
        #  There are probably a thousand better ways of doing this,
        #  see e.g. numpy views, Activeloop Datasets, HuggingFace Datasets, etc.
        if isinstance(index, int):
            return self._records[index]
        elif isinstance(index, slice):
            return RecordList(self._records[index])
        elif isinstance(index, str):
            return [record[index] for record in self._records]
        else:
            raise TypeError(index)

    def __eq__(self, o: object) -> bool:
        return self._records == o

    def __iter__(self) -> Iterator[Record]:
        return iter(self._records)

    def __len__(self) -> int:
        return len(self._records)
