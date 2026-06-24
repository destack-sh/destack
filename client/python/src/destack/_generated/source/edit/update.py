# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class ByteRange:
    """One byte range in source text."""

    # inclusive start byte offset
    start: int
    # exclusive end byte offset
    end: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_byte_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ByteRange:
        """Decode one ByteRange."""
        return decode_byte_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_byte_range(self)

    @classmethod
    def from_json(cls, value: Json) -> ByteRange:
        """Return one ByteRange from one JSON value."""
        return from_json_byte_range(value)


def encode_byte_range(writer: BinaryWriter, value: ByteRange) -> None:
    """Encode one ByteRange."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.end)


def decode_byte_range(reader: BinaryReader) -> ByteRange:
    """Decode one ByteRange."""
    start = reader.read_number()
    end = reader.read_number()

    return ByteRange(
        start=start,
        end=end,
    )


def to_json_byte_range(value: ByteRange) -> Json:
    """Return one JSON value for one ByteRange."""
    return {
        "start": value.start,
        "end": value.end,
    }


def from_json_byte_range(value: Json) -> ByteRange:
    """Return one ByteRange from one JSON value."""
    object_ = json_object(value)

    return ByteRange(
        start=json_int(json_field(object_, "start")),
        end=json_int(json_field(object_, "end")),
    )


@dataclass(frozen=True, slots=True)
class TextPatch:
    """One source text replacement."""

    # replaced byte range
    range: ByteRange
    # replacement text
    text: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_text_patch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TextPatch:
        """Decode one TextPatch."""
        return decode_text_patch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_text_patch(self)

    @classmethod
    def from_json(cls, value: Json) -> TextPatch:
        """Return one TextPatch from one JSON value."""
        return from_json_text_patch(value)


def encode_text_patch(writer: BinaryWriter, value: TextPatch) -> None:
    """Encode one TextPatch."""
    encode_byte_range(writer, value.range)
    writer.write_string(value.text)


def decode_text_patch(reader: BinaryReader) -> TextPatch:
    """Decode one TextPatch."""
    range_ = decode_byte_range(reader)
    text = reader.read_string()

    return TextPatch(
        range=range_,
        text=text,
    )


def to_json_text_patch(value: TextPatch) -> Json:
    """Return one JSON value for one TextPatch."""
    return {
        "range": to_json_byte_range(value.range),
        "text": value.text,
    }


def from_json_text_patch(value: Json) -> TextPatch:
    """Return one TextPatch from one JSON value."""
    object_ = json_object(value)

    return TextPatch(
        range=from_json_byte_range(json_field(object_, "range")),
        text=json_string(json_field(object_, "text")),
    )


@dataclass(frozen=True, slots=True)
class EditSetText:
    """Replace or create one text file."""

    # repository relative path
    path: str
    # full text content
    text: str
    kind: typing.Literal["setText"] = "setText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edit(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edit(self)


@dataclass(frozen=True, slots=True)
class EditEditText:
    """Apply text replacements to one tracked text file."""

    # repository relative path
    path: str
    # text replacements
    patches: Sequence[TextPatch]
    kind: typing.Literal["editText"] = "editText"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edit(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edit(self)


@dataclass(frozen=True, slots=True)
class EditSetBytes:
    """Replace or create one binary file."""

    # repository relative path
    path: str
    # full binary content
    bytes: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["setBytes"] = "setBytes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edit(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edit(self)


@dataclass(frozen=True, slots=True)
class EditRemove:
    """Remove one file."""

    # repository relative path
    path: str
    kind: typing.Literal["remove"] = "remove"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edit(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edit(self)


@dataclass(frozen=True, slots=True)
class EditMove:
    """Move one file."""

    # source repository relative path
    from_: str
    # destination repository relative path
    to: str
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edit(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edit(self)


"""One source file mutation."""
Edit: typing.TypeAlias = (
    EditSetText | EditEditText | EditSetBytes | EditRemove | EditMove
)


def encode_edit(writer: BinaryWriter, value: Edit) -> None:
    """Encode one Edit."""
    if value.kind == "setText":
        writer.write_unsigned(0)
        writer.write_string(value.path)
        writer.write_string(value.text)
    elif value.kind == "editText":
        writer.write_unsigned(1)
        writer.write_string(value.path)
        writer.write_unsigned(len(value.patches))
        for item_value_patches_0 in value.patches:
            encode_text_patch(writer, item_value_patches_0)
    elif value.kind == "setBytes":
        writer.write_unsigned(2)
        writer.write_string(value.path)
        writer.write_byte_slice(value.bytes)
    elif value.kind == "remove":
        writer.write_unsigned(3)
        writer.write_string(value.path)
    elif value.kind == "move":
        writer.write_unsigned(4)
        writer.write_string(value.from_)
        writer.write_string(value.to)
    else:
        raise SerdeError("unknown enum variant")


def decode_edit(reader: BinaryReader) -> Edit:
    """Decode one Edit."""
    variant = reader.read_number()

    if variant == 0:
        path = reader.read_string()
        text = reader.read_string()

        return EditSetText(
            path=path,
            text=text,
        )
    elif variant == 1:
        path = reader.read_string()
        patches = [decode_text_patch(reader) for _ in range(reader.read_number())]

        return EditEditText(
            path=path,
            patches=patches,
        )
    elif variant == 2:
        path = reader.read_string()
        bytes = reader.read_byte_slice()

        return EditSetBytes(
            path=path,
            bytes=bytes,
        )
    elif variant == 3:
        path = reader.read_string()

        return EditRemove(
            path=path,
        )
    elif variant == 4:
        from_ = reader.read_string()
        to = reader.read_string()

        return EditMove(
            from_=from_,
            to=to,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_edit(value: Edit) -> Json:
    """Return one JSON value for one Edit."""
    if value.kind == "setText":
        return {
            "kind": "setText",
            "path": value.path,
            "text": value.text,
        }
    elif value.kind == "editText":
        return {
            "kind": "editText",
            "path": value.path,
            "patches": [to_json_text_patch(item_0) for item_0 in value.patches],
        }
    elif value.kind == "setBytes":
        return {
            "kind": "setBytes",
            "path": value.path,
            "bytes": bytes_to_json(value.bytes),
        }
    elif value.kind == "remove":
        return {
            "kind": "remove",
            "path": value.path,
        }
    elif value.kind == "move":
        return {
            "kind": "move",
            "from": value.from_,
            "to": value.to,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_edit(value: Json) -> Edit:
    """Return one Edit from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "setText":
        return EditSetText(
            path=json_string(json_field(object_, "path")),
            text=json_string(json_field(object_, "text")),
        )
    elif kind == "editText":
        return EditEditText(
            path=json_string(json_field(object_, "path")),
            patches=[
                from_json_text_patch(item_0)
                for item_0 in json_array(json_field(object_, "patches"))
            ],
        )
    elif kind == "setBytes":
        return EditSetBytes(
            path=json_string(json_field(object_, "path")),
            bytes=bytes_from_json(json_field(object_, "bytes")),
        )
    elif kind == "remove":
        return EditRemove(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "move":
        return EditMove(
            from_=json_string(json_field(object_, "from")),
            to=json_string(json_field(object_, "to")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ByteRange",
    "encode_byte_range",
    "decode_byte_range",
    "to_json_byte_range",
    "from_json_byte_range",
    "TextPatch",
    "encode_text_patch",
    "decode_text_patch",
    "to_json_text_patch",
    "from_json_text_patch",
    "Edit",
    "encode_edit",
    "decode_edit",
    "to_json_edit",
    "from_json_edit",
    "EditSetText",
    "EditEditText",
    "EditSetBytes",
    "EditRemove",
    "EditMove",
]
