from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsJoinable,
    IsSubject,
    LocalNodeList,
    Node,
    NodeReference,
    NodeType,
    Region,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_regular,
    p_system,
)
from bench.pb2 import OrganizationData

if TYPE_CHECKING:
    from bench.language import Bench, Handle, Icon, Membership, Text

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(BuiltinEnum):
    REGISTERED = 20  # created org
    ACTIVATED = 50  # has main bench


@node_(NodeType.ORGANIZATION, roots=())
class Organization(IsJoinable, IsSubject, Node[OrganizationData]):
    """
    An Organization with Users and Teams.
    """

    # parent: Organization for nesting?
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    region: "Region" = p_system(37)
    status: OrganizationStatus = p_system(38)

    main_bench: Optional["Bench"] = p_system(
        40, array=False, require=False, references=NodeType.BENCH, fk=True
    )
    main_handle: Optional["Handle"] = p_system(
        41, require=False, array=False, references=NodeType.HANDLE, fk=True
    )
    if TYPE_CHECKING:
        main_bench_id: Optional[UUID] = None
        main_bench_ptr: Optional[NodeReference] = None
        main_handle_id: Optional[UUID] = None
        main_handle_ptr: Optional[NodeReference] = None

    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)
