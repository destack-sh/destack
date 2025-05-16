from typing import TYPE_CHECKING, Literal, override

from bench.language.core import (
    FieldType,
    IsClaimable,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    NodeType,
    PageNode,
    RemoteNodeList,
    TypeBase,
    TypeKind,
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
    def create(self, **kwargs) -> "Record":
        from bench.language import Record, Table

        if "table" not in kwargs:
            kwargs["table"] = self._node
        parent = self._get_parent()
        assert isinstance(parent, Table), f"cannot create Record in: {parent!r}"
        graph = self._get_child_graph(parent)
        node = Record(**kwargs, parent=parent)
        attach_node(node, parent=parent, graph=graph, move=False)
        return node

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

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase | None":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RECORD)
        else:
            field_types = field_types or [FieldType.MEMBER]
            return Type(
                kind=TypeKind.CUSTOM_OBJECT,
                base_type=self,
                base_field_types=field_types,
                property_field_types=field_types,
            )

    def to_type(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Table":
        table = Table(name=name, **kwargs)
        for field in fields:
            table.add_child(field)
        return table
