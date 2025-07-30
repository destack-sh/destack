from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from destack.language.core import (
    ClientType,
    Entity,
    NodeReference,
    NodeType,
    builtin_entity,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Machine, User

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.CLIENT, is_final=True)
@final
class Client(
    Entity,
):
    """A Client to connect with the system."""

    # meta
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

    # details
    device_type: Optional[str] = builtin_property(130)
    device_name: Optional[str] = builtin_property(131)
    operating_system: Optional[str] = builtin_property(132)
    browser_name: Optional[str] = builtin_property(133)
    browser_version: Optional[str] = builtin_property(44)
