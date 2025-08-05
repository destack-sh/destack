from typing import TYPE_CHECKING, NamedTuple, final

from ..builtin import (
    ActionType,
    Entity,
    NodeType,
    Region,
    RuntimePlatform,
    TraitType,
    ValueFactory,
    declare_action,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Branch, Snapshot


class CreateSpaceResult(NamedTuple):
    """The result of creating a new Space."""

    space: "Space"
    meta_branch: "Branch"
    meta_snapshot: "Snapshot"
    root_branch: "Branch"
    head_snapshot: "Snapshot"


@declare_entity(
    NodeType.SPACE,
    is_final=True,
    traits=(TraitType.FOLLOWABLE, TraitType.JOINABLE, TraitType.OWNABLE, TraitType.STARABLE),
)
@final
class Space(Entity):
    """
    A Space is the root of a Destack workspace.
    """

    space: "Space" = declare_property(
        5,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.SELF,
        description="The Space this Node is in.",
    )

    slug: str = declare_property(102, is_repr=True)

    # system_folder, home_folder, ...

    # infra
    region: Region = declare_property(120)

    @declare_action(
        101,
        type=ActionType.UNARY_IN_UNARY_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def append(self) -> None:
        """Append Events to the Space."""
        ...

    @declare_action(
        102,
        type=ActionType.UNARY_IN_STREAM_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def watch(self):
        """Watch for Events in the Space."""
        ...
