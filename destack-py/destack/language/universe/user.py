from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsCustomizable,
    IsFollowable,
    IsSubject,
    NodeType,
    RoleType,
    StringFormat,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Cursor, Handle, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.USER_STATUS)
class UserStatus(Enum):
    CREATING = 2
    ACTIVE = 10


@builtin_node(NodeType.USER, root_type=None)
class User(
    IsSubject,
    IsFollowable,
    IsCustomizable,
    Entity,
):
    """A User is a human using Destack."""

    # meta
    name: str = builtin_property(101, is_repr=True)
    slug: str = builtin_property(102, is_repr=True)

    status: UserStatus = builtin_property(
        110, can_write=RoleType.SYSTEM, is_repr=True, default=UserStatus.CREATING
    )
    last_logged_in_at: Optional[datetime] = builtin_property(111, can_write=RoleType.SYSTEM)
    # last_active_at, seen_at, ...
    is_staff: bool = builtin_property(112, default=False, can_write=RoleType.SYSTEM)

    handle: Optional["Handle"] = builtin_property(
        121, can_write=RoleType.SYSTEM, node_space_from="self"
    )
    cursor: Optional["Cursor"] = builtin_property(
        122, can_write=RoleType.SYSTEM, node_space_from="self"
    )
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    #  (multiple auth methods, multiple connected accounts, etc.)
    email: str | None = builtin_property(
        130,
        format=StringFormat.EMAIL,
        can_read=RoleType.OWNER,
        can_write=RoleType.SYSTEM,
        is_unique=True,
    )
    password_salt: Optional[bytes] = builtin_property(
        131, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_eq=False
    )
    password_hash: Optional[bytes] = builtin_property(
        132, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM, is_eq=False
    )
    # challenges?
    # password_reset_token, email_confirmation_token, ...
