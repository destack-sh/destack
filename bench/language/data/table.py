from typing import TYPE_CHECKING, override

from bench.language.core import (
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    NodeType,
    PageNode,
    RemoteNodeList,
    node_,
)
from bench.language.core.list import attach_node
from bench.pb2 import RecordData, TableData

if TYPE_CHECKING:
    from bench.language import Field, Record

# pyright: reportIncompatibleVariableOverride=false


class RecordNodeList(RemoteNodeList["Record", RecordData]):
    """
    A RemoteNodeList that is backed by a Table.
    Automatically sets 'table' as needed.
    """

    @override
    def add_child(self, node: "Record", move: bool = False) -> "Record":
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        attach_node(node, parent=parent, graph=graph, move=move)
        if getattr(node, "table_id") != self._node.id:
            node._do_set("table", self._node)
        return node

    @override
    def add_children(self, *nodes: "Record", move: bool = False) -> None:
        parent = self._get_parent()
        graph = self._get_child_graph(parent)
        for node in nodes:
            attach_node(node, parent=parent, graph=graph, move=move)
            if getattr(node, "table_id") != self._node.id:
                node._do_set("table", self._node)


@node_(NodeType.TABLE)
class Table(
    IsInstantiable,
    IsTemplatable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsClaimable,
    PageNode[TableData],
):
    """A Table of Records."""

    _record: RecordNodeList | None = None

    @property
    def records(self) -> RecordNodeList:
        if self._record is None:
            self._record = RecordNodeList(self, NodeType.RECORD)
        return self._record

    def __content_str__(self):
        return ""

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Table":
        table = Table(name=name, **kwargs)
        for field in fields:
            table.add_child(field)
        return table
