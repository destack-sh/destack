# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section
import destack._generated.heap.core.trace
import destack._generated.heap.local.heap.options
import destack._generated.heap.shared.heap.options
import destack._generated.mir.table.target
import destack._generated.program.dispatch
import destack._generated.program.frame
import destack._generated.program.function
import destack._generated.program.info
import destack._generated.program.layout
import destack._generated.program.memory.region
import destack._generated.program.memory.space
import destack._generated.program.native.code.code
import destack._generated.program.string
import destack._generated.program.type
import destack._generated.program.vm.code.code

@dataclass(frozen=True, slots=True)
class Program:
    """Program produced by the toolchain."""

    # program section directory
    sections: destack._generated.core.section.SectionDirectory
    # target ABI layout used by program layouts and pointer-sized integer types
    target_layout: destack._generated.mir.table.target.TargetLayout
    # local heap geometry used by lowered allocation plans
    local_heap: destack._generated.heap.local.heap.options.HeapOptions
    # shared heap geometry used by lowered allocation plans
    shared_heap: destack._generated.heap.shared.heap.options.SharedHeapOptions
    # program string table
    strings: destack._generated.program.string.StringTable
    # runtime type table
    types: destack._generated.program.type.TypeTable
    # runtime layouts keyed by layout id
    layouts: destack._generated.program.layout.LayoutTable
    # runtime frame table
    frames: destack._generated.program.frame.FrameTable
    # program function table
    functions: destack._generated.program.function.FunctionTable
    # runtime dispatch table
    dispatch: destack._generated.program.dispatch.DispatchTable
    # canonical trace table used by heap tables
    traces: destack._generated.heap.core.trace.TraceTable
    # program globals keyed by dense global id
    globals: destack._generated.program.memory.region.GlobalTable
    # optional source reflection table
    info: destack._generated.program.info.ProgramInfo | None
    # immutable constant storage owned by this program
    constant_space: destack._generated.program.memory.space.StaticImage
    # initial shared static storage for each runtime
    shared_static_space: destack._generated.program.memory.space.StaticImage
    # initial local static storage for each worker
    local_static_space: destack._generated.program.memory.space.StaticImage
    # VM code used for interpretation, deoptimization, and continuation resume
    vm: destack._generated.program.vm.code.code.Code
    # native code used as optional acceleration
    native: destack._generated.program.native.code.code.Code | None
    # program section storage
    storage: Sequence[int]

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
