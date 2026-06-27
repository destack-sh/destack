# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.table.trace
import destack._generated.program.dispatch
import destack._generated.program.frame
import destack._generated.program.function
import destack._generated.program.header
import destack._generated.program.info
import destack._generated.program.layout
import destack._generated.program.memory.space
import destack._generated.program.native.code
import destack._generated.program.type
import destack._generated.program.vm.code

@dataclass(frozen=True, slots=True)
class ProgramImage:
    """Durable executable image produced by the toolchain."""

    # serialized program compatibility header
    header: destack._generated.program.header.ProgramHeader
    # runtime type table
    types: destack._generated.program.type.TypeTable
    # runtime layouts keyed by layout id
    layouts: destack._generated.program.layout.LayoutTable
    # runtime frame table
    frames: destack._generated.program.frame.FrameTable
    # executable function table
    functions: destack._generated.program.function.FunctionTable
    # runtime dispatch table
    dispatch: destack._generated.program.dispatch.DispatchTable
    # canonical trace table used by heap tables
    traces: destack._generated.mir.table.trace.TraceTable
    # reflectable program table
    info: destack._generated.program.info.ProgramInfo
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
    def decode(cls, reader: BinaryReader) -> ProgramImage: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramImage: ...

def encode_program_image(writer: BinaryWriter, value: ProgramImage) -> None: ...
def decode_program_image(reader: BinaryReader) -> ProgramImage: ...
def to_json_program_image(value: ProgramImage) -> Json: ...
def from_json_program_image(value: Json) -> ProgramImage: ...

__all__ = [
    "ProgramImage",
    "encode_program_image",
    "decode_program_image",
    "to_json_program_image",
    "from_json_program_image",
]
