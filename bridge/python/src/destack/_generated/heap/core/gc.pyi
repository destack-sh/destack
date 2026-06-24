# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class GcOptions:
    """Collector configuration for one heap."""

    # the proportional heap growth target after one cycle
    growth_percent: int
    # the heap trigger as a percentage of the current goal
    trigger_percent: int
    # optional soft memory limit in bytes
    soft_limit_bytes: int | None
    # optional minimum heap floor in bytes
    minimum_heap_bytes: int | None
    # minimum collector work for one safepoint
    minimum_work_bytes: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GcOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GcOptions: ...

def encode_gc_options(writer: BinaryWriter, value: GcOptions) -> None: ...
def decode_gc_options(reader: BinaryReader) -> GcOptions: ...
def to_json_gc_options(value: GcOptions) -> Json: ...
def from_json_gc_options(value: Json) -> GcOptions: ...

__all__ = [
    "GcOptions",
    "encode_gc_options",
    "decode_gc_options",
    "to_json_gc_options",
    "from_json_gc_options",
]
