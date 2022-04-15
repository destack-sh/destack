from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Optional, Union

# TODO @Feature: come up with proper spec system
#  Should be used for fields (of records and functions), datasets and models.
#  Should be easy to use and to the most part easy to infer from type annotations.


@dataclass
class ComplexType:
    pass


@dataclass
class Category(ComplexType):
    categories: Dict[Any, str]


@dataclass
class Span(ComplexType):
    pass


FieldType = Union[int, str]


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
