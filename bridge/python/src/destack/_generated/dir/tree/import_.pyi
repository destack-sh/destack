# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.key
import destack._generated.dir.tree.literal

@dataclass(frozen=True, slots=True)
class ImportAttributeClause:
    """One import attribute clause."""

    # the clause introducer
    kind: ImportAttributeClauseKind
    # the attribute entries inside the clause body
    attributes: Sequence[ImportAttribute]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportAttributeClause: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ImportAttributeClause: ...

def encode_import_attribute_clause(
    writer: BinaryWriter, value: ImportAttributeClause
) -> None: ...
def decode_import_attribute_clause(reader: BinaryReader) -> ImportAttributeClause: ...
def to_json_import_attribute_clause(value: ImportAttributeClause) -> Json: ...
def from_json_import_attribute_clause(value: Json) -> ImportAttributeClause: ...

"""The kind of one import attribute clause."""
ImportAttributeClauseKind: typing.TypeAlias = typing.Literal["with"]

def encode_import_attribute_clause_kind(
    writer: BinaryWriter, value: ImportAttributeClauseKind
) -> None: ...
def decode_import_attribute_clause_kind(
    reader: BinaryReader,
) -> ImportAttributeClauseKind: ...
def to_json_import_attribute_clause_kind(value: ImportAttributeClauseKind) -> Json: ...
def from_json_import_attribute_clause_kind(
    value: Json,
) -> ImportAttributeClauseKind: ...

@dataclass(frozen=True, slots=True)
class ImportAttribute:
    """One import attribute entry."""

    # the attribute key
    key: destack._generated.dir.tree.key.Name
    # the attribute value
    value: ImportAttributeValue

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportAttribute: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ImportAttribute: ...

def encode_import_attribute(writer: BinaryWriter, value: ImportAttribute) -> None: ...
def decode_import_attribute(reader: BinaryReader) -> ImportAttribute: ...
def to_json_import_attribute(value: ImportAttribute) -> Json: ...
def from_json_import_attribute(value: Json) -> ImportAttribute: ...

@dataclass(frozen=True, slots=True)
class ImportAttributeValueScalarLiteral:
    """A scalar literal value."""

    scalar_literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImportAttributeValueArray:
    """An array value."""

    array: Sequence[ImportAttributeValue]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImportAttributeValueObject:
    """An object value."""

    object: Sequence[ImportAttribute]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImportAttributeValueError:
    """A parser placeholder for invalid syntax."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One static import attribute value."""
ImportAttributeValue: typing.TypeAlias = (
    ImportAttributeValueScalarLiteral
    | ImportAttributeValueArray
    | ImportAttributeValueObject
    | ImportAttributeValueError
)

def encode_import_attribute_value(
    writer: BinaryWriter, value: ImportAttributeValue
) -> None: ...
def decode_import_attribute_value(reader: BinaryReader) -> ImportAttributeValue: ...
def to_json_import_attribute_value(value: ImportAttributeValue) -> Json: ...
def from_json_import_attribute_value(value: Json) -> ImportAttributeValue: ...

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
