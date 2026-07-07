# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class ProgramInfo:
    """Reflected view of one program."""

    # reflected modules in this program
    modules: destack._generated.core.section.SectionSlice
    # reflected types keyed by program type id
    types: destack._generated.core.section.SectionSlice
    # reflected functions keyed by program function id
    functions: destack._generated.core.section.SectionSlice
    # reflected runtime bindings
    bindings: destack._generated.core.section.SectionSlice
    # reflected frame layouts keyed by frame layout id
    frames: destack._generated.core.section.SectionSlice
    # reflected globals keyed by program global id
    globals: destack._generated.core.section.SectionSlice
    # reflected entrypoints
    entries: destack._generated.core.section.SectionSlice
    # flattened child type operands used by variable-arity type payloads
    type_operands: destack._generated.core.section.SectionSlice
    # flattened object-like fields
    fields: destack._generated.core.section.SectionSlice
    # flattened tuple elements
    tuple_elements: destack._generated.core.section.SectionSlice
    # flattened reflected members
    members: destack._generated.core.section.SectionSlice
    # flattened reflected index signatures
    index_signatures: destack._generated.core.section.SectionSlice
    # flattened reflected function parameters
    function_parameters: destack._generated.core.section.SectionSlice
    # flattened reflected binding strings
    binding_strings: destack._generated.core.section.SectionSlice
    # flattened reflected variant cases
    variant_cases: destack._generated.core.section.SectionSlice
    # flattened reflected frame slots
    frame_slots: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramInfo: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramInfo: ...

def encode_program_info(writer: BinaryWriter, value: ProgramInfo) -> None: ...
def decode_program_info(reader: BinaryReader) -> ProgramInfo: ...
def to_json_program_info(value: ProgramInfo) -> Json: ...
def from_json_program_info(value: Json) -> ProgramInfo: ...

__all__ = [
    "ProgramInfo",
    "encode_program_info",
    "decode_program_info",
    "to_json_program_info",
    "from_json_program_info",
]
