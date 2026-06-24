# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class ClockOptions:
    """Runtime clock configuration."""

    # runtime wall-clock epoch in nanoseconds
    epoch_ns: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ClockOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ClockOptions: ...

def encode_clock_options(writer: BinaryWriter, value: ClockOptions) -> None: ...
def decode_clock_options(reader: BinaryReader) -> ClockOptions: ...
def to_json_clock_options(value: ClockOptions) -> Json: ...
def from_json_clock_options(value: Json) -> ClockOptions: ...

__all__ = [
    "ClockOptions",
    "encode_clock_options",
    "decode_clock_options",
    "to_json_clock_options",
    "from_json_clock_options",
]
