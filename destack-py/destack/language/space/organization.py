from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsJoinable,
    IsOwner,
    Node,
    NodeReference,
    NodeType,
    RoleType,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.proto import OrganizationProto

if TYPE_CHECKING:
    from destack.language import Handle, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(Enum):
    CREATING = 1
    ACTIVE = 10


@builtin_node(NodeType.ORGANIZATION, root_type=None)
class Organization(
    Global,
    Entity,
    HasSlug,
    HasIcon,
    HasName,
    IsOwner,
    IsJoinable,
    Node[OrganizationProto],
):
    """
    An Organization with Users and Teams.
    """

    slug: str = property_(33, is_repr=True)
    status: OrganizationStatus = property_(
        40, can_write=RoleType.SYSTEM, is_repr=True, default=OrganizationStatus.CREATING
    )
    space: "Space" = property_(50, can_write=RoleType.SYSTEM)
    handle: Optional["Handle"] = property_(51, can_write=RoleType.SYSTEM)
    if TYPE_CHECKING:
        space_ptr: NodeReference = property_()
        handle_ptr: Optional[NodeReference] = None
