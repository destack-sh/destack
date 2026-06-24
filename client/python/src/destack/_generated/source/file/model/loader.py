# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""How source content is interpreted as a module."""
Loader: typing.TypeAlias = (
    typing.Literal["destack"]
    | typing.Literal["typeScript"]
    | typing.Literal["javaScript"]
    | typing.Literal["json"]
    | typing.Literal["toml"]
    | typing.Literal["yaml"]
    | typing.Literal["text"]
    | typing.Literal["binary"]
    | typing.Literal["file"]
    | typing.Literal["base64"]
)


def encode_loader(writer: BinaryWriter, value: Loader) -> None:
    """Encode one Loader."""
    if value == "destack":
        writer.write_unsigned(0)
    elif value == "typeScript":
        writer.write_unsigned(1)
    elif value == "javaScript":
        writer.write_unsigned(2)
    elif value == "json":
        writer.write_unsigned(3)
    elif value == "toml":
        writer.write_unsigned(4)
    elif value == "yaml":
        writer.write_unsigned(5)
    elif value == "text":
        writer.write_unsigned(6)
    elif value == "binary":
        writer.write_unsigned(7)
    elif value == "file":
        writer.write_unsigned(8)
    elif value == "base64":
        writer.write_unsigned(9)
    else:
        raise SerdeError("unknown enum variant")


def decode_loader(reader: BinaryReader) -> Loader:
    """Decode one Loader."""
    variant = reader.read_number()

    if variant == 0:
        return "destack"
    elif variant == 1:
        return "typeScript"
    elif variant == 2:
        return "javaScript"
    elif variant == 3:
        return "json"
    elif variant == 4:
        return "toml"
    elif variant == 5:
        return "yaml"
    elif variant == 6:
        return "text"
    elif variant == 7:
        return "binary"
    elif variant == 8:
        return "file"
    elif variant == 9:
        return "base64"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_loader(value: Loader) -> Json:
    """Return one JSON value for one Loader."""
    return value


def from_json_loader(value: Json) -> Loader:
    """Return one Loader from one JSON value."""
    variant = json_string(value)

    if variant == "destack":
        return "destack"
    elif variant == "typeScript":
        return "typeScript"
    elif variant == "javaScript":
        return "javaScript"
    elif variant == "json":
        return "json"
    elif variant == "toml":
        return "toml"
    elif variant == "yaml":
        return "yaml"
    elif variant == "text":
        return "text"
    elif variant == "binary":
        return "binary"
    elif variant == "file":
        return "file"
    elif variant == "base64":
        return "base64"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Loader",
    "encode_loader",
    "decode_loader",
    "to_json_loader",
    "from_json_loader",
]
