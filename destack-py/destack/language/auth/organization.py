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
    IsInvite,
    IsMembership,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import OrganizationData, OrganizationInviteData, OrganizationMembershipData

if TYPE_CHECKING:
    from destack.language import Handle, Space

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(BuiltinEnum):
    CREATING = 1
    ACTIVE = 10


@node_(NodeType.ORGANIZATION, root_type=None)
class Organization(
    IsGlobal,
    IsSubject,
    HasSlug,
    HasIcon,
    HasName,
    Node[OrganizationData],
):
    """
    An Organization with Users and Teams.
    """

    # parent: Organization for nesting?
    status: OrganizationStatus = property_(
        38, can_write="system", is_repr=True, default=OrganizationStatus.CREATING
    )

    space: "Space" = property_(40, can_write="system")
    handle: Optional["Handle"] = property_(41, can_write="system")
    if TYPE_CHECKING:
        space_id: UUID = property_()
        space_ptr: NodeReference = property_()
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None


@enum_(EnumType.ORGANIZATION_ROLE_TYPE)
class OrganizationRoleType(BuiltinEnum):
    ADMIN = 10
    MEMBER = 50


@node_(NodeType.ORGANIZATION_INVITE)
class OrganizationInvite(IsInvite, IsDeletable, Node[OrganizationInviteData]):
    """
    An OrganizationInvite is an invite to an Organization.
    """

    parent: Optional["Organization"] = property_parent_(node_is_customizable=False)

    role_type: OrganizationRoleType = property_(45)


@node_(NodeType.ORGANIZATION_MEMBERSHIP)
class OrganizationMembership(IsMembership, IsDeletable, Node[OrganizationMembershipData]):
    """
    An OrganizationMembership is a membership to an Organization.
    """

    parent: Optional["Organization"] = property_parent_(node_is_customizable=False)

    role_type: OrganizationRoleType = property_(45)
