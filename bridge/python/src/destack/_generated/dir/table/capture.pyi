# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.symbol.scope
import destack._generated.dir.symbol.symbol
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class CaptureSegment:
    """Captures added by one DIR phase."""

    # the module id of the capture segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first capture frame id owned by this table segment
    first_frame_id: int
    # capture frames owned by this segment
    frames: Sequence[CaptureFrame]
    # capture for each function symbol
    capture_by_function: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId, Capture
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CaptureSegment: ...

def encode_capture_segment(writer: BinaryWriter, value: CaptureSegment) -> None: ...
def decode_capture_segment(reader: BinaryReader) -> CaptureSegment: ...
def to_json_capture_segment(value: CaptureSegment) -> Json: ...
def from_json_capture_segment(value: Json) -> CaptureSegment: ...

@dataclass(frozen=True, slots=True)
class CaptureFrame:
    """Lexical bindings lifted for one scope."""

    # the lexical scope lifted into this frame
    scope: destack._generated.dir.symbol.scope.GlobalScopeId
    # the checked frame representation type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the fields in lexical order
    fields: Sequence[CaptureFrameField]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureFrame: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CaptureFrame: ...

def encode_capture_frame(writer: BinaryWriter, value: CaptureFrame) -> None: ...
def decode_capture_frame(reader: BinaryReader) -> CaptureFrame: ...
def to_json_capture_frame(value: CaptureFrame) -> Json: ...
def from_json_capture_frame(value: Json) -> CaptureFrame: ...

@dataclass(frozen=True, slots=True)
class CaptureFrameField:
    """One binding stored in a capture frame."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked field type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureFrameField: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CaptureFrameField: ...

def encode_capture_frame_field(
    writer: BinaryWriter, value: CaptureFrameField
) -> None: ...
def decode_capture_frame_field(reader: BinaryReader) -> CaptureFrameField: ...
def to_json_capture_frame_field(value: CaptureFrameField) -> Json: ...
def from_json_capture_frame_field(value: Json) -> CaptureFrameField: ...

@dataclass(frozen=True, slots=True)
class Capture:
    """Captures for a function declaration."""

    # the lexical frames used by this function
    frames: Sequence[LocalCaptureFrameId]
    # the resolved captures in discovery order
    captures: Sequence[CapturedBinding]
    # the captured `this` binding
    this: CapturedReceiver | None
    # the capture directive applied to this function
    directive: CaptureDirective | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Capture: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Capture: ...

def encode_capture(writer: BinaryWriter, value: Capture) -> None: ...
def decode_capture(reader: BinaryReader) -> Capture: ...
def to_json_capture(value: Capture) -> Json: ...
def from_json_capture(value: Json) -> Capture: ...

"""Unique identifier for a capture frame."""
LocalCaptureFrameId: typing.TypeAlias = int

def encode_local_capture_frame_id(
    writer: BinaryWriter, value: LocalCaptureFrameId
) -> None: ...
def decode_local_capture_frame_id(reader: BinaryReader) -> LocalCaptureFrameId: ...
def to_json_local_capture_frame_id(value: LocalCaptureFrameId) -> Json: ...
def from_json_local_capture_frame_id(value: Json) -> LocalCaptureFrameId: ...

@dataclass(frozen=True, slots=True)
class CapturedBindingManage:
    """Preserve the variable through compiler-managed storage."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the capture frame that stores this binding
    frame: LocalCaptureFrameId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["manage"] = "manage"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CapturedBindingBorrow:
    """Borrow the binding directly from the enclosing scope."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["borrow"] = "borrow"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CapturedBindingCopy:
    """Copy the binding value into the closure environment."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["copy"] = "copy"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CapturedBindingMove:
    """Move the binding value into the closure environment."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A single captured lexical binding."""
CapturedBinding: typing.TypeAlias = (
    CapturedBindingManage
    | CapturedBindingBorrow
    | CapturedBindingCopy
    | CapturedBindingMove
)

def encode_captured_binding(writer: BinaryWriter, value: CapturedBinding) -> None: ...
def decode_captured_binding(reader: BinaryReader) -> CapturedBinding: ...
def to_json_captured_binding(value: CapturedBinding) -> Json: ...
def from_json_captured_binding(value: Json) -> CapturedBinding: ...

@dataclass(frozen=True, slots=True)
class CapturedReceiver:
    """A captured lexical receiver."""

    # the receiver symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the capture mode for the receiver
    mode: CaptureMode
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CapturedReceiver: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CapturedReceiver: ...

def encode_captured_receiver(writer: BinaryWriter, value: CapturedReceiver) -> None: ...
def decode_captured_receiver(reader: BinaryReader) -> CapturedReceiver: ...
def to_json_captured_receiver(value: CapturedReceiver) -> Json: ...
def from_json_captured_receiver(value: Json) -> CapturedReceiver: ...

"""The capture mode for a closure binding."""
CaptureMode: typing.TypeAlias = (
    typing.Literal["manage"]
    | typing.Literal["borrow"]
    | typing.Literal["copy"]
    | typing.Literal["move"]
)

def encode_capture_mode(writer: BinaryWriter, value: CaptureMode) -> None: ...
def decode_capture_mode(reader: BinaryReader) -> CaptureMode: ...
def to_json_capture_mode(value: CaptureMode) -> Json: ...
def from_json_capture_mode(value: Json) -> CaptureMode: ...

@dataclass(frozen=True, slots=True)
class CaptureDirective:
    """The capture directive for a closure."""

    # the default capture mode
    default: CaptureMode
    # per binding rules by name
    rules: Sequence[CaptureRule]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureDirective: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CaptureDirective: ...

def encode_capture_directive(writer: BinaryWriter, value: CaptureDirective) -> None: ...
def decode_capture_directive(reader: BinaryReader) -> CaptureDirective: ...
def to_json_capture_directive(value: CaptureDirective) -> Json: ...
def from_json_capture_directive(value: Json) -> CaptureDirective: ...

@dataclass(frozen=True, slots=True)
class CaptureRule:
    """A capture rule keyed by name."""

    # the binding name to override
    name: destack._generated.core.string.StringId
    # the capture mode to use for this binding
    mode: CaptureMode

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureRule: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CaptureRule: ...

def encode_capture_rule(writer: BinaryWriter, value: CaptureRule) -> None: ...
def decode_capture_rule(reader: BinaryReader) -> CaptureRule: ...
def to_json_capture_rule(value: CaptureRule) -> Json: ...
def from_json_capture_rule(value: Json) -> CaptureRule: ...

__all__ = [
    "CaptureSegment",
    "encode_capture_segment",
    "decode_capture_segment",
    "to_json_capture_segment",
    "from_json_capture_segment",
    "CaptureFrame",
    "encode_capture_frame",
    "decode_capture_frame",
    "to_json_capture_frame",
    "from_json_capture_frame",
    "CaptureFrameField",
    "encode_capture_frame_field",
    "decode_capture_frame_field",
    "to_json_capture_frame_field",
    "from_json_capture_frame_field",
    "Capture",
    "encode_capture",
    "decode_capture",
    "to_json_capture",
    "from_json_capture",
    "LocalCaptureFrameId",
    "encode_local_capture_frame_id",
    "decode_local_capture_frame_id",
    "to_json_local_capture_frame_id",
    "from_json_local_capture_frame_id",
    "CapturedBinding",
    "encode_captured_binding",
    "decode_captured_binding",
    "to_json_captured_binding",
    "from_json_captured_binding",
    "CapturedBindingManage",
    "CapturedBindingBorrow",
    "CapturedBindingCopy",
    "CapturedBindingMove",
    "CapturedReceiver",
    "encode_captured_receiver",
    "decode_captured_receiver",
    "to_json_captured_receiver",
    "from_json_captured_receiver",
    "CaptureMode",
    "encode_capture_mode",
    "decode_capture_mode",
    "to_json_capture_mode",
    "from_json_capture_mode",
    "CaptureDirective",
    "encode_capture_directive",
    "decode_capture_directive",
    "to_json_capture_directive",
    "from_json_capture_directive",
    "CaptureRule",
    "encode_capture_rule",
    "decode_capture_rule",
    "to_json_capture_rule",
    "from_json_capture_rule",
]
