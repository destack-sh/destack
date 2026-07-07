# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.section

@dataclass(frozen=True, slots=True)
class StaticImage:
    """Section-backed static memory image carried by a program."""

    # static bytes
    bytes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticImage: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StaticImage: ...

def encode_static_image(writer: BinaryWriter, value: StaticImage) -> None: ...
def decode_static_image(reader: BinaryReader) -> StaticImage: ...
def to_json_static_image(value: StaticImage) -> Json: ...
def from_json_static_image(value: Json) -> StaticImage: ...

__all__ = [
    "StaticImage",
    "encode_static_image",
    "decode_static_image",
    "to_json_static_image",
    "from_json_static_image",
]
