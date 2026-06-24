# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Instruction:
        """Decode one Instruction."""
        return decode_instruction(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)

    @classmethod
    def from_json(cls, value: Json) -> Instruction:
        """Return one Instruction from one JSON value."""
        return from_json_instruction(value)


def encode_instruction(writer: BinaryWriter, value: Instruction) -> None:
    """Encode one Instruction."""
    destack._generated.program.vm.op.encode_op(writer, value.op)
    writer.write_unsigned(value.a)
    writer.write_unsigned(value.b)
    writer.write_unsigned(value.c)
    writer.write_unsigned(value.d)


def decode_instruction(reader: BinaryReader) -> Instruction:
    """Decode one Instruction."""
    op = destack._generated.program.vm.op.decode_op(reader)
    a = reader.read_number()
    b = reader.read_number()
    c = reader.read_number()
    d = reader.read_number()

    return Instruction(
        op=op,
        a=a,
        b=b,
        c=c,
        d=d,
    )


def to_json_instruction(value: Instruction) -> Json:
    """Return one JSON value for one Instruction."""
    return {
        "op": destack._generated.program.vm.op.to_json_op(value.op),
        "a": value.a,
        "b": value.b,
        "c": value.c,
        "d": value.d,
    }


def from_json_instruction(value: Json) -> Instruction:
    """Return one Instruction from one JSON value."""
    object_ = json_object(value)

    return Instruction(
        op=destack._generated.program.vm.op.from_json_op(json_field(object_, "op")),
        a=json_int(json_field(object_, "a")),
        b=json_int(json_field(object_, "b")),
        c=json_int(json_field(object_, "c")),
        d=json_int(json_field(object_, "d")),
    )


__all__ = [
    "Instruction",
    "encode_instruction",
    "decode_instruction",
    "to_json_instruction",
    "from_json_instruction",
]
