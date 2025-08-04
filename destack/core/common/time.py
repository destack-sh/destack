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
    EnumType,
    NodeType,
    OptionEnum,
    TraitType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack import Space

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.BRANCH_TYPE)
class BranchType(OptionEnum):
    """The type of a Branch."""

    PARTIAL = declare_option(2)
    FULL = declare_option(10)
    ROOT = declare_option(11)


@declare_entity(
    NodeType.BRANCH,
    is_final=True,
    traits=(TraitType.OWNABLE,),
)
@final
class Branch(Entity):
    """
    A Branch is a version of a Snapshot.

    """

    parent: Optional["Space"] = declare_property_parent()
    type: BranchType = declare_property(100)

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


@declare_enum(EnumType.SNAPSHOT_TYPE)
class SnapshotType(OptionEnum):
    """The type of a Snapshot."""

    FULL = declare_option(10)
    ROOT = declare_option(11)


@declare_enum(EnumType.SNAPSHOT_STATUS)
class SnapshotStatus(OptionEnum):
    """The status of a Snapshot."""

    CREATING = declare_option(1, description="Under construction")
    ACTIVE = declare_option(10, description="Live and editable")
    PASSIVE = declare_option(50, description="Inactive and read-only")


@declare_entity(
    NodeType.SNAPSHOT,
    is_final=True,
    traits=(TraitType.OWNABLE,),
)
@final
class Snapshot(Entity):
    """
    A Snapshot is a point in Space-time.
    Snapshots may branch off of other Snapshots, either as a full copy or a partial override.

    To avoid breaking the universe, Snapshots cannot themselves be part of any other Snapshot.
     (Technically, Snapshots are part of themselves.)
    """

    parent: Union["Space", None] = declare_property_parent(is_readonly=True)

    type: SnapshotType = declare_property(100)
    status: SnapshotStatus = declare_property(110, default=SnapshotStatus.ACTIVE)

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
