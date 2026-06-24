# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""The format of a source file."""
FileType: typing.TypeAlias = (
    typing.Literal["destack"]
    | typing.Literal["destackDeclaration"]
    | typing.Literal["javaScript"]
    | typing.Literal["javaScriptXml"]
    | typing.Literal["typeScript"]
    | typing.Literal["typeScriptXml"]
    | typing.Literal["typeScriptDeclaration"]
    | typing.Literal["text"]
    | typing.Literal["toml"]
    | typing.Literal["yaml"]
    | typing.Literal["json"]
    | typing.Literal["env"]
    | typing.Literal["html"]
    | typing.Literal["markdown"]
    | typing.Literal["css"]
    | typing.Literal["svg"]
    | typing.Literal["wasm"]
    | typing.Literal["node"]
    | typing.Literal["sourceMap"]
    | typing.Literal["object"]
    | typing.Literal["image"]
    | typing.Literal["font"]
    | typing.Literal["audio"]
    | typing.Literal["video"]
    | typing.Literal["model"]
    | typing.Literal["neural"]
    | typing.Literal["document"]
    | typing.Literal["binary"]
    | typing.Literal["unknown"]
)

def encode_file_type(writer: BinaryWriter, value: FileType) -> None: ...
def decode_file_type(reader: BinaryReader) -> FileType: ...
def to_json_file_type(value: FileType) -> Json: ...
def from_json_file_type(value: Json) -> FileType: ...

__all__ = [
    "FileType",
    "encode_file_type",
    "decode_file_type",
    "to_json_file_type",
    "from_json_file_type",
]
