from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    ClientType,
    Entity,
    NodeType,
    builtin_entity,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.CLIENT, is_final=True)
@final
class Client(Entity):
    """A Client to connect with the system."""

    # meta
    type: ClientType = builtin_property(100, is_repr=True)

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
