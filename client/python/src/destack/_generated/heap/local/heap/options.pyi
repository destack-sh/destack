# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.heap.allocator.class_
import destack._generated.heap.core.gc

@dataclass(frozen=True, slots=True)
class HeapOptions:
    """The configuration for one heap instance."""

    # the collector configuration
    gc: destack._generated.heap.core.gc.GcOptions
    # the configured small-allocation class table
    size_classes: destack._generated.heap.allocator.class_.SizeClassTable
    # the byte size for heap young space
    heap_young_size_bytes: int
    # the maximum payload size routed to heap young space
    max_heap_young_allocation_size_bytes: int
    # the byte size for heap small-block spans
    heap_small_size_bytes: int
    # the virtual byte capacity for heap storage
    address_space_size_bytes: int
    # the byte size for allocator pages
    page_size_bytes: int
    # the byte size for one physical allocator chunk
    allocator_chunk_size_bytes: int
    # the required alignment for configured small-allocation classes
    small_allocation_alignment_bytes: int

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

__all__ = [
    "HeapOptions",
    "encode_heap_options",
    "decode_heap_options",
    "to_json_heap_options",
    "from_json_heap_options",
]
