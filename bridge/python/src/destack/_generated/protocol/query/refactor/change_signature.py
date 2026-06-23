# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.edit.edit

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
    )

    from destack._generated.protocol.source.edit.edit import (
        PatchSet,
    )


@dataclass(frozen=True, slots=True)
class ChangeSignatureRequest:
    """Request payload for change signature queries."""

    """The queried position."""
    position: QueryPosition
    """The new parameter list, comma-separated."""
    new_parameters: str
    """The new argument list, comma-separated."""
    new_arguments: str


def encode_change_signature_request(
    writer: Writer, value: ChangeSignatureRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )
    writer.write_string(value.new_parameters)
    writer.write_string(value.new_arguments)


def decode_change_signature_request(reader: Reader) -> ChangeSignatureRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )
    field_1 = reader.read_string()
    field_2 = reader.read_string()

    return ChangeSignatureRequest(
        position=field_0,
        new_parameters=field_1,
        new_arguments=field_2,
    )


@dataclass(frozen=True, slots=True)
class ChangeSignatureResponse:
    """Response payload for change signature queries."""

    """Change signature edit, if available."""
    edit: PatchSet | None


def encode_change_signature_response(
    writer: Writer, value: ChangeSignatureResponse
) -> None:
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.edit.edit.encode_patch_set(
            writer, value.edit
        )


def decode_change_signature_response(reader: Reader) -> ChangeSignatureResponse:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    )

    return ChangeSignatureResponse(
        edit=field_0,
    )


__all__ = [
    "ChangeSignatureRequest",
    "encode_change_signature_request",
    "decode_change_signature_request",
    "ChangeSignatureResponse",
    "encode_change_signature_response",
    "decode_change_signature_response",
]
