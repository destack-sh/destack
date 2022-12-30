from __future__ import annotations

import functools
import inspect
import re
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
    SCHEMA = "schema"
    OBJECT = "object"
    ARRAY = "array"
    NULL = "null"


LITERAL_TYPES = [ValueType.NULL, ValueType.BOOLEAN, ValueType.NUMBER, ValueType.STRING]

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


@dataclass(frozen=True)
class SchemaElement:
    name: Optional[str]
    type: ValueType
    required: bool = True
    schema_id: Optional[str] = None
    elements: Optional[list["SchemaElement"]] = None

    def __str__(self):
        return self.bsl

    @cached_property
    def bsl(self) -> str:
        return render_bsl(self)

    @property
    def keys(self) -> list[str]:
        if self.elements is None:
            return []
        else:
            return [e.name for e in self.elements]

    @property
    def input_(self) -> SchemaElement:
        return self.element("input")

    @property
    def output_(self) -> SchemaElement:
        return self.element("output")

    @functools.cache
    def element(self, key: str) -> SchemaElement:
        """Find a schema element by key (only works for objects)."""
        if self.type != ValueType.OBJECT:
            raise ValueError(f"find cannot be used on {self}")
        if self.elements is None:
            raise ValueError("elements is None")
        for e in self.elements:
            if e.name == key:
                return e
        raise ValueError(f"key {key} not found in {self}")

    @cached_property
    def is_resolved(self):
        """Check if this schema (and all sub-schemas) are resolved."""
        if self.type == ValueType.SCHEMA:
            return False
        elif self.type == ValueType.OBJECT or self.type == ValueType.ARRAY:
            if self.elements is None:
                raise ValueError("elements is None")
            return all(e.is_resolved for e in self.elements)
        else:
            return True

    def resolve(self, schemas: dict[str, "SchemaElement"]) -> SchemaElement:
        """Resolve this schema and all sub-schema references."""
        if self.type == ValueType.SCHEMA:
            if self.schema_id is None:
                raise ValueError("schema_id is None")
            if self.schema_id not in schemas:
                raise ValueError(f"schema_id {self.schema_id} not found")
            return schemas[self.schema_id].resolve(schemas)
        elif self.type == ValueType.OBJECT:
            if self.elements is None:
                raise ValueError("elements is None")
            return SchemaElement(
                name=self.name,
                type=self.type,
                required=self.required,
                elements=[e.resolve(schemas) for e in self.elements],
            )
        elif self.type == ValueType.ARRAY:
            if self.elements is None:
                raise ValueError("elements is None")
            return SchemaElement(
                name=self.name,
                type=self.type,
                required=self.required,
                elements=[e.resolve(schemas) for e in self.elements],
            )
        else:
            return self


def render_bsl(schema: SchemaElement) -> str:
    """Renders a schema to a bsl string."""
    name_str = f"{schema.name}: " if schema.name else ""
    required_str = "" if schema.required else "?"
    elements_str = ", ".join(render_bsl(e) for e in schema.elements) if schema.elements else ""
    if schema.type == ValueType.OBJECT:
        # output as name: { elem1, elem2, ... }
        return f"{name_str}{{ {elements_str} }}{required_str}"
    elif schema.type == ValueType.ARRAY:
        # output as name: [elem1]
        return f"{name_str}[{elements_str}]{required_str}"
    elif schema.type == ValueType.SCHEMA:
        return f"{name_str}{schema.schema_id}{required_str}"
    else:
        return f"{name_str}{schema.type.value}{required_str}"


def parse_bsl(bsl: str) -> SchemaElement:
    """
    Parse a schema from a bsl string.
    This is a basic parser and should probably be a more formal grammar later.
    """
    bsl = bsl.strip()

    # required
    required_match = re.match(r"^(.*)\?$", bsl)
    if required_match:
        required = False
        bsl = bsl[: required_match.start(1)]
    else:
        required = True

    # name
    name_match = re.match(r"^(?P<name>[a-zA-Z0-9_]+): ", bsl)
    if name_match:
        name = name_match.group("name")
        bsl = bsl[name_match.end() :]
    else:
        name = None

    # value type and elements
    elements = None
    schema_id = None

    object_match = re.match(r"^\{(?P<elements>.*)}$", bsl)
    array_match = re.match(r"^\[(?P<elements>.*)]$", bsl)
    type_match = re.match(rf"^(?P<type>{'|'.join(t.value for t in LITERAL_TYPES)})$", bsl)
    if object_match:  # object
        value_type = ValueType.OBJECT
        elements_bsl = object_match.group("elements").split(",")
        elements = [parse_bsl(e.strip()) for e in elements_bsl]
    elif array_match:  # array
        value_type = ValueType.ARRAY
        elements_bsl = array_match.group("elements").split(",")
        elements = [parse_bsl(e.strip()) for e in elements_bsl]
    elif type_match:  # value type
        value_type = ValueType(type_match.group("type"))
    else:  # schema reference (if not defined inline)
        schema_name_match = re.match(r"^(?P<schema_name>[a-zA-Z0-9_]+)$", bsl)
        if not schema_name_match:
            raise ValueError(f"invalid schema reference: {bsl}")
        value_type = ValueType.SCHEMA
        schema_id = schema_name_match.group("schema_name")
        elements = None

    return SchemaElement(
        name=name, required=required, type=value_type, schema_id=schema_id, elements=elements
    )


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
            "required": schema_element.required,
            "elements": elements,
        }

    @staticmethod
    def from_json(json: dict) -> "SchemaElement":
        elements = (
            [SchemaElementSerializer.from_json(e) for e in json.get("elements", [])]
            if json.get("elements") is not None
            else None
        )
        return SchemaElement(
            name=json.get("name"),
            type=ValueType(json["type"]),
            required=json.get("required", True),
            elements=elements,
        )


def derive_schema_from_records(records: list[dict], name: str | None = "record") -> SchemaElement:
    if not isinstance(records, list):
        raise ValueError("records must be a list")
    schema = derive_schema_from_record(records, name=name)
    if schema is None or schema.type != ValueType.ARRAY or schema.elements is None:
        raise ValueError(f"schema could not be derived (invalid array schema): {schema}")
    return schema.elements[0]


def derive_schema_from_record(record: PyValueType, name: str | None = None) -> SchemaElement | None:
    if isinstance(record, dict):
        if len(record) == 0:
            return None
        type_by_name: dict[str, SchemaElement] = OrderedDict()
        for key, value in record.items():
            if key in type_by_name:
                continue
            element = derive_schema_from_record(value, name=key)
            if element is not None:
                type_by_name[key] = element
            # otherwise just ignore
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


def derive_schema_from_function(function: typing.Callable) -> tuple[SchemaElement, SchemaElement]:
    signature = inspect.signature(function)

    input_schema_elements = []
    for param in signature.parameters.values():
        ptype = param.annotation
        if ptype in ("Model", "Dataset"):
            continue  # ignore non-value types
        element = derive_schema_from_type(ptype, name=param.name)
        if element is not None:
            input_schema_elements.append(element)
    input_schema = SchemaElement(
        name="input", type=ValueType.OBJECT, elements=input_schema_elements
    )

    return_type = signature.return_annotation
    if not return_type or return_type == inspect.Signature.empty:
        output_schema = SchemaElement(name="output", type=ValueType.NULL, elements=None)
    else:
        output_schema = derive_schema_from_type(return_type, name="output")
        if output_schema is None:
            output_schema = SchemaElement(name="output", type=ValueType.NULL, elements=None)

    return input_schema, output_schema


def derive_schema_from_type(
    typ: type | str, name: str, required: bool = True
) -> SchemaElement | None:
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
                return derive_schema_from_type(subtypes[0], name, required=False)
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
            element_type = derive_schema_from_type(subtypes[0], name, required=required)
            elements = [element_type] if element_type is not None else None
        else:
            elements = None
        return SchemaElement(name, ValueType.ARRAY, elements=elements)
    elif issubclass(resolved_type, dict):
        # not possible to derive schema from dict
        return SchemaElement(name, ValueType.OBJECT, required=required, elements=None)
    else:
        return SchemaElement(name, PYTYPE_TO_VALUE_TYPE[resolved_type], required=required)
