import typing
from typing import Optional

from bench.language.const import NodeType, ViewLayout
from bench.language.expression import Expression
from bench.language.module import ScopeNode, node, node_parent, struct_property
from bench.language.validation import enum_validator

if typing.TYPE_CHECKING:
    from bench.language import File, Statement


@node(NodeType.VIEW)
class View(ScopeNode):
    parent: typing.Union["Statement", "File"] = node_parent(NodeType.STATEMENT, NodeType.FILE)
    name: str | None = struct_property(default=None)
    layout: ViewLayout = struct_property(
        default=ViewLayout.TABLE, validate=enum_validator(ViewLayout)
    )
    query: Optional[Expression] = struct_property(default=None)
    sort: Optional[list[Expression]] = struct_property(default=None)

    def __str__(self):
        return f"{self.parent.path}:{self.name} ({self.layout})"

    def __repr__(self):
        return f"<DatabaseView {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"
