from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsScriptable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import (
        ContainerView,
        Dimension,
        Folder,
        Layer,
        Position,
        Scene,
        View,
        Window,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.VIEW, is_abstract=True)
class View(
    IsSpatial,
    Entity,
    HasName,
    IsOrdered,
    IsTaggable,
    IsScriptable,
    IsExtensible,
    IsDeletable,
):
    """A View is a graphical interface."""

    parent: Union["Window", "Scene", "Layer", "ContainerView", "Folder", None] = property_parent_(
        node_is_extensible=True
    )

    # sizing
    position: Optional["Position"] = property_(40)
    width: Optional["Dimension"] = property_(41)
    height: Optional["Dimension"] = property_(42)
    min_width: Optional["Dimension"] = property_(43)
    min_height: Optional["Dimension"] = property_(44)
    max_width: Optional["Dimension"] = property_(45)
    max_height: Optional["Dimension"] = property_(46)
