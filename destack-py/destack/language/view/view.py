from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsScriptable,
    IsTaggable,
    IsTemplatable,
    IsTracked,
    IsVisual,
    Spatial,
    TraitType,
    property_,
    property_parent_,
    trait_,
)

if TYPE_CHECKING:
    from destack.language import Dimension, Position, Window

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.VIEW)
class View(
    Spatial,
    Entity,
    HasName,
    IsVisual,
    IsTaggable,
    IsScriptable,
    IsTemplatable,
    IsTracked,
    IsDeletable,
):
    """A View is a graphical interface."""

    parent: Union["Window", "View", None] = property_parent_(node_is_customizable=True)

    # sizing
    position: Optional["Position"] = property_(40)
    width: Optional["Dimension"] = property_(41)
    height: Optional["Dimension"] = property_(42)
    min_width: Optional["Dimension"] = property_(43)
    min_height: Optional["Dimension"] = property_(44)
    max_width: Optional["Dimension"] = property_(45)
    max_height: Optional["Dimension"] = property_(46)
