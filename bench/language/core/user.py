from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.pb2 import InviteData, MembershipData, UserData

from .const import (
    NodeType,
    Region,
    StructType,
    UserStatus,
)
from .list import LocalNodeList
from .node import BenchNode, Node, node_
from .property import (
    p_internal,
    p_kernel,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from .validation import EMAIL_CONSTRAINT, NAME_CONSTRAINT

if TYPE_CHECKING:
    from bench.language import Bench, Handle, Icon, NodeReference, Text, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.USER, roots=())
class User(Node[UserData]):
    """A Bench user."""

    # ordinal/number: ...?
    # (main_handle is optional because we can create user without handle)
    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )
    handles: LocalNodeList["Handle"] = p_node_children(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    email: str | None = p_system(
        35, defer=True, unique=True, sensitive=True, constraint=EMAIL_CONSTRAINT
    )
    icon: Optional["Icon"] = p_regular(36, default=None, struct=StructType.ICON)
    main_bench: Optional["Bench"] = p_system(
        37, array=False, require=False, references=NodeType.BENCH, fk=True
    )
    if TYPE_CHECKING:
        main_bench_id: Optional[UUID] = None
        main_bench_ptr: Optional[NodeReference] = None
    region: "Region" = p_system(38, require=True)
    status: UserStatus = p_system(39)

    # auth
    # TODO :Architecture: refactor out authentication & challenges for Users/Client
    password_salt: Optional[bytes] = p_kernel(
        50, default=None, defer=True, encrypt=True, sensitive=True
    )
    password_hash: Optional[bytes] = p_kernel(
        51, default=None, defer=True, encrypt=True, sensitive=True
    )
    # challenges?
    # password_reset_token, email_confirmation_token, ...

    # activity
    last_logged_in_at: Optional[datetime] = p_system(70, default=None)
    # last_active_at: Optional[datetime] = ...
    # seen_at: Optional[datetime] = ...

    # flags
    is_staff: bool = p_system(90, default=False)

    @property
    def bench(self) -> "Bench":
        assert self.main_bench is not None, f"{self!r} is not activated"
        return self.main_bench


@node_(NodeType.MEMBERSHIP)
class Membership(BenchNode[MembershipData]):
    """
    A membership to this Bench (and its owner if it's the main Bench).
    """

    parent: "Bench | None" = p_node_parent(4, NodeType.BENCH)
    user: "User" = p_internal(30, require=True, array=False, references=NodeType.USER)
    is_owner: bool = p_regular(31, default=False)


@node_(NodeType.INVITE)
class Invite(BenchNode[InviteData]):
    """An invitation to become a member of this Bench."""

    parent: "Bench | None" = p_node_parent(4, NodeType.BENCH)
    user: Optional["User"] = p_internal(30, require=False, array=False, references=NodeType.USER)
    user_email: Optional[str] = p_regular(31)

    # membership properties once accepted
    is_owner: bool = p_regular(32, default=False)
    # roles, ...?
