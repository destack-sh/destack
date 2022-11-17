"""
The spec (short for 'specification') system defines machine- and human-readable schemas for the
primitive types in our system (e.g., records, datasets, models).

The core points are:
 - A field has a value of a (Python) implementation type. Fields may be nested lists, tuples and dicts.
 - The schema for a field type may be its implementation type (implicit), or an explicit
   spec type describing the type with a _Type which includes description on the type.
 - A record is an atomic unit of fields (field or dict[str, field]) that belong together.
 - Records are the basic data unit: datasets are record lists, models and functions process records, etc.
"""

from __future__ import annotations

import enum
import inspect
import types
import typing
from typing import (
    Any,
    Callable,
    List,
    Mapping,
    Optional,
    Tuple,
    Type,
    Union,
    cast,
)

import docstring_parser

# useful for evaluating type signatures, worst case we just vendor it in
# noinspection PyProtectedMember
from pydantic.typing import ForwardRef, evaluate_forwardref

from bench.utils.registry import get_qualified_name


def _impl_type_to_type(
    value: Type, optional: bool, default: Optional[Any], ignore_unknown: bool = False
) -> AnyType:
    from bench.model.base import ModelProvider
    from bench.utils.record import RecordBatch

    # default implementation types to their generic spec types
    impl_type_to_type: List[Tuple[Type, AnyType]] = [
        (RecordBatch, DatasetType(record_spec={}, optional=optional)),
        (ModelProvider, ModelType(input_spec={}, output_spec={}, optional=optional)),
        (RecordBatch, {}),
        *(
            (ptype, ValueType(dtype=dtype, optional=optional, default=default))
            for ptype, dtype in PTYPE_TO_DTYPE.items()
        ),
    ]
    # try exact types first
    for impl_type, spec_type in impl_type_to_type:
        if value == impl_type:
            return spec_type
    # then try subclasses (two steps since e.g. bool is subclass of int)
    for impl_type, spec_type in impl_type_to_type:
        if issubclass(value, impl_type):
            return spec_type

    if issubclass(value, enum.Enum):
        if isinstance(default, enum.Enum):
            default = default.value
        return EnumType(
            values=[item.name for item in value], optional=optional, default=default, ptype=value
        )

    if ignore_unknown:
        return value
    else:
        raise ValueError(f"unknown implementation type: {value}")


def type_to_spec(
    name: str,
    description: str,
    typ: Union[AnyType, AnySpec],
    optional: bool = False,
    default: Optional[Any] = None,
    ignore_spec: bool = False,
) -> AnySpec:
    # If value is already a spec either error or ignore
    if isinstance(typ, _Spec):
        if ignore_spec:
            # mypy thinks this is a redundant cast, but also complains if it's not here
            return cast(AnySpec, typ)  # type: ignore
        else:
            raise ValueError(f"type {typ} is already a spec type")

    # Unwrap and handle union types
    if isinstance(typ, (typing._UnionGenericAlias)):  # type: ignore
        union_types = typ.__args__
        # Optional[x] is secretly Union[x, None]
        if len(union_types) == 2 and union_types[1] == type(None):  # noqa
            # unwrap optional and set as optional
            return type_to_spec(
                name=name,
                description=description,
                typ=union_types[0],
                optional=True,
                default=default,
                ignore_spec=ignore_spec,
            )
        else:  # plain Union
            raise ValueError(f"union types not supported: {typ}")

    # Map collection types (e.g. list[str] to [str])
    if isinstance(typ, (typing._GenericAlias, types.GenericAlias)):  # type: ignore
        typ = typ.__args__

    # Map collection instances (e.g. [str] -> [FieldSpec]
    if isinstance(typ, Mapping):  # RecordType/FieldType is a dict
        typ = convert_to_record_type_spec(typ)  # type: ignore
    elif isinstance(typ, (list, tuple)):
        if len(typ) != 1:
            raise ValueError(f"list-like types must have exactly one element: {typ}")
        typ = [
            type_to_spec(  # type: ignore
                name=name,
                description=description,
                typ=val,
                optional=optional,
                default=default,
                ignore_spec=ignore_spec,
            )
            for val in typ
        ]

    # Map implementation types to spec types
    # Some value types may be referred to by their implementation types rather than
    # by their spec/type types (e.g. DatasetHandler -> DatasetType, int -> ValueType(int64)).
    if isinstance(typ, type):
        # TODO @Robustness: don't ignore unknown types in _impl_type_to_type
        typ = _impl_type_to_type(typ, optional, default, ignore_unknown=True)

    if isinstance(typ, DatasetType):
        return ArtifactSpec(
            name=name,
            description=description,
            type=DatasetType(record_spec=convert_to_record_spec(typ.record_spec)),
        )
    elif isinstance(typ, ModelType):
        return ArtifactSpec(
            name=name,
            description=description,
            type=ModelType(
                input_spec=convert_to_record_spec(typ.input_spec),
                output_spec=convert_to_record_spec(typ.output_spec),
            ),
        )

    # At this point, we aren't quite sure that 'value' is an appropriate type.
    # But as validation for specs is separate from conversion,
    # we will just ignore potential errors and pass on the value as a type.
    return FieldSpec(name=name, description=description, type=typ)  # type: ignore


def infer_name(func: Callable) -> str:
    name: str = func.__qualname__
    if name.endswith(".__init__"):
        name = name.split(".")[0]
    return name


def infer_description(obj: Union[Callable, Type]) -> Optional[str]:
    """Infers the description of the given object from its docstring"""
    if obj.__doc__ is None or obj == object.__init__:
        return None
    parsed_docstring = docstring_parser.parse(obj.__doc__)
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


def infer_config_type(func: Callable) -> ConfigTypeSpec:
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
        # TODO @Robustness: handle not-set default values appropriately
        default = None if param.default == inspect._empty else param.default
        spec_value = type_to_spec(name=name, description="", default=default, typ=annotation)
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


def _get_typed_annotation(param: inspect.Parameter, global_namespace: dict[str, Any]) -> Any:
    """Gets resolved type annotations for a parameter"""
    # Note: In Python 3.10, we should be able to replace this resoluton logic
    #  with https://docs.python.org/3/library/inspect.html#inspect.get_annotations
    annotation = param.annotation
    if isinstance(annotation, str):
        forward_ref = ForwardRef(annotation)
        annotation = evaluate_forwardref(forward_ref, global_namespace, global_namespace)
    return annotation
