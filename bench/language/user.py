from datetime import datetime

from bench.language.const import ClientType, NodeType, NotificationStatus, NotificationType
from bench.language.module import Node, node, node_parent, struct_internal


@node(NodeType.USER, managed=False)
class User(Node):
    """A (global) Bench user."""

    username: str = struct_internal(30)
    email: str = struct_internal(31, defer=True)

    @property
    def path(self):
        return self.username


@node(NodeType.CLIENT, stored=False)
class Client(Node):
    """A client to this Bench. Can be a user or a worker."""

    parent: User = node_parent(4, NodeType.USER)
    type: ClientType = struct_internal(30)
    device_name: str = struct_internal(31)
    browser_name: str = struct_internal(32)
    last_seen_at: int = struct_internal(33)
    closed_at: int = struct_internal(34)


@node(NodeType.NOTIFICATION)
class Notification(Node):
    """A notification for a user."""

    parent: User = node_parent(4, NodeType.USER)
    type: NotificationType = struct_internal(30)
    status: NotificationStatus = struct_internal(31)
    expires_at: datetime = struct_internal(32)
    read_at: datetime = struct_internal(33)
    archived_at: datetime = struct_internal(14)
