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
    type: ClientType = declare_property(100, is_repr=True, tag=None)

    # status
    access_token: Optional[str] = declare_property(120, tag=None)
    seen_at: Optional[datetime] = declare_property(121, tag=None)
    logged_in_at: Optional[datetime] = declare_property(122, tag=None)

    # details
    device_type: Optional[str] = declare_property(130, tag=None)
    device_name: Optional[str] = declare_property(131, tag=None)
    operating_system: Optional[str] = declare_property(132, tag=None)
    browser_name: Optional[str] = declare_property(133, tag=None)
    browser_version: Optional[str] = declare_property(44, tag=None)
