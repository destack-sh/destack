# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.query.navigation.definition

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
    )

    from destack._generated.protocol.query.navigation.definition import (
        NavigationTarget,
    )


@dataclass(frozen=True, slots=True)
class GotoImplementationRequest:
    """Request goto implementation at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_goto_implementation_request(
    writer: Writer, value: GotoImplementationRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_goto_implementation_request(reader: Reader) -> GotoImplementationRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return GotoImplementationRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class GotoImplementationResponse:
    """Response payload for goto implementation queries."""

    """Implementation targets."""
    targets: Sequence[NavigationTarget]


def encode_goto_implementation_response(
    writer: Writer, value: GotoImplementationResponse
) -> None:
    writer.write_unsigned(len(value.targets))
    for item_0 in value.targets:
        destack._generated.protocol.query.navigation.definition.encode_navigation_target(
            writer, item_0
        )


def decode_goto_implementation_response(reader: Reader) -> GotoImplementationResponse:
    field_0 = [
        destack._generated.protocol.query.navigation.definition.decode_navigation_target(
            reader
        )
        for _ in range(reader.read_number())
    ]

    return GotoImplementationResponse(
        targets=field_0,
    )


__all__ = [
    "GotoImplementationRequest",
    "encode_goto_implementation_request",
    "decode_goto_implementation_request",
    "GotoImplementationResponse",
    "encode_goto_implementation_response",
    "decode_goto_implementation_response",
]
