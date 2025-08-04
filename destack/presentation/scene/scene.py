from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    Event,
    NodeType,
    TraitType,
    declare_entity,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack.presentation import LayoutView


@declare_event(NodeType.SCENE_EVENT, is_abstract=True)
class SceneEvent(Event):
    """A Event regarding a Scene."""

    scene: "Scene" = declare_property(101)


@declare_entity(
    NodeType.SCENE,
    traits=(TraitType.OWNABLE, TraitType.ORDERED),
    event_types=(NodeType.SCENE_EVENT,),
    expected_ancestor_types=(NodeType.STAGE,),
)
class Scene(Entity):
    """A Scene contains some interactive part of a Stage."""

    root_view: Optional["LayoutView"] = declare_property(
        200, description="The root view of the Scene."
    )
