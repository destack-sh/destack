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
    builtin_property,
    builtin_property_parent,
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

    parent: Union["Window", "Scene", "Layer", "ContainerView", "Folder", None] = (
        builtin_property_parent(node_is_extensible=True)
    )

    # sizing
    position: Optional["Position"] = builtin_property(40)
    width: Optional["Dimension"] = builtin_property(41)
    height: Optional["Dimension"] = builtin_property(42)
    min_width: Optional["Dimension"] = builtin_property(43)
    min_height: Optional["Dimension"] = builtin_property(44)
    max_width: Optional["Dimension"] = builtin_property(45)
    max_height: Optional["Dimension"] = builtin_property(46)
