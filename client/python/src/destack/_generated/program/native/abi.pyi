# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class Abi:
    """Native ABI required by one native code payload."""

    # destack native ABI version
    version: int
    # target triple or equivalent target identity
    target: str
    # native pointer byte width expected by this code
    pointer_bytes: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Abi: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Abi: ...

def encode_abi(writer: BinaryWriter, value: Abi) -> None: ...
def decode_abi(reader: BinaryReader) -> Abi: ...
def to_json_abi(value: Abi) -> Json: ...
def from_json_abi(value: Json) -> Abi: ...

__all__ = [
    "Abi",
    "encode_abi",
    "decode_abi",
    "to_json_abi",
    "from_json_abi",
]
