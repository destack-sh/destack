from __future__ import annotations

import abc
import inspect
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    List,
    Mapping,
    NewType,
    Optional,
    Type,
    TypeVar,
    Union,
    cast,
)

import docstring_parser
import numpy as np
from PIL.Image import Image as PILImage
from pydantic.typing import Dict, ForwardRef, evaluate_forwardref

from bench.utils.registry import get_qualified_name

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
    Union[
        FieldTypePrimitive, List[FieldTypePrimitive], Mapping[str, FieldTypePrimitive]
    ]
]

ArtifactType = NewType("ArtifactType", str)

# TODO @Feature: come up with proper spec system
#  Should be used for fields (of records and functions), datasets and models.
#  Should be easy to use and to the most part easy to infer from type annotations.


@dataclass
class _Type(abc.ABC):
    pass


@dataclass
class ModelType(_Type):
    input_spec: RecordSpec
    output_spec: RecordSpec


@dataclass
class DatasetType(_Type):
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


# TODO @Cleanup: should RecordType exist (in addition to RecordSpec)?
#  Why not just FieldType (which already has a dict if circular definitions worked)?
RecordType = Mapping[str, FieldType]
RecordSpec = Mapping[str, Union[FieldType, FieldSpec]]
RecordSpecStrict = Mapping[str, FieldSpec]
AnyType = Union[FieldType, RecordType, ModelType, DatasetType]
AnySpec = Union[FieldSpec, RecordSpec, ModelSpec, DatasetSpec]
ConfigSpec = Mapping[str, Union[AnyType, AnySpec]]
ConfigSpecStrict = Mapping[str, AnySpec]
ArtifactSetSpec = Mapping[str, ArtifactSpec]


def reduce_to_record_spec(
    spec: Mapping[str, AnySpec], ignore_invalid: bool = False
) -> RecordSpecStrict:
    record_spec = {
        value.name: value for value in spec.values() if isinstance(value, FieldSpec)
    }
    if not ignore_invalid:
        bad_specs = {
            value for value in spec.values() if not isinstance(value, FieldSpec)
        }
        if bad_specs:
            raise ValueError(f"invalid value spec for record spec: {bad_specs}")
    return record_spec


def convert_to_record_spec(
    spec: Mapping[str, Union[AnyType, AnySpec]]
) -> RecordSpecStrict:
    spec = convert_to_spec(spec)
    spec = reduce_to_record_spec(spec)
    return spec


def convert_to_config_spec(
    spec: Mapping[str, Union[AnyType, AnySpec]]
) -> ConfigSpecStrict:
    spec = convert_to_spec(spec)
    # no special logic for config spec yet
    return spec


def _impl_type_to_type(value: Type) -> AnyType:
    from bench.dataset.base import DatasetHandler
    from bench.model.base import ModelHandler
    from bench.utils.record import Record

    # default implementation types to their generic spec types
    impl_type_to_type: Mapping[Type, AnyType] = {
        DatasetHandler: DatasetType(record_spec={}),
        ModelHandler: ModelType(input_spec={}, output_spec={}),
        Record: {},
    }
    mapped_type = impl_type_to_type.get(value)
    if mapped_type is None:
        raise ValueError(f"unknown type {value}")
    return mapped_type


def _type_to_spec(
    key: str,
    description: str,
    value: Union[AnyType, AnySpec],
    ignore_spec: bool = False,
) -> AnySpec:
    if isinstance(value, _Spec):
        if ignore_spec:
            return cast(AnySpec, value)
        else:
            raise ValueError(f"type {value} is not a spec type")
    elif isinstance(value, dict):  # RecordType
        return convert_to_record_spec(value)
    elif isinstance(value, DatasetType):
        return DatasetSpec(
            name=key,
            description=description,
            record_spec=convert_to_record_spec(value.record_spec),
        )
    elif isinstance(value, ModelType):
        return ModelSpec(
            name=key,
            description=description,
            input_spec=convert_to_record_spec(value.input_spec),
            output_spec=convert_to_record_spec(value.output_spec),
        )
    else:
        return FieldSpec(name=key, description=description, type=value)


def convert_to_spec(
    spec: Mapping[str, Union[AnyType, AnySpec]]
) -> Mapping[str, AnySpec]:
    converted_spec: dict[str, AnySpec] = {}
    for key, value in spec.items():
        converted_spec[key] = _type_to_spec(key=key, description="", value=value)
    return converted_spec


def infer_description(func: Callable) -> Optional[str]:
    """Infers the description of the given callable from its docstring"""
    if func.__doc__ is None:
        return None
    parsed_docstring = docstring_parser.parse(func.__doc__)
    return parsed_docstring.short_description


def infer_config_spec(func: Callable) -> ConfigSpecStrict:
    """Infers the config spec from the given callable's signature and docs"""
    signature: inspect.Signature = _get_typed_signature(func)
    spec: Dict[str, AnySpec] = {}
    for name, param in signature.parameters.items():
        if name == "self":
            # ignore self
            continue
        if param.kind in (param.VAR_POSITIONAL, param.VAR_KEYWORD):
            # TODO @Robustness: raise error in infer_config_spec if using *args/**kwargs?
            continue

        annotation = param.annotation
        if annotation == inspect.Parameter.empty:
            raise ValueError(
                f"function {get_qualified_name(func)} parameter {name} is not type-annotated"
            )
        spec_value = _type_to_spec(key=name, description="", value=annotation)
        spec[name] = spec_value

    # get additional descriptions out of docstring if available
    if func.__doc__:
        parsed_docstring = docstring_parser.parse(func.__doc__)
        for doc_param in parsed_docstring.params:
            spec_value = spec.get(doc_param.arg_name)
            if spec_value is None:
                raise ValueError(
                    f"documented parameter {doc_param} does not exist in {get_qualified_name(func)}"
                )
            if spec_value.description is None:
                spec_value.description = doc_param.description

    return spec


def _get_typed_signature(func: Callable) -> inspect.Signature:
    """Gets fully resolved signature of given callable"""
    signature = inspect.signature(func)
    global_namespace = getattr(func, "__globals__", {})
    typed_params = [
        inspect.Parameter(
            name=param.name,
            kind=param.kind,
            default=param.default,
            annotation=_get_typed_annotation(param, global_namespace),
        )
        for param in signature.parameters.values()
    ]
    typed_signature = inspect.Signature(typed_params)
    return typed_signature


def _get_typed_annotation(
    param: inspect.Parameter, global_namespace: Dict[str, Any]
) -> Any:
    """Gets resolved type annotations for a parameter"""
    annotation = param.annotation
    if isinstance(annotation, str):
        forward_ref = ForwardRef(annotation)
        annotation = evaluate_forwardref(
            forward_ref, global_namespace, global_namespace
        )
    return annotation
