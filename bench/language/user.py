from datetime import datetime
from typing import Optional

from bench.language.const import NodeType, NotificationStatus, NotificationKind
from bench.language.node import Node, ScopeNode, node, node_parent, struct_internal, struct_property


@node(NodeType.HANDLE, root=None, in_module=False, in_bench=False)
class Handle(Node):
    """A (global) Bench handle."""

    slug: str = struct_internal(30, unique=True)


@node(NodeType.USER, root=None, in_module=False, in_bench=False)
class User(ScopeNode):
    """A (global) Bench user."""

    handle: Handle = struct_internal(30, require=True, array=False, references=NodeType.HANDLE)
    slug: Optional[str] = struct_internal(31, protect=True, unique=True)
    name: Optional[str] = struct_property(32)
    email: str = struct_internal(33, defer=True, unique=True, protect=True)
    password_salt: Optional[bytes] = struct_internal(34, defer=True, encrypt=True, protect=True)
    password_hash: Optional[bytes] = struct_internal(35, defer=True, encrypt=True, protect=True)
    last_logged_in_at: Optional[datetime] = struct_internal(36, default=None, protect=True)

    @property
    def path(self):
        return self.slug


@node(NodeType.ORGANIZATION, root=None, in_module=False, in_bench=False)
class Organization(ScopeNode):
    """A (global) Bench organization."""

    handle: Handle = struct_internal(30, require=True, array=False, references=NodeType.HANDLE)
    slug: Optional[str] = struct_internal(31, protect=True, unique=True)
    name: str = struct_property(32)

    @property
    def path(self):
        return self.slug


@node(NodeType.CLIENT, root=NodeType.USER, in_bench=False, in_module=False)
class Client(Node):
    """A client to this Bench. Can be a user or a worker."""

    parent: User = node_parent(4, NodeType.USER)
    name: Optional[str] = struct_internal(30)
    device_name: str = struct_internal(31)
    browser_name: Optional[str] = struct_internal(32)
    last_seen_at: datetime = struct_internal(33)
    access_token: Optional[str] = struct_internal(
        34, default=None, protect=True, defer=True, unique=True
    )

    @property
    def user(self) -> User:
        return self.parent


@node(NodeType.NOTIFICATION, root=NodeType.USER, in_bench=False, in_module=False)
class Notification(Node):
    """A notification for a user."""

    parent: User = node_parent(4, NodeType.USER)
    kind: NotificationKind = struct_internal(30)
    status: NotificationStatus = struct_internal(31)
    expires_at: datetime = struct_internal(32)
    read_at: datetime = struct_internal(33)
