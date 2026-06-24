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
    json_object,
    json_string,
)

import destack._generated.dir.tree.key
import destack._generated.dir.tree.literal


@dataclass(frozen=True, slots=True)
class ImportAttributeClause:
    """One import attribute clause."""

    # the clause introducer
    kind: ImportAttributeClauseKind
    # the attribute entries inside the clause body
    attributes: Sequence[ImportAttribute]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_attribute_clause(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportAttributeClause:
        """Decode one ImportAttributeClause."""
        return decode_import_attribute_clause(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_attribute_clause(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportAttributeClause:
        """Return one ImportAttributeClause from one JSON value."""
        return from_json_import_attribute_clause(value)


def encode_import_attribute_clause(
    writer: BinaryWriter, value: ImportAttributeClause
) -> None:
    """Encode one ImportAttributeClause."""
    encode_import_attribute_clause_kind(writer, value.kind)
    writer.write_unsigned(len(value.attributes))
    for item_value_attributes_0 in value.attributes:
        encode_import_attribute(writer, item_value_attributes_0)


def decode_import_attribute_clause(reader: BinaryReader) -> ImportAttributeClause:
    """Decode one ImportAttributeClause."""
    kind = decode_import_attribute_clause_kind(reader)
    attributes = [decode_import_attribute(reader) for _ in range(reader.read_number())]

    return ImportAttributeClause(
        kind=kind,
        attributes=attributes,
    )


def to_json_import_attribute_clause(value: ImportAttributeClause) -> Json:
    """Return one JSON value for one ImportAttributeClause."""
    return {
        "kind": to_json_import_attribute_clause_kind(value.kind),
        "attributes": [to_json_import_attribute(item_0) for item_0 in value.attributes],
    }


def from_json_import_attribute_clause(value: Json) -> ImportAttributeClause:
    """Return one ImportAttributeClause from one JSON value."""
    object_ = json_object(value)

    return ImportAttributeClause(
        kind=from_json_import_attribute_clause_kind(json_field(object_, "kind")),
        attributes=[
            from_json_import_attribute(item_0)
            for item_0 in json_array(json_field(object_, "attributes"))
        ],
    )


"""The kind of one import attribute clause."""
ImportAttributeClauseKind: typing.TypeAlias = typing.Literal["with"]


def encode_import_attribute_clause_kind(
    writer: BinaryWriter, value: ImportAttributeClauseKind
) -> None:
    """Encode one ImportAttributeClauseKind."""
    if value == "with":
        writer.write_unsigned(0)
    else:
        raise SerdeError("unknown enum variant")


def decode_import_attribute_clause_kind(
    reader: BinaryReader,
) -> ImportAttributeClauseKind:
    """Decode one ImportAttributeClauseKind."""
    variant = reader.read_number()

    if variant == 0:
        return "with"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_import_attribute_clause_kind(value: ImportAttributeClauseKind) -> Json:
    """Return one JSON value for one ImportAttributeClauseKind."""
    return value


def from_json_import_attribute_clause_kind(value: Json) -> ImportAttributeClauseKind:
    """Return one ImportAttributeClauseKind from one JSON value."""
    variant = json_string(value)

    if variant == "with":
        return "with"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ImportAttribute:
    """One import attribute entry."""

    # the attribute key
    key: destack._generated.dir.tree.key.Name
    # the attribute value
    value: ImportAttributeValue

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_attribute(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportAttribute:
        """Decode one ImportAttribute."""
        return decode_import_attribute(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_attribute(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportAttribute:
        """Return one ImportAttribute from one JSON value."""
        return from_json_import_attribute(value)


def encode_import_attribute(writer: BinaryWriter, value: ImportAttribute) -> None:
    """Encode one ImportAttribute."""
    destack._generated.dir.tree.key.encode_name(writer, value.key)
    encode_import_attribute_value(writer, value.value)


def decode_import_attribute(reader: BinaryReader) -> ImportAttribute:
    """Decode one ImportAttribute."""
    key = destack._generated.dir.tree.key.decode_name(reader)
    value_ = decode_import_attribute_value(reader)

    return ImportAttribute(
        key=key,
        value=value_,
    )


def to_json_import_attribute(value: ImportAttribute) -> Json:
    """Return one JSON value for one ImportAttribute."""
    return {
        "key": destack._generated.dir.tree.key.to_json_name(value.key),
        "value": to_json_import_attribute_value(value.value),
    }


def from_json_import_attribute(value: Json) -> ImportAttribute:
    """Return one ImportAttribute from one JSON value."""
    object_ = json_object(value)

    return ImportAttribute(
        key=destack._generated.dir.tree.key.from_json_name(json_field(object_, "key")),
        value=from_json_import_attribute_value(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class ImportAttributeValueScalarLiteral:
    """A scalar literal value."""

    scalar_literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_attribute_value(self)


@dataclass(frozen=True, slots=True)
class ImportAttributeValueArray:
    """An array value."""

    array: Sequence[ImportAttributeValue]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_attribute_value(self)


@dataclass(frozen=True, slots=True)
class ImportAttributeValueObject:
    """An object value."""

    object: Sequence[ImportAttribute]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_attribute_value(self)


@dataclass(frozen=True, slots=True)
class ImportAttributeValueError:
    """A parser placeholder for invalid syntax."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_attribute_value(self)


"""One static import attribute value."""
ImportAttributeValue: typing.TypeAlias = (
    ImportAttributeValueScalarLiteral
    | ImportAttributeValueArray
    | ImportAttributeValueObject
    | ImportAttributeValueError
)


def encode_import_attribute_value(
    writer: BinaryWriter, value: ImportAttributeValue
) -> None:
    """Encode one ImportAttributeValue."""
    if value.kind == "scalarLiteral":
        writer.write_unsigned(0)
        destack._generated.dir.tree.literal.encode_scalar_literal(
            writer, value.scalar_literal
        )
    elif value.kind == "array":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.array))
        for item_value_array_0 in value.array:
            encode_import_attribute_value(writer, item_value_array_0)
    elif value.kind == "object":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.object))
        for item_value_object_0 in value.object:
            encode_import_attribute(writer, item_value_object_0)
    elif value.kind == "error":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_import_attribute_value(reader: BinaryReader) -> ImportAttributeValue:
    """Decode one ImportAttributeValue."""
    variant = reader.read_number()

    if variant == 0:
        scalar_literal = destack._generated.dir.tree.literal.decode_scalar_literal(
            reader
        )

        return ImportAttributeValueScalarLiteral(scalar_literal=scalar_literal)
    elif variant == 1:
        array = [
            decode_import_attribute_value(reader) for _ in range(reader.read_number())
        ]

        return ImportAttributeValueArray(array=array)
    elif variant == 2:
        object = [decode_import_attribute(reader) for _ in range(reader.read_number())]

        return ImportAttributeValueObject(object=object)
    elif variant == 3:
        return ImportAttributeValueError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_import_attribute_value(value: ImportAttributeValue) -> Json:
    """Return one JSON value for one ImportAttributeValue."""
    if value.kind == "scalarLiteral":
        return {
            "kind": "scalarLiteral",
            "scalar_literal": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.scalar_literal
            ),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "array": [to_json_import_attribute_value(item_0) for item_0 in value.array],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": [to_json_import_attribute(item_0) for item_0 in value.object],
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_import_attribute_value(value: Json) -> ImportAttributeValue:
    """Return one ImportAttributeValue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "scalarLiteral":
        return ImportAttributeValueScalarLiteral(
            scalar_literal=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "scalar_literal")
            )
        )
    elif kind == "array":
        return ImportAttributeValueArray(
            array=[
                from_json_import_attribute_value(item_0)
                for item_0 in json_array(json_field(object_, "array"))
            ]
        )
    elif kind == "object":
        return ImportAttributeValueObject(
            object=[
                from_json_import_attribute(item_0)
                for item_0 in json_array(json_field(object_, "object"))
            ]
        )
    elif kind == "error":
        return ImportAttributeValueError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ImportAttributeClause",
    "encode_import_attribute_clause",
    "decode_import_attribute_clause",
    "to_json_import_attribute_clause",
    "from_json_import_attribute_clause",
    "ImportAttributeClauseKind",
    "encode_import_attribute_clause_kind",
    "decode_import_attribute_clause_kind",
    "to_json_import_attribute_clause_kind",
    "from_json_import_attribute_clause_kind",
    "ImportAttribute",
    "encode_import_attribute",
    "decode_import_attribute",
    "to_json_import_attribute",
    "from_json_import_attribute",
    "ImportAttributeValue",
    "encode_import_attribute_value",
    "decode_import_attribute_value",
    "to_json_import_attribute_value",
    "from_json_import_attribute_value",
    "ImportAttributeValueScalarLiteral",
    "ImportAttributeValueArray",
    "ImportAttributeValueObject",
    "ImportAttributeValueError",
]
