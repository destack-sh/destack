from functools import cached_property
from typing import TYPE_CHECKING, Literal, Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    IsType,
    LocalNodeList,
    NodeType,
    PageNode,
    Type,
    TypeKind,
    node_,
    p_node_children,
    p_node_parent,
)
from bench.pb2.lang_pb2 import ChoiceData

if TYPE_CHECKING:
    from bench.language import Database, Option, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CHOICE)
class Choice(
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[ChoiceData],
):
    """A Choice of Options."""

    parent: Union["Page", "Database", None] = p_node_parent(4, NodeType.PAGE, NodeType.DATABASE)

    options: LocalNodeList["Option"] = p_node_children(NodeType.OPTION)

    def __content_str__(self):
        return ""

    def to_type_maybe(self, of: Literal["instance", "value"] = "instance") -> "IsType | None":
        return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.OPTION)

    def to_type(self) -> "IsType":
        typ = self.to_type_maybe()
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def instance_type(self) -> "IsType":
        return self.to_type()

    @staticmethod
    def new(name: str, *options: "Option", **kwargs) -> "Choice":
        choice = Choice(name=name, **kwargs)
        for option in options:
            choice.options.append(option)
        return choice
