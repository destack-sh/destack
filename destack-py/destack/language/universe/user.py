from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsActor,
    IsCustomizable,
    IsFollowable,
    NodeType,
    StringFormat,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Cursor, Handle, NodeReference, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.USER_STATUS)
class UserStatus(Enum):
    CREATING = 2
    ACTIVE = 10


@builtin_node(NodeType.USER)
class User(
    IsActor,
    IsFollowable,
    IsCustomizable,
    Entity,
):
    """A User is a human using Destack."""

    parent: Optional["Space"] = builtin_property_parent()
    slug: str = builtin_property(102, is_repr=True)

    status: UserStatus = builtin_property(110, is_repr=True, default=UserStatus.CREATING)
    last_logged_in_at: Optional[datetime] = builtin_property(111)
    # last_active_at, seen_at, ...
    is_staff: bool = builtin_property(112, default=False)

    handle: Optional["Handle"] = builtin_property(121)
    cursor: Optional["Cursor"] = builtin_property(122)
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    #  (multiple auth methods, multiple connected accounts, etc.)
    email: str | None = builtin_property(130, format=StringFormat.EMAIL, is_unique=True)
    password_salt: Optional[bytes] = builtin_property(131, is_eq=False)
    password_hash: Optional[bytes] = builtin_property(132, is_eq=False)
    # challenges?
    # password_reset_token, email_confirmation_token, ...
