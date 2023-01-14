from __future__ import annotations

import inspect
import typing
from collections import OrderedDict
from typing import Union

from bench.language.type import TypeElement, TypeTag

PyValueType = Union[int, float, bool, str, dict, list]

PYTYPE_TO_VALUE_TYPE = {
    int: TypeTag.NUMBER,
    float: TypeTag.NUMBER,
    bool: TypeTag.BOOLEAN,
    str: TypeTag.STRING,
    dict: TypeTag.STRUCT,
    list: TypeTag.ARRAY,
    type(None): TypeTag.NULL,
}


def get_value_type(obj: PyValueType) -> TypeTag:
    for pytype, value_type in PYTYPE_TO_VALUE_TYPE.items():
        if isinstance(obj, pytype):
            return value_type
    raise ValueError(f"unknown value type: {obj}")


def get_value_type_from_type(typ: type) -> TypeTag:
    # return directly if it's a known value type
    if typ in PYTYPE_TO_VALUE_TYPE:
        return PYTYPE_TO_VALUE_TYPE[typ]

    # otherwise try to derive from supertypes
    for pytype, value_type in PYTYPE_TO_VALUE_TYPE.items():
        if issubclass(typ, pytype):
            return value_type
    raise ValueError(f"unknown value type: {typ}")


def derive_schema_from_records(records: list[dict], name: str | None = "record") -> TypeElement:
    if not isinstance(records, list):
        raise ValueError("records must be a list")
    schema = derive_schema_from_record(records, name=name)
    if schema is None or schema.type != TypeTag.ARRAY or schema.elements is None:
        raise ValueError(f"schema could not be derived (invalid array schema): {schema}")
    return schema.elements[0]


def derive_schema_from_record(record: PyValueType, name: str | None = None) -> TypeElement | None:
    if isinstance(record, dict):
        if len(record) == 0:
            return None
        type_by_name: dict[str, TypeElement] = OrderedDict()
        for key, value in record.items():
            if key in type_by_name:
                continue
            element = derive_schema_from_record(value, name=key)
            if element is not None:
                type_by_name[key] = element
            # otherwise just ignore
        elements = list(type_by_name.values()) if type_by_name else None
        return TypeElement(name=name or "", type=TypeTag.STRUCT, elements=elements)
    elif isinstance(record, list):
        if len(record) == 0:
            return None
        value_type = None
        for item in record[:10]:  # limit to just the head of the list
            value_type = derive_schema_from_record(item)
            if value_type is not None:
                break
        if value_type is None:
            return None
        return TypeElement(name=name or "", type=TypeTag.ARRAY, elements=[value_type])
    else:
        return TypeElement(name=name or "", type=get_value_type(record))


def derive_schema_from_function(function: typing.Callable) -> tuple[TypeElement, TypeElement]:
    signature = inspect.signature(function)

    input_schema_elements = []
    for param in signature.parameters.values():
        ptype = param.annotation
        if ptype in ("Model", "Dataset"):
            continue  # ignore non-value types
        element = derive_schema_from_pytype(ptype, name=param.name)
        if element is not None:
            input_schema_elements.append(element)
    input_schema = TypeElement(name="input", type=TypeTag.STRUCT, elements=input_schema_elements)

    return_type = signature.return_annotation
    if not return_type or return_type == inspect.Signature.empty:
        output_schema = TypeElement(name="output", type=TypeTag.NULL, elements=None)
    else:
        output_schema = derive_schema_from_pytype(return_type, name="output")
        if output_schema is None:
            output_schema = TypeElement(name="output", type=TypeTag.NULL, elements=None)

    return input_schema, output_schema


def derive_schema_from_pytype(
    typ: type | str, name: str, required: bool = True
) -> TypeElement | None:
    if typ in ("Model", "Dataset"):
        return None
    # evaluate type annotations (assumes no special imports)
    resolved_type: type
    if isinstance(typ, str):
        resolved_type = eval(typ)
    else:
        resolved_type = typ

    # get type annotation from generic alias
    if hasattr(resolved_type, "__origin__"):
        if resolved_type.__origin__ is Union:
            subtypes = getattr(resolved_type, "__args__")
            if len(subtypes) == 2 or type(None) in subtypes:
                return derive_schema_from_pytype(subtypes[0], name, required=False)
            # don't support classic unions (yet), only optionals
            else:
                raise ValueError(f"unsupported union type: {typ}")
        else:
            resolved_type = resolved_type.__origin__
            subtypes = getattr(resolved_type, "__args__", None)
    else:
        subtypes = None

    if not isinstance(resolved_type, type):
        raise ValueError(f"invalid type: {typ}")

    if issubclass(resolved_type, list):
        if subtypes is not None:
            element_type = derive_schema_from_pytype(subtypes[0], name, required=required)
            elements = [element_type] if element_type is not None else None
        else:
            elements = None
        return TypeElement(name, TypeTag.ARRAY, elements=elements)
    elif issubclass(resolved_type, dict):
        # not possible to derive schema from dict
        return TypeElement(name, TypeTag.STRUCT, required=required, elements=None)
    else:
        return TypeElement(name, PYTYPE_TO_VALUE_TYPE[resolved_type], required=required)
