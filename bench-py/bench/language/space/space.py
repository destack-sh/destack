from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
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
from bench.pb2 import SpaceData, SpaceInviteData, SpaceMembershipData

if TYPE_CHECKING:
    from bench.language import Database, Handle, NodeReference, Package

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
    A Space is the OS for personal software.
    """

    # meta
    status: SpaceStatus = property_(40, is_repr=True, can_write="system")
    handle: Optional["Handle"] = property_(41, node_space_from="self", can_write="system")
    main_package: Optional["Package"] = property_(
        42, node_space_from="self", can_write="system", description="The Main Package."
    )
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        main_package_ptr: Optional[NodeReference] = None
        main_package_id: Optional[UUID] = None

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

    ADMIN = 10
    MEMBER = 50


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

    parent: Optional["Space"] = property_parent_()

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

    parent: Optional["Space"] = property_parent_()

    role_type: SpaceRoleType = property_(45, is_repr=True)
