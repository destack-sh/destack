from typing import (
    TYPE_CHECKING,
    final,
)

from ..builtin import (
    Entity,
    EnumType,
    NodeType,
    OptionEnum,
    TraitType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    pass


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

    type: SnapshotType = declare_property(100)
    status: SnapshotStatus = declare_property(110, default=SnapshotStatus.ACTIVE)
