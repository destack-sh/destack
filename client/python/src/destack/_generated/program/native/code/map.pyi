# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class CodeMap:
    """Native code map for entries, safepoints, roots, and deoptimization."""

    # native function code ranges
    function: destack._generated.core.section.SectionSlice
    # native continuation resume code ranges
    resume: destack._generated.core.section.SectionSlice
    # native safepoints keyed by safepoint id
    safepoint: destack._generated.core.section.SectionSlice
    # native roots referenced by safepoints
    root: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeMap: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeMap: ...

def encode_code_map(writer: BinaryWriter, value: CodeMap) -> None: ...
def decode_code_map(reader: BinaryReader) -> CodeMap: ...
def to_json_code_map(value: CodeMap) -> Json: ...
def from_json_code_map(value: Json) -> CodeMap: ...

__all__ = [
    "CodeMap",
    "encode_code_map",
    "decode_code_map",
    "to_json_code_map",
    "from_json_code_map",
]
