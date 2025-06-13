from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IndexIn,
    IsFollowable,
    IsOwner,
    IsSubject,
    Node,
    NodeType,
    StringFormat,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import UserData

if TYPE_CHECKING:
    from destack.language import Cursor, Handle, NodeReference, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.USER_STATUS)
class UserStatus(Enum):
    CREATING = 2
    ACTIVE = 10


@builtin_node(
    NodeType.USER,
    root_type=None,
    index=(IndexIn(columns=("email",), is_unique=True),),
)
class User(
    Global,
    Entity,
    HasName,
    HasIcon,
    HasSlug,
    IsOwner,
    IsFollowable,
    IsSubject,
    Node[UserData],
):
    """A User is a human using Destack."""

    # meta
    name: str = property_(31, is_repr=True)
    slug: str = property_(33, is_repr=True)
    status: UserStatus = property_(
        40, can_write="system", is_repr=True, default=UserStatus.CREATING
    )
    last_logged_in_at: Optional[datetime] = property_(41, can_write="system")
    # last_active_at, seen_at, ...
    is_staff: bool = property_(45, default=False, can_write="system")

    space: "Space" = property_(50, can_write="system", node_space_from="self")
    handle: Optional["Handle"] = property_(51, can_write="system", node_space_from="self")
    cursor: Optional["Cursor"] = property_(52, can_write="system", node_space_from="self")
    if TYPE_CHECKING:
        space_id: UUID = property_()
        space_ptr: NodeReference = property_()
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE: Incomplete: factor out auth/Credentials/Challenges/... for Users/Client
    email: str | None = property_(
        60, format=StringFormat.EMAIL, can_read="owner", can_write="system"
    )
    password_salt: Optional[bytes] = property_(61, can_read="system", can_write="system")
    password_hash: Optional[bytes] = property_(62, can_read="system", can_write="system")
    # challenges?
    # password_reset_token, email_confirmation_token, ...
