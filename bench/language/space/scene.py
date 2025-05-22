from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    Node,
    NodeType,
    node_,
    p_regular,
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
    Node[SceneData],
):
    """A Scene is a container for a specific interaction point."""

    root_view: Optional["IsContainerView"] = p_regular(
        40, description="The root view of the Scene."
    )
