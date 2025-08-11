from typing import TYPE_CHECKING, NamedTuple, final

from ..builtin import (
    ActionType,
    Entity,
    NodeType,
    Region,
    RuntimePlatform,
    TraitType,
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
    A Space is the root of an isolated "workspace" in the Destack computational universe.

    Every Space has its own logical time called "epoch".
    Epochs are monotonic integers that are incremented by 1 for each Event in the Space.
    Every moment/state in Spacetime therefore has a unique identifier (space @ epoch).

    NOTE: Epochs are managed as 64-bit unsigned integers but can migrate if they grow too large.
     (64 bits are sufficient for 1B Events per second per Space for over 500 years.)
    """

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
        raise NotImplementedError

    @declare_action(
        102,
        type=ActionType.UNARY_IN_STREAM_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def watch(self):
        """Watch for Events in the Space."""
        raise NotImplementedError
