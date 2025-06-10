from typing import TYPE_CHECKING, Union

from destack.language.core import (
    HasName,
    IsEntity,
    IsEnvironmental,
    IsInFolder,
    IsTemplatable,
    TraitType,
    property_parent_,
    trait_,
)

if TYPE_CHECKING:
    from destack.language import IsView, Scene

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.STYLE)
class IsStyle(
    HasName,
    IsTemplatable,
    IsEnvironmental,
    IsInFolder,
    IsEntity,
):
    """A Style is a graphical interface."""

    parent: Union["Scene", "IsView", None] = property_parent_(node_is_customizable=True)
