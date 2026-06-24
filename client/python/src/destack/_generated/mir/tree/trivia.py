# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_optional,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_typed_value_span(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypedValueSpan:
        """Decode one TypedValueSpan."""
        return decode_typed_value_span(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_typed_value_span(self)

    @classmethod
    def from_json(cls, value: Json) -> TypedValueSpan:
        """Return one TypedValueSpan from one JSON value."""
        return from_json_typed_value_span(value)


def encode_typed_value_span(writer: BinaryWriter, value: TypedValueSpan) -> None:
    """Encode one TypedValueSpan."""
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.name_span is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.name_span)
    destack._generated.source.file.model.span.encode_span(writer, value.type_span)


def decode_typed_value_span(reader: BinaryReader) -> TypedValueSpan:
    """Decode one TypedValueSpan."""
    span = destack._generated.source.file.model.span.decode_span(reader)
    name_span = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )
    type_span = destack._generated.source.file.model.span.decode_span(reader)

    return TypedValueSpan(
        span=span,
        name_span=name_span,
        type_span=type_span,
    )


def to_json_typed_value_span(value: TypedValueSpan) -> Json:
    """Return one JSON value for one TypedValueSpan."""
    return {
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **(
            {}
            if value.name_span is None
            else {
                "nameSpan": destack._generated.source.file.model.span.to_json_span(
                    value.name_span
                )
            }
        ),
        "typeSpan": destack._generated.source.file.model.span.to_json_span(
            value.type_span
        ),
    }


def from_json_typed_value_span(value: Json) -> TypedValueSpan:
    """Return one TypedValueSpan from one JSON value."""
    object_ = json_object(value)

    return TypedValueSpan(
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        name_span=json_optional(
            object_,
            "nameSpan",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
        type_span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "typeSpan")
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_header_spans(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionHeaderSpans:
        """Decode one FunctionHeaderSpans."""
        return decode_function_header_spans(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_header_spans(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionHeaderSpans:
        """Return one FunctionHeaderSpans from one JSON value."""
        return from_json_function_header_spans(value)


def encode_function_header_spans(
    writer: BinaryWriter, value: FunctionHeaderSpans
) -> None:
    """Encode one FunctionHeaderSpans."""
    destack._generated.source.file.model.span.encode_span(writer, value.open_paren)
    destack._generated.source.file.model.span.encode_span(writer, value.close_paren)
    destack._generated.source.file.model.span.encode_span(writer, value.return_colon)
    if value.open_brace is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.open_brace)


def decode_function_header_spans(reader: BinaryReader) -> FunctionHeaderSpans:
    """Decode one FunctionHeaderSpans."""
    open_paren = destack._generated.source.file.model.span.decode_span(reader)
    close_paren = destack._generated.source.file.model.span.decode_span(reader)
    return_colon = destack._generated.source.file.model.span.decode_span(reader)
    open_brace = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )

    return FunctionHeaderSpans(
        open_paren=open_paren,
        close_paren=close_paren,
        return_colon=return_colon,
        open_brace=open_brace,
    )


def to_json_function_header_spans(value: FunctionHeaderSpans) -> Json:
    """Return one JSON value for one FunctionHeaderSpans."""
    return {
        "openParen": destack._generated.source.file.model.span.to_json_span(
            value.open_paren
        ),
        "closeParen": destack._generated.source.file.model.span.to_json_span(
            value.close_paren
        ),
        "returnColon": destack._generated.source.file.model.span.to_json_span(
            value.return_colon
        ),
        **(
            {}
            if value.open_brace is None
            else {
                "openBrace": destack._generated.source.file.model.span.to_json_span(
                    value.open_brace
                )
            }
        ),
    }


def from_json_function_header_spans(value: Json) -> FunctionHeaderSpans:
    """Return one FunctionHeaderSpans from one JSON value."""
    object_ = json_object(value)

    return FunctionHeaderSpans(
        open_paren=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "openParen")
        ),
        close_paren=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "closeParen")
        ),
        return_colon=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "returnColon")
        ),
        open_brace=json_optional(
            object_,
            "openBrace",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_field_span(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FieldSpan:
        """Decode one FieldSpan."""
        return decode_field_span(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_field_span(self)

    @classmethod
    def from_json(cls, value: Json) -> FieldSpan:
        """Return one FieldSpan from one JSON value."""
        return from_json_field_span(value)


def encode_field_span(writer: BinaryWriter, value: FieldSpan) -> None:
    """Encode one FieldSpan."""
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    writer.write_unsigned(len(value.attribute_spans))
    for item_value_attribute_spans_0 in value.attribute_spans:
        destack._generated.source.file.model.span.encode_span(
            writer, item_value_attribute_spans_0
        )
    if value.name_span is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.name_span)
    destack._generated.source.file.model.span.encode_span(writer, value.type_span)


def decode_field_span(reader: BinaryReader) -> FieldSpan:
    """Decode one FieldSpan."""
    span = destack._generated.source.file.model.span.decode_span(reader)
    attribute_spans = [
        destack._generated.source.file.model.span.decode_span(reader)
        for _ in range(reader.read_number())
    ]
    name_span = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )
    type_span = destack._generated.source.file.model.span.decode_span(reader)

    return FieldSpan(
        span=span,
        attribute_spans=attribute_spans,
        name_span=name_span,
        type_span=type_span,
    )


def to_json_field_span(value: FieldSpan) -> Json:
    """Return one JSON value for one FieldSpan."""
    return {
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "attributeSpans": [
            destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.attribute_spans
        ],
        **(
            {}
            if value.name_span is None
            else {
                "nameSpan": destack._generated.source.file.model.span.to_json_span(
                    value.name_span
                )
            }
        ),
        "typeSpan": destack._generated.source.file.model.span.to_json_span(
            value.type_span
        ),
    }


def from_json_field_span(value: Json) -> FieldSpan:
    """Return one FieldSpan from one JSON value."""
    object_ = json_object(value)

    return FieldSpan(
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        attribute_spans=[
            destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "attributeSpans"))
        ],
        name_span=json_optional(
            object_,
            "nameSpan",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
        type_span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "typeSpan")
        ),
    )


@dataclass(frozen=True, slots=True)
class TypeDeclarationSpans:
    """Parsed type declaration delimiter spans."""

    # the optional equals span
    equals: destack._generated.source.file.model.span.Span | None
    # the optional opening brace span
    open_brace: destack._generated.source.file.model.span.Span | None
    # the optional closing brace span
    close_brace: destack._generated.source.file.model.span.Span | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_declaration_spans(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeDeclarationSpans:
        """Decode one TypeDeclarationSpans."""
        return decode_type_declaration_spans(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_declaration_spans(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeDeclarationSpans:
        """Return one TypeDeclarationSpans from one JSON value."""
        return from_json_type_declaration_spans(value)


def encode_type_declaration_spans(
    writer: BinaryWriter, value: TypeDeclarationSpans
) -> None:
    """Encode one TypeDeclarationSpans."""
    if value.equals is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.equals)
    if value.open_brace is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.open_brace)
    if value.close_brace is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.span.encode_span(writer, value.close_brace)


def decode_type_declaration_spans(reader: BinaryReader) -> TypeDeclarationSpans:
    """Decode one TypeDeclarationSpans."""
    equals = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )
    open_brace = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )
    close_brace = reader.read_option(
        lambda: destack._generated.source.file.model.span.decode_span(reader)
    )

    return TypeDeclarationSpans(
        equals=equals,
        open_brace=open_brace,
        close_brace=close_brace,
    )


def to_json_type_declaration_spans(value: TypeDeclarationSpans) -> Json:
    """Return one JSON value for one TypeDeclarationSpans."""
    return {
        **(
            {}
            if value.equals is None
            else {
                "equals": destack._generated.source.file.model.span.to_json_span(
                    value.equals
                )
            }
        ),
        **(
            {}
            if value.open_brace is None
            else {
                "openBrace": destack._generated.source.file.model.span.to_json_span(
                    value.open_brace
                )
            }
        ),
        **(
            {}
            if value.close_brace is None
            else {
                "closeBrace": destack._generated.source.file.model.span.to_json_span(
                    value.close_brace
                )
            }
        ),
    }


def from_json_type_declaration_spans(value: Json) -> TypeDeclarationSpans:
    """Return one TypeDeclarationSpans from one JSON value."""
    object_ = json_object(value)

    return TypeDeclarationSpans(
        equals=json_optional(
            object_,
            "equals",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
        open_brace=json_optional(
            object_,
            "openBrace",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
        close_brace=json_optional(
            object_,
            "closeBrace",
            lambda value: destack._generated.source.file.model.span.from_json_span(
                value
            ),
        ),
    )


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
