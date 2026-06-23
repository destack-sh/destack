# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class EditSetText:
    """Replace or create one text file."""

    """Repository relative path."""
    path: str
    """Full text content."""
    text: str
    kind: Literal["setText"] = "setText"


@dataclass(frozen=True, slots=True)
class EditEditText:
    """Apply text replacements to one tracked text file."""

    """Repository relative path."""
    path: str
    """Text replacements."""
    patches: Sequence[TextPatch]
    kind: Literal["editText"] = "editText"


@dataclass(frozen=True, slots=True)
class EditSetBytes:
    """Replace or create one binary file."""

    """Repository relative path."""
    path: str
    """Full binary content."""
    bytes: bytes | bytearray | Sequence[int]
    kind: Literal["setBytes"] = "setBytes"


@dataclass(frozen=True, slots=True)
class EditRemove:
    """Remove one file."""

    """Repository relative path."""
    path: str
    kind: Literal["remove"] = "remove"


@dataclass(frozen=True, slots=True)
class EditMove:
    """Move one file."""

    """Source repository relative path."""
    from_: str
    """Destination repository relative path."""
    to: str
    kind: Literal["move"] = "move"


"""One source file mutation."""
Edit: TypeAlias = EditSetText | EditEditText | EditSetBytes | EditRemove | EditMove


def encode_edit(writer: Writer, value: Edit) -> None:
    if value.kind == "setText":
        writer.write_unsigned(0)
        writer.write_string(value.path)
        writer.write_string(value.text)
    elif value.kind == "editText":
        writer.write_unsigned(1)
        writer.write_string(value.path)
        writer.write_unsigned(len(value.patches))
        for item_0 in value.patches:
            encode_text_patch(writer, item_0)
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


def decode_edit(reader: Reader) -> Edit:
    variant = reader.read_number()

    if variant == 0:
        field_0 = reader.read_string()
        field_1 = reader.read_string()

        return EditSetText(
            path=field_0,
            text=field_1,
        )
    elif variant == 1:
        field_0 = reader.read_string()
        field_1 = [decode_text_patch(reader) for _ in range(reader.read_number())]

        return EditEditText(
            path=field_0,
            patches=field_1,
        )
    elif variant == 2:
        field_0 = reader.read_string()
        field_1 = reader.read_byte_slice()

        return EditSetBytes(
            path=field_0,
            bytes=field_1,
        )
    elif variant == 3:
        field_0 = reader.read_string()

        return EditRemove(
            path=field_0,
        )
    elif variant == 4:
        field_0 = reader.read_string()
        field_1 = reader.read_string()

        return EditMove(
            from_=field_0,
            to=field_1,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class TextPatch:
    """One source text replacement."""

    """Replaced byte range."""
    range: ByteRange
    """Replacement text."""
    text: str


def encode_text_patch(writer: Writer, value: TextPatch) -> None:
    encode_byte_range(writer, value.range)
    writer.write_string(value.text)


def decode_text_patch(reader: Reader) -> TextPatch:
    field_0 = decode_byte_range(reader)
    field_1 = reader.read_string()

    return TextPatch(
        range=field_0,
        text=field_1,
    )


@dataclass(frozen=True, slots=True)
class ByteRange:
    """One byte range in source text."""

    """Inclusive start byte offset."""
    start: int
    """Exclusive end byte offset."""
    end: int


def encode_byte_range(writer: Writer, value: ByteRange) -> None:
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.end)


def decode_byte_range(reader: Reader) -> ByteRange:
    field_0 = reader.read_number()
    field_1 = reader.read_number()

    return ByteRange(
        start=field_0,
        end=field_1,
    )


__all__ = [
    "Edit",
    "encode_edit",
    "decode_edit",
    "EditSetText",
    "EditEditText",
    "EditSetBytes",
    "EditRemove",
    "EditMove",
    "TextPatch",
    "encode_text_patch",
    "decode_text_patch",
    "ByteRange",
    "encode_byte_range",
    "decode_byte_range",
]
