from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsGlobal,
    IsIcon,
    IsNamed,
    IsSlug,
    IsSubject,
    Node,
    NodeType,
    StringFormat,
    enum_,
    node_,
    p_kernel,
    p_system,
    property_,
)
from bench.pb2 import UserData

if TYPE_CHECKING:
    from bench.language import Bench, Cursor, Handle, NodeReference, TextLine, User

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.USER_STATUS)
class UserStatus(BuiltinEnum):
    INVITED = 10  # invited via email
    RESERVED = 20  # reserved a handle, unconfirmed
    WAITLISTED = 30  # got handle, waiting
    REGISTERED = 40  # got handle, ready to activate
    ACTIVATED = 50  # has bench, all ready to go


@node_(NodeType.USER, root_type=None)
class User(IsGlobal, IsSubject, IsNamed, IsIcon, IsSlug, Node[UserData]):
    """A User is a human using Bench."""

    line: Optional["TextLine"] = property_(35)
    is_staff: bool = p_system(39, default=False)

    # status
    status: UserStatus = p_system(40)
    last_logged_in_at: Optional[datetime] = p_system(41)
    # last_active_at: Optional[datetime] = ...
    # seen_at: Optional[datetime] = ...

    bench: Optional["Bench"] = p_system(50)
    handle: Optional["Handle"] = p_system(51)
    cursor: Optional["Cursor"] = property_(52)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # NOTE :Incomplete: factor out authentication, Credentials & Challenges for Users/Client
    email: str | None = p_system(60, unique=True, sensitive=True, format=StringFormat.EMAIL)
    password_salt: Optional[bytes] = p_kernel(61, sensitive=True)
    password_hash: Optional[bytes] = p_kernel(62, sensitive=True)
    # challenges?
    # password_reset_token, email_confirmation_token, ...
