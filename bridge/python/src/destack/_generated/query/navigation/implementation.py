# generated bridge target, do not edit

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
)

import destack._generated.query.core.target
import destack._generated.query.navigation.definition


@dataclass(frozen=True, slots=True)
class GotoImplementationRequest:
    """Request goto implementation at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_implementation_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoImplementationRequest:
        """Decode one GotoImplementationRequest."""
        return decode_goto_implementation_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_implementation_request(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoImplementationRequest:
        """Return one GotoImplementationRequest from one JSON value."""
        return from_json_goto_implementation_request(value)


def encode_goto_implementation_request(
    writer: BinaryWriter, value: GotoImplementationRequest
) -> None:
    """Encode one GotoImplementationRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_goto_implementation_request(
    reader: BinaryReader,
) -> GotoImplementationRequest:
    """Decode one GotoImplementationRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return GotoImplementationRequest(
        position=position,
    )


def to_json_goto_implementation_request(value: GotoImplementationRequest) -> Json:
    """Return one JSON value for one GotoImplementationRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_goto_implementation_request(value: Json) -> GotoImplementationRequest:
    """Return one GotoImplementationRequest from one JSON value."""
    object_ = json_object(value)

    return GotoImplementationRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class GotoImplementationResponse:
    """Response payload for goto implementation queries."""

    # implementation targets
    targets: Sequence[destack._generated.query.navigation.definition.NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_implementation_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoImplementationResponse:
        """Decode one GotoImplementationResponse."""
        return decode_goto_implementation_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_implementation_response(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoImplementationResponse:
        """Return one GotoImplementationResponse from one JSON value."""
        return from_json_goto_implementation_response(value)


def encode_goto_implementation_response(
    writer: BinaryWriter, value: GotoImplementationResponse
) -> None:
    """Encode one GotoImplementationResponse."""
    writer.write_unsigned(len(value.targets))
    for item_value_targets_0 in value.targets:
        destack._generated.query.navigation.definition.encode_navigation_target(
            writer, item_value_targets_0
        )


def decode_goto_implementation_response(
    reader: BinaryReader,
) -> GotoImplementationResponse:
    """Decode one GotoImplementationResponse."""
    targets = [
        destack._generated.query.navigation.definition.decode_navigation_target(reader)
        for _ in range(reader.read_number())
    ]

    return GotoImplementationResponse(
        targets=targets,
    )


def to_json_goto_implementation_response(value: GotoImplementationResponse) -> Json:
    """Return one JSON value for one GotoImplementationResponse."""
    return {
        "targets": [
            destack._generated.query.navigation.definition.to_json_navigation_target(
                item_0
            )
            for item_0 in value.targets
        ],
    }


def from_json_goto_implementation_response(value: Json) -> GotoImplementationResponse:
    """Return one GotoImplementationResponse from one JSON value."""
    object_ = json_object(value)

    return GotoImplementationResponse(
        targets=[
            destack._generated.query.navigation.definition.from_json_navigation_target(
                item_0
            )
            for item_0 in json_array(json_field(object_, "targets"))
        ],
    )


__all__ = [
    "GotoImplementationRequest",
    "encode_goto_implementation_request",
    "decode_goto_implementation_request",
    "to_json_goto_implementation_request",
    "from_json_goto_implementation_request",
    "GotoImplementationResponse",
    "encode_goto_implementation_response",
    "decode_goto_implementation_response",
    "to_json_goto_implementation_response",
    "from_json_goto_implementation_response",
]
