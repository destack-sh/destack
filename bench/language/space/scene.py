from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsIcon,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsSlug,
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
    IsInstantiable,
    IsNamed,
    IsModal,
    IsBlockable,
    IsArchivable,
    IsDeletable,
    IsSlug,
    IsIcon,
    Node[SceneData],
):
    """A Scene is a container for a specific interaction point."""

    root_view: Optional["IsContainerView"] = property_(
        40, description="The root view of the Scene."
    )
