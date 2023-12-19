from typing import Union

from bench.language import Session, wire
from bench.language.const import NodeType
from bench.language.module import Node, Struct

# nocheckin: auto-gen AnyNodeData/AnyStructData?
AnyNodeData = Union[wire.ModuleData, wire.FileData, wire.StatementData, wire.FieldData]


def pack_struct(struct: Struct) -> wire.SomeNodeData:
    """Pack a struct and any contained structs."""
    raise NotImplementedError("nocheckin: pack_struct")


def unpack_struct(struct: Struct) -> wire.SomeStructData:
    """Unpack a struct and any contained structs."""
    raise NotImplementedError("nocheckin: unpack_struct")


def pack_node(node: Node) -> wire.SomeNodeData:
    raise NotImplementedError("nocheckin: pack_node_flat")


def unpack_node(data: wire.SomeNodeData, parent: Node, session: Session | None) -> Node:
    raise NotImplementedError("nocheckin: unpack_node_flat")


def pack_node_inline(root: Node, exclude: set[NodeType] = None) -> wire.SomeNodeData:
    """Pack a node and all its inline descendants"""
    raise NotImplementedError("nocheckin: pack_node_inline")


def unpack_node_inline(
    nodes: list[wire.SomeNodeData], parent: Node | None, session: Session | None
) -> Node:
    raise NotImplementedError("nocheckin: unpack_node")


def wrap_some_node(node: AnyNodeData) -> wire.SomeNodeData:
    """Wraps a concrete node type into a generic node type (different message type!)."""
    raise NotImplementedError("nocheckin: wrap_some_node")


def unwrap_some_node(node: wire.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type (different message type!)."""
    raise NotImplementedError("nocheckin: unwrap_some_node")
