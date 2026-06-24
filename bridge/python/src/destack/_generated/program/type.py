# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    nested_bytes,
)

import destack._generated.mir.metadata.layout
import destack._generated.mir.tree.node
import destack._generated.mir.tree.type


@dataclass(frozen=True, slots=True)
class TypeTable:
    """Runtime type metadata carried by one durable program."""

    # dense runtime type records keyed by program type id
    types: Sequence[destack._generated.mir.tree.type.Type | None]
    # MIR type id by program type id
    mir_types: Sequence[destack._generated.mir.tree.node.LocalNodeId | None]
    # program type id by MIR type id
    type_ids: Mapping[destack._generated.mir.tree.node.LocalNodeId, TypeId]
    # dense runtime layout ids keyed by program type id
    layout_by_type: Sequence[destack._generated.mir.metadata.layout.LayoutId | None]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeTable:
        """Decode one TypeTable."""
        return decode_type_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_table(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeTable:
        """Return one TypeTable from one JSON value."""
        return from_json_type_table(value)


def encode_type_table(writer: BinaryWriter, value: TypeTable) -> None:
    """Encode one TypeTable."""
    writer.write_unsigned(len(value.types))
    for item_value_types_0 in value.types:
        if item_value_types_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.type.encode_type(writer, item_value_types_0)
    writer.write_unsigned(len(value.mir_types))
    for item_value_mir_types_0 in value.mir_types:
        if item_value_mir_types_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, item_value_mir_types_0
            )
    entries_value_type_ids_0 = []
    for key_value_type_ids_0, item_value_type_ids_0 in value.type_ids.items():

        def write_key_value_type_ids_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_type_ids_0
            )

        key_bytes = nested_bytes(write_key_value_type_ids_0)
        entries_value_type_ids_0.append(
            (key_value_type_ids_0, item_value_type_ids_0, key_bytes)
        )
    entries_value_type_ids_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_type_ids_0))
    for entry_value_type_ids_0 in entries_value_type_ids_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_type_ids_0[0]
        )
        encode_type_id(writer, entry_value_type_ids_0[1])
    writer.write_unsigned(len(value.layout_by_type))
    for item_value_layout_by_type_0 in value.layout_by_type:
        if item_value_layout_by_type_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.metadata.layout.encode_layout_id(
                writer, item_value_layout_by_type_0
            )


def decode_type_table(reader: BinaryReader) -> TypeTable:
    """Decode one TypeTable."""
    types = [
        reader.read_option(lambda: destack._generated.mir.tree.type.decode_type(reader))
        for _ in range(reader.read_number())
    ]
    mir_types = [
        reader.read_option(
            lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
        )
        for _ in range(reader.read_number())
    ]
    type_ids = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): decode_type_id(
            reader
        )
        for _ in range(reader.read_number())
    }
    layout_by_type = [
        reader.read_option(
            lambda: destack._generated.mir.metadata.layout.decode_layout_id(reader)
        )
        for _ in range(reader.read_number())
    ]

    return TypeTable(
        types=types,
        mir_types=mir_types,
        type_ids=type_ids,
        layout_by_type=layout_by_type,
    )


def to_json_type_table(value: TypeTable) -> Json:
    """Return one JSON value for one TypeTable."""
    return {
        "types": [
            None
            if item_0 is None
            else destack._generated.mir.tree.type.to_json_type(item_0)
            for item_0 in value.types
        ],
        "mirTypes": [
            None
            if item_0 is None
            else destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.mir_types
        ],
        "typeIds": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_type_id(item_0),
            ]
            for key_0, item_0 in value.type_ids.items()
        ],
        "layoutByType": [
            None
            if item_0 is None
            else destack._generated.mir.metadata.layout.to_json_layout_id(item_0)
            for item_0 in value.layout_by_type
        ],
    }


def from_json_type_table(value: Json) -> TypeTable:
    """Return one TypeTable from one JSON value."""
    object_ = json_object(value)

    return TypeTable(
        types=[
            None
            if item_0 is None
            else destack._generated.mir.tree.type.from_json_type(item_0)
            for item_0 in json_array(json_field(object_, "types"))
        ],
        mir_types=[
            None
            if item_0 is None
            else destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "mirTypes"))
        ],
        type_ids={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_type_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "typeIds"))
        },
        layout_by_type=[
            None
            if item_0 is None
            else destack._generated.mir.metadata.layout.from_json_layout_id(item_0)
            for item_0 in json_array(json_field(object_, "layoutByType"))
        ],
    )


"""Durable runtime type id inside one program."""
TypeId: typing.TypeAlias = int


def encode_type_id(writer: BinaryWriter, value: TypeId) -> None:
    """Encode one TypeId."""
    writer.write_unsigned(value)


def decode_type_id(reader: BinaryReader) -> TypeId:
    """Decode one TypeId."""
    return reader.read_number()


def to_json_type_id(value: TypeId) -> Json:
    """Return one JSON value for one TypeId."""
    return value


def from_json_type_id(value: Json) -> TypeId:
    """Return one TypeId from one JSON value."""
    return json_int(value)


__all__ = [
    "TypeTable",
    "encode_type_table",
    "decode_type_table",
    "to_json_type_table",
    "from_json_type_table",
    "TypeId",
    "encode_type_id",
    "decode_type_id",
    "to_json_type_id",
    "from_json_type_id",
]
