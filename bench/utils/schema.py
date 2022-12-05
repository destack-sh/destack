from __future__ import annotations

import inspect
from collections import OrderedDict
from enum import Enum
from typing import Optional, Union

from attr import dataclass


class ValueType(Enum):
    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    OBJECT = "object"
    ARRAY = "array"
    NULL = "null"


PyValueType = Union[int, float, bool, str, dict, list]

PYTYPE_TO_VALUE_TYPE = {
    int: ValueType.NUMBER,
    float: ValueType.NUMBER,
    bool: ValueType.BOOLEAN,
    str: ValueType.STRING,
    dict: ValueType.OBJECT,
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


@dataclass
class SchemaElement:
    name: str
    type: ValueType
    choices: Optional[list[PyValueType]] = None
    elements: Optional[list["SchemaElement"]] = None

    @property
    def keys(self) -> list[str]:
        return [e.name for e in self.elements]

    def __str__(self):
        elements_str = ", ".join(str(e) for e in self.elements) if self.elements else ""
        if self.type == ValueType.OBJECT:
            # output as name={elem1, elem2, ...}
            return f"{self.name}={{{elements_str}}}"
        elif self.type == ValueType.ARRAY:
            # output as name=[elem1, elem2, ...]
            return f"{self.name}=[{elements_str}]"
        else:
            if not self.choices:
                return f"{self.name}={self.type.value}"
            else:
                return f"{self.name}={self.type.value}(enum)"


def derive_schema_from_records(records: list[dict], name: str | None = "record") -> SchemaElement:
    if not isinstance(records, list):
        raise ValueError("records must be a list")
    schema = derive_schema_from_record(records, name=name)
    if schema is None or isinstance(schema, ValueType):
        raise ValueError(f"schema could not be derived: {schema}")
    return schema.elements[0]


def derive_schema_from_record(
    record: PyValueType, name: str | None = None
) -> ValueType | SchemaElement | None:
    if isinstance(record, dict):
        if len(record) == 0:
            return None
        type_by_name: dict[str, SchemaElement] = OrderedDict()
        for key, value in record.items():
            if key in type_by_name:
                continue
            type_by_name[key] = derive_schema_from_record(value, name=key)
        elements = list(type_by_name.values()) if type_by_name else None
        return SchemaElement(name=name or "", type=ValueType.OBJECT, elements=elements)
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
        return SchemaElement(name=name or "", type=ValueType.ARRAY, elements=[value_type])
    else:
        return SchemaElement(name=name or "", type=get_value_type(record))


def derive_schema_from_function(function: callable) -> tuple[SchemaElement, SchemaElement]:
    signature = inspect.signature(function)

    input_schema = SchemaElement(name="input", type=ValueType.OBJECT, elements=[])
    for param in signature.parameters.values():
        ptype = param.annotation
        if ptype in ("Model", "Dataset"):
            continue  # ignore non-value types
        element = derive_schema_from_type(ptype, name=param.name)
        if element is not None:
            input_schema.elements.append(element)

    output_schema = derive_schema_from_type(signature.return_annotation or "None", name="output")
    if output_schema is None:
        output_schema = SchemaElement(name="output", type=ValueType.NULL, elements=None)

    return input_schema, output_schema


def derive_schema_from_type(typ: type | str, name: str) -> SchemaElement | None:
    if typ in ("Model", "Dataset"):
        return None
    # evaluate type annotations (assumes no imports)
    if isinstance(typ, str):
        typ = eval(typ)

    # get type annotation from generic alias
    if hasattr(typ, "__origin__"):
        typ = typ.__origin__
        subtypes = getattr(typ, "__args__", None)
    else:
        subtypes = None

    if typ is Union:
        # don't support unions
        return None

    if issubclass(typ, list):
        if subtypes is not None:
            element_type = derive_schema_from_type(subtypes[0], name)
            elements = [element_type] if element_type is not None else None
        else:
            elements = None
        return SchemaElement(name, ValueType.ARRAY, elements=elements)
    elif issubclass(typ, dict):
        # not possible to derive schema from dict
        return SchemaElement(name, ValueType.OBJECT, elements=None)
    else:
        return SchemaElement(name, PYTYPE_TO_VALUE_TYPE[typ])


class SchemaElementSerializer:
    @staticmethod
    def to_json(schema_element: "SchemaElement") -> dict:
        elements = (
            [SchemaElementSerializer.to_json(e) for e in schema_element.elements]
            if schema_element.elements
            else None
        )
        return {
            "name": schema_element.name,
            "type": schema_element.type.value,
            "choices": schema_element.choices,
            "elements": elements,
        }

    @staticmethod
    def from_json(json: dict) -> "SchemaElement":
        elements = (
            [SchemaElementSerializer.from_json(e) for e in json.get("elements")]
            if json.get("elements") is not None
            else None
        )
        return SchemaElement(
            name=json["name"],
            type=ValueType(json["type"]),
            choices=json.get("choices"),
            elements=elements,
        )


class SchemaElementArraySerializer:
    @staticmethod
    def to_json(schema_element: list["SchemaElement"]) -> list[dict]:
        return [SchemaElementSerializer.to_json(e) for e in schema_element]

    @staticmethod
    def from_json(json: list[dict]) -> list["SchemaElement"]:
        return [SchemaElementSerializer.from_json(e) for e in json]


class SchemaObjectSerializer:
    @staticmethod
    def from_json(name: str, json: dict | list[dict]) -> "SchemaElement":
        if isinstance(json, dict):
            return SchemaElementSerializer.from_json(json)
        else:
            return SchemaElement(
                name=name,
                type=ValueType.OBJECT,
                choices=None,
                elements=[SchemaElementSerializer.from_json(e) for e in json],
            )
