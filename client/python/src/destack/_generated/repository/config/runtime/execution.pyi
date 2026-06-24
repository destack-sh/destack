# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Execution mode for runtime scheduling and replay."""
ExecutionMode: typing.TypeAlias = (
    typing.Literal["strict"]
    | typing.Literal["fast"]
    | typing.Literal["record"]
    | typing.Literal["replay"]
)

def encode_execution_mode(writer: BinaryWriter, value: ExecutionMode) -> None: ...
def decode_execution_mode(reader: BinaryReader) -> ExecutionMode: ...
def to_json_execution_mode(value: ExecutionMode) -> Json: ...
def from_json_execution_mode(value: Json) -> ExecutionMode: ...

"""Replay payload selection for record/replay."""
ReplayPayloadMode: typing.TypeAlias = (
    typing.Literal["resultsOnly"] | typing.Literal["argumentsAndResults"]
)

def encode_replay_payload_mode(
    writer: BinaryWriter, value: ReplayPayloadMode
) -> None: ...
def decode_replay_payload_mode(reader: BinaryReader) -> ReplayPayloadMode: ...
def to_json_replay_payload_mode(value: ReplayPayloadMode) -> Json: ...
def from_json_replay_payload_mode(value: Json) -> ReplayPayloadMode: ...

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
