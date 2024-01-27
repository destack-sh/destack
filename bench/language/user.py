from datetime import datetime
from typing import Optional

from bench.language.const import NodeType, NotificationKind, NotificationStatus
from bench.language.node import Node, ScopeNode, node, node_parent, struct_internal, struct_property
from bench.utils.casing import IdentifierType


@node(NodeType.HANDLE, root=None, identifier=IdentifierType.VARIABLE)
class Handle(Node):
    """A (global) Bench handle."""

    slug: str = struct_internal(30, unique=True)


@node(NodeType.USER, root=None, identifier=IdentifierType.VARIABLE)
class User(ScopeNode):
    """A (global) Bench user."""

    handle: Handle = struct_internal(30, require=True, array=False, references=NodeType.HANDLE)
    slug: Optional[str] = struct_internal(31, system=True, unique=True)
    name: Optional[str] = struct_property(32, default=None)
    email: str = struct_internal(33, defer=True, unique=True, system=True)
    password_salt: Optional[bytes] = struct_internal(
        34, default=None, defer=True, encrypt=True, system=True
    )
    password_hash: Optional[bytes] = struct_internal(
        35, default=None, defer=True, encrypt=True, system=True
    )
    last_logged_in_at: Optional[datetime] = struct_internal(36, default=None, system=True)


@node(NodeType.ORGANIZATION, root=None, identifier=IdentifierType.VARIABLE)
class Organization(ScopeNode):
    """A (global) Bench organization."""

    handle: Handle = struct_internal(30, require=True, array=False, references=NodeType.HANDLE)
    slug: Optional[str] = struct_internal(31, system=True, unique=True)
    name: str = struct_property(32)


@node(NodeType.CLIENT, root=NodeType.USER, identifier=IdentifierType.VARIABLE)
class Client(Node):
    """A client to this Bench. Can be a user or a worker."""

    parent: User = node_parent(4, NodeType.USER)
    name: Optional[str] = struct_internal(30, default=None)
    device_name: str = struct_internal(31)
    browser_name: Optional[str] = struct_internal(32, default=None)
    last_seen_at: datetime = struct_internal(33)
    access_token: Optional[str] = struct_internal(
        34, default=None, system=True, defer=True, unique=True
    )

    def __content_str__(self) -> str:
        if self.browser_name:
            return f"{self.device_name} {self.browser_name}"
        else:
            return self.device_name

    @property
    def user(self) -> User:
        return self.parent


@node(NodeType.NOTIFICATION, root=NodeType.USER)
class Notification(Node):
    """A notification for a user."""

    parent: User = node_parent(4, NodeType.USER)
    kind: NotificationKind = struct_internal(30)
    status: NotificationStatus = struct_internal(31)
    expires_at: datetime = struct_internal(32)
    read_at: datetime = struct_internal(33)
