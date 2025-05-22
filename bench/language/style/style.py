from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsBlockable,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    Trait,
    p_node_parent,
    trait_,
)

if TYPE_CHECKING:
    from bench.language import IsView, Page, Space

# pyright: reportIncompatibleVariableOverride=false


@trait_(Trait.STYLE)
class IsStyle(
    IsTemplatable,
    IsModal,
    IsNamed,
    IsBlockable,
    Node if TYPE_CHECKING else object,
):
    """A Style is a graphical interface."""

    parent: Union["Space", "IsView", "Page", None] = p_node_parent()
