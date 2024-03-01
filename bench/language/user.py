from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import (
    NodeType,
    NotificationKind,
    OrganizationStatus,
    StructType,
    UserStatus,
)
from bench.language.graph import NodeList
from bench.language.node import Node, _Passthrough, node, HasBase
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_child,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import SLUG_REGEX, validate_name, validate_slug
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, NotificationData
from bench.sql.core import Constraint, ConstraintType
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Icon,
        Package,
        Role,
        Server,
        Space,
        Text,
    )


@node(
    NodeType.HANDLE,
    roots=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    identifier=IdentifierType.VARIABLE,
    constraints=(
        Constraint(
            "bench_slug_is_slug",
            ConstraintType.CHECK,
            # ::casts are to match the introspected postgres format
            condition=f"((slug)::text ~ '{SLUG_REGEX}'::text)",
        ),
    ),
)
class Handle(Node):
    """A Bench @handle. Can only be created/edited by the system."""

    parent: Union["User", "Organization", "Bench"] = p_node_parent(
        4, NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
    )
    slug: str = p_system(30, unique=True, validate=validate_slug)


@node(NodeType.USER, roots=(), identifier=IdentifierType.VARIABLE)
class User(Node):
    """A Bench user."""

    # ordinal/number: ...?
    # (main_handle is optional because we can create user without handle)
    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE
    )
    handles: NodeList[Handle] = p_node_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: Optional[str] = p_regular(33, default=None, validate=validate_name)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    email: str = p_system(35, defer=True, unique=True, sensitive=True)
    icon: Optional["Icon"] = p_regular(36, default=None, struct=StructType.ICON)
    main_bench: Optional["Bench"] = p_system(
        37, array=False, require=False, references=NodeType.BENCH
    )
    status: UserStatus = p_system(38)

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

    @property
    def bench(self) -> "Bench":
        assert self.main_bench is not None, f"{self!r} is not activated"
        return self.main_bench


@node(NodeType.ORGANIZATION, roots=(), identifier=IdentifierType.VARIABLE)
class Organization(Node):
    """
    A Bench organization with Users as members.
    Until activation only its creator has access.
    """

    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE
    )  # not actually optional but Handle.parent = Organization
    handles: NodeList[Handle] = p_node_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, validate=validate_name)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    main_bench: Optional["Bench"] = p_system(
        36, array=False, require=False, references=NodeType.BENCH
    )
    status: OrganizationStatus = p_system(37)

    # flags
    # ...

    @property
    def bench(self) -> "Bench":
        assert self.main_bench is not None, f"{self!r} is not activated"
        return self.main_bench


@node(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH), identifier=IdentifierType.VARIABLE)
class Client(Node):
    """A client to this Bench."""

    parent: Union[User, "Server"] = p_node_parent(4, NodeType.USER, NodeType.SERVER)
    # type: ...
    name: Optional[str] = p_regular(32, default=None, validate=validate_name)
    device_name: str = p_regular(33, validate=validate_name)
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


@node(NodeType.MEMBERSHIP)
class Membership(Node):
    """
    A membership to this Bench (and its owner if it's the main Bench).
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    user: "User" = p_internal(30, require=True, array=False, references=NodeType.USER)
    is_owner: bool = p_regular(31, default=False)

    # roles are defined (and resolved) in the main bench
    roles: NodeList["Role"] = p_node_child(NodeType.ROLE)


@node(NodeType.INVITE)
class Invite(Node):
    """An invitation to become a member of this Bench."""

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    user: Optional["User"] = p_internal(30, require=False, array=False, references=NodeType.USER)
    user_email: Optional[str] = p_regular(31)

    # membership properties once accepted
    is_owner: bool = p_regular(32, default=False)
    roles: list["Role"] | None = p_regular(33, require=False, array=True, references=NodeType.ROLE)


@node(
    NodeType.NOTIFICATION,
    passthrough=(("value", _Passthrough.Full),),
    index_in_search=True,
    local=True,
)
class Notification(HasBase, HasValues):
    """
    A notification for the Bench's owner.
    As with all owner Bench stuff, the main Bench's main package is the 'truth'.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    kind: NotificationKind = p_regular(30)
    # -> builtin_type / custom_type / ... 'type' as union?
    type: Optional["Block"] = p_system(32, require=False, array=False, references=NodeType.BLOCK)
    expires_at: Optional[datetime] = p_internal(33, default=None)
    read_at: Optional[datetime] = p_internal(34, default=None)
    sender: Optional["Block"] = p_internal(
        35, require=False, array=False, references=NodeType.BLOCK
    )

    # content
    title: Optional[str] = p_regular(40)
    text: Optional["Text"] = p_regular(41, require=False, array=False, struct=StructType.TEXT)
    value_packed: Any | None = p_value_packed(42)
    secret_value_packed: Any | None = p_secret_value_packed(43)
    value: Any = p_value_runtime(42, 43)

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(self, data: NotificationData) -> Optional[NodeReferenceData]:
        return data.type_ptr
