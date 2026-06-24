# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""Execution mode for runtime scheduling and replay."""
ExecutionMode: typing.TypeAlias = (
    typing.Literal["strict"]
    | typing.Literal["fast"]
    | typing.Literal["record"]
    | typing.Literal["replay"]
)


def encode_execution_mode(writer: BinaryWriter, value: ExecutionMode) -> None:
    """Encode one ExecutionMode."""
    if value == "strict":
        writer.write_unsigned(0)
    elif value == "fast":
        writer.write_unsigned(1)
    elif value == "record":
        writer.write_unsigned(2)
    elif value == "replay":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_execution_mode(reader: BinaryReader) -> ExecutionMode:
    """Decode one ExecutionMode."""
    variant = reader.read_number()

    if variant == 0:
        return "strict"
    elif variant == 1:
        return "fast"
    elif variant == 2:
        return "record"
    elif variant == 3:
        return "replay"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_execution_mode(value: ExecutionMode) -> Json:
    """Return one JSON value for one ExecutionMode."""
    return value


def from_json_execution_mode(value: Json) -> ExecutionMode:
    """Return one ExecutionMode from one JSON value."""
    variant = json_string(value)

    if variant == "strict":
        return "strict"
    elif variant == "fast":
        return "fast"
    elif variant == "record":
        return "record"
    elif variant == "replay":
        return "replay"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Replay payload selection for record/replay."""
ReplayPayloadMode: typing.TypeAlias = (
    typing.Literal["resultsOnly"] | typing.Literal["argumentsAndResults"]
)


def encode_replay_payload_mode(writer: BinaryWriter, value: ReplayPayloadMode) -> None:
    """Encode one ReplayPayloadMode."""
    if value == "resultsOnly":
        writer.write_unsigned(0)
    elif value == "argumentsAndResults":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_replay_payload_mode(reader: BinaryReader) -> ReplayPayloadMode:
    """Decode one ReplayPayloadMode."""
    variant = reader.read_number()

    if variant == 0:
        return "resultsOnly"
    elif variant == 1:
        return "argumentsAndResults"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_replay_payload_mode(value: ReplayPayloadMode) -> Json:
    """Return one JSON value for one ReplayPayloadMode."""
    return value


def from_json_replay_payload_mode(value: Json) -> ReplayPayloadMode:
    """Return one ReplayPayloadMode from one JSON value."""
    variant = json_string(value)

    if variant == "resultsOnly":
        return "resultsOnly"
    elif variant == "argumentsAndResults":
        return "argumentsAndResults"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "ExecutionMode",
    "encode_execution_mode",
    "decode_execution_mode",
    "to_json_execution_mode",
    "from_json_execution_mode",
    "ReplayPayloadMode",
    "encode_replay_payload_mode",
    "decode_replay_payload_mode",
    "to_json_replay_payload_mode",
    "from_json_replay_payload_mode",
]
