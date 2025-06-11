from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    HasName,
    IsDeletable,
    IsEnvironmental,
    IsInFolder,
    IsScriptable,
    IsTemplatable,
    IsTracked,
    IsVisual,
    TraitType,
    property_,
    property_parent_,
    trait_,
)

if TYPE_CHECKING:
    from destack.language import Dimension, Position, Window

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.VIEW)
class IsView(
    HasName,
    IsVisual,
    IsEnvironmental,
    IsScriptable,
    IsTemplatable,
    IsInFolder,
    IsTracked,
    IsDeletable,
):
    """A View is a graphical interface."""

    parent: Union["Window", "IsView", None] = property_parent_(node_is_customizable=True)
    # variant_of, ...

    # sizing
    position: Optional["Position"] = property_(40)
    width: Optional["Dimension"] = property_(41)
    height: Optional["Dimension"] = property_(42)
    min_width: Optional["Dimension"] = property_(43)
    min_height: Optional["Dimension"] = property_(44)
    max_width: Optional["Dimension"] = property_(45)
    max_height: Optional["Dimension"] = property_(46)
