from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language.const import (
    NodeType,
    NotificationKind,
    OrganizationStatus,
    StructType,
    UserStatus,
)
from bench.language.graph import NodeList
from bench.language.node import BasedNode, Node, node
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
from bench.language.validation import EMAIL_CONSTRAINT, NAME_CONSTRAINT, SLUG_CONSTRAINT, SLUG_REGEX
from bench.language.value import HasValues
from bench.proto.wire import (
    AnyNodeData,
    HandleData,
    InviteData,
    MembershipData,
    NodeReferenceData,
    NotificationData,
    OrganizationData,
    UserData,
)
from bench.sql.core import Constraint, ConstraintType
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import Bench, Block, Client, Icon, Package, Role, Text

# pyright: reportIncompatibleVariableOverride=false


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
class Handle(Node[HandleData]):
    """A Bench @handle. Can only be created/edited by the system."""

    parent: Union["User", "Organization", "Bench"] = p_node_parent(
        4, NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
    )
    slug: str = p_system(30, unique=True, constraint=SLUG_CONSTRAINT)


@node(NodeType.USER, roots=(), identifier=IdentifierType.VARIABLE)
class User(Node[UserData]):
    """A Bench user."""

    # ordinal/number: ...?
    # (main_handle is optional because we can create user without handle)
    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )
    handles: NodeList[Handle] = p_node_child(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    email: str = p_system(35, defer=True, unique=True, sensitive=True, constraint=EMAIL_CONSTRAINT)
    icon: Optional["Icon"] = p_regular(36, default=None, struct=StructType.ICON)
    main_bench: Optional["Bench"] = p_system(
        37, array=False, require=False, references=NodeType.BENCH, fk=True
    )
    if TYPE_CHECKING:
        main_bench_id: Optional[UUID] = None
        main_bench_ptr: Optional[NodeReferenceData] = None
    status: UserStatus = p_system(38)

    # auth
    password_salt: Optional[bytes] = p_kernel(
        50, default=None, defer=True, encrypt=True, sensitive=True
    )
    password_hash: Optional[bytes] = p_kernel(
        51, default=None, defer=True, encrypt=True, sensitive=True
    )
    # challenges?
    # password_reset_token: Optional[UUID] = ...
    # email_confirmation_token: Optional[UUID] = ...

    # activity
    last_logged_in_at: Optional[datetime] = p_system(70, default=None)
    # last_active_at: Optional[datetime] = ...
    # last_seen_at: Optional[datetime] = ...

    # flags
    is_staff: bool = p_system(90, default=False)

    clients: NodeList["Client"] = p_node_child(NodeType.CLIENT)

    @property
    def bench(self) -> "Bench":
        assert self.main_bench is not None, f"{self!r} is not activated"
        return self.main_bench


@node(NodeType.ORGANIZATION, roots=(), identifier=IdentifierType.VARIABLE)
class Organization(Node[OrganizationData]):
    """
    A Bench organization with Users as members.
    Until activation only its creator has access.
    """

    main_handle: Optional[Handle] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )  # not actually optional but Handle.parent = Organization
    handles: NodeList[Handle] = p_node_child(NodeType.HANDLE)
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


@node(NodeType.MEMBERSHIP)
class Membership(Node[MembershipData]):
    """
    A membership to this Bench (and its owner if it's the main Bench).
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    user: "User" = p_internal(30, require=True, array=False, references=NodeType.USER)
    is_owner: bool = p_regular(31, default=False)

    # roles are defined (and resolved) in the main bench
    roles: NodeList["Role"] = p_node_child(NodeType.ROLE)


@node(NodeType.INVITE)
class Invite(Node[InviteData]):
    """An invitation to become a member of this Bench."""

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    user: Optional["User"] = p_internal(30, require=False, array=False, references=NodeType.USER)
    user_email: Optional[str] = p_regular(31)

    # membership properties once accepted
    is_owner: bool = p_regular(32, default=False)
    roles: list["Role"] = p_regular(33, require=False, array=True, references=NodeType.ROLE)


@node(NodeType.NOTIFICATION, passthrough="value", index_in_search=True, local=True)
class Notification(BasedNode[NotificationData], HasValues):
    """
    A Notification for someone in that Bench.
    As with most Bench stuff, the main Bench's main package is the 'truth'.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    kind: NotificationKind = p_regular(30)
    # -> builtin_type / custom_type / ... 'type' as union?
    type: Optional["Block"] = p_system(32, require=False, array=False, references=NodeType.BLOCK)
    expires_at: Optional[datetime] = p_internal(33, default=None)
    read_at: Optional[datetime] = p_internal(34, default=None)
    origin: Optional["Block"] = p_internal(
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
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(NotificationData, data)).type_ptr
