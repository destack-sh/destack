from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    HasIcon,
    HasName,
    HasSlug,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsEnvironmental,
    IsInPackage,
    IsOwnable,
    IsScriptable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import SceneData

if TYPE_CHECKING:
    from destack.language import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCENE)
class Scene(
    HasName,
    IsEnvironmental,
    HasSlug,
    HasIcon,
    IsScriptable,
    IsOwnable,
    IsTemplatable,
    IsBlockable,
    IsArchivable,
    IsDeletable,
    IsTracked,
    IsInPackage,
    Node[SceneData],
):
    """A Scene is a container for a specific interaction point."""

    root_view: Optional["IsContainerView"] = property_(
        40, description="The root view of the Scene."
    )
