# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class NameString:
    """Textual name."""

    string: str
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


@dataclass(frozen=True, slots=True)
class NameIndex:
    """Positional name."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


"""Searchable source name."""
Name: typing.TypeAlias = NameString | NameIndex


def encode_name(writer: BinaryWriter, value: Name) -> None:
    """Encode one Name."""
    if value.kind == "string":
        writer.write_unsigned(0)
        writer.write_string(value.string)
    elif value.kind == "index":
        writer.write_unsigned(1)
        writer.write_unsigned(value.index)
    else:
        raise SerdeError("unknown enum variant")


def decode_name(reader: BinaryReader) -> Name:
    """Decode one Name."""
    variant = reader.read_number()

    if variant == 0:
        string = reader.read_string()

        return NameString(string=string)
    elif variant == 1:
        index = reader.read_number()

        return NameIndex(index=index)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_name(value: Name) -> Json:
    """Return one JSON value for one Name."""
    if value.kind == "string":
        return {
            "kind": "string",
            "string": value.string,
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "index": value.index,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_name(value: Json) -> Name:
    """Return one Name from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "string":
        return NameString(string=json_string(json_field(object_, "string")))
    elif kind == "index":
        return NameIndex(index=json_int(json_field(object_, "index")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Name",
    "encode_name",
    "decode_name",
    "to_json_name",
    "from_json_name",
    "NameString",
    "NameIndex",
]
