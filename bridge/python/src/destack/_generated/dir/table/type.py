# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    nested_bytes,
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class TypeSegment:
    """Type slots added by one DIR phase."""

    # the module id of the type segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first type id owned by this table segment
    first_type_id: int
    # canonical type entries
    types: Sequence[destack._generated.dir.type.type.Type]
    # the source for each type id
    sources: Sequence[destack._generated.dir.tree.node.LocalNodeIdAny]
    # effective checked type keyed by DIR node occurrence
    node_types: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.type.GlobalTypeId,
    ]
    # checked declaration type keyed by symbol
    symbol_types: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.type.type.GlobalTypeId,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeSegment:
        """Decode one TypeSegment."""
        return decode_type_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeSegment:
        """Return one TypeSegment from one JSON value."""
        return from_json_type_segment(value)


def encode_type_segment(writer: BinaryWriter, value: TypeSegment) -> None:
    """Encode one TypeSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_type_id)
    writer.write_unsigned(len(value.types))
    for item_value_types_0 in value.types:
        destack._generated.dir.type.type.encode_type(writer, item_value_types_0)
    writer.write_unsigned(len(value.sources))
    for item_value_sources_0 in value.sources:
        destack._generated.dir.tree.node.encode_local_node_id_any(
            writer, item_value_sources_0
        )
    entries_value_node_types_0 = []
    for key_value_node_types_0, item_value_node_types_0 in value.node_types.items():

        def write_key_value_node_types_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_node_types_0
            )

        key_bytes = nested_bytes(write_key_value_node_types_0)
        entries_value_node_types_0.append(
            (key_value_node_types_0, item_value_node_types_0, key_bytes)
        )
    entries_value_node_types_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_node_types_0))
    for entry_value_node_types_0 in entries_value_node_types_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_node_types_0[0]
        )
        destack._generated.dir.type.type.encode_global_type_id(
            writer, entry_value_node_types_0[1]
        )
    entries_value_symbol_types_0 = []
    for (
        key_value_symbol_types_0,
        item_value_symbol_types_0,
    ) in value.symbol_types.items():

        def write_key_value_symbol_types_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_symbol_types_0
            )

        key_bytes = nested_bytes(write_key_value_symbol_types_0)
        entries_value_symbol_types_0.append(
            (key_value_symbol_types_0, item_value_symbol_types_0, key_bytes)
        )
    entries_value_symbol_types_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_symbol_types_0))
    for entry_value_symbol_types_0 in entries_value_symbol_types_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_symbol_types_0[0]
        )
        destack._generated.dir.type.type.encode_global_type_id(
            writer, entry_value_symbol_types_0[1]
        )


def decode_type_segment(reader: BinaryReader) -> TypeSegment:
    """Decode one TypeSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_type_id = reader.read_number()
    types = [
        destack._generated.dir.type.type.decode_type(reader)
        for _ in range(reader.read_number())
    ]
    sources = [
        destack._generated.dir.tree.node.decode_local_node_id_any(reader)
        for _ in range(reader.read_number())
    ]
    node_types = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    }
    symbol_types = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    }

    return TypeSegment(
        module_id=module_id,
        first_type_id=first_type_id,
        types=types,
        sources=sources,
        node_types=node_types,
        symbol_types=symbol_types,
    )


def to_json_type_segment(value: TypeSegment) -> Json:
    """Return one JSON value for one TypeSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstTypeId": value.first_type_id,
        "types": [
            destack._generated.dir.type.type.to_json_type(item_0)
            for item_0 in value.types
        ],
        "sources": [
            destack._generated.dir.tree.node.to_json_local_node_id_any(item_0)
            for item_0 in value.sources
        ],
        "nodeTypes": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.type.to_json_global_type_id(item_0),
            ]
            for key_0, item_0 in value.node_types.items()
        ],
        "symbolTypes": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                destack._generated.dir.type.type.to_json_global_type_id(item_0),
            ]
            for key_0, item_0 in value.symbol_types.items()
        ],
    }


def from_json_type_segment(value: Json) -> TypeSegment:
    """Return one TypeSegment from one JSON value."""
    object_ = json_object(value)

    return TypeSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_type_id=json_int(json_field(object_, "firstTypeId")),
        types=[
            destack._generated.dir.type.type.from_json_type(item_0)
            for item_0 in json_array(json_field(object_, "types"))
        ],
        sources=[
            destack._generated.dir.tree.node.from_json_local_node_id_any(item_0)
            for item_0 in json_array(json_field(object_, "sources"))
        ],
        node_types={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "nodeTypes"))
        },
        symbol_types={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "symbolTypes"))
        },
    )


__all__ = [
    "TypeSegment",
    "encode_type_segment",
    "decode_type_segment",
    "to_json_type_segment",
    "from_json_type_segment",
]
