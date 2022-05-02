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
    Tuple,
    Type,
    TypeVar,
    Union,
    cast,
)

import docstring_parser
import numpy as np
from PIL.Image import Image as PILImage
from pydantic.typing import ForwardRef, evaluate_forwardref

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


@dataclass
class _Type(abc.ABC):
    pass


@dataclass
class ArtifactType:
    type: str


@dataclass
class ModelType(_Type):
    input_spec: Union[RecordSpec, RecordType]
    output_spec: Union[RecordSpec, RecordType]


@dataclass
class DatasetType(_Type):
    record_spec: Union[RecordType, RecordSpec]


@dataclass
class _Spec(abc.ABC):
    # name may be empty but not None if not set
    name: str
    # description may be empty but not None if not set
    description: str


@dataclass
class FieldSpec(_Spec):
    type: FieldType


@dataclass
class ArtifactSpec(_Spec):
    type: str


@dataclass
class ModelSpec(_Spec):
    input_spec: RecordSpec
    output_spec: RecordSpec


@dataclass
class DatasetSpec(_Spec):
    record_spec: RecordSpec


@dataclass
class RecordSpec(_Spec):
    type: RecordTypeStrict


@dataclass
class ConfigSpec(_Spec):
    type: ConfigTypeStrict


RecordType = Mapping[str, Union[FieldType, FieldSpec]]
RecordTypeStrict = Mapping[str, FieldSpec]

AnyType = Union[FieldType, RecordType, ModelType, DatasetType]
AnySpec = Union[FieldSpec, RecordSpec, ModelSpec, DatasetSpec]

ConfigType = Mapping[str, Union[AnyType, AnySpec]]
ConfigTypeStrict = Mapping[str, AnySpec]

ArtifactSetType = Mapping[str, ArtifactType]
ArtifactSetSpec = Mapping[str, ArtifactSpec]


def reduce_to_record_type(
    spec: Mapping[str, AnySpec], ignore_invalid: bool = False
) -> RecordTypeStrict:
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


def convert_to_record_type(
    spec: Mapping[str, Union[AnyType, AnySpec]]
) -> RecordTypeStrict:
    spec = convert_to_spec(spec)
    spec = reduce_to_record_type(spec)
    return spec


def convert_to_record_spec(
    spec: Union[RecordSpec, Mapping[str, Union[AnyType, AnySpec]]]
) -> RecordSpec:
    if isinstance(spec, RecordSpec):
        converted_spec = convert_to_record_type(spec.type)
        spec = RecordSpec(
            name=spec.name, description=spec.description, type=converted_spec
        )
        return spec
    else:
        converted_spec = convert_to_record_type(spec)
        spec = RecordSpec(name="", description="", type=converted_spec)
        return spec


def convert_to_config_type(
    spec: Mapping[str, Union[AnyType, AnySpec]]
) -> ConfigTypeStrict:
    spec = convert_to_spec(spec)
    # no special logic for config spec yet
    return spec


def convert_to_config_spec(
    spec: Union[ConfigSpec, Mapping[str, Union[AnyType, AnySpec]]]
) -> ConfigSpec:
    if isinstance(spec, ConfigSpec):
        converted_spec = convert_to_spec(spec.type)
        spec = ConfigSpec(
            name=spec.name, description=spec.description, type=converted_spec
        )
        return spec
    else:
        converted_spec = convert_to_spec(spec)
        spec = ConfigSpec(name="", description="", type=converted_spec)
        return spec


def convert_to_spec(
    spec: Mapping[str, Union[AnyType, AnySpec]]
) -> Mapping[str, AnySpec]:
    converted_spec: dict[str, AnySpec] = {}
    for key, value in spec.items():
        converted_spec[key] = _type_to_spec(
            key=key, description="", value=value, ignore_spec=True
        )
    return converted_spec


def _impl_type_to_type(value: Type, ignore_unknown: bool = False) -> AnyType:
    from bench.dataset.base import DatasetHandler
    from bench.model.base import ModelHandler
    from bench.utils.record import RecordBatch

    # default implementation types to their generic spec types
    impl_type_to_type: List[Tuple[Type, AnyType]] = [
        (DatasetHandler, DatasetType(record_spec={})),
        (ModelHandler, ModelType(input_spec={}, output_spec={})),
        (RecordBatch, {}),
    ]
    for impl_type, spec_type in impl_type_to_type:
        if value == impl_type or issubclass(value, impl_type):
            return spec_type

    if ignore_unknown:
        return value
    else:
        raise ValueError(f"unknown type {value}")


def _type_to_spec(
    key: str,
    description: str,
    value: Union[AnyType, AnySpec],
    ignore_spec: bool = False,
) -> AnySpec:
    # Some value types may be referred to by their implementation types rather than
    # by their spec/type types (e.g. DatasetHandler instead of DatasetType/DatasetSpec).
    # This maps implementation types to the spec types we expect here.
    if isinstance(value, type):
        value = _impl_type_to_type(value, ignore_unknown=True)

    if isinstance(value, _Spec):
        if ignore_spec:
            # mypy thinks this is a redundant cast, but also complains if it's not here
            return cast(AnySpec, value)  # type: ignore
        else:
            raise ValueError(f"type {value} is already a spec type")
    elif isinstance(value, dict):  # RecordType
        return RecordSpec(
            name=key, description=description, type=convert_to_record_type(value)
        )
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
        # At this point, we can't be sure that 'value' is an appropriate type.
        # But as validation for specs is separate from conversion,
        # we will just ignore this potential error here to be caught later.
        return FieldSpec(name=key, description=description, type=value)  # type: ignore


def infer_name(func: Callable) -> str:
    name: str = func.__qualname__
    if name.endswith(".__init__"):
        name = name.split(".")[0]
    return name


def infer_description(func: Callable) -> Optional[str]:
    """Infers the description of the given callable from its docstring"""
    if func.__doc__ is None:
        return None
    parsed_docstring = docstring_parser.parse(func.__doc__)
    return parsed_docstring.short_description


def infer_output_type(func: Callable) -> Optional[Type]:
    signature: inspect.Signature = _get_typed_signature(func)
    return_type = signature.return_annotation
    if return_type == signature.empty:
        return None
    else:
        return return_type


def infer_config_spec(func: Callable) -> ConfigSpec:
    """Infers the full spec from the given callable's signature and docs"""
    name = infer_name(func)
    description = infer_description(func) or ""
    config_type = infer_config_type(func)
    return ConfigSpec(name=name, description=description, type=config_type)


def infer_config_type(func: Callable) -> ConfigTypeStrict:
    """Infers the config type from the given callable's signature and docs"""
    signature: inspect.Signature = _get_typed_signature(func)
    spec: dict[str, AnySpec] = {}
    for name, param in signature.parameters.items():
        if name == "self":
            # Only class functions have '.' in their qualified name (like Class.func)
            if "." not in func.__qualname__:
                raise ValueError(
                    f"non-class function {get_qualified_name(func)} has parameter 'self'"
                )

            # ignore self in class functions
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

    # patch unspecified descriptions from docstring if available
    if func.__doc__:
        parsed_docstring = docstring_parser.parse(func.__doc__)
        for doc_param in parsed_docstring.params:
            spec_value: Optional[AnySpec] = spec.get(doc_param.arg_name)  # type: ignore
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
    param: inspect.Parameter, global_namespace: dict[str, Any]
) -> Any:
    """Gets resolved type annotations for a parameter"""
    # Note: In Python 3.10, we should be able to replace this resoluton logic
    #  with https://docs.python.org/3/library/inspect.html#inspect.get_annotations
    annotation = param.annotation
    if isinstance(annotation, str):
        forward_ref = ForwardRef(annotation)
        annotation = evaluate_forwardref(
            forward_ref, global_namespace, global_namespace
        )
    return annotation
