from collections.abc import AsyncGenerator, Generator
from contextlib import contextmanager
from typing import TYPE_CHECKING, NamedTuple, Optional, final

from destack.language.core import (
    ACTIVE_SPACE,
    ActionType,
    Entity,
    NodeReference,
    NodeType,
    Region,
    RuntimePlatform,
    TraitType,
    ValueFactory,
    builtin_action,
    builtin_entity,
    builtin_property,
)
from destack.language.universe.universe import Universe
from destack.utils.uuid import UUID, uuid4

if TYPE_CHECKING:
    from destack.language import Branch, Event, NodeReference, Session, Snapshot

# pyright: reportIncompatibleVariableOverride=false


class CreateSpaceResult(NamedTuple):
    """The result of creating a new Space."""

    space: "Space"
    meta_branch: "Branch"
    meta_snapshot: "Snapshot"
    root_branch: "Branch"
    head_snapshot: "Snapshot"


@builtin_entity(
    NodeType.SPACE,
    is_final=True,
    traits=(TraitType.FOLLOWABLE, TraitType.JOINABLE, TraitType.OWNABLE, TraitType.STARABLE),
)
@final
class Space(Entity):
    """
    A Space is the root of a Destack workspace.
    """

    space: "Space" = builtin_property(
        5,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.SELF,
        description="The Space this Node is in.",
    )
    if TYPE_CHECKING:
        space_ptr: Optional[NodeReference] = None

    slug: str = builtin_property(102, is_repr=True)

    # system_folder, home_folder, ...

    # infra
    region: Region = builtin_property(120)

    @builtin_action(
        101,
        type=ActionType.UNARY_IN_UNARY_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def append(
        self,
        events: "list[Event]",
    ) -> None:
        """Append Events to the Space."""
        ...

    @builtin_action(
        102,
        type=ActionType.UNARY_IN_STREAM_OUT,
        platforms=(RuntimePlatform.SYSTEM,),
    )
    async def watch(
        self,
    ) -> AsyncGenerator[list["Event"], None]:
        """Watch for Events in the Space."""
        ...

    @contextmanager
    def active(self: "Space") -> Generator["Space", None, None]:
        """Set this Space as the active Space."""
        token = ACTIVE_SPACE.set(self)
        try:
            ACTIVE_SPACE.set(self)
            yield self
        finally:
            ACTIVE_SPACE.reset(token)

    @staticmethod
    def create_space(
        session: "Session",
        *,
        id: UUID | None = None,
        name: str,
        region: Region,
        slug: str,
        owned_by: "Entity | NodeReference",
    ) -> CreateSpaceResult:
        """
        Create a new Space with a root Branch, meta Snapshot and head Snapshot.
        NOTE: this requires a bit of a dance because of the circular dependencies.
        """

        from destack.language import (
            Branch,
            BranchType,
            Entity,
            NodeReference,
            Snapshot,
            SnapshotType,
        )

        if isinstance(owned_by, Entity):
            owned_by = owned_by.to_ref()

        space_id = id or uuid4()
        epoch = session.remote_epoch
        now = session.context.now()
        space_ptr = NodeReference(
            type=NodeType.SPACE,
            id=space_id,
            space_id=space_id,
            branch_id=Universe.ROOT_BRANCH_ID,
            snapshot_id=Universe.META_SNAPSHOT_ID,
        )

        # Snapshots/Branches live in the meta Branch/Snapshot
        meta_branch_ptr = NodeReference(
            type=NodeType.BRANCH,
            id=Universe.META_BRANCH_ID,
            space_id=space_id,
            branch_id=Universe.META_BRANCH_ID,
            snapshot_id=Universe.META_SNAPSHOT_ID,
        )
        meta_snapshot_ptr = NodeReference(
            type=NodeType.SNAPSHOT,
            id=Universe.META_SNAPSHOT_ID,
            space_id=space_id,
            branch_id=Universe.META_BRANCH_ID,
            snapshot_id=Universe.META_SNAPSHOT_ID,
        )
        root_branch_ptr = NodeReference(
            type=NodeType.BRANCH,
            id=Universe.ROOT_BRANCH_ID,
            space_id=space_id,
            branch_id=Universe.ROOT_BRANCH_ID,
            snapshot_id=Universe.META_SNAPSHOT_ID,
        )
        head_snapshot_ptr = NodeReference(
            type=NodeType.SNAPSHOT,
            id=Universe.HEAD_SNAPSHOT_ID,
            space_id=space_id,
            branch_id=Universe.META_BRANCH_ID,
            snapshot_id=Universe.META_BRANCH_ID,
        )
        meta_branch = Branch(
            id=Universe.META_BRANCH_ID,
            type=BranchType.ROOT,
            name="Meta",
            space_ptr=space_ptr,
            snapshot_ptr=meta_snapshot_ptr,
            branch_ptr=meta_branch_ptr,
            created_epoch=epoch,
            created_at=now,
            created_by_ptr=owned_by,
            updated_epoch=epoch,
            updated_at=now,
            updated_by_ptr=owned_by,
        )
        meta_snapshot = Snapshot(
            id=Universe.META_SNAPSHOT_ID,
            name="Meta",
            space_ptr=space_ptr,
            branch_ptr=meta_branch_ptr,
            snapshot_ptr=meta_snapshot_ptr,
            created_epoch=epoch,
            created_at=now,
            created_by_ptr=owned_by,
            updated_epoch=epoch,
            updated_at=now,
            updated_by_ptr=owned_by,
            type=SnapshotType.FULL,
        )
        head_snapshot = Snapshot(
            id=Universe.HEAD_SNAPSHOT_ID,
            name="Head",
            space_ptr=space_ptr,
            branch_ptr=meta_branch_ptr,
            snapshot_ptr=meta_snapshot_ptr,
            created_epoch=epoch,
            created_at=now,
            created_by_ptr=owned_by,
            updated_epoch=epoch,
            updated_at=now,
            updated_by_ptr=owned_by,
            type=SnapshotType.FULL,
        )
        root_branch = Branch(
            id=Universe.ROOT_BRANCH_ID,
            name="Root",
            space_ptr=space_ptr,
            snapshot_ptr=meta_snapshot_ptr,
            created_epoch=epoch,
            created_at=now,
            created_by_ptr=owned_by,
            updated_epoch=epoch,
            updated_at=now,
            updated_by_ptr=owned_by,
            type=BranchType.ROOT,
            branch_ptr=meta_branch_ptr,
        )

        # Space lives in the root Branch at head Snapshot
        # NOTE :Cleanup: I have no idea why pyright hates this specific Space init
        space = Space(
            id=space_id,
            space_ptr=space_ptr,
            branch_ptr=root_branch_ptr,  # type: ignore
            snapshot_ptr=head_snapshot_ptr,  # type: ignore
            name=name,  # type: ignore
            slug=slug,
            region=region,
            created_epoch=epoch,  # type: ignore
            created_at=now,  # type: ignore
            created_by_ptr=owned_by,  # type: ignore
            updated_epoch=epoch,  # type: ignore
            updated_at=now,  # type: ignore
            updated_by_ptr=owned_by,  # type: ignore
            owned_by_ptr=owned_by,  # type: ignore
        )
        session.create(space)
        session.create(meta_branch)
        session.create(meta_snapshot)
        session.create(root_branch)
        session.create(head_snapshot)
        return CreateSpaceResult(
            space=space,
            meta_branch=meta_branch,
            meta_snapshot=meta_snapshot,
            root_branch=root_branch,
            head_snapshot=head_snapshot,
        )
