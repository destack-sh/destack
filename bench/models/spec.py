from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Union


@dataclass
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
