from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    IsExtensible,
    IsOrdered,
    IsOwnable,
    NodeType,
    builtin_node,
    builtin_property,
)

from ..view import LayoutView

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCENE_EVENT, frozen=True, is_abstract=True)
class SceneEvent(Event["Scene"]):
    """A Event regarding a Scene."""

    node: "Scene" = builtin_property(101)


@builtin_node(
    NodeType.SCENE,
    event_types=(NodeType.SCENE_EVENT,),
    expected_ancestor_types=(NodeType.STAGE,),
)
class Scene(
    IsOwnable,
    IsOrdered,
    IsExtensible,
    Entity,
):
    """A Scene is a container for an interaction point."""

    root_view: Optional["LayoutView"] = builtin_property(
        200, description="The root view of the Scene."
    )
    icon: "Icon | None" = builtin_property(102)
