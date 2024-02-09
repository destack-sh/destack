from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.link import NodeList
from bench.language.node import (
    Node,
    ScopeNode,
    node,
    p_parent,
    p_internal,
    p_regular,
    p_system,
    p_child,
)
from bench.sql.core import Constraint, ConstraintType
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import Bench, Space, Server, RichText, Role


@node(
    NodeType.HANDLE,
    roots=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    identifier=IdentifierType.VARIABLE,
    constraints=(
        Constraint("bench_slug_is_slug", ConstraintType.CHECK, condition="(slug ~ '^[a-z0-9-]+$')"),
    ),
)
class Handle(Node):
    """A Bench @handle. Can only be created/edited by the system."""

    parent: Union["User", "Organization", "Bench"] = p_parent(
        4, NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
    )
    slug: str = p_system(30, unique=True)


@node(NodeType.USER, roots=(), identifier=IdentifierType.VARIABLE)
class User(ScopeNode):
    """A Bench user."""

    # (main_handle is optional because we can create user without handle)
    main_handle: Optional[Handle] = p_system(
        30, require=False, array=False, references=NodeType.HANDLE
    )
    handles: NodeList[Handle] = p_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(31, unique=True)  # must match main handle
    name: Optional[str] = p_regular(32, default=None)
    text: Optional["RichText"] = p_regular(33, default=None, struct=StructType.RICH_TEXT)
    email: str = p_system(34, defer=True, unique=True, sensitive=True)
    main_bench: Optional["Bench"] = p_system(
        35, array=False, require=False, references=NodeType.BENCH
    )

    # auth
    password_salt: Optional[bytes] = p_system(
        40, default=None, defer=True, encrypt=True, sensitive=True
    )
    password_hash: Optional[bytes] = p_system(
        42, default=None, defer=True, encrypt=True, sensitive=True
    )
    last_logged_in_at: Optional[datetime] = p_system(43, default=None)
    # last_active_at: Optional[datetime] = ...
    # last_seen_at: Optional[datetime] = ...

    # flags
    is_staff: bool = p_system(60, default=False)
    is_activated: bool = p_system(61, default=False)


@node(NodeType.ORGANIZATION, roots=(), identifier=IdentifierType.VARIABLE)
class Organization(ScopeNode):
    """A Bench organization with Users as members."""

    main_handle: Optional[Handle] = p_system(
        30, require=False, array=False, references=NodeType.HANDLE
    )  # not actually optional but Handle.parent = Organization
    handles: NodeList[Handle] = p_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(31, unique=True)  # must match main handle
    name: str = p_regular(32)
    text: Optional["RichText"] = p_regular(33, default=None, struct=StructType.RICH_TEXT)
    main_bench: Optional["Bench"] = p_system(
        35, array=False, require=False, references=NodeType.BENCH
    )

    # flags
    # ...


@node(NodeType.MEMBERSHIP)
class Membership(ScopeNode):
    """A membership to a Bench or Organization."""

    parent: Union["Bench", "Organization"] = p_parent(4, NodeType.BENCH, NodeType.ORGANIZATION)
    user: "User" = p_internal(30, require=True, array=False, references=NodeType.USER)
    is_owner: bool = p_regular(31, default=False)

    # roles are defined (and resolved) in the main bench
    roles: NodeList["Role"] = p_child(NodeType.ROLE)


@node(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH), identifier=IdentifierType.VARIABLE)
class Client(Node):
    """A client to this Bench."""

    parent: Union[User, "Server"] = p_parent(4, NodeType.USER, NodeType.SERVER)
    # type: ...
    name: Optional[str] = p_regular(32, default=None)
    device_name: str = p_regular(33)
    browser_name: Optional[str] = p_regular(34, default=None)
    last_seen_at: datetime = p_system(35)
    logged_in_at: Optional[datetime] = p_system(36, default=None)
    access_token: Optional[str] = p_system(
        37, default=None, defer=True, unique=True, sensitive=True
    )

    # for user clients
    main_space: Optional["Space"] = p_system(
        40, array=False, require=False, references=NodeType.SPACE
    )

    def __content_str__(self) -> str:
        if self.browser_name:
            return f"{self.device_name} {self.browser_name}"
        else:
            return self.device_name

    @property
    def user(self) -> User:
        return self.parent


# NOTE: Maybe notification should live in your Bench as well?
@node(NodeType.NOTIFICATION, roots=(NodeType.USER,))
class Notification(Node):
    """A notification for a user."""

    parent: User = p_parent(4, NodeType.USER)
    # kind: ...?
    # -> builtin_type / custom_type / ... 'type' as union
    expires_at: datetime = p_internal(33)
    read_at: datetime = p_internal(34)
    # source: ...
