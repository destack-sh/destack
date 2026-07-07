# generated client target, do not edit

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
    # the interned type entries
    types: Sequence[destack._generated.dir.type.type.Type]
    # the structural flags per type, computed at intern time
    flags: Sequence[destack._generated.dir.type.type.TypeFlags]
    # the type id lists referenced by type payloads
    type_ids: ListPool
    # the tuple element lists referenced by type payloads
    elements: ListPool
    # the shape field lists referenced by type payloads
    fields: ListPool
    # the function parameter lists referenced by type payloads
    parameters: ListPool
    # the index signature lists referenced by type payloads
    index_signatures: ListPool
    # the string lists referenced by type payloads
    strings: ListPool
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
    # reduced checked type keyed by surface type
    reduced_types: Mapping[
        destack._generated.dir.type.type.GlobalTypeId,
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
    writer.write_unsigned(len(value.flags))
    for item_value_flags_0 in value.flags:
        destack._generated.dir.type.type.encode_type_flags(writer, item_value_flags_0)
    encode_list_pool(writer, value.type_ids)
    encode_list_pool(writer, value.elements)
    encode_list_pool(writer, value.fields)
    encode_list_pool(writer, value.parameters)
    encode_list_pool(writer, value.index_signatures)
    encode_list_pool(writer, value.strings)
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
    entries_value_reduced_types_0 = []
    for (
        key_value_reduced_types_0,
        item_value_reduced_types_0,
    ) in value.reduced_types.items():

        def write_key_value_reduced_types_0(writer: BinaryWriter) -> None:
            destack._generated.dir.type.type.encode_global_type_id(
                writer, key_value_reduced_types_0
            )

        key_bytes = nested_bytes(write_key_value_reduced_types_0)
        entries_value_reduced_types_0.append(
            (key_value_reduced_types_0, item_value_reduced_types_0, key_bytes)
        )
    entries_value_reduced_types_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_reduced_types_0))
    for entry_value_reduced_types_0 in entries_value_reduced_types_0:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, entry_value_reduced_types_0[0]
        )
        destack._generated.dir.type.type.encode_global_type_id(
            writer, entry_value_reduced_types_0[1]
        )


def decode_type_segment(reader: BinaryReader) -> TypeSegment:
    """Decode one TypeSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_type_id = reader.read_number()
    types = [
        destack._generated.dir.type.type.decode_type(reader)
        for _ in range(reader.read_number())
    ]
    flags = [
        destack._generated.dir.type.type.decode_type_flags(reader)
        for _ in range(reader.read_number())
    ]
    type_ids = decode_list_pool(reader)
    elements = decode_list_pool(reader)
    fields = decode_list_pool(reader)
    parameters = decode_list_pool(reader)
    index_signatures = decode_list_pool(reader)
    strings = decode_list_pool(reader)
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
    reduced_types = {
        destack._generated.dir.type.type.decode_global_type_id(
            reader
        ): destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    }

    return TypeSegment(
        module_id=module_id,
        first_type_id=first_type_id,
        types=types,
        flags=flags,
        type_ids=type_ids,
        elements=elements,
        fields=fields,
        parameters=parameters,
        index_signatures=index_signatures,
        strings=strings,
        node_types=node_types,
        symbol_types=symbol_types,
        reduced_types=reduced_types,
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
        "flags": [
            destack._generated.dir.type.type.to_json_type_flags(item_0)
            for item_0 in value.flags
        ],
        "typeIds": to_json_list_pool(value.type_ids),
        "elements": to_json_list_pool(value.elements),
        "fields": to_json_list_pool(value.fields),
        "parameters": to_json_list_pool(value.parameters),
        "indexSignatures": to_json_list_pool(value.index_signatures),
        "strings": to_json_list_pool(value.strings),
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
        "reducedTypes": [
            [
                destack._generated.dir.type.type.to_json_global_type_id(key_0),
                destack._generated.dir.type.type.to_json_global_type_id(item_0),
            ]
            for key_0, item_0 in value.reduced_types.items()
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
        flags=[
            destack._generated.dir.type.type.from_json_type_flags(item_0)
            for item_0 in json_array(json_field(object_, "flags"))
        ],
        type_ids=from_json_list_pool(json_field(object_, "typeIds")),
        elements=from_json_list_pool(json_field(object_, "elements")),
        fields=from_json_list_pool(json_field(object_, "fields")),
        parameters=from_json_list_pool(json_field(object_, "parameters")),
        index_signatures=from_json_list_pool(json_field(object_, "indexSignatures")),
        strings=from_json_list_pool(json_field(object_, "strings")),
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
        reduced_types={
            destack._generated.dir.type.type.from_json_global_type_id(
                key_0
            ): destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "reducedTypes"))
        },
    )


@dataclass(frozen=True, slots=True)
class ListPool:
    """Interned lists of one type payload kind."""

    # the first element owned by this segment
    first: int
    # the stored list elements
    elements: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_list_pool(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ListPool:
        """Decode one ListPool."""
        return decode_list_pool(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_list_pool(self)

    @classmethod
    def from_json(cls, value: Json) -> ListPool:
        """Return one ListPool from one JSON value."""
        return from_json_list_pool(value)


def encode_list_pool(writer: BinaryWriter, value: ListPool) -> None:
    """Encode one ListPool."""
    writer.write_unsigned(value.first)
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_elements_0
        )


def decode_list_pool(reader: BinaryReader) -> ListPool:
    """Decode one ListPool."""
    first = reader.read_number()
    elements = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return ListPool(
        first=first,
        elements=elements,
    )


def to_json_list_pool(value: ListPool) -> Json:
    """Return one JSON value for one ListPool."""
    return {
        "first": value.first,
        "elements": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.elements
        ],
    }


def from_json_list_pool(value: Json) -> ListPool:
    """Return one ListPool from one JSON value."""
    object_ = json_object(value)

    return ListPool(
        first=json_int(json_field(object_, "first")),
        elements=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


__all__ = [
    "TypeSegment",
    "encode_type_segment",
    "decode_type_segment",
    "to_json_type_segment",
    "from_json_type_segment",
    "ListPool",
    "encode_list_pool",
    "decode_list_pool",
    "to_json_list_pool",
    "from_json_list_pool",
]
