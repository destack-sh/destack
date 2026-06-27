# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_optional,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_image(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramImage:
        """Decode one ProgramImage."""
        return decode_program_image(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_image(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramImage:
        """Return one ProgramImage from one JSON value."""
        return from_json_program_image(value)


def encode_program_image(writer: BinaryWriter, value: ProgramImage) -> None:
    """Encode one ProgramImage."""
    destack._generated.program.header.encode_program_header(writer, value.header)
    destack._generated.program.type.encode_type_table(writer, value.types)
    destack._generated.program.layout.encode_layout_table(writer, value.layouts)
    destack._generated.program.frame.encode_frame_table(writer, value.frames)
    destack._generated.program.function.encode_function_table(writer, value.functions)
    destack._generated.program.dispatch.encode_dispatch_table(writer, value.dispatch)
    destack._generated.mir.table.trace.encode_trace_table(writer, value.traces)
    destack._generated.program.info.encode_program_info(writer, value.info)
    destack._generated.program.memory.space.encode_static_space(
        writer, value.constant_space
    )
    destack._generated.program.memory.space.encode_static_space(
        writer, value.shared_static_space
    )
    destack._generated.program.memory.space.encode_static_space(
        writer, value.local_static_space
    )
    destack._generated.program.vm.code.encode_code(writer, value.vm)
    if value.native is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.native.code.encode_code(writer, value.native)


def decode_program_image(reader: BinaryReader) -> ProgramImage:
    """Decode one ProgramImage."""
    header = destack._generated.program.header.decode_program_header(reader)
    types = destack._generated.program.type.decode_type_table(reader)
    layouts = destack._generated.program.layout.decode_layout_table(reader)
    frames = destack._generated.program.frame.decode_frame_table(reader)
    functions = destack._generated.program.function.decode_function_table(reader)
    dispatch = destack._generated.program.dispatch.decode_dispatch_table(reader)
    traces = destack._generated.mir.table.trace.decode_trace_table(reader)
    info = destack._generated.program.info.decode_program_info(reader)
    constant_space = destack._generated.program.memory.space.decode_static_space(reader)
    shared_static_space = destack._generated.program.memory.space.decode_static_space(
        reader
    )
    local_static_space = destack._generated.program.memory.space.decode_static_space(
        reader
    )
    vm = destack._generated.program.vm.code.decode_code(reader)
    native = reader.read_option(
        lambda: destack._generated.program.native.code.decode_code(reader)
    )

    return ProgramImage(
        header=header,
        types=types,
        layouts=layouts,
        frames=frames,
        functions=functions,
        dispatch=dispatch,
        traces=traces,
        info=info,
        constant_space=constant_space,
        shared_static_space=shared_static_space,
        local_static_space=local_static_space,
        vm=vm,
        native=native,
    )


def to_json_program_image(value: ProgramImage) -> Json:
    """Return one JSON value for one ProgramImage."""
    return {
        "header": destack._generated.program.header.to_json_program_header(
            value.header
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
        "traces": destack._generated.mir.table.trace.to_json_trace_table(value.traces),
        "info": destack._generated.program.info.to_json_program_info(value.info),
        "constantSpace": destack._generated.program.memory.space.to_json_static_space(
            value.constant_space
        ),
        "sharedStaticSpace": destack._generated.program.memory.space.to_json_static_space(
            value.shared_static_space
        ),
        "localStaticSpace": destack._generated.program.memory.space.to_json_static_space(
            value.local_static_space
        ),
        "vm": destack._generated.program.vm.code.to_json_code(value.vm),
        **(
            {}
            if value.native is None
            else {
                "native": destack._generated.program.native.code.to_json_code(
                    value.native
                )
            }
        ),
    }


def from_json_program_image(value: Json) -> ProgramImage:
    """Return one ProgramImage from one JSON value."""
    object_ = json_object(value)

    return ProgramImage(
        header=destack._generated.program.header.from_json_program_header(
            json_field(object_, "header")
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
        traces=destack._generated.mir.table.trace.from_json_trace_table(
            json_field(object_, "traces")
        ),
        info=destack._generated.program.info.from_json_program_info(
            json_field(object_, "info")
        ),
        constant_space=destack._generated.program.memory.space.from_json_static_space(
            json_field(object_, "constantSpace")
        ),
        shared_static_space=destack._generated.program.memory.space.from_json_static_space(
            json_field(object_, "sharedStaticSpace")
        ),
        local_static_space=destack._generated.program.memory.space.from_json_static_space(
            json_field(object_, "localStaticSpace")
        ),
        vm=destack._generated.program.vm.code.from_json_code(json_field(object_, "vm")),
        native=json_optional(
            object_,
            "native",
            lambda value: destack._generated.program.native.code.from_json_code(value),
        ),
    )


__all__ = [
    "ProgramImage",
    "encode_program_image",
    "decode_program_image",
    "to_json_program_image",
    "from_json_program_image",
]
