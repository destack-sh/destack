# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.vm.op

@dataclass(frozen=True, slots=True)
class Instruction:
    """One decoded program instruction."""

    # the instruction operation
    op: destack._generated.program.vm.op.Op
    # first 32-bit operand
    a: int
    # second 32-bit operand
    b: int
    # third 32-bit operand
    c: int
    # fourth 32-bit operand
    d: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Instruction: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Instruction: ...

def encode_instruction(writer: BinaryWriter, value: Instruction) -> None: ...
def decode_instruction(reader: BinaryReader) -> Instruction: ...
def to_json_instruction(value: Instruction) -> Json: ...
def from_json_instruction(value: Json) -> Instruction: ...

__all__ = [
    "Instruction",
    "encode_instruction",
    "decode_instruction",
    "to_json_instruction",
    "from_json_instruction",
]
