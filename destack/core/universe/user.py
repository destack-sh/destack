from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    NodeType,
    TraitType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.USER,
    is_final=True,
    traits=(TraitType.ACTOR, TraitType.FOLLOWABLE),
)
@final
class User(Entity):
    """A User is a human using Destack."""

    slug: str = declare_property(102, is_repr=True)

    last_logged_in_at: Optional[datetime] = declare_property(111)
    # last_active_at, seen_at, ...

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    #  (multiple auth methods, multiple connected accounts, etc.)
    email: str | None = declare_property(130, is_unique=True)
    password_salt: Optional[bytes] = declare_property(131, is_eq=False)
    password_hash: Optional[bytes] = declare_property(132, is_eq=False)
    # challenges?
    # password_reset_token, email_confirmation_token, ...
