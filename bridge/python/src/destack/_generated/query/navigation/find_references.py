# generated bridge target, do not edit

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
    json_bool,
    json_field,
    json_object,
    json_string,
)

import destack._generated.query.core.target


@dataclass(frozen=True, slots=True)
class FindReferencesRequest:
    """Request find references at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition
    # whether to include the declaration in results
    include_declaration: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_find_references_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FindReferencesRequest:
        """Decode one FindReferencesRequest."""
        return decode_find_references_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_find_references_request(self)

    @classmethod
    def from_json(cls, value: Json) -> FindReferencesRequest:
        """Return one FindReferencesRequest from one JSON value."""
        return from_json_find_references_request(value)


def encode_find_references_request(
    writer: BinaryWriter, value: FindReferencesRequest
) -> None:
    """Encode one FindReferencesRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)
    writer.write_bool(value.include_declaration)


def decode_find_references_request(reader: BinaryReader) -> FindReferencesRequest:
    """Decode one FindReferencesRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)
    include_declaration = reader.read_bool()

    return FindReferencesRequest(
        position=position,
        include_declaration=include_declaration,
    )


def to_json_find_references_request(value: FindReferencesRequest) -> Json:
    """Return one JSON value for one FindReferencesRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
        "includeDeclaration": value.include_declaration,
    }


def from_json_find_references_request(value: Json) -> FindReferencesRequest:
    """Return one FindReferencesRequest from one JSON value."""
    object_ = json_object(value)

    return FindReferencesRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
        include_declaration=json_bool(json_field(object_, "includeDeclaration")),
    )


@dataclass(frozen=True, slots=True)
class FindReferencesResponse:
    """Response payload for find references queries."""

    # reference occurrences
    references: Sequence[Reference]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_find_references_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FindReferencesResponse:
        """Decode one FindReferencesResponse."""
        return decode_find_references_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_find_references_response(self)

    @classmethod
    def from_json(cls, value: Json) -> FindReferencesResponse:
        """Return one FindReferencesResponse from one JSON value."""
        return from_json_find_references_response(value)


def encode_find_references_response(
    writer: BinaryWriter, value: FindReferencesResponse
) -> None:
    """Encode one FindReferencesResponse."""
    writer.write_unsigned(len(value.references))
    for item_value_references_0 in value.references:
        encode_reference(writer, item_value_references_0)


def decode_find_references_response(reader: BinaryReader) -> FindReferencesResponse:
    """Decode one FindReferencesResponse."""
    references = [decode_reference(reader) for _ in range(reader.read_number())]

    return FindReferencesResponse(
        references=references,
    )


def to_json_find_references_response(value: FindReferencesResponse) -> Json:
    """Return one JSON value for one FindReferencesResponse."""
    return {
        "references": [to_json_reference(item_0) for item_0 in value.references],
    }


def from_json_find_references_response(value: Json) -> FindReferencesResponse:
    """Return one FindReferencesResponse from one JSON value."""
    object_ = json_object(value)

    return FindReferencesResponse(
        references=[
            from_json_reference(item_0)
            for item_0 in json_array(json_field(object_, "references"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Reference:
    """One symbol reference occurrence."""

    # the referenced source target
    target: destack._generated.query.core.target.QueryTarget
    # the reference role
    role: ReferenceRole

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Reference:
        """Decode one Reference."""
        return decode_reference(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference(self)

    @classmethod
    def from_json(cls, value: Json) -> Reference:
        """Return one Reference from one JSON value."""
        return from_json_reference(value)


def encode_reference(writer: BinaryWriter, value: Reference) -> None:
    """Encode one Reference."""
    destack._generated.query.core.target.encode_query_target(writer, value.target)
    encode_reference_role(writer, value.role)


def decode_reference(reader: BinaryReader) -> Reference:
    """Decode one Reference."""
    target = destack._generated.query.core.target.decode_query_target(reader)
    role = decode_reference_role(reader)

    return Reference(
        target=target,
        role=role,
    )


def to_json_reference(value: Reference) -> Json:
    """Return one JSON value for one Reference."""
    return {
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
        "role": to_json_reference_role(value.role),
    }


def from_json_reference(value: Json) -> Reference:
    """Return one Reference from one JSON value."""
    object_ = json_object(value)

    return Reference(
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
        role=from_json_reference_role(json_field(object_, "role")),
    )


"""Role of one reference occurrence."""
ReferenceRole: typing.TypeAlias = (
    typing.Literal["declaration"]
    | typing.Literal["read"]
    | typing.Literal["write"]
    | typing.Literal["type"]
    | typing.Literal["import"]
    | typing.Literal["export"]
)


def encode_reference_role(writer: BinaryWriter, value: ReferenceRole) -> None:
    """Encode one ReferenceRole."""
    if value == "declaration":
        writer.write_unsigned(0)
    elif value == "read":
        writer.write_unsigned(1)
    elif value == "write":
        writer.write_unsigned(2)
    elif value == "type":
        writer.write_unsigned(3)
    elif value == "import":
        writer.write_unsigned(4)
    elif value == "export":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_reference_role(reader: BinaryReader) -> ReferenceRole:
    """Decode one ReferenceRole."""
    variant = reader.read_number()

    if variant == 0:
        return "declaration"
    elif variant == 1:
        return "read"
    elif variant == 2:
        return "write"
    elif variant == 3:
        return "type"
    elif variant == 4:
        return "import"
    elif variant == 5:
        return "export"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_reference_role(value: ReferenceRole) -> Json:
    """Return one JSON value for one ReferenceRole."""
    return value


def from_json_reference_role(value: Json) -> ReferenceRole:
    """Return one ReferenceRole from one JSON value."""
    variant = json_string(value)

    if variant == "declaration":
        return "declaration"
    elif variant == "read":
        return "read"
    elif variant == "write":
        return "write"
    elif variant == "type":
        return "type"
    elif variant == "import":
        return "import"
    elif variant == "export":
        return "export"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "FindReferencesRequest",
    "encode_find_references_request",
    "decode_find_references_request",
    "to_json_find_references_request",
    "from_json_find_references_request",
    "FindReferencesResponse",
    "encode_find_references_response",
    "decode_find_references_response",
    "to_json_find_references_response",
    "from_json_find_references_response",
    "Reference",
    "encode_reference",
    "decode_reference",
    "to_json_reference",
    "from_json_reference",
    "ReferenceRole",
    "encode_reference_role",
    "decode_reference_role",
    "to_json_reference_role",
    "from_json_reference_role",
]
