# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.mir.metadata.data
import destack._generated.mir.metadata.dispatch
import destack._generated.mir.metadata.drop
import destack._generated.mir.metadata.frame
import destack._generated.mir.metadata.function
import destack._generated.mir.metadata.layout
import destack._generated.mir.metadata.memory
import destack._generated.mir.metadata.type


@dataclass(frozen=True, slots=True)
class Metadata:
    """Structured MIR metadata domains."""

    # target data layout
    data_layout: destack._generated.mir.metadata.data.DataLayout
    # canonical type metadata
    types: destack._generated.mir.metadata.type.TypeMetadata
    # canonical layout metadata
    layout: destack._generated.mir.metadata.layout.LayoutMetadata
    # canonical dispatch metadata
    dispatch: destack._generated.mir.metadata.dispatch.DispatchMetadata
    # canonical drop metadata
    drop: destack._generated.mir.metadata.drop.DropMetadata
    # canonical frame metadata
    frame: destack._generated.mir.metadata.frame.FrameTable
    # derived function and call metadata
    functions: destack._generated.mir.metadata.function.FunctionMetadataTable
    # memory and alias metadata
    memory: destack._generated.mir.metadata.memory.MemoryMetadata

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Metadata:
        """Decode one Metadata."""
        return decode_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> Metadata:
        """Return one Metadata from one JSON value."""
        return from_json_metadata(value)


def encode_metadata(writer: BinaryWriter, value: Metadata) -> None:
    """Encode one Metadata."""
    destack._generated.mir.metadata.data.encode_data_layout(writer, value.data_layout)
    destack._generated.mir.metadata.type.encode_type_metadata(writer, value.types)
    destack._generated.mir.metadata.layout.encode_layout_metadata(writer, value.layout)
    destack._generated.mir.metadata.dispatch.encode_dispatch_metadata(
        writer, value.dispatch
    )
    destack._generated.mir.metadata.drop.encode_drop_metadata(writer, value.drop)
    destack._generated.mir.metadata.frame.encode_frame_table(writer, value.frame)
    destack._generated.mir.metadata.function.encode_function_metadata_table(
        writer, value.functions
    )
    destack._generated.mir.metadata.memory.encode_memory_metadata(writer, value.memory)


def decode_metadata(reader: BinaryReader) -> Metadata:
    """Decode one Metadata."""
    data_layout = destack._generated.mir.metadata.data.decode_data_layout(reader)
    types = destack._generated.mir.metadata.type.decode_type_metadata(reader)
    layout = destack._generated.mir.metadata.layout.decode_layout_metadata(reader)
    dispatch = destack._generated.mir.metadata.dispatch.decode_dispatch_metadata(reader)
    drop = destack._generated.mir.metadata.drop.decode_drop_metadata(reader)
    frame = destack._generated.mir.metadata.frame.decode_frame_table(reader)
    functions = destack._generated.mir.metadata.function.decode_function_metadata_table(
        reader
    )
    memory = destack._generated.mir.metadata.memory.decode_memory_metadata(reader)

    return Metadata(
        data_layout=data_layout,
        types=types,
        layout=layout,
        dispatch=dispatch,
        drop=drop,
        frame=frame,
        functions=functions,
        memory=memory,
    )


def to_json_metadata(value: Metadata) -> Json:
    """Return one JSON value for one Metadata."""
    return {
        "dataLayout": destack._generated.mir.metadata.data.to_json_data_layout(
            value.data_layout
        ),
        "types": destack._generated.mir.metadata.type.to_json_type_metadata(
            value.types
        ),
        "layout": destack._generated.mir.metadata.layout.to_json_layout_metadata(
            value.layout
        ),
        "dispatch": destack._generated.mir.metadata.dispatch.to_json_dispatch_metadata(
            value.dispatch
        ),
        "drop": destack._generated.mir.metadata.drop.to_json_drop_metadata(value.drop),
        "frame": destack._generated.mir.metadata.frame.to_json_frame_table(value.frame),
        "functions": destack._generated.mir.metadata.function.to_json_function_metadata_table(
            value.functions
        ),
        "memory": destack._generated.mir.metadata.memory.to_json_memory_metadata(
            value.memory
        ),
    }


def from_json_metadata(value: Json) -> Metadata:
    """Return one Metadata from one JSON value."""
    object_ = json_object(value)

    return Metadata(
        data_layout=destack._generated.mir.metadata.data.from_json_data_layout(
            json_field(object_, "dataLayout")
        ),
        types=destack._generated.mir.metadata.type.from_json_type_metadata(
            json_field(object_, "types")
        ),
        layout=destack._generated.mir.metadata.layout.from_json_layout_metadata(
            json_field(object_, "layout")
        ),
        dispatch=destack._generated.mir.metadata.dispatch.from_json_dispatch_metadata(
            json_field(object_, "dispatch")
        ),
        drop=destack._generated.mir.metadata.drop.from_json_drop_metadata(
            json_field(object_, "drop")
        ),
        frame=destack._generated.mir.metadata.frame.from_json_frame_table(
            json_field(object_, "frame")
        ),
        functions=destack._generated.mir.metadata.function.from_json_function_metadata_table(
            json_field(object_, "functions")
        ),
        memory=destack._generated.mir.metadata.memory.from_json_memory_metadata(
            json_field(object_, "memory")
        ),
    )


__all__ = [
    "Metadata",
    "encode_metadata",
    "decode_metadata",
    "to_json_metadata",
    "from_json_metadata",
]
