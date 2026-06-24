# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

def encode_loader(writer: BinaryWriter, value: Loader) -> None: ...
def decode_loader(reader: BinaryReader) -> Loader: ...
def to_json_loader(value: Loader) -> Json: ...
def from_json_loader(value: Json) -> Loader: ...

__all__ = [
    "Loader",
    "encode_loader",
    "decode_loader",
    "to_json_loader",
    "from_json_loader",
]
