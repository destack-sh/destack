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

import destack._generated.heap.local.heap.options
import destack._generated.heap.shared.heap.options


@dataclass(frozen=True, slots=True)
class ProgramHeader:
    """Serialized program compatibility header."""

    # pointer byte width used by layouts and pointer-sized integer types
    pointer_bytes: int
    # local heap geometry used by lowered allocation sites
    local_heap: destack._generated.heap.local.heap.options.HeapOptions
    # shared heap geometry used by lowered allocation sites
    shared_heap: destack._generated.heap.shared.heap.options.SharedHeapOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_header(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramHeader:
        """Decode one ProgramHeader."""
        return decode_program_header(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_header(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramHeader:
        """Return one ProgramHeader from one JSON value."""
        return from_json_program_header(value)


def encode_program_header(writer: BinaryWriter, value: ProgramHeader) -> None:
    """Encode one ProgramHeader."""
    writer.write_byte(value.pointer_bytes)
    destack._generated.heap.local.heap.options.encode_heap_options(
        writer, value.local_heap
    )
    destack._generated.heap.shared.heap.options.encode_shared_heap_options(
        writer, value.shared_heap
    )


def decode_program_header(reader: BinaryReader) -> ProgramHeader:
    """Decode one ProgramHeader."""
    pointer_bytes = reader.read_byte()
    local_heap = destack._generated.heap.local.heap.options.decode_heap_options(reader)
    shared_heap = (
        destack._generated.heap.shared.heap.options.decode_shared_heap_options(reader)
    )

    return ProgramHeader(
        pointer_bytes=pointer_bytes,
        local_heap=local_heap,
        shared_heap=shared_heap,
    )


def to_json_program_header(value: ProgramHeader) -> Json:
    """Return one JSON value for one ProgramHeader."""
    return {
        "pointerBytes": value.pointer_bytes,
        "localHeap": destack._generated.heap.local.heap.options.to_json_heap_options(
            value.local_heap
        ),
        "sharedHeap": destack._generated.heap.shared.heap.options.to_json_shared_heap_options(
            value.shared_heap
        ),
    }


def from_json_program_header(value: Json) -> ProgramHeader:
    """Return one ProgramHeader from one JSON value."""
    object_ = json_object(value)

    return ProgramHeader(
        pointer_bytes=json_int(json_field(object_, "pointerBytes")),
        local_heap=destack._generated.heap.local.heap.options.from_json_heap_options(
            json_field(object_, "localHeap")
        ),
        shared_heap=destack._generated.heap.shared.heap.options.from_json_shared_heap_options(
            json_field(object_, "sharedHeap")
        ),
    )


__all__ = [
    "ProgramHeader",
    "encode_program_header",
    "decode_program_header",
    "to_json_program_header",
    "from_json_program_header",
]
