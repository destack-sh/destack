# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.heap.allocator.class_
import destack._generated.heap.core.gc

@dataclass(frozen=True, slots=True)
class SharedHeapOptions:
    """The configuration for one shared heap instance."""

    # the collector configuration
    gc: destack._generated.heap.core.gc.GcOptions
    # the configured small-block class table
    size_classes: destack._generated.heap.allocator.class_.SizeClassTable
    # the byte size for shared heap small-block spans
    heap_small_size_bytes: int
    # the virtual byte capacity for shared heap storage
    address_space_size_bytes: int
    # the byte size for allocator pages
    page_size_bytes: int
    # the byte size for one physical allocator chunk
    allocator_chunk_size_bytes: int
    # the required alignment for configured small-block classes
    small_allocation_alignment_bytes: int

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
    "SharedHeapOptions",
    "encode_shared_heap_options",
    "decode_shared_heap_options",
    "to_json_shared_heap_options",
    "from_json_shared_heap_options",
]
