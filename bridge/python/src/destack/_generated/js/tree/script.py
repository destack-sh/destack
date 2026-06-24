# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
)

import destack._generated.core.string
import destack._generated.js.tree.node
import destack._generated.js.tree.tree


@dataclass(frozen=True, slots=True)
class Module:
    """One lowered JavaScript or TypeScript module tree."""

    # the lowered script tree
    tree: destack._generated.js.tree.tree.Tree
    # the root nodes in the lowered tree
    roots: Sequence[destack._generated.js.tree.node.LocalNodeIdAny]
    # the string pool for the lowered tree
    strings: destack._generated.core.string.StringPoolData

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Module:
        """Decode one Module."""
        return decode_module(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module(self)

    @classmethod
    def from_json(cls, value: Json) -> Module:
        """Return one Module from one JSON value."""
        return from_json_module(value)


def encode_module(writer: BinaryWriter, value: Module) -> None:
    """Encode one Module."""
    destack._generated.js.tree.tree.encode_tree(writer, value.tree)
    writer.write_unsigned(len(value.roots))
    for item_value_roots_0 in value.roots:
        destack._generated.js.tree.node.encode_local_node_id_any(
            writer, item_value_roots_0
        )
    destack._generated.core.string.encode_string_pool_data(writer, value.strings)


def decode_module(reader: BinaryReader) -> Module:
    """Decode one Module."""
    tree = destack._generated.js.tree.tree.decode_tree(reader)
    roots = [
        destack._generated.js.tree.node.decode_local_node_id_any(reader)
        for _ in range(reader.read_number())
    ]
    strings = destack._generated.core.string.decode_string_pool_data(reader)

    return Module(
        tree=tree,
        roots=roots,
        strings=strings,
    )


def to_json_module(value: Module) -> Json:
    """Return one JSON value for one Module."""
    return {
        "tree": destack._generated.js.tree.tree.to_json_tree(value.tree),
        "roots": [
            destack._generated.js.tree.node.to_json_local_node_id_any(item_0)
            for item_0 in value.roots
        ],
        "strings": destack._generated.core.string.to_json_string_pool_data(
            value.strings
        ),
    }


def from_json_module(value: Json) -> Module:
    """Return one Module from one JSON value."""
    object_ = json_object(value)

    return Module(
        tree=destack._generated.js.tree.tree.from_json_tree(
            json_field(object_, "tree")
        ),
        roots=[
            destack._generated.js.tree.node.from_json_local_node_id_any(item_0)
            for item_0 in json_array(json_field(object_, "roots"))
        ],
        strings=destack._generated.core.string.from_json_string_pool_data(
            json_field(object_, "strings")
        ),
    )


__all__ = [
    "Module",
    "encode_module",
    "decode_module",
    "to_json_module",
    "from_json_module",
]
