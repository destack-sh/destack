from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    NodeType,
    Timestamp,
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

    slug: str = declare_property(102, is_repr=True, tag=None)

    last_logged_in_at: Optional[Timestamp] = declare_property(111, tag=None)
    # last_active_at, seen_at, ...

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    #  (multiple auth methods, multiple connected accounts, etc.)
    email: str | None = declare_property(130, tag=None)
    password_salt: Optional[str] = declare_property(131, is_eq=False, tag=None)
    password_hash: Optional[str] = declare_property(132, is_eq=False, tag=None)
    # challenges?
    # password_reset_token, email_confirmation_token, ...
