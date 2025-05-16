from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.core import (
    EMAIL_CONSTRAINT,
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsSubject,
    Node,
    NodeType,
    Region,
    enum_,
    node_,
    p_kernel,
    p_regular,
    p_system,
)
from bench.pb2 import UserData

if TYPE_CHECKING:
    from bench.language import Bench, Cursor, Handle, Icon, NodeReference, TextLine, User

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.USER_STATUS)
class UserStatus(BuiltinEnum):
    INVITED = 10  # invited via email
    RESERVED = 20  # reserved a handle, unconfirmed
    WAITLISTED = 30  # got handle, waiting
    REGISTERED = 40  # got handle, ready to activate
    ACTIVATED = 50  # has bench, all ready to go


@node_(NodeType.USER, roots=())
class User(IsSubject, Node[UserData]):
    """A User is a human using Bench."""

    slug: Optional[str] = p_system(31, unique=True)  # must match main handle
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    icon: Optional["Icon"] = p_regular(33)
    line: Optional["TextLine"] = p_regular(34)
    region: "Region" = p_system(35)
    is_staff: bool = p_system(39, default=False)

    # status
    status: UserStatus = p_system(40)
    last_logged_in_at: Optional[datetime] = p_system(41)
    # last_active_at: Optional[datetime] = ...
    # seen_at: Optional[datetime] = ...

    bench: Optional["Bench"] = p_system(50, fk=True)
    handle: Optional["Handle"] = p_system(51, fk=True)
    cursor: Optional["Cursor"] = p_regular(52, fk=True)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        handle_ptr: Optional[NodeReference] = None
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None

    # auth
    # TODO :Architecture: refactor out authentication & challenges for Users/Client
    email: str | None = p_system(
        60, defer=True, unique=True, sensitive=True, constraint=EMAIL_CONSTRAINT
    )
    password_salt: Optional[bytes] = p_kernel(61, defer=True, sensitive=True)
    password_hash: Optional[bytes] = p_kernel(62, defer=True, sensitive=True)
    # challenges?
    # password_reset_token, email_confirmation_token, ...
