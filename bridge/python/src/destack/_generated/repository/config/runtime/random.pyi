# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class RandomOptions:
    """Runtime randomness configuration."""

    # seed for deterministic randomness streams
    seed: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RandomOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RandomOptions: ...

def encode_random_options(writer: BinaryWriter, value: RandomOptions) -> None: ...
def decode_random_options(reader: BinaryReader) -> RandomOptions: ...
def to_json_random_options(value: RandomOptions) -> Json: ...
def from_json_random_options(value: Json) -> RandomOptions: ...

__all__ = [
    "RandomOptions",
    "encode_random_options",
    "decode_random_options",
    "to_json_random_options",
    "from_json_random_options",
]
