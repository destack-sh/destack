from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Event,
    IsExtensible,
    IsOrdered,
    IsOwnable,
    IsViewable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Folder, Icon, Window

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCENE_EVENT, frozen=True, is_abstract=True)
class SceneEvent(Event["Scene"]):
    """A Event regarding a Scene."""

    node: "Scene" = builtin_property(101)


@builtin_node(NodeType.SCENE, event_types=())
class Scene(
    IsViewable,
    IsOwnable,
    IsOrdered,
    IsExtensible,
    Entity,
):
    """A Scene is a container for an interaction point."""

    parent: Union["Folder", "Window", None] = builtin_property_parent()
    root_view: Optional["ContainerView"] = builtin_property(
        200, description="The root view of the Scene."
    )
    icon: "Icon | None" = builtin_property(102)
