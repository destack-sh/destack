from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IndexIn,
    IsGlobal,
    IsSubject,
    Node,
    NodeType,
    StringFormat,
    enum_,
    node_,
    property_,
)
from bench.pb2 import UserData

if TYPE_CHECKING:
    from bench.language import Bench, Cursor, Handle, NodeReference, User

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.USER_STATUS)
class UserStatus(BuiltinEnum):
    CREATING = 2
    ACTIVE = 10


@node_(
    NodeType.USER,
    root_type=None,
    index=(IndexIn(columns=("email",), is_unique=True),),
)
class User(
    IsGlobal,
    IsSubject,
    HasName,
    HasIcon,
    HasSlug,
    Node[UserData],
):
    """A User is a human using Bench."""

    # status
    status: UserStatus = property_(
        40, can_write="system", is_repr=True, default=UserStatus.CREATING
    )
    last_logged_in_at: Optional[datetime] = property_(41, can_write="system")
    # last_active_at, seen_at, ...
    is_staff: bool = property_(45, default=False, can_write="system")

    bench: "Bench" = property_(50, can_write="system")
    handle: Optional["Handle"] = property_(51, can_write="system")
    cursor: Optional["Cursor"] = property_(52, can_write="system", node_bench_from="self")
    if TYPE_CHECKING:
        bench_id: UUID = property_()
        bench_ptr: NodeReference = property_()
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE: Incomplete: factor out authentication, Credentials & Challenges for Users/Client
    email: str | None = property_(
        60, format=StringFormat.EMAIL, can_read="owner", can_write="system"
    )
    password_salt: Optional[bytes] = property_(61, can_read="system", can_write="system")
    password_hash: Optional[bytes] = property_(62, can_read="system", can_write="system")
    # challenges?
    # password_reset_token, email_confirmation_token, ...
