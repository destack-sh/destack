from typing import Optional

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsJoinable,
    IsOrdered,
    Node,
    NodeType,
    enum_,
    node_,
    property_parent_,
)
from destack.pb2 import RoleData

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ROLE_TYPE)
class RoleType(BuiltinEnum):
    """The role of a Role"""

    ADMIN = 1
    DEVELOPER = 4
    USER = 7
    SPECTATOR = 10


@node_(NodeType.ROLE)
class Role(
    HasSlug,
    HasIcon,
    HasName,
    Global,
    Entity,
    IsOrdered,
    Node[RoleData],
):
    parent: Optional["IsJoinable"] = property_parent_(node_is_customizable=False)
