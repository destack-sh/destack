from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Enum,
    EnumType,
    Event,
    HasIcon,
    IsOwnable,
    Node,
    NodeType,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import SceneEventProto, SceneProto

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Folder, Window

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SCENE_EVENT_TYPE)
class SceneEventType(Enum):
    """A Type of Scene Event."""

    ENTERED = 1, "Entered", "Entered the Scene", "fas fa-circle"
    EXITED = 2, "Exited", "Exited the Scene", "fas fa-circle"


@builtin_node(NodeType.SCENE_EVENT)
class SceneEvent(
    Event["Scene"],
    Node[SceneEventProto],
):
    """A Event regarding a Scene."""

    type: SceneEventType = property_(30)
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
