from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IsGlobal,
    IsJoinable,
    IsOwner,
    NodeReference,
    NodeType,
    RoleType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Handle, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(Enum):
    CREATING = 1
    ACTIVE = 10


@builtin_node(NodeType.ORGANIZATION, root_type=None)
class Organization(IsGlobal, HasSlug, HasIcon, HasName, IsOwner, IsJoinable, Entity):
    """
    An Organization with Users and Teams.
    """

    slug: str = builtin_property(33, is_repr=True)
    status: OrganizationStatus = builtin_property(
        40, can_write=RoleType.SYSTEM, is_repr=True, default=OrganizationStatus.CREATING
    )
    space: "Space" = builtin_property(50, can_write=RoleType.SYSTEM)
    handle: Optional["Handle"] = builtin_property(51, can_write=RoleType.SYSTEM)
    if TYPE_CHECKING:
        space_ptr: NodeReference = builtin_property()
        handle_ptr: Optional[NodeReference] = None
