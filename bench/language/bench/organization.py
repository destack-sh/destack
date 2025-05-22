from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsDeletable,
    IsGlobal,
    IsInvite,
    IsMembership,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    Region,
    StringFormat,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import OrganizationData, OrganizationInviteData, OrganizationMembershipData

if TYPE_CHECKING:
    from bench.language import Bench, Handle, Icon, TextLine

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(BuiltinEnum):
    REGISTERED = 20  # created org
    ACTIVATED = 50  # has main bench


@node_(NodeType.ORGANIZATION, root_type=None)
class Organization(IsGlobal, IsSubject, Node[OrganizationData]):
    """
    An Organization with Users and Teams.
    """

    # parent: Organization for nesting?
    slug: Optional[str] = p_system(
        32, unique=True, format=StringFormat.SLUG
    )  # must match main handle
    name: str = p_regular(33, format=StringFormat.NAME)
    icon: Optional["Icon"] = p_regular(35)
    line: Optional["TextLine"] = p_regular(34)
    region: "Region" = p_system(37)
    status: OrganizationStatus = p_system(38)

    bench: Optional["Bench"] = p_system(40)
    handle: Optional["Handle"] = p_system(41)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None


@node_(NodeType.ORGANIZATION_INVITE)
class OrganizationInvite(IsInvite, IsDeletable, Node[OrganizationInviteData]):
    """
    An OrganizationInvite is an invite to an Organization.
    """

    parent: "Organization" = p_node_parent()


@node_(NodeType.ORGANIZATION_MEMBERSHIP)
class OrganizationMembership(IsMembership, IsDeletable, Node[OrganizationMembershipData]):
    """
    An OrganizationMembership is a membership to an Organization.
    """

    parent: "Organization" = p_node_parent()
