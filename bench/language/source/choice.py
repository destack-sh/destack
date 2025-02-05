from functools import cached_property
from typing import TYPE_CHECKING, Literal, Union

from bench.language.core import (
    InlineSourceNode,
    LocalNodeList,
    NodeType,
    Type,
    TypeBase,
    TypeKind,
    node_,
    p_node_children,
    p_node_parent,
)
from bench.pb2.lang_pb2 import ChoiceData

if TYPE_CHECKING:
    from bench.language import Field, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CHOICE, passthrough_get=("fields",))
class Choice(InlineSourceNode[ChoiceData]):
    """A Choice of Fields."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

    def __content_str__(self):
        return ""

    def to_type_maybe(self, of: Literal["instance", "value"] = "instance") -> "TypeBase | None":
        return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.FIELD)

    def to_type(self) -> "TypeBase":
        typ = self.to_type_maybe()
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def instance_type(self) -> "TypeBase":
        return self.to_type()

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Choice":
        choice = Choice(name=name, **kwargs)
        for field in fields:
            choice.fields.append(field)
        return choice
