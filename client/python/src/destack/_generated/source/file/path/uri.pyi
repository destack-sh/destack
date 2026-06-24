# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""A generic URI."""
Uri: typing.TypeAlias = str

def encode_uri(writer: BinaryWriter, value: Uri) -> None: ...
def decode_uri(reader: BinaryReader) -> Uri: ...
def to_json_uri(value: Uri) -> Json: ...
def from_json_uri(value: Json) -> Uri: ...

__all__ = [
    "Uri",
    "encode_uri",
    "decode_uri",
    "to_json_uri",
    "from_json_uri",
]
