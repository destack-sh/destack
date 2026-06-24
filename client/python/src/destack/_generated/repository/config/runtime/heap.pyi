# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class HeapOptions:
    """Runtime heap configuration."""

    # soft memory limit inherited by local and shared heap collectors
    memory_limit_bytes: int | None
    # the proportional heap growth target percentage
    growth_percent: int
    # worker-local heap policy
    local: LocalHeapOptions
    # runtime-shared heap policy
    shared: SharedHeapOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HeapOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HeapOptions: ...

def encode_heap_options(writer: BinaryWriter, value: HeapOptions) -> None: ...
def decode_heap_options(reader: BinaryReader) -> HeapOptions: ...
def to_json_heap_options(value: HeapOptions) -> Json: ...
def from_json_heap_options(value: Json) -> HeapOptions: ...

@dataclass(frozen=True, slots=True)
class LocalHeapOptions:
    """Runtime local-heap policy."""

    # the byte width for worker-local young space
    young_size_bytes: int
    # minimum heap bytes before normal growth pacing applies
    min_bytes: int | None
    # hard limit for total retained local heap bytes
    max_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalHeapOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalHeapOptions: ...

def encode_local_heap_options(
    writer: BinaryWriter, value: LocalHeapOptions
) -> None: ...
def decode_local_heap_options(reader: BinaryReader) -> LocalHeapOptions: ...
def to_json_local_heap_options(value: LocalHeapOptions) -> Json: ...
def from_json_local_heap_options(value: Json) -> LocalHeapOptions: ...

@dataclass(frozen=True, slots=True)
class SharedHeapOptions:
    """Runtime shared-heap policy."""

    # minimum heap bytes before normal growth pacing applies
    min_bytes: int | None
    # hard limit for total retained shared heap bytes
    max_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SharedHeapOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SharedHeapOptions: ...

def encode_shared_heap_options(
    writer: BinaryWriter, value: SharedHeapOptions
) -> None: ...
def decode_shared_heap_options(reader: BinaryReader) -> SharedHeapOptions: ...
def to_json_shared_heap_options(value: SharedHeapOptions) -> Json: ...
def from_json_shared_heap_options(value: Json) -> SharedHeapOptions: ...

__all__ = [
    "HeapOptions",
    "encode_heap_options",
    "decode_heap_options",
    "to_json_heap_options",
    "from_json_heap_options",
    "LocalHeapOptions",
    "encode_local_heap_options",
    "decode_local_heap_options",
    "to_json_local_heap_options",
    "from_json_local_heap_options",
    "SharedHeapOptions",
    "encode_shared_heap_options",
    "decode_shared_heap_options",
    "to_json_shared_heap_options",
    "from_json_shared_heap_options",
]
