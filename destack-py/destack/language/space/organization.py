from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

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
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import OrganizationData

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
    Node[OrganizationData],
):
    """
    An Organization with Users and Teams.
    """

    slug: str = property_(33, is_repr=True)
    status: OrganizationStatus = property_(
        40, can_write="system", is_repr=True, default=OrganizationStatus.CREATING
    )
    space: "Space" = property_(50, can_write="system")
    handle: Optional["Handle"] = property_(51, can_write="system")
    if TYPE_CHECKING:
        space_id: UUID = property_()
        space_ptr: NodeReference = property_()
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None
