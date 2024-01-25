import typing
from typing import Optional

from bench.language.const import NodeType, StructType
from bench.language.expression import Expression
from bench.language.node import Node, node, node_parent, struct_property

if typing.TYPE_CHECKING:
    from bench.language import Block


@node(NodeType.VIEW)
class View(Node):
    parent: typing.Union["Block"] = node_parent(4, NodeType.BLOCK)
    name: str | None = struct_property(30, default=None)
    order_key: str | None = struct_property(31, default=None)
    node_type: NodeType = struct_property(32)
    filter: Optional[Expression] = struct_property(33, default=None, struct=StructType.EXPRESSION)
    sort: Optional[list[Expression]] = struct_property(
        34, default=None, struct=StructType.EXPRESSION
    )

    def __content_str__(self):
        return f"{self.node_type}[{self.filter}, {self.sort or '<default sort>'}]"
