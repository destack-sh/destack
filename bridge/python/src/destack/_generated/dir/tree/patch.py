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
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.dir.tree.node
import destack._generated.dir.tree.tree
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class Patch:
    """A durable overlay over one base DIR tree."""

    # the patch name
    name: str
    # the owning module id
    module_id: destack._generated.source.file.model.module.ModuleId
    # the tree containing nodes introduced by this patch
    tree: destack._generated.dir.tree.tree.Tree
    # replacement roots keyed by the base node they replace
    replacement_by_node: Mapping[
        destack._generated.dir.tree.node.LocalNodeIdAny,
        destack._generated.dir.tree.node.LocalNodeIdAny,
    ]
    # node ids deleted by this patch
    deleted_nodes: Sequence[destack._generated.dir.tree.node.LocalNodeIdAny]
    # parent overrides keyed by the visible child node
    parent_by_node: Mapping[
        destack._generated.dir.tree.node.LocalNodeIdAny,
        destack._generated.dir.tree.node.LocalNodeIdAny | None,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_patch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Patch:
        """Decode one Patch."""
        return decode_patch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_patch(self)

    @classmethod
    def from_json(cls, value: Json) -> Patch:
        """Return one Patch from one JSON value."""
        return from_json_patch(value)


def encode_patch(writer: BinaryWriter, value: Patch) -> None:
    """Encode one Patch."""
    writer.write_string(value.name)
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.dir.tree.tree.encode_tree(writer, value.tree)
    entries_value_replacement_by_node_0 = []
    for (
        key_value_replacement_by_node_0,
        item_value_replacement_by_node_0,
    ) in value.replacement_by_node.items():

        def write_key_value_replacement_by_node_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_local_node_id_any(
                writer, key_value_replacement_by_node_0
            )

        key_bytes = nested_bytes(write_key_value_replacement_by_node_0)
        entries_value_replacement_by_node_0.append(
            (
                key_value_replacement_by_node_0,
                item_value_replacement_by_node_0,
                key_bytes,
            )
        )
    entries_value_replacement_by_node_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_replacement_by_node_0))
    for entry_value_replacement_by_node_0 in entries_value_replacement_by_node_0:
        destack._generated.dir.tree.node.encode_local_node_id_any(
            writer, entry_value_replacement_by_node_0[0]
        )
        destack._generated.dir.tree.node.encode_local_node_id_any(
            writer, entry_value_replacement_by_node_0[1]
        )
    writer.write_unsigned(len(value.deleted_nodes))
    for item_value_deleted_nodes_0 in value.deleted_nodes:
        destack._generated.dir.tree.node.encode_local_node_id_any(
            writer, item_value_deleted_nodes_0
        )
    entries_value_parent_by_node_0 = []
    for (
        key_value_parent_by_node_0,
        item_value_parent_by_node_0,
    ) in value.parent_by_node.items():

        def write_key_value_parent_by_node_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_local_node_id_any(
                writer, key_value_parent_by_node_0
            )

        key_bytes = nested_bytes(write_key_value_parent_by_node_0)
        entries_value_parent_by_node_0.append(
            (key_value_parent_by_node_0, item_value_parent_by_node_0, key_bytes)
        )
    entries_value_parent_by_node_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_parent_by_node_0))
    for entry_value_parent_by_node_0 in entries_value_parent_by_node_0:
        destack._generated.dir.tree.node.encode_local_node_id_any(
            writer, entry_value_parent_by_node_0[0]
        )
        if entry_value_parent_by_node_0[1] is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id_any(
                writer, entry_value_parent_by_node_0[1]
            )


def decode_patch(reader: BinaryReader) -> Patch:
    """Decode one Patch."""
    name = reader.read_string()
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    tree = destack._generated.dir.tree.tree.decode_tree(reader)
    replacement_by_node = {
        destack._generated.dir.tree.node.decode_local_node_id_any(
            reader
        ): destack._generated.dir.tree.node.decode_local_node_id_any(reader)
        for _ in range(reader.read_number())
    }
    deleted_nodes = [
        destack._generated.dir.tree.node.decode_local_node_id_any(reader)
        for _ in range(reader.read_number())
    ]
    parent_by_node = {
        destack._generated.dir.tree.node.decode_local_node_id_any(
            reader
        ): reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id_any(reader)
        )
        for _ in range(reader.read_number())
    }

    return Patch(
        name=name,
        module_id=module_id,
        tree=tree,
        replacement_by_node=replacement_by_node,
        deleted_nodes=deleted_nodes,
        parent_by_node=parent_by_node,
    )


def to_json_patch(value: Patch) -> Json:
    """Return one JSON value for one Patch."""
    return {
        "name": value.name,
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "tree": destack._generated.dir.tree.tree.to_json_tree(value.tree),
        "replacementByNode": [
            [
                destack._generated.dir.tree.node.to_json_local_node_id_any(key_0),
                destack._generated.dir.tree.node.to_json_local_node_id_any(item_0),
            ]
            for key_0, item_0 in value.replacement_by_node.items()
        ],
        "deletedNodes": [
            destack._generated.dir.tree.node.to_json_local_node_id_any(item_0)
            for item_0 in value.deleted_nodes
        ],
        "parentByNode": [
            [
                destack._generated.dir.tree.node.to_json_local_node_id_any(key_0),
                None
                if item_0 is None
                else destack._generated.dir.tree.node.to_json_local_node_id_any(item_0),
            ]
            for key_0, item_0 in value.parent_by_node.items()
        ],
    }


def from_json_patch(value: Json) -> Patch:
    """Return one Patch from one JSON value."""
    object_ = json_object(value)

    return Patch(
        name=json_string(json_field(object_, "name")),
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        tree=destack._generated.dir.tree.tree.from_json_tree(
            json_field(object_, "tree")
        ),
        replacement_by_node={
            destack._generated.dir.tree.node.from_json_local_node_id_any(
                key_0
            ): destack._generated.dir.tree.node.from_json_local_node_id_any(item_0)
            for key_0, item_0 in json_array(json_field(object_, "replacementByNode"))
        },
        deleted_nodes=[
            destack._generated.dir.tree.node.from_json_local_node_id_any(item_0)
            for item_0 in json_array(json_field(object_, "deletedNodes"))
        ],
        parent_by_node={
            destack._generated.dir.tree.node.from_json_local_node_id_any(key_0): None
            if item_0 is None
            else destack._generated.dir.tree.node.from_json_local_node_id_any(item_0)
            for key_0, item_0 in json_array(json_field(object_, "parentByNode"))
        },
    )


__all__ = [
    "Patch",
    "encode_patch",
    "decode_patch",
    "to_json_patch",
    "from_json_patch",
]
