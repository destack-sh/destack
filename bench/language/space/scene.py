from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    VIEW_NODE_TYPES,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    NodeType,
    PageNode,
    node_,
    p_regular,
)
from bench.pb2 import SceneData

if TYPE_CHECKING:
    from bench.language import ContainerViewBase

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCENE)
class Scene(IsOwnable, IsInstantiable, IsNamed, IsModal, PageNode[SceneData]):
    """A Scene is a container for a specific interaction point."""

    root_view: Optional["ContainerViewBase"] = p_regular(
        40,
        array=False,
        require=False,
        references=VIEW_NODE_TYPES.tuple,
        description="The root view of the Scene.",
    )
