from typing import TYPE_CHECKING, Union

from destack.language.core import (
    HasName,
    Instance,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsVisual,
    Spatial,
    TraitType,
    builtin_trait,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Scene, Theme, View

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.STYLE)
class Style(
    Spatial,
    Instance,
    HasName,
    IsVisual,
    IsOrdered,
    IsTaggable,
    IsDeletable,
):
    """A Style is a style definition."""

    parent: Union["Scene", "View", "Theme", None] = property_parent_(node_is_customizable=True)
