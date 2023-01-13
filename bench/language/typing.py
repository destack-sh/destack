from __future__ import annotations

import inspect
import typing
from collections import OrderedDict
from enum import Enum
from functools import cached_property
from typing import Optional, Union

from attr import dataclass


class ValueType(Enum):
    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    ARRAY = "array"
    MAP = "map"
    STRUCT = "struct"
    FUNCTION = "function"
    UNION = "union"
    NULL = "null"
    TYPE_REFERENCE = "schema"


LITERAL_TYPES = [ValueType.NULL, ValueType.BOOLEAN, ValueType.NUMBER, ValueType.STRING]

PyValueType = Union[int, float, bool, str, dict, list]

PYTYPE_TO_VALUE_TYPE = {
    int: ValueType.NUMBER,
    float: ValueType.NUMBER,
    bool: ValueType.BOOLEAN,
    str: ValueType.STRING,
    dict: ValueType.STRUCT,
    list: ValueType.ARRAY,
    type(None): ValueType.NULL,
}


def get_value_type(obj: PyValueType) -> ValueType:
    for pytype, value_type in PYTYPE_TO_VALUE_TYPE.items():
        if isinstance(obj, pytype):
            return value_type
    raise ValueError(f"unknown value type: {obj}")


def get_value_type_from_type(typ: type) -> ValueType:
    # return directly if it's a known value type
    if typ in PYTYPE_TO_VALUE_TYPE:
        return PYTYPE_TO_VALUE_TYPE[typ]

    # otherwise try to derive from supertypes
    for pytype, value_type in PYTYPE_TO_VALUE_TYPE.items():
        if issubclass(typ, pytype):
            return value_type
    raise ValueError(f"unknown value type: {typ}")


@dataclass(frozen=True)
class TypeElement:
    name: Optional[str]
    type: ValueType
    required: bool = True
    description: Optional[str] = None
    reference: Optional[str] = None
    elements: Optional[list["TypeElement"]] = None

    def __str__(self):
        return self.btl

    @cached_property
    def btl(self) -> str:
        raise NotImplementedError

    @property
    def keys(self) -> list[str]:
        if self.elements is None:
            return []
        else:
            return [e.name for e in self.elements]

    @property
    def input_(self) -> TypeElement:
        return self.element("input")

    @property
    def output_(self) -> TypeElement:
        return self.element("output")

    def element(self, key: str) -> TypeElement:
        """Find a schema element by key (only works for objects)."""
        if self.type != ValueType.STRUCT:
            raise ValueError(f"find cannot be used on {self}")
        if self.elements is None:
            raise ValueError("elements is None")
        for e in self.elements:
            if e.name == key:
                return e
        raise KeyError(f"key {key} not found in {self}")

    @cached_property
    def is_resolved(self):
        """Check if this schema (and all sub-schemas) are resolved."""
        if self.type == ValueType.TYPE_REFERENCE:
            return False
        elif self.elements is not None:
            return all(e.is_resolved for e in self.elements)
        else:
            return True

    def resolve(self, types: dict[str, "TypeElement"]) -> TypeElement:
        """Resolve this schema and all sub-schema references."""
        if self.type == ValueType.TYPE_REFERENCE:
            if self.reference is None:
                raise ValueError("schema_id is None")
            if self.reference not in types:
                raise ValueError(f"schema_id {self.reference} not found")
            return types[self.reference].resolve(types)
        elif self.elements is not None:
            return TypeElement(
                name=self.name,
                type=self.type,
                required=self.required,
                elements=[e.resolve(types) for e in self.elements],
            )
        else:
            return self


def derive_schema_from_records(records: list[dict], name: str | None = "record") -> TypeElement:
    if not isinstance(records, list):
        raise ValueError("records must be a list")
    schema = derive_schema_from_record(records, name=name)
    if schema is None or schema.type != ValueType.ARRAY or schema.elements is None:
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
        return TypeElement(name=name or "", type=ValueType.STRUCT, elements=elements)
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
        return TypeElement(name=name or "", type=ValueType.ARRAY, elements=[value_type])
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
    input_schema = TypeElement(name="input", type=ValueType.STRUCT, elements=input_schema_elements)

    return_type = signature.return_annotation
    if not return_type or return_type == inspect.Signature.empty:
        output_schema = TypeElement(name="output", type=ValueType.NULL, elements=None)
    else:
        output_schema = derive_schema_from_pytype(return_type, name="output")
        if output_schema is None:
            output_schema = TypeElement(name="output", type=ValueType.NULL, elements=None)

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
        return TypeElement(name, ValueType.ARRAY, elements=elements)
    elif issubclass(resolved_type, dict):
        # not possible to derive schema from dict
        return TypeElement(name, ValueType.STRUCT, required=required, elements=None)
    else:
        return TypeElement(name, PYTYPE_TO_VALUE_TYPE[resolved_type], required=required)
