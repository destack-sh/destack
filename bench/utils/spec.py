from __future__ import annotations

import abc
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    Dict,
    List,
    NewType,
    Optional,
    Type,
    TypeVar,
    Union,
)

import numpy as np
from PIL.Image import Image as PILImage

if TYPE_CHECKING:
    NDArray = np.ndarray
else:
    NDArray = Any

Text = NewType("Text", str)
Audio = TypeVar("Audio", bytes, NDArray)
Image = TypeVar("Image", bytes, NDArray, PILImage)
Video = TypeVar("Video", bytes, NDArray)
Json = NewType("Json", dict)

FieldTypePrimitive = Union[str, int, float, Text, Audio, Image, Video, Json]
FieldType = Type[
    Union[FieldTypePrimitive, List[FieldTypePrimitive], Dict[str, FieldTypePrimitive]]
]

ArtifactType = NewType("ArtifactType", str)

# TODO @Feature: come up with proper spec system
#  Should be used for fields (of records and functions), datasets and models.
#  Should be easy to use and to the most part easy to infer from type annotations.


@dataclass
class ModelType:
    input_spec: RecordSpec
    output_spec: RecordSpec


@dataclass
class DatasetType:
    record_spec: RecordSpec


@dataclass
class _Spec(abc.ABC):
    name: str
    description: str


@dataclass
class FieldSpec(_Spec):
    type: FieldType


@dataclass
class ArtifactSpec(_Spec):
    type: Optional[ArtifactType]


@dataclass
class ModelSpec(ModelType, _Spec):
    pass


@dataclass
class DatasetSpec(DatasetType, _Spec):
    pass


RecordSpec = Dict[str, Union[FieldType, FieldSpec]]
ConfigSpec = Dict[str, Union[FieldType, FieldSpec, ModelSpec, DatasetSpec]]
ArtifactSetSpec = Dict[str, ArtifactSpec]
