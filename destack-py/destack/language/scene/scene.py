from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    HasIcon,
    HasName,
    IsDeletable,
    IsOwnable,
    IsScriptable,
    IsSpatial,
    IsTaggable,
    IsTemplatable,
    IsTracked,
    IsVisual,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import SceneData

if TYPE_CHECKING:
    from destack.language import Folder, IsContainerView, Window

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCENE)
class Scene(
    HasName,
    HasIcon,
    IsVisual,
    IsTaggable,
    IsScriptable,
    IsOwnable,
    IsTemplatable,
    IsDeletable,
    IsTracked,
    IsSpatial,
    Node[SceneData],
):
    """A Scene is a container for a specific interaction point."""

    parent: Union["Folder", "Scene", "Window", None] = property_parent_(node_is_customizable=True)
    root_view: Optional["IsContainerView"] = property_(
        40, description="The root view of the Scene."
    )
