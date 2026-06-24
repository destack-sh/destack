# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeMap: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeMap: ...

def encode_code_map(writer: BinaryWriter, value: CodeMap) -> None: ...
def decode_code_map(reader: BinaryReader) -> CodeMap: ...
def to_json_code_map(value: CodeMap) -> Json: ...
def from_json_code_map(value: Json) -> CodeMap: ...

@dataclass(frozen=True, slots=True)
class FunctionCode:
    """One native function code range."""

    # the function covered by this range
    function: destack._generated.program.function.FunctionId
    # the native code byte range
    range: CodeRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionCode: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionCode: ...

def encode_function_code(writer: BinaryWriter, value: FunctionCode) -> None: ...
def decode_function_code(reader: BinaryReader) -> FunctionCode: ...
def to_json_function_code(value: FunctionCode) -> Json: ...
def from_json_function_code(value: Json) -> FunctionCode: ...

@dataclass(frozen=True, slots=True)
class CodeRange:
    """One native image byte range."""

    # the byte offset from the native image base
    offset: int
    # the byte length of this range
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CodeRange: ...

def encode_code_range(writer: BinaryWriter, value: CodeRange) -> None: ...
def decode_code_range(reader: BinaryReader) -> CodeRange: ...
def to_json_code_range(value: CodeRange) -> Json: ...
def from_json_code_range(value: Json) -> CodeRange: ...

@dataclass(frozen=True, slots=True)
class ResumeCode:
    """One native resume entry code range."""

    # the frame state resumed by this range
    frame_state: destack._generated.mir.metadata.frame.FrameStateId
    # the native code byte range
    range: CodeRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeCode: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ResumeCode: ...

def encode_resume_code(writer: BinaryWriter, value: ResumeCode) -> None: ...
def decode_resume_code(reader: BinaryReader) -> ResumeCode: ...
def to_json_resume_code(value: ResumeCode) -> Json: ...
def from_json_resume_code(value: Json) -> ResumeCode: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Safepoint: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Safepoint: ...

def encode_safepoint(writer: BinaryWriter, value: Safepoint) -> None: ...
def decode_safepoint(reader: BinaryReader) -> Safepoint: ...
def to_json_safepoint(value: Safepoint) -> Json: ...
def from_json_safepoint(value: Json) -> Safepoint: ...

@dataclass(frozen=True, slots=True)
class NativeRoot:
    """One native root location at one safepoint."""

    # signed byte offset from the native frame base
    offset: int
    # the root value type
    ty: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NativeRoot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NativeRoot: ...

def encode_native_root(writer: BinaryWriter, value: NativeRoot) -> None: ...
def decode_native_root(reader: BinaryReader) -> NativeRoot: ...
def to_json_native_root(value: NativeRoot) -> Json: ...
def from_json_native_root(value: Json) -> NativeRoot: ...

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
