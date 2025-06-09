from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsGlobal,
    IsInSpace,
    IsInvite,
    IsMembership,
    IsOwnable,
    IsTracked,
    Node,
    NodeType,
    Region,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import SpaceData, SpaceInviteData, SpaceMembershipData

if TYPE_CHECKING:
    from destack.language import Database, Folder, Handle, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SPACE_STATUS)
class SpaceStatus(BuiltinEnum):
    """The status of a Space"""

    CREATING = 1
    QUEUED = 3
    RUNNING = 10
    PAUSED = 20


@node_(NodeType.SPACE, root_type=None)
class Space(
    HasName,
    HasSlug,
    HasIcon,
    IsTracked,
    IsGlobal,
    IsOwnable,
    IsInSpace,
    Node[SpaceData],
):
    """
    A Space is the home of your personal software studio.
    """

    # meta
    status: SpaceStatus = property_(40, is_repr=True, can_write="system")
    handle: Optional["Handle"] = property_(41, node_space_from="self", can_write="system")
    root_folder: Optional["Folder"] = property_(
        42, node_space_from="self", can_write="system", description="The root Folder."
    )
    home_folder: Optional["Folder"] = property_(
        43, node_space_from="self", can_write="system", description="The home Folder."
    )
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        root_folder_ptr: Optional[NodeReference] = None
        root_folder_id: Optional[UUID] = None
        home_folder_ptr: Optional[NodeReference] = None
        home_folder_id: Optional[UUID] = None

    # infra
    region: Region = property_(50, can_write="system")
    cell_name: str | None = property_(51, can_write="system")  # -> Cell?
    database: Optional["Database"] = property_(55, node_space_from="self", can_write="system")
    # search, analytics, vault, cache, ...
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None
        database_id: Optional[UUID] = None


@enum_(EnumType.SPACE_ROLE_TYPE)
class SpaceRoleType(BuiltinEnum):
    """The role of a Space"""

    ADMIN = 1
    DEVELOPER = 3
    USER = 5
    SPECTATOR = 10


@node_(NodeType.SPACE_INVITE)
class SpaceInvite(
    IsGlobal,
    IsInvite,
    IsDeletable,
    IsInSpace,
    Node[SpaceInviteData],
):
    """
    A SpaceInvite is an invite to a Space.
    """

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)

    role_type: SpaceRoleType = property_(45, is_repr=True)


@node_(NodeType.SPACE_MEMBERSHIP)
class SpaceMembership(
    IsGlobal,
    IsMembership,
    IsDeletable,
    IsInSpace,
    Node[SpaceMembershipData],
):
    """
    A SpaceMembership is a membership to a Space.
    """

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)

    role_type: SpaceRoleType = property_(45, is_repr=True)
