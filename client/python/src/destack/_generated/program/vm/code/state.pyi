# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class ResumeTable:
    """VM resume states keyed by execution frame state."""

    # resume states by dense frame state id
    states: destack._generated.core.section.SectionSlice
    # resume state ids sorted by lowered program point
    points: destack._generated.core.section.SectionSlice
    # flattened frame entry bindings
    bindings: destack._generated.core.section.SectionSlice

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

__all__ = [
    "ResumeTable",
    "encode_resume_table",
    "decode_resume_table",
    "to_json_resume_table",
    "from_json_resume_table",
]
