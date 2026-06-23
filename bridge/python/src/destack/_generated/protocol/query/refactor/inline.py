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
class InlineRequest:
    """Request payload for inline refactor queries."""

    """The queried position."""
    position: QueryPosition


def encode_inline_request(writer: Writer, value: InlineRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_inline_request(reader: Reader) -> InlineRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return InlineRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class InlineResponse:
    """Response payload for inline refactor queries."""

    """Inline edit, if available."""
    edit: PatchSet | None


def encode_inline_response(writer: Writer, value: InlineResponse) -> None:
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.edit.edit.encode_patch_set(
            writer, value.edit
        )


def decode_inline_response(reader: Reader) -> InlineResponse:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    )

    return InlineResponse(
        edit=field_0,
    )


__all__ = [
    "InlineRequest",
    "encode_inline_request",
    "decode_inline_request",
    "InlineResponse",
    "encode_inline_response",
    "decode_inline_response",
]
