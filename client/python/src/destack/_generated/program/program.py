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
    destack._generated.program.header.encode_program_header(writer, value.header)
    destack._generated.program.type.encode_type_table(writer, value.types)
    destack._generated.mir.metadata.layout.encode_layout_table(writer, value.layouts)
    destack._generated.mir.metadata.frame.encode_frame_table(writer, value.frames)
    destack._generated.program.function.encode_function_table(writer, value.functions)
    destack._generated.mir.metadata.trace.encode_trace_table(writer, value.traces)
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


def decode_program(reader: BinaryReader) -> Program:
    """Decode one Program."""
    header = destack._generated.program.header.decode_program_header(reader)
    types = destack._generated.program.type.decode_type_table(reader)
    layouts = destack._generated.mir.metadata.layout.decode_layout_table(reader)
    frames = destack._generated.mir.metadata.frame.decode_frame_table(reader)
    functions = destack._generated.program.function.decode_function_table(reader)
    traces = destack._generated.mir.metadata.trace.decode_trace_table(reader)
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

    return Program(
        header=header,
        types=types,
        layouts=layouts,
        frames=frames,
        functions=functions,
        traces=traces,
        constant_space=constant_space,
        shared_static_space=shared_static_space,
        local_static_space=local_static_space,
        vm=vm,
        native=native,
    )


def to_json_program(value: Program) -> Json:
    """Return one JSON value for one Program."""
    return {
        "header": destack._generated.program.header.to_json_program_header(
            value.header
        ),
        "types": destack._generated.program.type.to_json_type_table(value.types),
        "layouts": destack._generated.mir.metadata.layout.to_json_layout_table(
            value.layouts
        ),
        "frames": destack._generated.mir.metadata.frame.to_json_frame_table(
            value.frames
        ),
        "functions": destack._generated.program.function.to_json_function_table(
            value.functions
        ),
        "traces": destack._generated.mir.metadata.trace.to_json_trace_table(
            value.traces
        ),
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


def from_json_program(value: Json) -> Program:
    """Return one Program from one JSON value."""
    object_ = json_object(value)

    return Program(
        header=destack._generated.program.header.from_json_program_header(
            json_field(object_, "header")
        ),
        types=destack._generated.program.type.from_json_type_table(
            json_field(object_, "types")
        ),
        layouts=destack._generated.mir.metadata.layout.from_json_layout_table(
            json_field(object_, "layouts")
        ),
        frames=destack._generated.mir.metadata.frame.from_json_frame_table(
            json_field(object_, "frames")
        ),
        functions=destack._generated.program.function.from_json_function_table(
            json_field(object_, "functions")
        ),
        traces=destack._generated.mir.metadata.trace.from_json_trace_table(
            json_field(object_, "traces")
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
    "Program",
    "encode_program",
    "decode_program",
    "to_json_program",
    "from_json_program",
]
