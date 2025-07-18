from collections.abc import Generator
from contextlib import contextmanager
from typing import (
    TYPE_CHECKING,
    Optional,
    Self,
    Union,
)

from ..builtin import (
    ACTIVE_SNAPSHOT,
    Entity,
    Enum,
    EnumType,
    IsOwnable,
    NodeType,
    ValueFactory,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.BRANCH)
class Branch(
    IsOwnable,
    Entity,
):
    """A Branch is a version of a Snapshot."""

    parent: Optional["Space"] = builtin_property_parent()


@builtin_enum(EnumType.SNAPSHOT_TYPE)
class SnapshotType(Enum):
    """The type of a Snapshot."""

    PARTIAL = 1
    FULL = 10
    ROOT = 11


@builtin_enum(EnumType.SNAPSHOT_STATUS)
class SnapshotStatus(Enum):
    """The status of a Snapshot."""

    CREATING = 1, "Creating", "Under construction"
    ACTIVE = 10, "Active", "Live and editable"
    PASSIVE = 50, "Passive", "Inactive and read-only"


@builtin_node(NodeType.SNAPSHOT)
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

    snapshot: "Snapshot" = builtin_property(
        12,
        is_readonly=True,
        is_managed=True,
        default_factory=ValueFactory.SELF,
        description="The Snapshot itself. Cannot be any other Snapshot than this Snapshot",
    )

    type: SnapshotType = builtin_property(100)
    status: SnapshotStatus = builtin_property(110, default=SnapshotStatus.ACTIVE)

    def into(self, snapshot: "Snapshot") -> "Self":
        if snapshot.id == self.id:
            return self
        else:
            raise RuntimeError(f"cannot turn {self!r} into another Snapshot ({snapshot!r})")

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
