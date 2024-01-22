import typing
from typing import Optional

from bench.language.const import NodeType, StructType
from bench.language.expression import Expression
from bench.language.node import Node, node, node_parent, struct_property

if typing.TYPE_CHECKING:
    from bench.language import Statement


@node(NodeType.VIEW)
class View(Node):
    parent: typing.Union["Statement"] = node_parent(4, NodeType.STATEMENT)
    name: str | None = struct_property(30, default=None)
    order_key: str | None = struct_property(31, default=None)
    node_type: NodeType = struct_property(32)
    filter: Optional[Expression] = struct_property(33, default=None, struct_t=StructType.EXPRESSION)
    sort: Optional[list[Expression]] = struct_property(
        34, default=None, struct_t=StructType.EXPRESSION
    )

    def __str__(self):
        return f"{self.parent.path}:{self.name} {self.node_type} ({self.filter}, {self.layout})"

    def __repr__(self):
        return f"<View {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"
