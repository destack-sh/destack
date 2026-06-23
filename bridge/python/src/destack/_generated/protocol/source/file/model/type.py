# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

"""The format of a source file."""
FileType: TypeAlias = (
    Literal["destack"]
    | Literal["destackDeclaration"]
    | Literal["javaScript"]
    | Literal["javaScriptXml"]
    | Literal["typeScript"]
    | Literal["typeScriptXml"]
    | Literal["typeScriptDeclaration"]
    | Literal["text"]
    | Literal["toml"]
    | Literal["yaml"]
    | Literal["json"]
    | Literal["env"]
    | Literal["html"]
    | Literal["markdown"]
    | Literal["css"]
    | Literal["svg"]
    | Literal["wasm"]
    | Literal["node"]
    | Literal["sourceMap"]
    | Literal["object"]
    | Literal["image"]
    | Literal["font"]
    | Literal["audio"]
    | Literal["video"]
    | Literal["model"]
    | Literal["neural"]
    | Literal["document"]
    | Literal["binary"]
    | Literal["unknown"]
)


def encode_file_type(writer: Writer, value: FileType) -> None:
    if value == "destack":
        writer.write_unsigned(0)
    elif value == "destackDeclaration":
        writer.write_unsigned(1)
    elif value == "javaScript":
        writer.write_unsigned(2)
    elif value == "javaScriptXml":
        writer.write_unsigned(3)
    elif value == "typeScript":
        writer.write_unsigned(4)
    elif value == "typeScriptXml":
        writer.write_unsigned(5)
    elif value == "typeScriptDeclaration":
        writer.write_unsigned(6)
    elif value == "text":
        writer.write_unsigned(7)
    elif value == "toml":
        writer.write_unsigned(8)
    elif value == "yaml":
        writer.write_unsigned(9)
    elif value == "json":
        writer.write_unsigned(10)
    elif value == "env":
        writer.write_unsigned(11)
    elif value == "html":
        writer.write_unsigned(12)
    elif value == "markdown":
        writer.write_unsigned(13)
    elif value == "css":
        writer.write_unsigned(14)
    elif value == "svg":
        writer.write_unsigned(15)
    elif value == "wasm":
        writer.write_unsigned(16)
    elif value == "node":
        writer.write_unsigned(17)
    elif value == "sourceMap":
        writer.write_unsigned(18)
    elif value == "object":
        writer.write_unsigned(19)
    elif value == "image":
        writer.write_unsigned(20)
    elif value == "font":
        writer.write_unsigned(21)
    elif value == "audio":
        writer.write_unsigned(22)
    elif value == "video":
        writer.write_unsigned(23)
    elif value == "model":
        writer.write_unsigned(24)
    elif value == "neural":
        writer.write_unsigned(25)
    elif value == "document":
        writer.write_unsigned(26)
    elif value == "binary":
        writer.write_unsigned(27)
    elif value == "unknown":
        writer.write_unsigned(28)
    else:
        raise SerdeError("unknown enum variant")


def decode_file_type(reader: Reader) -> FileType:
    variant = reader.read_number()

    if variant == 0:
        return "destack"
    elif variant == 1:
        return "destackDeclaration"
    elif variant == 2:
        return "javaScript"
    elif variant == 3:
        return "javaScriptXml"
    elif variant == 4:
        return "typeScript"
    elif variant == 5:
        return "typeScriptXml"
    elif variant == 6:
        return "typeScriptDeclaration"
    elif variant == 7:
        return "text"
    elif variant == 8:
        return "toml"
    elif variant == 9:
        return "yaml"
    elif variant == 10:
        return "json"
    elif variant == 11:
        return "env"
    elif variant == 12:
        return "html"
    elif variant == 13:
        return "markdown"
    elif variant == 14:
        return "css"
    elif variant == 15:
        return "svg"
    elif variant == 16:
        return "wasm"
    elif variant == 17:
        return "node"
    elif variant == 18:
        return "sourceMap"
    elif variant == 19:
        return "object"
    elif variant == 20:
        return "image"
    elif variant == 21:
        return "font"
    elif variant == 22:
        return "audio"
    elif variant == 23:
        return "video"
    elif variant == 24:
        return "model"
    elif variant == 25:
        return "neural"
    elif variant == 26:
        return "document"
    elif variant == 27:
        return "binary"
    elif variant == 28:
        return "unknown"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "FileType",
    "encode_file_type",
    "decode_file_type",
]
