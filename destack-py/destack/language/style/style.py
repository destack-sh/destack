from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsTemplatable,
    IsVisual,
    Spatial,
    TraitType,
    builtin_trait,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Scene, View

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.STYLE)
class Style(
    Spatial,
    Entity,
    HasName,
    IsVisual,
    IsOrdered,
    IsTaggable,
    IsTemplatable,
    IsDeletable,
):
    """A Style is a graphical interface."""

    parent: Union["Scene", "View", None] = property_parent_(node_is_customizable=True)
