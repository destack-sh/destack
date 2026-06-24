# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    nested_bytes,
)

from destack._impl.mir.metadata.type import (
    TypeMetadataImpl,
)

import destack._generated.core.string
import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class TypeMetadata(TypeMetadataImpl):
    """Canonical type metadata for one MIR module."""

    # nominal lineage keyed by type id
    lineage_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, TypeLineage]
    # runtime type descriptor globals keyed by type id
    descriptor_by_type: Mapping[
        destack._generated.mir.tree.node.LocalNodeId,
        destack._generated.mir.tree.node.LocalNodeId,
    ]
    # canonical display names keyed by type id
    display_name_by_type: Mapping[
        destack._generated.mir.tree.node.LocalNodeId,
        destack._generated.core.string.StringId,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeMetadata:
        """Decode one TypeMetadata."""
        return decode_type_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeMetadata:
        """Return one TypeMetadata from one JSON value."""
        return from_json_type_metadata(value)


def encode_type_metadata(writer: BinaryWriter, value: TypeMetadata) -> None:
    """Encode one TypeMetadata."""
    entries_value_lineage_by_type_0 = []
    for (
        key_value_lineage_by_type_0,
        item_value_lineage_by_type_0,
    ) in value.lineage_by_type.items():

        def write_key_value_lineage_by_type_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_lineage_by_type_0
            )

        key_bytes = nested_bytes(write_key_value_lineage_by_type_0)
        entries_value_lineage_by_type_0.append(
            (key_value_lineage_by_type_0, item_value_lineage_by_type_0, key_bytes)
        )
    entries_value_lineage_by_type_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_lineage_by_type_0))
    for entry_value_lineage_by_type_0 in entries_value_lineage_by_type_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_lineage_by_type_0[0]
        )
        encode_type_lineage(writer, entry_value_lineage_by_type_0[1])
    entries_value_descriptor_by_type_0 = []
    for (
        key_value_descriptor_by_type_0,
        item_value_descriptor_by_type_0,
    ) in value.descriptor_by_type.items():

        def write_key_value_descriptor_by_type_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_descriptor_by_type_0
            )

        key_bytes = nested_bytes(write_key_value_descriptor_by_type_0)
        entries_value_descriptor_by_type_0.append(
            (key_value_descriptor_by_type_0, item_value_descriptor_by_type_0, key_bytes)
        )
    entries_value_descriptor_by_type_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_descriptor_by_type_0))
    for entry_value_descriptor_by_type_0 in entries_value_descriptor_by_type_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_descriptor_by_type_0[0]
        )
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_descriptor_by_type_0[1]
        )
    entries_value_display_name_by_type_0 = []
    for (
        key_value_display_name_by_type_0,
        item_value_display_name_by_type_0,
    ) in value.display_name_by_type.items():

        def write_key_value_display_name_by_type_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_display_name_by_type_0
            )

        key_bytes = nested_bytes(write_key_value_display_name_by_type_0)
        entries_value_display_name_by_type_0.append(
            (
                key_value_display_name_by_type_0,
                item_value_display_name_by_type_0,
                key_bytes,
            )
        )
    entries_value_display_name_by_type_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_display_name_by_type_0))
    for entry_value_display_name_by_type_0 in entries_value_display_name_by_type_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_display_name_by_type_0[0]
        )
        destack._generated.core.string.encode_string_id(
            writer, entry_value_display_name_by_type_0[1]
        )


def decode_type_metadata(reader: BinaryReader) -> TypeMetadata:
    """Decode one TypeMetadata."""
    lineage_by_type = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): decode_type_lineage(reader)
        for _ in range(reader.read_number())
    }
    descriptor_by_type = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    }
    display_name_by_type = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): destack._generated.core.string.decode_string_id(reader)
        for _ in range(reader.read_number())
    }

    return TypeMetadata(
        lineage_by_type=lineage_by_type,
        descriptor_by_type=descriptor_by_type,
        display_name_by_type=display_name_by_type,
    )


def to_json_type_metadata(value: TypeMetadata) -> Json:
    """Return one JSON value for one TypeMetadata."""
    return {
        "lineageByType": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_type_lineage(item_0),
            ]
            for key_0, item_0 in value.lineage_by_type.items()
        ],
        "descriptorByType": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                destack._generated.mir.tree.node.to_json_local_node_id(item_0),
            ]
            for key_0, item_0 in value.descriptor_by_type.items()
        ],
        "displayNameByType": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                destack._generated.core.string.to_json_string_id(item_0),
            ]
            for key_0, item_0 in value.display_name_by_type.items()
        ],
    }


def from_json_type_metadata(value: Json) -> TypeMetadata:
    """Return one TypeMetadata from one JSON value."""
    object_ = json_object(value)

    return TypeMetadata(
        lineage_by_type={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_type_lineage(item_0)
            for key_0, item_0 in json_array(json_field(object_, "lineageByType"))
        },
        descriptor_by_type={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "descriptorByType"))
        },
        display_name_by_type={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): destack._generated.core.string.from_json_string_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "displayNameByType"))
        },
    )


@dataclass(frozen=True, slots=True)
class TypeLineage:
    """Lineage metadata for nominal types."""

    # optional parent type for class inheritance
    parent: destack._generated.mir.tree.node.LocalNodeId | None
    # interfaces implemented by this type
    interfaces: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # true when the type is sealed to external extension
    is_sealed: bool
    # true when the type is final and cannot be subclassed
    is_final: bool
    # true when the type is abstract and cannot be instantiated
    is_abstract: bool
    # true when the type represents an interface
    is_interface: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_lineage(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeLineage:
        """Decode one TypeLineage."""
        return decode_type_lineage(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_lineage(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeLineage:
        """Return one TypeLineage from one JSON value."""
        return from_json_type_lineage(value)


def encode_type_lineage(writer: BinaryWriter, value: TypeLineage) -> None:
    """Encode one TypeLineage."""
    if value.parent is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.parent)
    writer.write_unsigned(len(value.interfaces))
    for item_value_interfaces_0 in value.interfaces:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_interfaces_0
        )
    writer.write_bool(value.is_sealed)
    writer.write_bool(value.is_final)
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_interface)


def decode_type_lineage(reader: BinaryReader) -> TypeLineage:
    """Decode one TypeLineage."""
    parent = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    interfaces = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_sealed = reader.read_bool()
    is_final = reader.read_bool()
    is_abstract = reader.read_bool()
    is_interface = reader.read_bool()

    return TypeLineage(
        parent=parent,
        interfaces=interfaces,
        is_sealed=is_sealed,
        is_final=is_final,
        is_abstract=is_abstract,
        is_interface=is_interface,
    )


def to_json_type_lineage(value: TypeLineage) -> Json:
    """Return one JSON value for one TypeLineage."""
    return {
        **(
            {}
            if value.parent is None
            else {
                "parent": destack._generated.mir.tree.node.to_json_local_node_id(
                    value.parent
                )
            }
        ),
        "interfaces": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.interfaces
        ],
        "isSealed": value.is_sealed,
        "isFinal": value.is_final,
        "isAbstract": value.is_abstract,
        "isInterface": value.is_interface,
    }


def from_json_type_lineage(value: Json) -> TypeLineage:
    """Return one TypeLineage from one JSON value."""
    object_ = json_object(value)

    return TypeLineage(
        parent=json_optional(
            object_,
            "parent",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        interfaces=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "interfaces"))
        ],
        is_sealed=json_bool(json_field(object_, "isSealed")),
        is_final=json_bool(json_field(object_, "isFinal")),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_interface=json_bool(json_field(object_, "isInterface")),
    )


__all__ = [
    "TypeMetadata",
    "encode_type_metadata",
    "decode_type_metadata",
    "to_json_type_metadata",
    "from_json_type_metadata",
    "TypeLineage",
    "encode_type_lineage",
    "decode_type_lineage",
    "to_json_type_lineage",
    "from_json_type_lineage",
]
