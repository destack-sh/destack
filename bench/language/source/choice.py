from functools import cached_property
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    LocalNodeList,
    NodeType,
    SourceNode,
    StructType,
    TypeBase,
    TypeKind,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Field, Icon, Page, Text

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CHOICE, passthrough_get=("fields",))
class Choice(SourceNode[BlockData]):
    """A Choice of Fields."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    # content
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )

    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

    def __content_str__(self):
        return ""

    def to_type_maybe(self) -> "TypeBase | None":
        return TypeBase(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.FIELD)

    def to_type(self) -> "TypeBase":
        typ = self.to_type_maybe()
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def instance_type(self) -> "TypeBase":
        return self.to_type()

    @staticmethod
    def new(name: str, **kwargs) -> "Choice":
        return Choice(name=name, **kwargs)
