from __future__ import annotations

import abc
from typing import Any, Dict, List, Union

Record = Dict[str, Any]


def is_record(obj: Any) -> bool:
    return isinstance(obj, dict)


class RecordBatch(abc.ABC):
    """
    An ordered list of Records for unified Record batch processing.
    """

    def __getitem__(self, item: Union[int, slice, str]) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    def __iter__(self):
        raise NotImplementedError

    def __len__(self):
        raise NotImplementedError


class ListRecordBatch(RecordBatch):
    def __init__(self, records: List[Record]):
        self._records = records

    def __getitem__(self, item: Union[int, slice, str]) -> Union[Record, RecordBatch]:
        # TODO @Performance: improve ListRecordBatch indexing and views.
        #  There are probably a thousand better ways of doing this,
        #  see e.g. numpy views, Activeloop Datasets, HuggingFace Datasets, etc.
        if isinstance(item, str):
            return ListRecordBatch([record[item] for record in self])
        elif isinstance(item, slice):
            return ListRecordBatch(self._records[item])
        elif isinstance(item, int):
            return self._records[item]
        else:
            raise TypeError(item)

    def __iter__(self):
        return iter(self._records)

    def __len__(self):
        return len(self._records)
