# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.runtime.execution

@dataclass(frozen=True, slots=True)
class TraceOptions:
    """Runtime trace configuration."""

    # the configured restore contract
    restore: RestoreMode
    # base path for trace logs
    path: str | None
    # template for auto-generated log file names
    template: str | None
    # chunk size in megabytes for log rotation
    chunk_size_mb: int | None
    # trace payload selection for recorded binding calls
    payload: destack._generated.repository.config.runtime.execution.ReplayPayloadMode

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceOptions: ...

def encode_trace_options(writer: BinaryWriter, value: TraceOptions) -> None: ...
def decode_trace_options(reader: BinaryReader) -> TraceOptions: ...
def to_json_trace_options(value: TraceOptions) -> Json: ...
def from_json_trace_options(value: Json) -> TraceOptions: ...

"""Restore contract for one world checkpoint or replay boundary."""
RestoreMode: typing.TypeAlias = typing.Literal["world"] | typing.Literal["image"]

def encode_restore_mode(writer: BinaryWriter, value: RestoreMode) -> None: ...
def decode_restore_mode(reader: BinaryReader) -> RestoreMode: ...
def to_json_restore_mode(value: RestoreMode) -> Json: ...
def from_json_restore_mode(value: Json) -> RestoreMode: ...

__all__ = [
    "TraceOptions",
    "encode_trace_options",
    "decode_trace_options",
    "to_json_trace_options",
    "from_json_trace_options",
    "RestoreMode",
    "encode_restore_mode",
    "decode_restore_mode",
    "to_json_restore_mode",
    "from_json_restore_mode",
]
