# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Program:
        """Decode one Program."""
        return decode_program(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program(self)

    @classmethod
    def from_json(cls, value: Json) -> Program:
        """Return one Program from one JSON value."""
        return from_json_program(value)


def encode_program(writer: BinaryWriter, value: Program) -> None:
    """Encode one Program."""
    destack._generated.core.section.encode_section_directory(writer, value.sections)
    destack._generated.mir.table.target.encode_target_layout(
        writer, value.target_layout
    )
    destack._generated.heap.local.heap.options.encode_heap_options(
        writer, value.local_heap
    )
    destack._generated.heap.shared.heap.options.encode_shared_heap_options(
        writer, value.shared_heap
    )
    destack._generated.program.string.encode_string_table(writer, value.strings)
    destack._generated.program.type.encode_type_table(writer, value.types)
    destack._generated.program.layout.encode_layout_table(writer, value.layouts)
    destack._generated.program.frame.encode_frame_table(writer, value.frames)
    destack._generated.program.function.encode_function_table(writer, value.functions)
    destack._generated.program.dispatch.encode_dispatch_table(writer, value.dispatch)
    destack._generated.heap.core.trace.encode_trace_table(writer, value.traces)
    destack._generated.program.memory.region.encode_global_table(writer, value.globals)
    if value.info is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.info.encode_program_info(writer, value.info)
    destack._generated.program.memory.space.encode_static_image(
        writer, value.constant_space
    )
    destack._generated.program.memory.space.encode_static_image(
        writer, value.shared_static_space
    )
    destack._generated.program.memory.space.encode_static_image(
        writer, value.local_static_space
    )
    destack._generated.program.vm.code.code.encode_code(writer, value.vm)
    if value.native is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.native.code.code.encode_code(writer, value.native)
    writer.write_unsigned(len(value.storage))
    for item_value_storage_0 in value.storage:
        writer.write_unsigned(item_value_storage_0)


def decode_program(reader: BinaryReader) -> Program:
    """Decode one Program."""
    sections = destack._generated.core.section.decode_section_directory(reader)
    target_layout = destack._generated.mir.table.target.decode_target_layout(reader)
    local_heap = destack._generated.heap.local.heap.options.decode_heap_options(reader)
    shared_heap = (
        destack._generated.heap.shared.heap.options.decode_shared_heap_options(reader)
    )
    strings = destack._generated.program.string.decode_string_table(reader)
    types = destack._generated.program.type.decode_type_table(reader)
    layouts = destack._generated.program.layout.decode_layout_table(reader)
    frames = destack._generated.program.frame.decode_frame_table(reader)
    functions = destack._generated.program.function.decode_function_table(reader)
    dispatch = destack._generated.program.dispatch.decode_dispatch_table(reader)
    traces = destack._generated.heap.core.trace.decode_trace_table(reader)
    globals = destack._generated.program.memory.region.decode_global_table(reader)
    info = reader.read_option(
        lambda: destack._generated.program.info.decode_program_info(reader)
    )
    constant_space = destack._generated.program.memory.space.decode_static_image(reader)
    shared_static_space = destack._generated.program.memory.space.decode_static_image(
        reader
    )
    local_static_space = destack._generated.program.memory.space.decode_static_image(
        reader
    )
    vm = destack._generated.program.vm.code.code.decode_code(reader)
    native = reader.read_option(
        lambda: destack._generated.program.native.code.code.decode_code(reader)
    )
    storage = [reader.read_unsigned() for _ in range(reader.read_number())]

    return Program(
        sections=sections,
        target_layout=target_layout,
        local_heap=local_heap,
        shared_heap=shared_heap,
        strings=strings,
        types=types,
        layouts=layouts,
        frames=frames,
        functions=functions,
        dispatch=dispatch,
        traces=traces,
        globals=globals,
        info=info,
        constant_space=constant_space,
        shared_static_space=shared_static_space,
        local_static_space=local_static_space,
        vm=vm,
        native=native,
        storage=storage,
    )


def to_json_program(value: Program) -> Json:
    """Return one JSON value for one Program."""
    return {
        "sections": destack._generated.core.section.to_json_section_directory(
            value.sections
        ),
        "targetLayout": destack._generated.mir.table.target.to_json_target_layout(
            value.target_layout
        ),
        "localHeap": destack._generated.heap.local.heap.options.to_json_heap_options(
            value.local_heap
        ),
        "sharedHeap": destack._generated.heap.shared.heap.options.to_json_shared_heap_options(
            value.shared_heap
        ),
        "strings": destack._generated.program.string.to_json_string_table(
            value.strings
        ),
        "types": destack._generated.program.type.to_json_type_table(value.types),
        "layouts": destack._generated.program.layout.to_json_layout_table(
            value.layouts
        ),
        "frames": destack._generated.program.frame.to_json_frame_table(value.frames),
        "functions": destack._generated.program.function.to_json_function_table(
            value.functions
        ),
        "dispatch": destack._generated.program.dispatch.to_json_dispatch_table(
            value.dispatch
        ),
        "traces": destack._generated.heap.core.trace.to_json_trace_table(value.traces),
        "globals": destack._generated.program.memory.region.to_json_global_table(
            value.globals
        ),
        **(
            {}
            if value.info is None
            else {
                "info": destack._generated.program.info.to_json_program_info(value.info)
            }
        ),
        "constantSpace": destack._generated.program.memory.space.to_json_static_image(
            value.constant_space
        ),
        "sharedStaticSpace": destack._generated.program.memory.space.to_json_static_image(
            value.shared_static_space
        ),
        "localStaticSpace": destack._generated.program.memory.space.to_json_static_image(
            value.local_static_space
        ),
        "vm": destack._generated.program.vm.code.code.to_json_code(value.vm),
        **(
            {}
            if value.native is None
            else {
                "native": destack._generated.program.native.code.code.to_json_code(
                    value.native
                )
            }
        ),
        "storage": [item_0 for item_0 in value.storage],
    }


def from_json_program(value: Json) -> Program:
    """Return one Program from one JSON value."""
    object_ = json_object(value)

    return Program(
        sections=destack._generated.core.section.from_json_section_directory(
            json_field(object_, "sections")
        ),
        target_layout=destack._generated.mir.table.target.from_json_target_layout(
            json_field(object_, "targetLayout")
        ),
        local_heap=destack._generated.heap.local.heap.options.from_json_heap_options(
            json_field(object_, "localHeap")
        ),
        shared_heap=destack._generated.heap.shared.heap.options.from_json_shared_heap_options(
            json_field(object_, "sharedHeap")
        ),
        strings=destack._generated.program.string.from_json_string_table(
            json_field(object_, "strings")
        ),
        types=destack._generated.program.type.from_json_type_table(
            json_field(object_, "types")
        ),
        layouts=destack._generated.program.layout.from_json_layout_table(
            json_field(object_, "layouts")
        ),
        frames=destack._generated.program.frame.from_json_frame_table(
            json_field(object_, "frames")
        ),
        functions=destack._generated.program.function.from_json_function_table(
            json_field(object_, "functions")
        ),
        dispatch=destack._generated.program.dispatch.from_json_dispatch_table(
            json_field(object_, "dispatch")
        ),
        traces=destack._generated.heap.core.trace.from_json_trace_table(
            json_field(object_, "traces")
        ),
        globals=destack._generated.program.memory.region.from_json_global_table(
            json_field(object_, "globals")
        ),
        info=json_optional(
            object_,
            "info",
            lambda value: destack._generated.program.info.from_json_program_info(value),
        ),
        constant_space=destack._generated.program.memory.space.from_json_static_image(
            json_field(object_, "constantSpace")
        ),
        shared_static_space=destack._generated.program.memory.space.from_json_static_image(
            json_field(object_, "sharedStaticSpace")
        ),
        local_static_space=destack._generated.program.memory.space.from_json_static_image(
            json_field(object_, "localStaticSpace")
        ),
        vm=destack._generated.program.vm.code.code.from_json_code(
            json_field(object_, "vm")
        ),
        native=json_optional(
            object_,
            "native",
            lambda value: destack._generated.program.native.code.code.from_json_code(
                value
            ),
        ),
        storage=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "storage"))
        ],
    )


__all__ = [
    "Program",
    "encode_program",
    "decode_program",
    "to_json_program",
    "from_json_program",
]
