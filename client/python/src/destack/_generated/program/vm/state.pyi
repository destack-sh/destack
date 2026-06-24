# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.frame
import destack._generated.mir.tree.value
import destack._generated.program.function

@dataclass(frozen=True, slots=True)
class ResumeTable:
    """VM resume states keyed by execution frame state."""

    # resume states by dense frame state id
    states: Sequence[ResumeState]
    # frame state id by lowered program point
    state_by_point: Mapping[
        ProgramPoint, destack._generated.mir.metadata.frame.FrameStateId
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ResumeTable: ...

def encode_resume_table(writer: BinaryWriter, value: ResumeTable) -> None: ...
def decode_resume_table(reader: BinaryReader) -> ResumeTable: ...
def to_json_resume_table(value: ResumeTable) -> Json: ...
def from_json_resume_table(value: Json) -> ResumeTable: ...

@dataclass(frozen=True, slots=True)
class ResumeState:
    """VM state for one resumable frame."""

    # the lowered VM program point
    point: ProgramPoint
    # the source MIR point within the lowered block
    mir_point: int
    # entry bindings for block-entry states
    entry: FrameEntry | None
    # caller return destination for post-call states
    return_destination: destack._generated.mir.tree.value.Value | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeState: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ResumeState: ...

def encode_resume_state(writer: BinaryWriter, value: ResumeState) -> None: ...
def decode_resume_state(reader: BinaryReader) -> ResumeState: ...
def to_json_resume_state(value: ResumeState) -> Json: ...
def from_json_resume_state(value: Json) -> ResumeState: ...

@dataclass(frozen=True, slots=True)
class ProgramPoint:
    """One lowered VM program point."""

    # the owning function
    function: destack._generated.program.function.FunctionId
    # the lowered block index
    block: int
    # the lowered program counter inside the block
    pc: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramPoint: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramPoint: ...

def encode_program_point(writer: BinaryWriter, value: ProgramPoint) -> None: ...
def decode_program_point(reader: BinaryReader) -> ProgramPoint: ...
def to_json_program_point(value: ProgramPoint) -> Json: ...
def from_json_program_point(value: Json) -> ProgramPoint: ...

@dataclass(frozen=True, slots=True)
class FrameEntry:
    """VM entry bindings for one resume state."""

    # the slot bindings applied on entry
    bindings: Sequence[FrameBinding]
    # the implicit received value slot
    received_value: destack._generated.mir.metadata.frame.FrameSlotId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameEntry: ...

def encode_frame_entry(writer: BinaryWriter, value: FrameEntry) -> None: ...
def decode_frame_entry(reader: BinaryReader) -> FrameEntry: ...
def to_json_frame_entry(value: FrameEntry) -> Json: ...
def from_json_frame_entry(value: Json) -> FrameEntry: ...

@dataclass(frozen=True, slots=True)
class FrameBinding:
    """One frame slot binding."""

    # the source frame slot
    source: destack._generated.mir.metadata.frame.FrameSlotId
    # the destination frame slot
    destination: destack._generated.mir.metadata.frame.FrameSlotId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameBinding: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameBinding: ...

def encode_frame_binding(writer: BinaryWriter, value: FrameBinding) -> None: ...
def decode_frame_binding(reader: BinaryReader) -> FrameBinding: ...
def to_json_frame_binding(value: FrameBinding) -> Json: ...
def from_json_frame_binding(value: Json) -> FrameBinding: ...

__all__ = [
    "ResumeTable",
    "encode_resume_table",
    "decode_resume_table",
    "to_json_resume_table",
    "from_json_resume_table",
    "ResumeState",
    "encode_resume_state",
    "decode_resume_state",
    "to_json_resume_state",
    "from_json_resume_state",
    "ProgramPoint",
    "encode_program_point",
    "decode_program_point",
    "to_json_program_point",
    "from_json_program_point",
    "FrameEntry",
    "encode_frame_entry",
    "decode_frame_entry",
    "to_json_frame_entry",
    "from_json_frame_entry",
    "FrameBinding",
    "encode_frame_binding",
    "decode_frame_binding",
    "to_json_frame_binding",
    "from_json_frame_binding",
]
