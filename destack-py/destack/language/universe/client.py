from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    ClientType,
    Entity,
    IsActor,
    NodeReference,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Cursor, Machine, User

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CLIENT)
class Client(
    Entity,
):
    """A Client to connect with the system."""

    # meta
    parent: Optional[IsActor] = builtin_property_parent()
    type: ClientType = builtin_property(100, is_repr=True)

    machine: Optional["Machine"] = builtin_property(110)
    user: Optional["User"] = builtin_property(111)
    if TYPE_CHECKING:
        machine_ptr: Optional[NodeReference] = None
        user_ptr: Optional[NodeReference] = None

    # status
    access_token: Optional[str] = builtin_property(120, is_unique=True)
    seen_at: Optional[datetime] = builtin_property(121)
    logged_in_at: Optional[datetime] = builtin_property(122)
    cursor: Optional["Cursor"] = builtin_property(123)

    # details
    device_type: Optional[str] = builtin_property(130)
    device_name: Optional[str] = builtin_property(131)
    operating_system: Optional[str] = builtin_property(132)
    browser_name: Optional[str] = builtin_property(133)
    browser_version: Optional[str] = builtin_property(44)
