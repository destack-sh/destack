# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class Task:
    """Named toolchain or shell invocation."""

    # destack command name to run
    run: str | None
    # explicit shell command to execute
    exec: str | None
    # build target selected for this task
    target: str | None
    # product selected for this task
    product: str | None
    # profile selected for this task
    profile: str | None
    # active source graph modes added by this task
    modes: Sequence[str]
    # active source graph roles added by this task
    roles: Sequence[str]
    # active source graph features added by this task
    features: Sequence[str]
    # active source graph tags added by this task
    tags: Sequence[str]
    # environment variables passed to this task
    env: Mapping[str, str]
    # structured command arguments
    arguments: Mapping[str, typing.Any]
    # tasks that must complete before this task
    depends_on: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Task: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Task: ...

def encode_task(writer: BinaryWriter, value: Task) -> None: ...
def decode_task(reader: BinaryReader) -> Task: ...
def to_json_task(value: Task) -> Json: ...
def from_json_task(value: Json) -> Task: ...

__all__ = [
    "Task",
    "encode_task",
    "decode_task",
    "to_json_task",
    "from_json_task",
]
