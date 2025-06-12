from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    HasIcon,
    IsOwnable,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import SceneData

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Folder, Window

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCENE)
class Scene(
    ContainerView,
    HasIcon,
    IsOwnable,
    Node[SceneData],
):
    """A Scene is a container for a specific interaction point."""

    parent: Union["Folder", "Scene", "Window", None] = property_parent_(node_is_customizable=True)

    root_view: Optional["ContainerView"] = property_(100, description="The root view of the Scene.")
