from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsBlockable,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    NodeTrait,
    node_trait_,
    p_node_parent,
)

if TYPE_CHECKING:
    from bench.language import Page, Space, ViewBase

# pyright: reportIncompatibleVariableOverride=false


@node_trait_(NodeTrait.STYLE)
class IsStyle(
    IsTemplatable,
    IsModal,
    IsNamed,
    IsBlockable,
    Node if TYPE_CHECKING else object,
):
    """A Style is a graphical interface."""

    parent: Union["Space", "ViewBase", "Page", None] = p_node_parent(4)
