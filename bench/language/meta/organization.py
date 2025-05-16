from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsSubject,
    LocalNodeList,
    Node,
    NodeReference,
    NodeType,
    Region,
    enum_,
    node_,
    p_node_children,
    p_regular,
    p_system,
)
from bench.pb2 import OrganizationData

if TYPE_CHECKING:
    from bench.language import Bench, Handle, Icon, Membership, TextLine

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(BuiltinEnum):
    REGISTERED = 20  # created org
    ACTIVATED = 50  # has main bench


@node_(NodeType.ORGANIZATION, roots=())
class Organization(IsSubject, Node[OrganizationData]):
    """
    An Organization with Users and Teams.
    """

    # parent: Organization for nesting?
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    icon: Optional["Icon"] = p_regular(35)
    line: Optional["TextLine"] = p_regular(34)
    region: "Region" = p_system(37)
    status: OrganizationStatus = p_system(38)

    bench: Optional["Bench"] = p_system(40, fk=True)
    handle: Optional["Handle"] = p_system(41, fk=True)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None

    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)
