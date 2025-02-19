from typing import TYPE_CHECKING, Union

from bench.language.core import (
    ColorType,
    InlineSourceNode,
    NodeType,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import IdentityData

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.IDENTITY)
class Identity(InlineSourceNode[IdentityData]):
    """A unique Identity to assume."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    color: ColorType | None = p_regular(40)
