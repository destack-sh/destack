from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    HasIcon,
    HasName,
    HasSlug,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsOwnable,
    IsTemplatable,
    Node,
    NodeType,
    node_,
    property_,
)
from bench.pb2 import SceneData

if TYPE_CHECKING:
    from bench.language import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCENE)
class Scene(
    IsOwnable,
    IsTemplatable,
    HasName,
    IsModal,
    IsBlockable,
    IsArchivable,
    IsDeletable,
    HasSlug,
    HasIcon,
    IsInPackage,
    Node[SceneData],
):
    """A Scene is a container for a specific interaction point."""

    root_view: Optional["IsContainerView"] = property_(
        40, description="The root view of the Scene."
    )
