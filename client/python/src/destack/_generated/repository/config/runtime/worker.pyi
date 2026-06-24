# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class WorkerOptions:
    """Runtime worker configuration."""

    # maximum number of live workers in one runtime
    limit: int | None
    # maximum number of host threads allocated to workers
    thread_limit: int | None
    # stack reservation in bytes for one worker
    stack_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkerOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WorkerOptions: ...

def encode_worker_options(writer: BinaryWriter, value: WorkerOptions) -> None: ...
def decode_worker_options(reader: BinaryReader) -> WorkerOptions: ...
def to_json_worker_options(value: WorkerOptions) -> Json: ...
def from_json_worker_options(value: Json) -> WorkerOptions: ...

__all__ = [
    "WorkerOptions",
    "encode_worker_options",
    "decode_worker_options",
    "to_json_worker_options",
    "from_json_worker_options",
]
