from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Event,
    HasIcon,
    IsOwnable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Folder, Window

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCENE_EVENT, is_abstract=True)
class SceneEvent(Event["Scene"]):
    """A Event regarding a Scene."""

    node: "Scene" = builtin_property(35)


@builtin_node(NodeType.SCENE_ENTERED_EVENT)
class SceneEnteredEvent(SceneEvent):
    """A Scene was entered."""

    node: "Scene" = builtin_property(35)


@builtin_node(NodeType.SCENE_EXITED_EVENT)
class SceneExitedEvent(SceneEvent):
    """A Scene was exited."""

    node: "Scene" = builtin_property(35)


@builtin_node(NodeType.SCENE)
class Scene(HasIcon, IsOwnable, ContainerView):
    """A Scene is a container for a specific interaction point."""

    parent: Union["Folder", "Scene", "Window", None] = builtin_property_parent(
        node_is_extensible=True
    )
    root_view: Optional["ContainerView"] = builtin_property(
        100, description="The root view of the Scene."
    )
