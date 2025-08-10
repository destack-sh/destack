from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    ClientType,
    Entity,
    NodeType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.CLIENT, is_final=True)
@final
class Client(Entity):
    """A Client to connect with the system."""

    # meta
    type: ClientType = declare_property(100, is_repr=True)

    # status
    access_token: Optional[str] = declare_property(120)
    seen_at: Optional[datetime] = declare_property(121)
    logged_in_at: Optional[datetime] = declare_property(122)

    # details
    device_type: Optional[str] = declare_property(130)
    device_name: Optional[str] = declare_property(131)
    operating_system: Optional[str] = declare_property(132)
    browser_name: Optional[str] = declare_property(133)
    browser_version: Optional[str] = declare_property(44)
