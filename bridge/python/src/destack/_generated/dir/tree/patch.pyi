# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Patch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Patch: ...

def encode_patch(writer: BinaryWriter, value: Patch) -> None: ...
def decode_patch(reader: BinaryReader) -> Patch: ...
def to_json_patch(value: Patch) -> Json: ...
def from_json_patch(value: Json) -> Patch: ...

__all__ = [
    "Patch",
    "encode_patch",
    "decode_patch",
    "to_json_patch",
    "from_json_patch",
]
