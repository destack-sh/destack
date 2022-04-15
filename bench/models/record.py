from __future__ import annotations

from typing import Any, Dict, Union

Record = Dict[str, Any]


# Incomplete interface inspired by Ray's Dataset and HF's Dataset
#  need to allow for column-based and streaming dataset processing.
class RecordBatch:
    def __getitem__(self, index: Union[int, str]) -> Union[Record, RecordBatch]:
        raise NotImplementedError
