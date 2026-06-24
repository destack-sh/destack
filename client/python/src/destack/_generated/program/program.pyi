# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.program.program import (
    ProgramImpl,
)

import destack._generated.mir.metadata.frame
import destack._generated.mir.metadata.layout
import destack._generated.mir.metadata.trace
import destack._generated.program.function
import destack._generated.program.header
import destack._generated.program.memory.space
import destack._generated.program.native.code
import destack._generated.program.type
import destack._generated.program.vm.code

@dataclass(frozen=True, slots=True)
class Program(ProgramImpl):
    """Durable executable program produced by the toolchain."""

    # serialized program compatibility header
    header: destack._generated.program.header.ProgramHeader
    # runtime type metadata
    types: destack._generated.program.type.TypeTable
    # runtime layouts keyed by layout id
    layouts: destack._generated.mir.metadata.layout.LayoutTable
    # runtime frame metadata
    frames: destack._generated.mir.metadata.frame.FrameTable
    # runtime function metadata
    functions: destack._generated.program.function.FunctionTable
    # canonical trace table used by heap metadata
    traces: destack._generated.mir.metadata.trace.TraceTable
    # immutable constant storage owned by this program
    constant_space: destack._generated.program.memory.space.StaticSpace
    # initial shared static storage for each runtime
    shared_static_space: destack._generated.program.memory.space.StaticSpace
    # initial local static storage for each worker
    local_static_space: destack._generated.program.memory.space.StaticSpace
    # VM code used for interpretation, deoptimization, and continuation resume
    vm: destack._generated.program.vm.code.Code
    # native code used as optional acceleration
    native: destack._generated.program.native.code.Code | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Program: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Program: ...

def encode_program(writer: BinaryWriter, value: Program) -> None: ...
def decode_program(reader: BinaryReader) -> Program: ...
def to_json_program(value: Program) -> Json: ...
def from_json_program(value: Json) -> Program: ...

__all__ = [
    "Program",
    "encode_program",
    "decode_program",
    "to_json_program",
    "from_json_program",
]
