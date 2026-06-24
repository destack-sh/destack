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

import destack._generated.mir.metadata.frame
import destack._generated.program.function
import destack._generated.program.type


@dataclass(frozen=True, slots=True)
class CodeMap:
    """Native code map for entries, safepoints, roots, and deoptimization."""

    # native function code ranges
    function: Sequence[FunctionCode]
    # native continuation resume code ranges
    resume: Sequence[ResumeCode]
    # native safepoints keyed by safepoint id
    safepoint: Sequence[Safepoint | None]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_map(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeMap:
        """Decode one CodeMap."""
        return decode_code_map(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_map(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeMap:
        """Return one CodeMap from one JSON value."""
        return from_json_code_map(value)


def encode_code_map(writer: BinaryWriter, value: CodeMap) -> None:
    """Encode one CodeMap."""
    writer.write_unsigned(len(value.function))
    for item_value_function_0 in value.function:
        encode_function_code(writer, item_value_function_0)
    writer.write_unsigned(len(value.resume))
    for item_value_resume_0 in value.resume:
        encode_resume_code(writer, item_value_resume_0)
    writer.write_unsigned(len(value.safepoint))
    for item_value_safepoint_0 in value.safepoint:
        if item_value_safepoint_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_safepoint(writer, item_value_safepoint_0)


def decode_code_map(reader: BinaryReader) -> CodeMap:
    """Decode one CodeMap."""
    function = [decode_function_code(reader) for _ in range(reader.read_number())]
    resume = [decode_resume_code(reader) for _ in range(reader.read_number())]
    safepoint = [
        reader.read_option(lambda: decode_safepoint(reader))
        for _ in range(reader.read_number())
    ]

    return CodeMap(
        function=function,
        resume=resume,
        safepoint=safepoint,
    )


def to_json_code_map(value: CodeMap) -> Json:
    """Return one JSON value for one CodeMap."""
    return {
        "function": [to_json_function_code(item_0) for item_0 in value.function],
        "resume": [to_json_resume_code(item_0) for item_0 in value.resume],
        "safepoint": [
            None if item_0 is None else to_json_safepoint(item_0)
            for item_0 in value.safepoint
        ],
    }


def from_json_code_map(value: Json) -> CodeMap:
    """Return one CodeMap from one JSON value."""
    object_ = json_object(value)

    return CodeMap(
        function=[
            from_json_function_code(item_0)
            for item_0 in json_array(json_field(object_, "function"))
        ],
        resume=[
            from_json_resume_code(item_0)
            for item_0 in json_array(json_field(object_, "resume"))
        ],
        safepoint=[
            None if item_0 is None else from_json_safepoint(item_0)
            for item_0 in json_array(json_field(object_, "safepoint"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FunctionCode:
    """One native function code range."""

    # the function covered by this range
    function: destack._generated.program.function.FunctionId
    # the native code byte range
    range: CodeRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_code(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionCode:
        """Decode one FunctionCode."""
        return decode_function_code(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_code(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionCode:
        """Return one FunctionCode from one JSON value."""
        return from_json_function_code(value)


def encode_function_code(writer: BinaryWriter, value: FunctionCode) -> None:
    """Encode one FunctionCode."""
    destack._generated.program.function.encode_function_id(writer, value.function)
    encode_code_range(writer, value.range)


def decode_function_code(reader: BinaryReader) -> FunctionCode:
    """Decode one FunctionCode."""
    function = destack._generated.program.function.decode_function_id(reader)
    range_ = decode_code_range(reader)

    return FunctionCode(
        function=function,
        range=range_,
    )


def to_json_function_code(value: FunctionCode) -> Json:
    """Return one JSON value for one FunctionCode."""
    return {
        "function": destack._generated.program.function.to_json_function_id(
            value.function
        ),
        "range": to_json_code_range(value.range),
    }


def from_json_function_code(value: Json) -> FunctionCode:
    """Return one FunctionCode from one JSON value."""
    object_ = json_object(value)

    return FunctionCode(
        function=destack._generated.program.function.from_json_function_id(
            json_field(object_, "function")
        ),
        range=from_json_code_range(json_field(object_, "range")),
    )


@dataclass(frozen=True, slots=True)
class CodeRange:
    """One native image byte range."""

    # the byte offset from the native image base
    offset: int
    # the byte length of this range
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeRange:
        """Decode one CodeRange."""
        return decode_code_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_range(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeRange:
        """Return one CodeRange from one JSON value."""
        return from_json_code_range(value)


def encode_code_range(writer: BinaryWriter, value: CodeRange) -> None:
    """Encode one CodeRange."""
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.byte_len)


def decode_code_range(reader: BinaryReader) -> CodeRange:
    """Decode one CodeRange."""
    offset = reader.read_number()
    byte_len = reader.read_number()

    return CodeRange(
        offset=offset,
        byte_len=byte_len,
    )


def to_json_code_range(value: CodeRange) -> Json:
    """Return one JSON value for one CodeRange."""
    return {
        "offset": value.offset,
        "byteLen": value.byte_len,
    }


def from_json_code_range(value: Json) -> CodeRange:
    """Return one CodeRange from one JSON value."""
    object_ = json_object(value)

    return CodeRange(
        offset=json_int(json_field(object_, "offset")),
        byte_len=json_int(json_field(object_, "byteLen")),
    )


@dataclass(frozen=True, slots=True)
class ResumeCode:
    """One native resume entry code range."""

    # the frame state resumed by this range
    frame_state: destack._generated.mir.metadata.frame.FrameStateId
    # the native code byte range
    range: CodeRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resume_code(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeCode:
        """Decode one ResumeCode."""
        return decode_resume_code(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resume_code(self)

    @classmethod
    def from_json(cls, value: Json) -> ResumeCode:
        """Return one ResumeCode from one JSON value."""
        return from_json_resume_code(value)


def encode_resume_code(writer: BinaryWriter, value: ResumeCode) -> None:
    """Encode one ResumeCode."""
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.frame_state
    )
    encode_code_range(writer, value.range)


def decode_resume_code(reader: BinaryReader) -> ResumeCode:
    """Decode one ResumeCode."""
    frame_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)
    range_ = decode_code_range(reader)

    return ResumeCode(
        frame_state=frame_state,
        range=range_,
    )


def to_json_resume_code(value: ResumeCode) -> Json:
    """Return one JSON value for one ResumeCode."""
    return {
        "frameState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.frame_state
        ),
        "range": to_json_code_range(value.range),
    }


def from_json_resume_code(value: Json) -> ResumeCode:
    """Return one ResumeCode from one JSON value."""
    object_ = json_object(value)

    return ResumeCode(
        frame_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "frameState")
        ),
        range=from_json_code_range(json_field(object_, "range")),
    )


@dataclass(frozen=True, slots=True)
class Safepoint:
    """One native safepoint."""

    # the safepoint id passed through the native ABI
    id: int
    # the function containing this safepoint
    function: destack._generated.program.function.FunctionId
    # the byte offset from the native image base
    offset: int
    # the VM frame state corresponding to this safepoint
    frame_state: destack._generated.mir.metadata.frame.FrameStateId
    # native roots live at this safepoint
    roots: Sequence[NativeRoot]
    # materialization target when this safepoint can deoptimize
    deopt: destack._generated.mir.metadata.frame.FrameStateId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_safepoint(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Safepoint:
        """Decode one Safepoint."""
        return decode_safepoint(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_safepoint(self)

    @classmethod
    def from_json(cls, value: Json) -> Safepoint:
        """Return one Safepoint from one JSON value."""
        return from_json_safepoint(value)


def encode_safepoint(writer: BinaryWriter, value: Safepoint) -> None:
    """Encode one Safepoint."""
    writer.write_unsigned(value.id)
    destack._generated.program.function.encode_function_id(writer, value.function)
    writer.write_unsigned(value.offset)
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.frame_state
    )
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        encode_native_root(writer, item_value_roots_0)
    if value.deopt is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.metadata.frame.encode_frame_state_id(writer, value.deopt)


def decode_safepoint(reader: BinaryReader) -> Safepoint:
    """Decode one Safepoint."""
    id = reader.read_number()
    function = destack._generated.program.function.decode_function_id(reader)
    offset = reader.read_number()
    frame_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)
    roots = [decode_native_root(reader) for _ in range(reader.read_number())]
    deopt = reader.read_option(
        lambda: destack._generated.mir.metadata.frame.decode_frame_state_id(reader)
    )

    return Safepoint(
        id=id,
        function=function,
        offset=offset,
        frame_state=frame_state,
        roots=roots,
        deopt=deopt,
    )


def to_json_safepoint(value: Safepoint) -> Json:
    """Return one JSON value for one Safepoint."""
    return {
        "id": value.id,
        "function": destack._generated.program.function.to_json_function_id(
            value.function
        ),
        "offset": value.offset,
        "frameState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.frame_state
        ),
        "roots": [to_json_native_root(item_0) for item_0 in value.roots],
        **(
            {}
            if value.deopt is None
            else {
                "deopt": destack._generated.mir.metadata.frame.to_json_frame_state_id(
                    value.deopt
                )
            }
        ),
    }


def from_json_safepoint(value: Json) -> Safepoint:
    """Return one Safepoint from one JSON value."""
    object_ = json_object(value)

    return Safepoint(
        id=json_int(json_field(object_, "id")),
        function=destack._generated.program.function.from_json_function_id(
            json_field(object_, "function")
        ),
        offset=json_int(json_field(object_, "offset")),
        frame_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "frameState")
        ),
        roots=[
            from_json_native_root(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
        deopt=json_optional(
            object_,
            "deopt",
            lambda value: (
                destack._generated.mir.metadata.frame.from_json_frame_state_id(value)
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class NativeRoot:
    """One native root location at one safepoint."""

    # signed byte offset from the native frame base
    offset: int
    # the root value type
    ty: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_native_root(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NativeRoot:
        """Decode one NativeRoot."""
        return decode_native_root(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_native_root(self)

    @classmethod
    def from_json(cls, value: Json) -> NativeRoot:
        """Return one NativeRoot from one JSON value."""
        return from_json_native_root(value)


def encode_native_root(writer: BinaryWriter, value: NativeRoot) -> None:
    """Encode one NativeRoot."""
    writer.write_signed(value.offset)
    destack._generated.program.type.encode_type_id(writer, value.ty)


def decode_native_root(reader: BinaryReader) -> NativeRoot:
    """Decode one NativeRoot."""
    offset = reader.read_signed_number()
    ty = destack._generated.program.type.decode_type_id(reader)

    return NativeRoot(
        offset=offset,
        ty=ty,
    )


def to_json_native_root(value: NativeRoot) -> Json:
    """Return one JSON value for one NativeRoot."""
    return {
        "offset": value.offset,
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
    }


def from_json_native_root(value: Json) -> NativeRoot:
    """Return one NativeRoot from one JSON value."""
    object_ = json_object(value)

    return NativeRoot(
        offset=json_int(json_field(object_, "offset")),
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
    )


__all__ = [
    "CodeMap",
    "encode_code_map",
    "decode_code_map",
    "to_json_code_map",
    "from_json_code_map",
    "FunctionCode",
    "encode_function_code",
    "decode_function_code",
    "to_json_function_code",
    "from_json_function_code",
    "CodeRange",
    "encode_code_range",
    "decode_code_range",
    "to_json_code_range",
    "from_json_code_range",
    "ResumeCode",
    "encode_resume_code",
    "decode_resume_code",
    "to_json_resume_code",
    "from_json_resume_code",
    "Safepoint",
    "encode_safepoint",
    "decode_safepoint",
    "to_json_safepoint",
    "from_json_safepoint",
    "NativeRoot",
    "encode_native_root",
    "decode_native_root",
    "to_json_native_root",
    "from_json_native_root",
]
