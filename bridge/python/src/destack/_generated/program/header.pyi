# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.heap.local.heap.options
import destack._generated.heap.shared.heap.options

@dataclass(frozen=True, slots=True)
class ProgramHeader:
    """Serialized program compatibility header."""

    # pointer byte width used by layouts and pointer-sized integer types
    pointer_bytes: int
    # local heap geometry used by lowered allocation sites
    local_heap: destack._generated.heap.local.heap.options.HeapOptions
    # shared heap geometry used by lowered allocation sites
    shared_heap: destack._generated.heap.shared.heap.options.SharedHeapOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramHeader: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramHeader: ...

def encode_program_header(writer: BinaryWriter, value: ProgramHeader) -> None: ...
def decode_program_header(reader: BinaryReader) -> ProgramHeader: ...
def to_json_program_header(value: ProgramHeader) -> Json: ...
def from_json_program_header(value: Json) -> ProgramHeader: ...

__all__ = [
    "ProgramHeader",
    "encode_program_header",
    "decode_program_header",
    "to_json_program_header",
    "from_json_program_header",
]
