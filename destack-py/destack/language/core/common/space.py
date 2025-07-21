from collections.abc import Generator
from contextlib import contextmanager
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    ACTIVE_SPACE,
    BEGINNING_OF_TIME,
    EPSILON,
    VERSION,
    Entity,
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsStarable,
    NodeType,
    Region,
    ValueFactory,
    builtin_constant,
    builtin_node,
    builtin_property,
)
from destack.language.registry import (
    ENUM_DEFINITION_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
)
from destack.utils.uuid import UUID, uuid4

if TYPE_CHECKING:
    from destack.language import (
        Branch,
        Handle,
        IsActor,
        NodeReference,
        Session,
        Snapshot,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.UNIVERSE, is_abstract=True)
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
    TRAITS = builtin_constant(
        11,
        value=lambda: list(TRAIT_DEFINITION_BY_TYPE.values()),
        description="All Trait definitions.",
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


@builtin_node(NodeType.SPACE)
class Space(
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsStarable,
    Entity,
):
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


def create_space(
    session: "Session",
    *,
    id: UUID | None = None,
    name: str,
    slug: str,
    owned_by: "IsActor | NodeReference | None" = None,
) -> tuple[Space, "Branch", "Snapshot"]:
    """Create a new Space with a root Branch and Snapshot."""

    from destack.language import (
        REGION,
        Branch,
        BranchType,
        IsActor,
        NodeReference,
        Snapshot,
        SnapshotType,
    )

    if isinstance(owned_by, IsActor):
        owned_by = owned_by.to_ref()

    space_id = id or uuid4()
    epoch = session.epoch
    now = session.oracle.utc()
    space_ptr = NodeReference(
        type=NodeType.SPACE,
        id=space_id,
        space_id=space_id,
    )
    snapshot_id = uuid4()
    snapshot_ptr = NodeReference(
        type=NodeType.SNAPSHOT,
        id=snapshot_id,
        space_id=space_id,
        snapshot_id=snapshot_id,
    )
    branch_id = uuid4()
    branch_ptr = NodeReference(
        type=NodeType.BRANCH,
        id=branch_id,
        space_id=space_id,
    )
    snapshot = Snapshot(
        id=snapshot_id,
        name="Root",
        space_ptr=space_ptr,
        created_epoch=epoch,
        created_at=now,
        updated_epoch=epoch,
        updated_at=now,
        type=SnapshotType.FULL,
        branch_ptr=branch_ptr,
    )
    branch = Branch(
        id=branch_id,
        name="Main",
        space_ptr=space_ptr,
        created_epoch=epoch,
        created_at=now,
        updated_epoch=epoch,
        updated_at=now,
        type=BranchType.ROOT,
        branch_ptr=branch_ptr,
        snapshot_ptr=snapshot_ptr,
    )
    space = Space(
        id=space_id,
        name=name,
        slug=slug,
        region=REGION,
        branch_ptr=branch_ptr,
        snapshot_ptr=snapshot_ptr,
        created_epoch=epoch,
        created_at=now,
        updated_epoch=epoch,
        updated_at=now,
        owned_by_ptr=owned_by,
    )
    session.create(space)
    session.create(snapshot)
    session.create(branch)
    return space, branch, snapshot
