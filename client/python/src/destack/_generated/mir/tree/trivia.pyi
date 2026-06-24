# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class TypedValueSpan:
    """One parsed typed value span."""

    # the enclosing span of the typed value occurrence
    span: destack._generated.source.file.model.span.Span
    # the optional name span inside the occurrence
    name_span: destack._generated.source.file.model.span.Span | None
    # the type span inside the occurrence
    type_span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypedValueSpan: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypedValueSpan: ...

def encode_typed_value_span(writer: BinaryWriter, value: TypedValueSpan) -> None: ...
def decode_typed_value_span(reader: BinaryReader) -> TypedValueSpan: ...
def to_json_typed_value_span(value: TypedValueSpan) -> Json: ...
def from_json_typed_value_span(value: Json) -> TypedValueSpan: ...

@dataclass(frozen=True, slots=True)
class FunctionHeaderSpans:
    """Parsed function header delimiter spans."""

    # the opening parenthesis span
    open_paren: destack._generated.source.file.model.span.Span
    # the closing parenthesis span
    close_paren: destack._generated.source.file.model.span.Span
    # the return type colon span
    return_colon: destack._generated.source.file.model.span.Span
    # the optional body opening brace span
    open_brace: destack._generated.source.file.model.span.Span | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionHeaderSpans: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionHeaderSpans: ...

def encode_function_header_spans(
    writer: BinaryWriter, value: FunctionHeaderSpans
) -> None: ...
def decode_function_header_spans(reader: BinaryReader) -> FunctionHeaderSpans: ...
def to_json_function_header_spans(value: FunctionHeaderSpans) -> Json: ...
def from_json_function_header_spans(value: Json) -> FunctionHeaderSpans: ...

@dataclass(frozen=True, slots=True)
class FieldSpan:
    """One parsed field declaration span."""

    # the enclosing span of the field declaration
    span: destack._generated.source.file.model.span.Span
    # the attribute spans on the field declaration
    attribute_spans: Sequence[destack._generated.source.file.model.span.Span]
    # the optional field name span
    name_span: destack._generated.source.file.model.span.Span | None
    # the field type span
    type_span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FieldSpan: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FieldSpan: ...

def encode_field_span(writer: BinaryWriter, value: FieldSpan) -> None: ...
def decode_field_span(reader: BinaryReader) -> FieldSpan: ...
def to_json_field_span(value: FieldSpan) -> Json: ...
def from_json_field_span(value: Json) -> FieldSpan: ...

@dataclass(frozen=True, slots=True)
class TypeDeclarationSpans:
    """Parsed type declaration delimiter spans."""

    # the optional equals span
    equals: destack._generated.source.file.model.span.Span | None
    # the optional opening brace span
    open_brace: destack._generated.source.file.model.span.Span | None
    # the optional closing brace span
    close_brace: destack._generated.source.file.model.span.Span | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeDeclarationSpans: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeDeclarationSpans: ...

def encode_type_declaration_spans(
    writer: BinaryWriter, value: TypeDeclarationSpans
) -> None: ...
def decode_type_declaration_spans(reader: BinaryReader) -> TypeDeclarationSpans: ...
def to_json_type_declaration_spans(value: TypeDeclarationSpans) -> Json: ...
def from_json_type_declaration_spans(value: Json) -> TypeDeclarationSpans: ...

__all__ = [
    "TypedValueSpan",
    "encode_typed_value_span",
    "decode_typed_value_span",
    "to_json_typed_value_span",
    "from_json_typed_value_span",
    "FunctionHeaderSpans",
    "encode_function_header_spans",
    "decode_function_header_spans",
    "to_json_function_header_spans",
    "from_json_function_header_spans",
    "FieldSpan",
    "encode_field_span",
    "decode_field_span",
    "to_json_field_span",
    "from_json_field_span",
    "TypeDeclarationSpans",
    "encode_type_declaration_spans",
    "decode_type_declaration_spans",
    "to_json_type_declaration_spans",
    "from_json_type_declaration_spans",
]
