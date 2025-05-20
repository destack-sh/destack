from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    NodeType,
    PageNode,
    TypeBase,
    node_,
    p_node_parent,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    IsTemplatable,
    IsModal,
    IsNamed,
    TypeBase,
    PageNode[BlockData],
):
    """A Schema for a specific Type."""

    parent: Union["Page", None] = p_node_parent(4)
