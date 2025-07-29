from collections.abc import Generator
from contextlib import contextmanager
from typing import TYPE_CHECKING, NamedTuple, Optional, final

from destack.language.core import (
    ACTIVE_SPACE,
    BEGINNING_OF_TIME,
    EPSILON,
    VERSION,
    Entity,
    NodeReference,
    NodeType,
    Region,
    TraitType,
    ValueFactory,
    builtin_constant,
    builtin_node,
    builtin_property,
)
from destack.language.registry import (
    ENUM_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)
from destack.utils.uuid import UUID, uuid4

if TYPE_CHECKING:
    from destack.language import (
        Branch,
        Handle,
        NodeReference,
        Session,
        Snapshot,
    )

# pyright: reportIncompatibleVariableOverride=false

_UNIVERSE_SPACE_ID = UUID(int=0)
_UNIVERSE_ACTOR_ID = UUID(int=10)
_UNIVERSE_CLIENT_ID = UUID(int=11)

_META_SNAPSHOT_ID = UUID(int=20)
_META_BRANCH_ID = UUID(int=21)
_HEAD_SNAPSHOT_ID = UUID(int=30)
_ROOT_BRANCH_ID = UUID(int=31)


@builtin_node(
    NodeType.UNIVERSE,
    is_final=True,
)
@final
class Universe(Entity):
    """The Destack computational universe."""

    VERSION = builtin_constant(
        1,
        value=VERSION,
        description="The current version of Destack.",
    )
    EPSILON = builtin_constant(
        2,
        value=EPSILON,
        description="The float epsilon used for floating point comparisons.",
    )
    BEGINNING_OF_TIME = builtin_constant(
        3,
        value=BEGINNING_OF_TIME,
        description="The beginning of time. (1970-01-01T00:00:00+00:00)",
    )

    NODES = builtin_constant(
        10,
        value=lambda: list(NODE_DEFINITION_BY_TYPE.values()),
        description="All Node definitions.",
    )
    STRUCTS = builtin_constant(
        12,
        value=lambda: list(STRUCT_DEFINITION_BY_TYPE.values()),
        description="All Struct definitions.",
    )
    ENUMS = builtin_constant(
        13,
        value=lambda: list(ENUM_DEFINITION_BY_TYPE.values()),
        description="All Enum definitions.",
    )

    SPACE_ID = builtin_constant(
        100,
        value=_UNIVERSE_SPACE_ID,
        description="The system Space ID.",
    )
    SPACE = builtin_constant(
        101,
        description="The system Space.",
        value=lambda: NodeReference(
            type=NodeType.SPACE,
            id=_UNIVERSE_SPACE_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )
    META_SNAPSHOT_ID = builtin_constant(
        102,
        value=_META_SNAPSHOT_ID,
        description="The 'meta' Snapshot.id, the Snapshot containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    META_BRANCH_ID = builtin_constant(
        104,
        value=_META_BRANCH_ID,
        description="The 'meta' Branch.id, the Branch containing time-related Entities (like Snapshots, Branches, etc.)",
    )
    ROOT_BRANCH_ID = builtin_constant(
        106,
        value=_ROOT_BRANCH_ID,
        description="The 'root' Branch.id, the Branch all other Branches originate from.",
    )
    HEAD_SNAPSHOT_ID = builtin_constant(
        108,
        value=_HEAD_SNAPSHOT_ID,
        description="The 'head' Snapshot.id, the current active Snapshot.",
    )

    ACTOR = builtin_constant(
        120,
        description="God himself, the creator of the Universe.",
        value=lambda: NodeReference(
            type=NodeType.ENTITY,
            id=_UNIVERSE_ACTOR_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )
    CLIENT = builtin_constant(
        121,
        description="God's terminal, for when He needs to do something.",
        value=lambda: NodeReference(
            type=NodeType.CLIENT,
            id=_UNIVERSE_CLIENT_ID,
            space_id=_UNIVERSE_SPACE_ID,
            branch_id=_ROOT_BRANCH_ID,
            snapshot_id=_HEAD_SNAPSHOT_ID,
        ),
    )


class CreateSpaceResult(NamedTuple):
    """The result of creating a new Space."""

    space: "Space"
    root_branch: "Branch"
    meta_branch: "Branch"
    meta_snapshot: "Snapshot"
    head_snapshot: "Snapshot"


@builtin_node(
    NodeType.SPACE,
    is_final=True,
    traits=(TraitType.FOLLOWABLE, TraitType.JOINABLE, TraitType.OWNABLE, TraitType.STARABLE),
)
@final
class Space(Entity):
    """
    A Space is the home of your personal software studio.
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

    handle: Optional["Handle"] = builtin_property(111)
    # system_folder, home_folder, ...

    # infra
    region: Region = builtin_property(120)

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
        epoch = session.epoch
        now = session.oracle.now()
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
        space = Space(
            id=space_id,
            space_ptr=space_ptr,
            branch_ptr=root_branch_ptr,
            snapshot_ptr=head_snapshot_ptr,
            name=name,
            slug=slug,
            region=region,
            created_epoch=epoch,
            created_at=now,
            created_by_ptr=owned_by,
            updated_epoch=epoch,
            updated_at=now,
            updated_by_ptr=owned_by,
            owned_by_ptr=owned_by,
        )
        session.create(space)
        session.create(meta_branch)
        session.create(meta_snapshot)
        session.create(root_branch)
        session.create(head_snapshot)
        return CreateSpaceResult(space, root_branch, meta_branch, meta_snapshot, head_snapshot)
