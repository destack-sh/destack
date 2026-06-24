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
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.mir.tree.node
import destack._generated.mir.tree.tree


@dataclass(frozen=True, slots=True)
class Patch:
    """A durable overlay over one base MIR tree."""

    # the patch name
    name: str
    # the tree containing nodes introduced by this patch
    tree: destack._generated.mir.tree.tree.Tree
    # replacement roots keyed by the base node they replace
    replacement_by_node: Mapping[
        destack._generated.mir.tree.node.LocalNodeIdAny,
        destack._generated.mir.tree.node.LocalNodeIdAny,
    ]
    # node ids hidden by this patch
    dead_nodes: Sequence[destack._generated.mir.tree.node.LocalNodeIdAny]

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
    destack._generated.mir.tree.tree.encode_tree(writer, value.tree)
    entries_value_replacement_by_node_0 = []
    for (
        key_value_replacement_by_node_0,
        item_value_replacement_by_node_0,
    ) in value.replacement_by_node.items():

        def write_key_value_replacement_by_node_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id_any(
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
        destack._generated.mir.tree.node.encode_local_node_id_any(
            writer, entry_value_replacement_by_node_0[0]
        )
        destack._generated.mir.tree.node.encode_local_node_id_any(
            writer, entry_value_replacement_by_node_0[1]
        )
    writer.write_unsigned(len(value.dead_nodes))
    for item_value_dead_nodes_0 in value.dead_nodes:
        destack._generated.mir.tree.node.encode_local_node_id_any(
            writer, item_value_dead_nodes_0
        )


def decode_patch(reader: BinaryReader) -> Patch:
    """Decode one Patch."""
    name = reader.read_string()
    tree = destack._generated.mir.tree.tree.decode_tree(reader)
    replacement_by_node = {
        destack._generated.mir.tree.node.decode_local_node_id_any(
            reader
        ): destack._generated.mir.tree.node.decode_local_node_id_any(reader)
        for _ in range(reader.read_number())
    }
    dead_nodes = [
        destack._generated.mir.tree.node.decode_local_node_id_any(reader)
        for _ in range(reader.read_number())
    ]

    return Patch(
        name=name,
        tree=tree,
        replacement_by_node=replacement_by_node,
        dead_nodes=dead_nodes,
    )


def to_json_patch(value: Patch) -> Json:
    """Return one JSON value for one Patch."""
    return {
        "name": value.name,
        "tree": destack._generated.mir.tree.tree.to_json_tree(value.tree),
        "replacementByNode": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id_any(key_0),
                destack._generated.mir.tree.node.to_json_local_node_id_any(item_0),
            ]
            for key_0, item_0 in value.replacement_by_node.items()
        ],
        "deadNodes": [
            destack._generated.mir.tree.node.to_json_local_node_id_any(item_0)
            for item_0 in value.dead_nodes
        ],
    }


def from_json_patch(value: Json) -> Patch:
    """Return one Patch from one JSON value."""
    object_ = json_object(value)

    return Patch(
        name=json_string(json_field(object_, "name")),
        tree=destack._generated.mir.tree.tree.from_json_tree(
            json_field(object_, "tree")
        ),
        replacement_by_node={
            destack._generated.mir.tree.node.from_json_local_node_id_any(
                key_0
            ): destack._generated.mir.tree.node.from_json_local_node_id_any(item_0)
            for key_0, item_0 in json_array(json_field(object_, "replacementByNode"))
        },
        dead_nodes=[
            destack._generated.mir.tree.node.from_json_local_node_id_any(item_0)
            for item_0 in json_array(json_field(object_, "deadNodes"))
        ],
    )


__all__ = [
    "Patch",
    "encode_patch",
    "decode_patch",
    "to_json_patch",
    "from_json_patch",
]
