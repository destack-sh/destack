# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_shared_heap_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SharedHeapOptions:
        """Decode one SharedHeapOptions."""
        return decode_shared_heap_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_shared_heap_options(self)

    @classmethod
    def from_json(cls, value: Json) -> SharedHeapOptions:
        """Return one SharedHeapOptions from one JSON value."""
        return from_json_shared_heap_options(value)


def encode_shared_heap_options(writer: BinaryWriter, value: SharedHeapOptions) -> None:
    """Encode one SharedHeapOptions."""
    destack._generated.heap.core.gc.encode_gc_options(writer, value.gc)
    destack._generated.heap.allocator.class_.encode_size_class_table(
        writer, value.size_classes
    )
    writer.write_unsigned(value.heap_small_size_bytes)
    writer.write_unsigned(value.address_space_size_bytes)
    writer.write_unsigned(value.page_size_bytes)
    writer.write_unsigned(value.allocator_chunk_size_bytes)
    writer.write_unsigned(value.small_allocation_alignment_bytes)


def decode_shared_heap_options(reader: BinaryReader) -> SharedHeapOptions:
    """Decode one SharedHeapOptions."""
    gc = destack._generated.heap.core.gc.decode_gc_options(reader)
    size_classes = destack._generated.heap.allocator.class_.decode_size_class_table(
        reader
    )
    heap_small_size_bytes = reader.read_number()
    address_space_size_bytes = reader.read_number()
    page_size_bytes = reader.read_number()
    allocator_chunk_size_bytes = reader.read_number()
    small_allocation_alignment_bytes = reader.read_number()

    return SharedHeapOptions(
        gc=gc,
        size_classes=size_classes,
        heap_small_size_bytes=heap_small_size_bytes,
        address_space_size_bytes=address_space_size_bytes,
        page_size_bytes=page_size_bytes,
        allocator_chunk_size_bytes=allocator_chunk_size_bytes,
        small_allocation_alignment_bytes=small_allocation_alignment_bytes,
    )


def to_json_shared_heap_options(value: SharedHeapOptions) -> Json:
    """Return one JSON value for one SharedHeapOptions."""
    return {
        "gc": destack._generated.heap.core.gc.to_json_gc_options(value.gc),
        "sizeClasses": destack._generated.heap.allocator.class_.to_json_size_class_table(
            value.size_classes
        ),
        "heapSmallSizeBytes": value.heap_small_size_bytes,
        "addressSpaceSizeBytes": value.address_space_size_bytes,
        "pageSizeBytes": value.page_size_bytes,
        "allocatorChunkSizeBytes": value.allocator_chunk_size_bytes,
        "smallAllocationAlignmentBytes": value.small_allocation_alignment_bytes,
    }


def from_json_shared_heap_options(value: Json) -> SharedHeapOptions:
    """Return one SharedHeapOptions from one JSON value."""
    object_ = json_object(value)

    return SharedHeapOptions(
        gc=destack._generated.heap.core.gc.from_json_gc_options(
            json_field(object_, "gc")
        ),
        size_classes=destack._generated.heap.allocator.class_.from_json_size_class_table(
            json_field(object_, "sizeClasses")
        ),
        heap_small_size_bytes=json_int(json_field(object_, "heapSmallSizeBytes")),
        address_space_size_bytes=json_int(json_field(object_, "addressSpaceSizeBytes")),
        page_size_bytes=json_int(json_field(object_, "pageSizeBytes")),
        allocator_chunk_size_bytes=json_int(
            json_field(object_, "allocatorChunkSizeBytes")
        ),
        small_allocation_alignment_bytes=json_int(
            json_field(object_, "smallAllocationAlignmentBytes")
        ),
    )


__all__ = [
    "SharedHeapOptions",
    "encode_shared_heap_options",
    "decode_shared_heap_options",
    "to_json_shared_heap_options",
    "from_json_shared_heap_options",
]
