# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
        QueryTarget,
    )


@dataclass(frozen=True, slots=True)
class FindReferencesRequest:
    """Request find references at a cursor position."""

    """The queried position."""
    position: QueryPosition
    """Whether to include the declaration in results."""
    include_declaration: bool


def encode_find_references_request(
    writer: Writer, value: FindReferencesRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )
    writer.write_bool(value.include_declaration)


def decode_find_references_request(reader: Reader) -> FindReferencesRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )
    field_1 = reader.read_bool()

    return FindReferencesRequest(
        position=field_0,
        include_declaration=field_1,
    )


@dataclass(frozen=True, slots=True)
class FindReferencesResponse:
    """Response payload for find references queries."""

    """Reference occurrences."""
    references: Sequence[Reference]


def encode_find_references_response(
    writer: Writer, value: FindReferencesResponse
) -> None:
    writer.write_unsigned(len(value.references))
    for item_0 in value.references:
        encode_reference(writer, item_0)


def decode_find_references_response(reader: Reader) -> FindReferencesResponse:
    field_0 = [decode_reference(reader) for _ in range(reader.read_number())]

    return FindReferencesResponse(
        references=field_0,
    )


@dataclass(frozen=True, slots=True)
class Reference:
    """One symbol reference occurrence."""

    """The referenced source target."""
    target: QueryTarget
    """The reference role."""
    role: ReferenceRole


def encode_reference(writer: Writer, value: Reference) -> None:
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )
    encode_reference_role(writer, value.role)


def decode_reference(reader: Reader) -> Reference:
    field_0 = destack._generated.protocol.query.core.target.decode_query_target(reader)
    field_1 = decode_reference_role(reader)

    return Reference(
        target=field_0,
        role=field_1,
    )


"""Role of one reference occurrence."""
ReferenceRole: TypeAlias = (
    Literal["declaration"]
    | Literal["read"]
    | Literal["write"]
    | Literal["type"]
    | Literal["import"]
    | Literal["export"]
)


def encode_reference_role(writer: Writer, value: ReferenceRole) -> None:
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


def decode_reference_role(reader: Reader) -> ReferenceRole:
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


__all__ = [
    "FindReferencesRequest",
    "encode_find_references_request",
    "decode_find_references_request",
    "FindReferencesResponse",
    "encode_find_references_response",
    "decode_find_references_response",
    "Reference",
    "encode_reference",
    "decode_reference",
    "ReferenceRole",
    "encode_reference_role",
    "decode_reference_role",
]
