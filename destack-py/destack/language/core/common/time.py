from collections.abc import Generator
from contextlib import contextmanager
from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    final,
)

from ..builtin import (
    ACTIVE_BRANCH,
    ACTIVE_SNAPSHOT,
    Entity,
    Enum,
    EnumType,
    IsOwnable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.BRANCH_TYPE)
class BranchType(Enum):
    """The type of a Branch."""

    ROOT = 1
    FULL = 2


@builtin_node(NodeType.BRANCH, is_final=True)
@final
class Branch(
    IsOwnable,
    Entity,
):
    """
    A Branch is a version of a Snapshot.

    """

    parent: Optional["Space"] = builtin_property_parent()
    type: BranchType = builtin_property(100)

    @contextmanager
    def active(self) -> Generator[None, None, None]:
        """
        Context manager to set the active Branch to this Branch.
        """
        token = ACTIVE_BRANCH.set(self)
        try:
            yield
        finally:
            ACTIVE_BRANCH.reset(token)


@builtin_enum(EnumType.SNAPSHOT_TYPE)
class SnapshotType(Enum):
    """The type of a Snapshot."""

    ROOT = 1
    FULL = 2


@builtin_enum(EnumType.SNAPSHOT_STATUS)
class SnapshotStatus(Enum):
    """The status of a Snapshot."""

    CREATING = 1, "Creating", "Under construction"
    ACTIVE = 10, "Active", "Live and editable"
    PASSIVE = 50, "Passive", "Inactive and read-only"


@builtin_node(NodeType.SNAPSHOT, is_final=True)
@final
class Snapshot(
    IsOwnable,
    Entity,
):
    """
    A Snapshot is a point in Space-time.
    Snapshots may branch off of other Snapshots, either as a full copy or a partial override.

    To avoid breaking the universe, Snapshots cannot themselves be part of any other Snapshot.
     (Technically, Snapshots are part of themselves.)
    """

    parent: Union["Space", None] = builtin_property_parent(is_readonly=True)

    type: SnapshotType = builtin_property(100)
    status: SnapshotStatus = builtin_property(110, default=SnapshotStatus.ACTIVE)

    @contextmanager
    def active(self) -> Generator[None, None, None]:
        """
        Context manager to set the active Snapshot to this Snapshot.
        """
        token = ACTIVE_SNAPSHOT.set(self)
        try:
            yield
        finally:
            ACTIVE_SNAPSHOT.reset(token)
