from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.USER,
    is_final=True,
    traits=(TraitType.ACTOR, TraitType.FOLLOWABLE),
)
@final
class User(Entity):
    """A User is a human using Destack."""

    parent: Optional["Space"] = builtin_property_parent()
    slug: str = builtin_property(102, is_repr=True)

    last_logged_in_at: Optional[datetime] = builtin_property(111)
    # last_active_at, seen_at, ...
    is_staff: bool = builtin_property(112, default=False)

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    #  (multiple auth methods, multiple connected accounts, etc.)
    email: str | None = builtin_property(130, is_unique=True)
    password_salt: Optional[bytes] = builtin_property(131, is_eq=False)
    password_hash: Optional[bytes] = builtin_property(132, is_eq=False)
    # challenges?
    # password_reset_token, email_confirmation_token, ...
