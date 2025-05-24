from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsDeletable,
    IsGlobal,
    IsIcon,
    IsInvite,
    IsMembership,
    IsNamed,
    IsRegional,
    IsSlug,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import OrganizationData, OrganizationInviteData, OrganizationMembershipData

if TYPE_CHECKING:
    from bench.language import Bench, Handle, TextLine

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(BuiltinEnum):
    ACTIVE = 50


@node_(NodeType.ORGANIZATION, root_type=None)
class Organization(
    IsGlobal,
    IsSubject,
    IsSlug,
    IsIcon,
    IsNamed,
    IsRegional,
    Node[OrganizationData],
):
    """
    An Organization with Users and Teams.
    """

    # parent: Organization for nesting?
    line: Optional["TextLine"] = property_(35)
    status: OrganizationStatus = property_(38, can_write="system")

    bench: Optional["Bench"] = property_(40, can_write="system")
    handle: Optional["Handle"] = property_(41, can_write="system")
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
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

    parent: "Organization" = property_parent_()

    role: OrganizationRoleType = property_(45)


@node_(NodeType.ORGANIZATION_MEMBERSHIP)
class OrganizationMembership(IsMembership, IsDeletable, Node[OrganizationMembershipData]):
    """
    An OrganizationMembership is a membership to an Organization.
    """

    parent: "Organization" = property_parent_()

    role: OrganizationRoleType = property_(45)
