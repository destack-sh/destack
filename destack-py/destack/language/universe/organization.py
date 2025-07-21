from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsActor,
    IsJoinable,
    NodeReference,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Handle, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(Enum):
    CREATING = 1
    ACTIVE = 10


@builtin_node(NodeType.ORGANIZATION)
class Organization(
    IsActor,
    IsJoinable,
    Entity,
):
    """
    An Organization with Users and Teams.
    """

    parent: Optional["Space"] = builtin_property_parent()
    slug: str = builtin_property(101, is_repr=True)
    status: OrganizationStatus = builtin_property(
        102, is_repr=True, default=OrganizationStatus.CREATING
    )
    handle: Optional["Handle"] = builtin_property(111)
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
