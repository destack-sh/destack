from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IsEntity,
    IsGlobal,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    enum_,
    node_,
    property_,
)
from destack.pb2 import (
    OrganizationData,
)

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
    IsEntity,
    IsSubject,
    HasSlug,
    HasIcon,
    HasName,
    Node[OrganizationData],
):
    """
    An Organization with Users and Teams.
    """

    # meta
    slug: str = property_(33, is_repr=True)
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
