from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsOrdered,
    IsScriptable,
    IsTaggable,
    IsTemplatable,
    IsVisual,
    Spatial,
    TraitType,
    builtin_trait,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import ContainerView, Dimension, Layer, Position, Scene, Window

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.VIEW)
class View(
    Spatial,
    Entity,
    HasName,
    IsVisual,
    IsOrdered,
    IsTaggable,
    IsScriptable,
    IsTemplatable,
    IsDeletable,
):
    """A View is a graphical interface."""

    parent: Union["Window", "Scene", "Layer", "ContainerView", None] = property_parent_(
        node_is_customizable=True
    )

    # sizing
    position: Optional["Position"] = property_(40)
    width: Optional["Dimension"] = property_(41)
    height: Optional["Dimension"] = property_(42)
    min_width: Optional["Dimension"] = property_(43)
    min_height: Optional["Dimension"] = property_(44)
    max_width: Optional["Dimension"] = property_(45)
    max_height: Optional["Dimension"] = property_(46)
