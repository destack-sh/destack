from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Event,
    HasIcon,
    IsOwnable,
    Node,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import SceneEnteredEventProto, SceneExitedEventProto, SceneProto

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Folder, Window

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCENE_ENTERED_EVENT)
class SceneEnteredEvent(
    Event["Scene"],
    Node[SceneEnteredEventProto],
):
    """A Event regarding a Scene."""

    node: "Scene" = property_(35)


@builtin_node(NodeType.SCENE_EXITED_EVENT)
class SceneExitedEvent(
    Event["Scene"],
    Node[SceneExitedEventProto],
):
    """A Event regarding a Scene."""

    node: "Scene" = property_(35)


@builtin_node(NodeType.SCENE)
class Scene(
    ContainerView,
    HasIcon,
    IsOwnable,
    Node[SceneProto],
):
    """A Scene is a container for a specific interaction point."""

    parent: Union["Folder", "Scene", "Window", None] = property_parent_(node_is_customizable=True)

    root_view: Optional["ContainerView"] = property_(100, description="The root view of the Scene.")
