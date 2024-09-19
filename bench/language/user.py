from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import (
    BlockType,
    NodeType,
    OrganizationStatus,
    StructType,
    UserStatus,
)
from bench.language.graph import NodeList
from bench.language.node import BenchNode, Node, node_
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.validation import (
    EMAIL_CONSTRAINT,
    NAME_CONSTRAINT,
    SLUG_CONSTRAINT,
    constraint,
)
from bench.proto.wire import (
    HandleData,
    InviteData,
    MembershipData,
    OrganizationData,
    UserData,
)

if TYPE_CHECKING:
    from bench.language import Bench, Block, Client, Icon, NodeReference, Text

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE, roots=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH))
class Handle(BenchNode[HandleData]):
    """A Bench @handle. Can only be created/edited by the system."""

    parent: Union["User", "Organization", "Bench"] = p_node_parent(
        4, NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
    )
    slug: str = p_system(30, unique=True, constraint=SLUG_CONSTRAINT)

    @property
    def _is_attached(self) -> bool:
        return self.parent is not None  # bench may not be present if it's not in a bench


@node_(NodeType.USER, roots=())
class User(Node[UserData]):
    """A Bench user."""

    # ordinal/number: ...?
    # (main_handle is optional because we can create user without handle)
    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )
    handles: NodeList[Handle] = p_node_children(NodeType.HANDLE)
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
    status: UserStatus = p_system(38)

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

    clients: NodeList["Client"] = p_node_children(NodeType.CLIENT)

    @property
    def bench(self) -> "Bench":
        assert self.main_bench is not None, f"{self!r} is not activated"
        return self.main_bench


@node_(NodeType.ORGANIZATION, roots=())
class Organization(Node[OrganizationData]):
    """
    A Bench organization with Users as members.
    Until activation only its creator has access.
    """

    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )  # not actually optional but Handle.parent = Organization
    handles: NodeList[Handle] = p_node_children(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    main_bench: Optional["Bench"] = p_system(
        36, array=False, require=False, references=NodeType.BENCH, fk=True
    )
    status: OrganizationStatus = p_system(37)

    # flags
    # ...

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
    roles: list["Block"] = p_regular(
        33,
        require=False,
        array=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.ROLE),
    )
