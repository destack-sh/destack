# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_capture_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureSegment:
        """Decode one CaptureSegment."""
        return decode_capture_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_capture_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> CaptureSegment:
        """Return one CaptureSegment from one JSON value."""
        return from_json_capture_segment(value)


def encode_capture_segment(writer: BinaryWriter, value: CaptureSegment) -> None:
    """Encode one CaptureSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_frame_id)
    writer.write_unsigned(len(value.frames))
    for item_value_frames_0 in value.frames:
        encode_capture_frame(writer, item_value_frames_0)
    entries_value_capture_by_function_0 = []
    for (
        key_value_capture_by_function_0,
        item_value_capture_by_function_0,
    ) in value.capture_by_function.items():

        def write_key_value_capture_by_function_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_capture_by_function_0
            )

        key_bytes = nested_bytes(write_key_value_capture_by_function_0)
        entries_value_capture_by_function_0.append(
            (
                key_value_capture_by_function_0,
                item_value_capture_by_function_0,
                key_bytes,
            )
        )
    entries_value_capture_by_function_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_capture_by_function_0))
    for entry_value_capture_by_function_0 in entries_value_capture_by_function_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_capture_by_function_0[0]
        )
        encode_capture(writer, entry_value_capture_by_function_0[1])


def decode_capture_segment(reader: BinaryReader) -> CaptureSegment:
    """Decode one CaptureSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_frame_id = reader.read_number()
    frames = [decode_capture_frame(reader) for _ in range(reader.read_number())]
    capture_by_function = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): decode_capture(reader)
        for _ in range(reader.read_number())
    }

    return CaptureSegment(
        module_id=module_id,
        first_frame_id=first_frame_id,
        frames=frames,
        capture_by_function=capture_by_function,
    )


def to_json_capture_segment(value: CaptureSegment) -> Json:
    """Return one JSON value for one CaptureSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstFrameId": value.first_frame_id,
        "frames": [to_json_capture_frame(item_0) for item_0 in value.frames],
        "captureByFunction": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                to_json_capture(item_0),
            ]
            for key_0, item_0 in value.capture_by_function.items()
        ],
    }


def from_json_capture_segment(value: Json) -> CaptureSegment:
    """Return one CaptureSegment from one JSON value."""
    object_ = json_object(value)

    return CaptureSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_frame_id=json_int(json_field(object_, "firstFrameId")),
        frames=[
            from_json_capture_frame(item_0)
            for item_0 in json_array(json_field(object_, "frames"))
        ],
        capture_by_function={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): from_json_capture(item_0)
            for key_0, item_0 in json_array(json_field(object_, "captureByFunction"))
        },
    )


@dataclass(frozen=True, slots=True)
class CaptureFrame:
    """Lexical bindings lifted for one scope."""

    # the lexical scope lifted into this frame
    scope: destack._generated.dir.symbol.scope.GlobalScopeId
    # the checked frame representation type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the fields in lexical order
    fields: Sequence[CaptureFrameField]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_capture_frame(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureFrame:
        """Decode one CaptureFrame."""
        return decode_capture_frame(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_capture_frame(self)

    @classmethod
    def from_json(cls, value: Json) -> CaptureFrame:
        """Return one CaptureFrame from one JSON value."""
        return from_json_capture_frame(value)


def encode_capture_frame(writer: BinaryWriter, value: CaptureFrame) -> None:
    """Encode one CaptureFrame."""
    destack._generated.dir.symbol.scope.encode_global_scope_id(writer, value.scope)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_capture_frame_field(writer, item_value_fields_0)


def decode_capture_frame(reader: BinaryReader) -> CaptureFrame:
    """Decode one CaptureFrame."""
    scope = destack._generated.dir.symbol.scope.decode_global_scope_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    fields = [decode_capture_frame_field(reader) for _ in range(reader.read_number())]

    return CaptureFrame(
        scope=scope,
        ty=ty,
        fields=fields,
    )


def to_json_capture_frame(value: CaptureFrame) -> Json:
    """Return one JSON value for one CaptureFrame."""
    return {
        "scope": destack._generated.dir.symbol.scope.to_json_global_scope_id(
            value.scope
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        "fields": [to_json_capture_frame_field(item_0) for item_0 in value.fields],
    }


def from_json_capture_frame(value: Json) -> CaptureFrame:
    """Return one CaptureFrame from one JSON value."""
    object_ = json_object(value)

    return CaptureFrame(
        scope=destack._generated.dir.symbol.scope.from_json_global_scope_id(
            json_field(object_, "scope")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        fields=[
            from_json_capture_frame_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CaptureFrameField:
    """One binding stored in a capture frame."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked field type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_capture_frame_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureFrameField:
        """Decode one CaptureFrameField."""
        return decode_capture_frame_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_capture_frame_field(self)

    @classmethod
    def from_json(cls, value: Json) -> CaptureFrameField:
        """Return one CaptureFrameField from one JSON value."""
        return from_json_capture_frame_field(value)


def encode_capture_frame_field(writer: BinaryWriter, value: CaptureFrameField) -> None:
    """Encode one CaptureFrameField."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_capture_frame_field(reader: BinaryReader) -> CaptureFrameField:
    """Decode one CaptureFrameField."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return CaptureFrameField(
        symbol=symbol,
        ty=ty,
    )


def to_json_capture_frame_field(value: CaptureFrameField) -> Json:
    """Return one JSON value for one CaptureFrameField."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_capture_frame_field(value: Json) -> CaptureFrameField:
    """Return one CaptureFrameField from one JSON value."""
    object_ = json_object(value)

    return CaptureFrameField(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_capture(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Capture:
        """Decode one Capture."""
        return decode_capture(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_capture(self)

    @classmethod
    def from_json(cls, value: Json) -> Capture:
        """Return one Capture from one JSON value."""
        return from_json_capture(value)


def encode_capture(writer: BinaryWriter, value: Capture) -> None:
    """Encode one Capture."""
    writer.write_unsigned(len(value.frames))
    for item_value_frames_0 in value.frames:
        encode_local_capture_frame_id(writer, item_value_frames_0)
    writer.write_unsigned(len(value.captures))
    for item_value_captures_0 in value.captures:
        encode_captured_binding(writer, item_value_captures_0)
    if value.this is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_captured_receiver(writer, value.this)
    if value.directive is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_capture_directive(writer, value.directive)


def decode_capture(reader: BinaryReader) -> Capture:
    """Decode one Capture."""
    frames = [
        decode_local_capture_frame_id(reader) for _ in range(reader.read_number())
    ]
    captures = [decode_captured_binding(reader) for _ in range(reader.read_number())]
    this = reader.read_option(lambda: decode_captured_receiver(reader))
    directive = reader.read_option(lambda: decode_capture_directive(reader))

    return Capture(
        frames=frames,
        captures=captures,
        this=this,
        directive=directive,
    )


def to_json_capture(value: Capture) -> Json:
    """Return one JSON value for one Capture."""
    return {
        "frames": [to_json_local_capture_frame_id(item_0) for item_0 in value.frames],
        "captures": [to_json_captured_binding(item_0) for item_0 in value.captures],
        **(
            {}
            if value.this is None
            else {"this": to_json_captured_receiver(value.this)}
        ),
        **(
            {}
            if value.directive is None
            else {"directive": to_json_capture_directive(value.directive)}
        ),
    }


def from_json_capture(value: Json) -> Capture:
    """Return one Capture from one JSON value."""
    object_ = json_object(value)

    return Capture(
        frames=[
            from_json_local_capture_frame_id(item_0)
            for item_0 in json_array(json_field(object_, "frames"))
        ],
        captures=[
            from_json_captured_binding(item_0)
            for item_0 in json_array(json_field(object_, "captures"))
        ],
        this=json_optional(
            object_, "this", lambda value: from_json_captured_receiver(value)
        ),
        directive=json_optional(
            object_, "directive", lambda value: from_json_capture_directive(value)
        ),
    )


"""Unique identifier for a capture frame."""
LocalCaptureFrameId: typing.TypeAlias = int


def encode_local_capture_frame_id(
    writer: BinaryWriter, value: LocalCaptureFrameId
) -> None:
    """Encode one LocalCaptureFrameId."""
    writer.write_unsigned(value)


def decode_local_capture_frame_id(reader: BinaryReader) -> LocalCaptureFrameId:
    """Decode one LocalCaptureFrameId."""
    return reader.read_number()


def to_json_local_capture_frame_id(value: LocalCaptureFrameId) -> Json:
    """Return one JSON value for one LocalCaptureFrameId."""
    return value


def from_json_local_capture_frame_id(value: Json) -> LocalCaptureFrameId:
    """Return one LocalCaptureFrameId from one JSON value."""
    return json_int(value)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_captured_binding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_captured_binding(self)


@dataclass(frozen=True, slots=True)
class CapturedBindingBorrow:
    """Borrow the binding directly from the enclosing scope."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["borrow"] = "borrow"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_captured_binding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_captured_binding(self)


@dataclass(frozen=True, slots=True)
class CapturedBindingCopy:
    """Copy the binding value into the closure environment."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["copy"] = "copy"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_captured_binding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_captured_binding(self)


@dataclass(frozen=True, slots=True)
class CapturedBindingMove:
    """Move the binding value into the closure environment."""

    # the captured symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked binding type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_captured_binding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_captured_binding(self)


"""A single captured lexical binding."""
CapturedBinding: typing.TypeAlias = (
    CapturedBindingManage
    | CapturedBindingBorrow
    | CapturedBindingCopy
    | CapturedBindingMove
)


def encode_captured_binding(writer: BinaryWriter, value: CapturedBinding) -> None:
    """Encode one CapturedBinding."""
    if value.kind == "manage":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
        encode_local_capture_frame_id(writer, value.frame)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "borrow":
        writer.write_unsigned(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "copy":
        writer.write_unsigned(2)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "move":
        writer.write_unsigned(3)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    else:
        raise SerdeError("unknown enum variant")


def decode_captured_binding(reader: BinaryReader) -> CapturedBinding:
    """Decode one CapturedBinding."""
    variant = reader.read_number()

    if variant == 0:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        frame = decode_local_capture_frame_id(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return CapturedBindingManage(
            symbol=symbol,
            frame=frame,
            ty=ty,
        )
    elif variant == 1:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return CapturedBindingBorrow(
            symbol=symbol,
            ty=ty,
        )
    elif variant == 2:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return CapturedBindingCopy(
            symbol=symbol,
            ty=ty,
        )
    elif variant == 3:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return CapturedBindingMove(
            symbol=symbol,
            ty=ty,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_captured_binding(value: CapturedBinding) -> Json:
    """Return one JSON value for one CapturedBinding."""
    if value.kind == "manage":
        return {
            "kind": "manage",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
            "frame": to_json_local_capture_frame_id(value.frame),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "borrow":
        return {
            "kind": "borrow",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "copy":
        return {
            "kind": "copy",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "move":
        return {
            "kind": "move",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_captured_binding(value: Json) -> CapturedBinding:
    """Return one CapturedBinding from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "manage":
        return CapturedBindingManage(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
            frame=from_json_local_capture_frame_id(json_field(object_, "frame")),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "borrow":
        return CapturedBindingBorrow(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "copy":
        return CapturedBindingCopy(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "move":
        return CapturedBindingMove(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CapturedReceiver:
    """A captured lexical receiver."""

    # the receiver symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the capture mode for the receiver
    mode: CaptureMode
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_captured_receiver(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CapturedReceiver:
        """Decode one CapturedReceiver."""
        return decode_captured_receiver(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_captured_receiver(self)

    @classmethod
    def from_json(cls, value: Json) -> CapturedReceiver:
        """Return one CapturedReceiver from one JSON value."""
        return from_json_captured_receiver(value)


def encode_captured_receiver(writer: BinaryWriter, value: CapturedReceiver) -> None:
    """Encode one CapturedReceiver."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    encode_capture_mode(writer, value.mode)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_captured_receiver(reader: BinaryReader) -> CapturedReceiver:
    """Decode one CapturedReceiver."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    mode = decode_capture_mode(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return CapturedReceiver(
        symbol=symbol,
        mode=mode,
        ty=ty,
    )


def to_json_captured_receiver(value: CapturedReceiver) -> Json:
    """Return one JSON value for one CapturedReceiver."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "mode": to_json_capture_mode(value.mode),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_captured_receiver(value: Json) -> CapturedReceiver:
    """Return one CapturedReceiver from one JSON value."""
    object_ = json_object(value)

    return CapturedReceiver(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        mode=from_json_capture_mode(json_field(object_, "mode")),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


"""The capture mode for a closure binding."""
CaptureMode: typing.TypeAlias = (
    typing.Literal["manage"]
    | typing.Literal["borrow"]
    | typing.Literal["copy"]
    | typing.Literal["move"]
)


def encode_capture_mode(writer: BinaryWriter, value: CaptureMode) -> None:
    """Encode one CaptureMode."""
    if value == "manage":
        writer.write_unsigned(0)
    elif value == "borrow":
        writer.write_unsigned(1)
    elif value == "copy":
        writer.write_unsigned(2)
    elif value == "move":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_capture_mode(reader: BinaryReader) -> CaptureMode:
    """Decode one CaptureMode."""
    variant = reader.read_number()

    if variant == 0:
        return "manage"
    elif variant == 1:
        return "borrow"
    elif variant == 2:
        return "copy"
    elif variant == 3:
        return "move"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_capture_mode(value: CaptureMode) -> Json:
    """Return one JSON value for one CaptureMode."""
    return value


def from_json_capture_mode(value: Json) -> CaptureMode:
    """Return one CaptureMode from one JSON value."""
    variant = json_string(value)

    if variant == "manage":
        return "manage"
    elif variant == "borrow":
        return "borrow"
    elif variant == "copy":
        return "copy"
    elif variant == "move":
        return "move"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CaptureDirective:
    """The capture directive for a closure."""

    # the default capture mode
    default: CaptureMode
    # per binding rules by name
    rules: Sequence[CaptureRule]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_capture_directive(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureDirective:
        """Decode one CaptureDirective."""
        return decode_capture_directive(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_capture_directive(self)

    @classmethod
    def from_json(cls, value: Json) -> CaptureDirective:
        """Return one CaptureDirective from one JSON value."""
        return from_json_capture_directive(value)


def encode_capture_directive(writer: BinaryWriter, value: CaptureDirective) -> None:
    """Encode one CaptureDirective."""
    encode_capture_mode(writer, value.default)
    writer.write_unsigned(len(value.rules))
    for item_value_rules_0 in value.rules:
        encode_capture_rule(writer, item_value_rules_0)


def decode_capture_directive(reader: BinaryReader) -> CaptureDirective:
    """Decode one CaptureDirective."""
    default = decode_capture_mode(reader)
    rules = [decode_capture_rule(reader) for _ in range(reader.read_number())]

    return CaptureDirective(
        default=default,
        rules=rules,
    )


def to_json_capture_directive(value: CaptureDirective) -> Json:
    """Return one JSON value for one CaptureDirective."""
    return {
        "default": to_json_capture_mode(value.default),
        "rules": [to_json_capture_rule(item_0) for item_0 in value.rules],
    }


def from_json_capture_directive(value: Json) -> CaptureDirective:
    """Return one CaptureDirective from one JSON value."""
    object_ = json_object(value)

    return CaptureDirective(
        default=from_json_capture_mode(json_field(object_, "default")),
        rules=[
            from_json_capture_rule(item_0)
            for item_0 in json_array(json_field(object_, "rules"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CaptureRule:
    """A capture rule keyed by name."""

    # the binding name to override
    name: destack._generated.core.string.StringId
    # the capture mode to use for this binding
    mode: CaptureMode

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_capture_rule(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CaptureRule:
        """Decode one CaptureRule."""
        return decode_capture_rule(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_capture_rule(self)

    @classmethod
    def from_json(cls, value: Json) -> CaptureRule:
        """Return one CaptureRule from one JSON value."""
        return from_json_capture_rule(value)


def encode_capture_rule(writer: BinaryWriter, value: CaptureRule) -> None:
    """Encode one CaptureRule."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    encode_capture_mode(writer, value.mode)


def decode_capture_rule(reader: BinaryReader) -> CaptureRule:
    """Decode one CaptureRule."""
    name = destack._generated.core.string.decode_string_id(reader)
    mode = decode_capture_mode(reader)

    return CaptureRule(
        name=name,
        mode=mode,
    )


def to_json_capture_rule(value: CaptureRule) -> Json:
    """Return one JSON value for one CaptureRule."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "mode": to_json_capture_mode(value.mode),
    }


def from_json_capture_rule(value: Json) -> CaptureRule:
    """Return one CaptureRule from one JSON value."""
    object_ = json_object(value)

    return CaptureRule(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        mode=from_json_capture_mode(json_field(object_, "mode")),
    )


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
