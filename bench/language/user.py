from datetime import datetime

from bench.language.const import ClientType, NodeType, NotificationStatus, NotificationType
from bench.language.module import Node, ScopeNode, node, node_parent, struct_internal


@node(NodeType.HANDLE, detached=True, root=None)
class Handle(Node):
    """A (global) Bench handle."""

    slug: str = struct_internal(30, unique=True)


@node(NodeType.USER, detached=True, root=None)
class User(ScopeNode):
    """A (global) Bench user."""

    handle: Handle = struct_internal(30, array=False, references=NodeType.HANDLE)
    username: str = struct_internal(31)
    name: str = struct_internal(32)
    email: str = struct_internal(33, defer=True)
    password_hash: str = struct_internal(34, defer=True, encrypt=True)

    @property
    def path(self):
        return self.username


@node(NodeType.ORGANIZATION, detached=True, root=None)
class Organization(ScopeNode):
    """A (global) Bench organization."""

    handle: Handle = struct_internal(30, array=False, references=NodeType.HANDLE)


@node(NodeType.CLIENT, stored=False, root=NodeType.USER)
class Client(Node):
    """A client to this Bench. Can be a user or a worker."""

    parent: User = node_parent(4, NodeType.USER)
    type: ClientType = struct_internal(30)
    device_name: str = struct_internal(31)
    browser_name: str = struct_internal(32)
    last_seen_at: int = struct_internal(33)
    closed_at: int = struct_internal(34)


@node(NodeType.NOTIFICATION, root=NodeType.USER)
class Notification(Node):
    """A notification for a user."""

    parent: User = node_parent(4, NodeType.USER)
    type: NotificationType = struct_internal(30)
    status: NotificationStatus = struct_internal(31)
    expires_at: datetime = struct_internal(32)
    read_at: datetime = struct_internal(33)
