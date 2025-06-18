from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsFollowable,
    IsOwner,
    IsSubject,
    Node,
    NodeType,
    RoleType,
    StringFormat,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.proto import UserProto

if TYPE_CHECKING:
    from destack.language import Cursor, Handle, NodeReference, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.USER_STATUS)
class UserStatus(Enum):
    CREATING = 2
    ACTIVE = 10


@builtin_node(NodeType.USER, root_type=None)
class User(
    Global,
    Entity,
    HasName,
    HasIcon,
    HasSlug,
    IsOwner,
    IsFollowable,
    IsSubject,
    Node[UserProto],
):
    """A User is a human using Destack."""

    # meta
    name: str = property_(31, is_repr=True)
    slug: str = property_(33, is_repr=True)
    status: UserStatus = property_(
        40, can_write=RoleType.SYSTEM, is_repr=True, default=UserStatus.CREATING
    )
    last_logged_in_at: Optional[datetime] = property_(41, can_write=RoleType.SYSTEM)
    # last_active_at, seen_at, ...
    is_staff: bool = property_(45, default=False, can_write=RoleType.SYSTEM)

    space: "Space" = property_(50, can_write=RoleType.SYSTEM, node_space_from="self")
    handle: Optional["Handle"] = property_(51, can_write=RoleType.SYSTEM, node_space_from="self")
    cursor: Optional["Cursor"] = property_(52, can_write=RoleType.SYSTEM, node_space_from="self")
    if TYPE_CHECKING:
        space_ptr: NodeReference = property_()
        handle_ptr: Optional[NodeReference] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    #  (multiple auth methods, multiple connected accounts, etc.)
    email: str | None = property_(
        60,
        format=StringFormat.EMAIL,
        can_read=RoleType.OWNER,
        can_write=RoleType.SYSTEM,
        is_unique=True,
    )
    password_salt: Optional[bytes] = property_(
        61, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    password_hash: Optional[bytes] = property_(
        62, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    # challenges?
    # password_reset_token, email_confirmation_token, ...
