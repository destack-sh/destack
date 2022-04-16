from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Dict, List, Optional, Type, Union

import numpy

# TODO @Feature: come up with proper spec system
#  Should be used for fields (of records and functions), datasets and models.
#  Should be easy to use and to the most part easy to infer from type annotations.


@dataclass
class Span:
    start: int
    end: int


if TYPE_CHECKING:
    # TODO @Cleanup: mypy can't handle recursive FieldType definition
    #  We will probably change how field types are defined anyway, so fix this later.
    FieldType = Any
else:
    FieldType = Type[
        Union[str, int, Span, numpy.ndarray, List["FieldType"], Dict[str, "FieldType"]]
    ]


@dataclass
class FeatureSpec:
    name: str
    type: FieldType
    description: Optional[str] = None


@dataclass
class ModelSpec:
    input_spec: RecordSpec
    output_spec: RecordSpec


@dataclass
class DatasetSpec:
    record_spec: RecordSpec


RecordSpec = Dict[str, Union[FieldType, FeatureSpec]]
ConfigSpec = Dict[str, Union[FieldType, FeatureSpec, ModelSpec, DatasetSpec]]
