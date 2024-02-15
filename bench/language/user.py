from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, Any

from bench.language.const import NodeType, StructType
from bench.language.graph import NodeList
from bench.language.node import (
    Node,
    node,
    _Passthrough,
)
from bench.language.property import (
    p_parent,
    p_child,
    p_regular,
    p_internal,
    p_system,
    p_kernel,
    p_value_packed,
    p_secret_value_packed,
    p_value_runtime,
)
from bench.language.value import HasValues
from bench.sql.core import Constraint, ConstraintType
from bench.utils.casing import IdentifierType
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Bench, RichText, Role, Server, Space, Block, Package


@node(
    NodeType.HANDLE,
    roots=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    identifier=IdentifierType.VARIABLE,
    constraints=(
        Constraint(
            "bench_slug_is_slug",
            ConstraintType.CHECK,
            # ::casts are to match the introspected postgres format
            condition="((slug)::text ~ '^[a-z0-9-]+$'::text)",
        ),
    ),
)
class Handle(Node):
    """A Bench @handle. Can only be created/edited by the system."""

    parent: Union["User", "Organization", "Bench"] = p_parent(
        4, NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
    )
    slug: str = p_system(30, unique=True)


class UserStatus(IdEnum):
    INVITED = 1  # invited via email
    RESERVED = 2  # reserved a handle, unconfirmed
    REGISTERED = 3  # confirmed email
    ACTIVATED = 10  # has main bench


@node(NodeType.USER, roots=(), identifier=IdentifierType.VARIABLE)
class User(Node):
    """A Bench user."""

    # ordinal/number: ...?
    # (main_handle is optional because we can create user without handle)
    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE
    )
    handles: NodeList[Handle] = p_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: Optional[str] = p_regular(33, default=None)
    text: Optional["RichText"] = p_regular(34, default=None, struct=StructType.RICH_TEXT)
    email: str = p_system(35, defer=True, unique=True, sensitive=True)
    main_bench: Optional["Bench"] = p_system(
        36, array=False, require=False, references=NodeType.BENCH
    )
    status: UserStatus = p_system(37)

    # auth
    password_salt: Optional[bytes] = p_kernel(
        50, default=None, defer=True, encrypt=True, sensitive=True
    )
    password_hash: Optional[bytes] = p_kernel(
        51, default=None, defer=True, encrypt=True, sensitive=True
    )
    # password_reset_token: Optional[UUID] = ...
    # email_confirmation_token: Optional[UUID] = ...

    # activity
    last_logged_in_at: Optional[datetime] = p_system(70, default=None)
    # last_active_at: Optional[datetime] = ...
    # last_seen_at: Optional[datetime] = ...

    # flags
    is_staff: bool = p_system(90, default=False)
    is_activated: bool = p_system(91, default=False)


class OrganizationStatus(IdEnum):
    REGISTERED = 3  # confirmed email
    ACTIVATED = 10  # has main bench


@node(NodeType.ORGANIZATION, roots=(), identifier=IdentifierType.VARIABLE)
class Organization(Node):
    """A Bench organization with Users as members."""

    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE
    )  # not actually optional but Handle.parent = Organization
    handles: NodeList[Handle] = p_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33)
    text: Optional["RichText"] = p_regular(34, default=None, struct=StructType.RICH_TEXT)
    main_bench: Optional["Bench"] = p_system(
        36, array=False, require=False, references=NodeType.BENCH
    )
    status: OrganizationStatus = p_system(37)

    # flags
    # ...


@node(NodeType.MEMBERSHIP)
class Membership(Node):
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
    access_token: Optional[str] = p_kernel(
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


class NotificationKind(IdEnum):
    """
    The level of interaction required for a notification.
    """

    PASSIVE = 1  # no quick action required, not urgent
    ACTIVE = 2  # important action required / may want to know this as soon as possible
    URGENT = 3  # immediate action required


@node(
    NodeType.NOTIFICATION,
    passthrough=(("value", _Passthrough.Full),),
    index_in_search=True,
)
class Notification(HasValues):
    """
    A notification for the Bench's owner.
    As with all owner Bench stuff, the main Bench's main package is the 'truth'.
    """

    parent: "Package" = p_parent(4, NodeType.PACKAGE)
    kind: NotificationKind = p_regular(30)
    # -> builtin_type / custom_type / ... 'type' as union
    expires_at: datetime = p_internal(33)
    read_at: datetime = p_internal(34)
    sender: Optional["Block"] = p_internal(
        35, require=False, array=False, references=NodeType.BLOCK
    )
    sender_bench: Optional["Bench"] = p_internal(
        36, require=False, array=False, references=NodeType.BENCH
    )

    # content
    title: Optional[str] = p_regular(40)
    text: Optional["RichText"] = p_regular(
        41, require=False, array=False, struct=StructType.RICH_TEXT
    )
    value_packed: Any | None = p_value_packed(42)
    secret_value_packed: Any | None = p_secret_value_packed(43)
    value: Any = p_value_runtime(42, 43)
