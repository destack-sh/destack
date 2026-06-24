# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.tree.function
import destack._generated.dir.tree.literal
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GlobalStaticId:
    """Global static id across modules."""

    # the module id of the global static value
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global static value
    local_id: LocalStaticId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_static_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalStaticId:
        """Decode one GlobalStaticId."""
        return decode_global_static_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_static_id(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalStaticId:
        """Return one GlobalStaticId from one JSON value."""
        return from_json_global_static_id(value)


def encode_global_static_id(writer: BinaryWriter, value: GlobalStaticId) -> None:
    """Encode one GlobalStaticId."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_static_id(writer, value.local_id)


def decode_global_static_id(reader: BinaryReader) -> GlobalStaticId:
    """Decode one GlobalStaticId."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    local_id = decode_local_static_id(reader)

    return GlobalStaticId(
        module_id=module_id,
        local_id=local_id,
    )


def to_json_global_static_id(value: GlobalStaticId) -> Json:
    """Return one JSON value for one GlobalStaticId."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "localId": to_json_local_static_id(value.local_id),
    }


def from_json_global_static_id(value: Json) -> GlobalStaticId:
    """Return one GlobalStaticId from one JSON value."""
    object_ = json_object(value)

    return GlobalStaticId(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        local_id=from_json_local_static_id(json_field(object_, "localId")),
    )


"""Unique identifier for a local static value."""
LocalStaticId: typing.TypeAlias = int


def encode_local_static_id(writer: BinaryWriter, value: LocalStaticId) -> None:
    """Encode one LocalStaticId."""
    writer.write_unsigned(value)


def decode_local_static_id(reader: BinaryReader) -> LocalStaticId:
    """Decode one LocalStaticId."""
    return reader.read_number()


def to_json_local_static_id(value: LocalStaticId) -> Json:
    """Return one JSON value for one LocalStaticId."""
    return value


def from_json_local_static_id(value: Json) -> LocalStaticId:
    """Return one LocalStaticId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class StaticTermScalarLiteral:
    """Scalar literal."""

    value: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


@dataclass(frozen=True, slots=True)
class StaticTermType:
    """Type value."""

    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


@dataclass(frozen=True, slots=True)
class StaticTermArray:
    """Array value."""

    elements: Sequence[StaticTerm]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


@dataclass(frozen=True, slots=True)
class StaticTermFixedArray:
    """Fixed array value."""

    # the repeated value
    value: StaticTerm
    # the fixed array length
    length: int
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


@dataclass(frozen=True, slots=True)
class StaticTermTuple:
    """Tuple value."""

    elements: Sequence[StaticTerm]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


@dataclass(frozen=True, slots=True)
class StaticTermObject:
    """Structural object value."""

    # the object properties
    properties: Sequence[StaticProperty]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


@dataclass(frozen=True, slots=True)
class StaticTermStruct:
    """Nominal struct value."""

    # the struct type selected for this value
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the struct properties
    properties: Sequence[StaticProperty]
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_term(self)


"""Concrete static value produced by checked static evaluation."""
StaticTerm: typing.TypeAlias = (
    StaticTermScalarLiteral
    | StaticTermType
    | StaticTermArray
    | StaticTermFixedArray
    | StaticTermTuple
    | StaticTermObject
    | StaticTermStruct
)


def encode_static_term(writer: BinaryWriter, value: StaticTerm) -> None:
    """Encode one StaticTerm."""
    if value.kind == "scalarLiteral":
        writer.write_unsigned(0)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.value)
    elif value.kind == "type":
        writer.write_unsigned(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "array":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            encode_static_term(writer, item_value_elements_0)
    elif value.kind == "fixedArray":
        writer.write_unsigned(3)
        encode_static_term(writer, value.value)
        writer.write_unsigned(value.length)
    elif value.kind == "tuple":
        writer.write_unsigned(4)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            encode_static_term(writer, item_value_elements_0)
    elif value.kind == "object":
        writer.write_unsigned(5)
        writer.write_unsigned(len(value.properties))
        for item_value_properties_0 in value.properties:
            encode_static_property(writer, item_value_properties_0)
    elif value.kind == "struct":
        writer.write_unsigned(6)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
        writer.write_unsigned(len(value.properties))
        for item_value_properties_0 in value.properties:
            encode_static_property(writer, item_value_properties_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_static_term(reader: BinaryReader) -> StaticTerm:
    """Decode one StaticTerm."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.dir.tree.literal.decode_scalar_literal(reader)

        return StaticTermScalarLiteral(
            value=value_,
        )
    elif variant == 1:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return StaticTermType(
            ty=ty,
        )
    elif variant == 2:
        elements = [decode_static_term(reader) for _ in range(reader.read_number())]

        return StaticTermArray(
            elements=elements,
        )
    elif variant == 3:
        value_ = decode_static_term(reader)
        length = reader.read_number()

        return StaticTermFixedArray(
            value=value_,
            length=length,
        )
    elif variant == 4:
        elements = [decode_static_term(reader) for _ in range(reader.read_number())]

        return StaticTermTuple(
            elements=elements,
        )
    elif variant == 5:
        properties = [
            decode_static_property(reader) for _ in range(reader.read_number())
        ]

        return StaticTermObject(
            properties=properties,
        )
    elif variant == 6:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)
        properties = [
            decode_static_property(reader) for _ in range(reader.read_number())
        ]

        return StaticTermStruct(
            ty=ty,
            properties=properties,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_static_term(value: StaticTerm) -> Json:
    """Return one JSON value for one StaticTerm."""
    if value.kind == "scalarLiteral":
        return {
            "kind": "scalarLiteral",
            "value": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.value
            ),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "elements": [to_json_static_term(item_0) for item_0 in value.elements],
        }
    elif value.kind == "fixedArray":
        return {
            "kind": "fixedArray",
            "value": to_json_static_term(value.value),
            "length": value.length,
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "elements": [to_json_static_term(item_0) for item_0 in value.elements],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "properties": [
                to_json_static_property(item_0) for item_0 in value.properties
            ],
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
            "properties": [
                to_json_static_property(item_0) for item_0 in value.properties
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_static_term(value: Json) -> StaticTerm:
    """Return one StaticTerm from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "scalarLiteral":
        return StaticTermScalarLiteral(
            value=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "type":
        return StaticTermType(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "array":
        return StaticTermArray(
            elements=[
                from_json_static_term(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "fixedArray":
        return StaticTermFixedArray(
            value=from_json_static_term(json_field(object_, "value")),
            length=json_int(json_field(object_, "length")),
        )
    elif kind == "tuple":
        return StaticTermTuple(
            elements=[
                from_json_static_term(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "object":
        return StaticTermObject(
            properties=[
                from_json_static_property(item_0)
                for item_0 in json_array(json_field(object_, "properties"))
            ],
        )
    elif kind == "struct":
        return StaticTermStruct(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
            properties=[
                from_json_static_property(item_0)
                for item_0 in json_array(json_field(object_, "properties"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class StaticPropertyField:
    """Static field."""

    # the property key
    key: destack._generated.dir.symbol.key.StaticKey
    # the property value
    value: StaticTerm
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_property(self)


@dataclass(frozen=True, slots=True)
class StaticPropertyMethod:
    """Static member function."""

    # the optional method key
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the method signature
    signature: destack._generated.dir.tree.function.FunctionSignature
    # the method body
    body: StaticTerm
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_property(self)


@dataclass(frozen=True, slots=True)
class StaticPropertySpread:
    """Static spread."""

    # the spread value
    value: StaticTerm
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_property(self)


"""Static object property in a checked static context."""
StaticProperty: typing.TypeAlias = (
    StaticPropertyField | StaticPropertyMethod | StaticPropertySpread
)


def encode_static_property(writer: BinaryWriter, value: StaticProperty) -> None:
    """Encode one StaticProperty."""
    if value.kind == "field":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
        encode_static_term(writer, value.value)
    elif value.kind == "method":
        writer.write_unsigned(1)
        if value.key is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
        destack._generated.dir.tree.function.encode_function_signature(
            writer, value.signature
        )
        encode_static_term(writer, value.body)
    elif value.kind == "spread":
        writer.write_unsigned(2)
        encode_static_term(writer, value.value)
    else:
        raise SerdeError("unknown enum variant")


def decode_static_property(reader: BinaryReader) -> StaticProperty:
    """Decode one StaticProperty."""
    variant = reader.read_number()

    if variant == 0:
        key = destack._generated.dir.symbol.key.decode_static_key(reader)
        value_ = decode_static_term(reader)

        return StaticPropertyField(
            key=key,
            value=value_,
        )
    elif variant == 1:
        key = reader.read_option(
            lambda: destack._generated.dir.symbol.key.decode_static_key(reader)
        )
        signature = destack._generated.dir.tree.function.decode_function_signature(
            reader
        )
        body = decode_static_term(reader)

        return StaticPropertyMethod(
            key=key,
            signature=signature,
            body=body,
        )
    elif variant == 2:
        value_ = decode_static_term(reader)

        return StaticPropertySpread(
            value=value_,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_static_property(value: StaticProperty) -> Json:
    """Return one JSON value for one StaticProperty."""
    if value.kind == "field":
        return {
            "kind": "field",
            "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
            "value": to_json_static_term(value.value),
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            **(
                {}
                if value.key is None
                else {
                    "key": destack._generated.dir.symbol.key.to_json_static_key(
                        value.key
                    )
                }
            ),
            "signature": destack._generated.dir.tree.function.to_json_function_signature(
                value.signature
            ),
            "body": to_json_static_term(value.body),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            "value": to_json_static_term(value.value),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_static_property(value: Json) -> StaticProperty:
    """Return one StaticProperty from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return StaticPropertyField(
            key=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "key")
            ),
            value=from_json_static_term(json_field(object_, "value")),
        )
    elif kind == "method":
        return StaticPropertyMethod(
            key=json_optional(
                object_,
                "key",
                lambda value: destack._generated.dir.symbol.key.from_json_static_key(
                    value
                ),
            ),
            signature=destack._generated.dir.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            body=from_json_static_term(json_field(object_, "body")),
        )
    elif kind == "spread":
        return StaticPropertySpread(
            value=from_json_static_term(json_field(object_, "value")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "GlobalStaticId",
    "encode_global_static_id",
    "decode_global_static_id",
    "to_json_global_static_id",
    "from_json_global_static_id",
    "LocalStaticId",
    "encode_local_static_id",
    "decode_local_static_id",
    "to_json_local_static_id",
    "from_json_local_static_id",
    "StaticTerm",
    "encode_static_term",
    "decode_static_term",
    "to_json_static_term",
    "from_json_static_term",
    "StaticTermScalarLiteral",
    "StaticTermType",
    "StaticTermArray",
    "StaticTermFixedArray",
    "StaticTermTuple",
    "StaticTermObject",
    "StaticTermStruct",
    "StaticProperty",
    "encode_static_property",
    "decode_static_property",
    "to_json_static_property",
    "from_json_static_property",
    "StaticPropertyField",
    "StaticPropertyMethod",
    "StaticPropertySpread",
]
