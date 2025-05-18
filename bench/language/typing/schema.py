from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    NodeType,
    PageNode,
    node_,
    p_node_parent,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Field, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[BlockData],
):
    """A Schema with Fields."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    def __content_str__(self):
        return ""

    @staticmethod
    def new(name: str, *fields: "Field", **kwargs) -> "Schema":
        cls = Schema(name=name, **kwargs)
        for field in fields:
            cls.add_child(field)
        return cls
