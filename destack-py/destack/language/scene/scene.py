from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

from ..view import LayoutView

if TYPE_CHECKING:
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.SCENE_EVENT, is_abstract=True)
class SceneEvent(Event["Scene"]):
    """A Event regarding a Scene."""

    node: "Scene" = builtin_property(101)


@builtin_entity(
    NodeType.SCENE,
    traits=(TraitType.OWNABLE, TraitType.ORDERED),
    event_types=(NodeType.SCENE_EVENT,),
    expected_ancestor_types=(NodeType.STAGE,),
)
class Scene(Entity):
    """A Scene contains some interactive part of a Stage."""

    root_view: Optional["LayoutView"] = builtin_property(
        200, description="The root view of the Scene."
    )
    icon: "Icon | None" = builtin_property(102)
