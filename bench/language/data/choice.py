from typing import TYPE_CHECKING, Literal, Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    NodeType,
    PageNode,
    Type,
    TypeBase,
    TypeType,
    node_,
    p_node_parent,
)
from bench.pb2.lang_pb2 import ChoiceData

if TYPE_CHECKING:
    from bench.language import Option, Page, Table

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CHOICE)
class Choice(
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[ChoiceData],
):
    """A Choice of Options."""

    parent: Union["Page", "Table", None] = p_node_parent(4, NodeType.PAGE, NodeType.TABLE)

    def __content_str__(self):
        return ""

    def to_type_maybe(self, of: Literal["instance", "value"] = "instance") -> "TypeBase | None":
        return Type(kind=TypeType.NODE, base_type=self, node_type=NodeType.OPTION)

    def to_type(self) -> "TypeBase":
        typ = self.to_type_maybe()
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @staticmethod
    def new(name: str, *options: "Option", **kwargs) -> "Choice":
        choice = Choice(name=name, **kwargs)
        for option in options:
            choice.add_child(option)
        return choice
