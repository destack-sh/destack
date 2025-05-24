from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsGlobal,
    IsIcon,
    IsNamed,
    IsRegional,
    IsSlug,
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
    from bench.language import Bench, Cursor, Handle, NodeReference, TextLine, User

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.USER_STATUS)
class UserStatus(BuiltinEnum):
    WAITLIST = 30
    REGISTERED = 40
    ACTIVE = 50


@node_(NodeType.USER, root_type=None)
class User(
    IsGlobal,
    IsSubject,
    IsNamed,
    IsIcon,
    IsSlug,
    IsRegional,
    Node[UserData],
):
    """A User is a human using Bench."""

    line: Optional["TextLine"] = property_(35)
    is_staff: bool = property_(39, default=False, can_write="system")

    # status
    status: UserStatus = property_(40, can_write="system")
    last_logged_in_at: Optional[datetime] = property_(41, can_write="system")
    # last_active_at: Optional[datetime] = ...
    # seen_at: Optional[datetime] = ...

    bench: Optional["Bench"] = property_(50, can_write="system")
    handle: Optional["Handle"] = property_(51, can_write="system")
    cursor: Optional["Cursor"] = property_(52, can_write="system")
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE :Incomplete: factor out authentication, Credentials & Challenges for Users/Client
    email: str | None = property_(
        60, is_unique=True, format=StringFormat.EMAIL, can_read="owner", can_write="system"
    )
    password_salt: Optional[bytes] = property_(61, can_read="system", can_write="system")
    password_hash: Optional[bytes] = property_(62, can_read="system", can_write="system")
    # challenges?
    # password_reset_token, email_confirmation_token, ...
