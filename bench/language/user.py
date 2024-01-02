from bench.language.const import ClientType, NodeType
from bench.language.module import Node, node, struct_internal


@node(NodeType.USER, managed=False)
class User(Node):
    """A (global) Bench user."""

    username: str = struct_internal(20)
    email: str = struct_internal(21, defer=True)


@node(NodeType.CLIENT, stored=False)
class Client(Node):
    """A client to this Bench. Can be a user or a worker."""

    type: ClientType = struct_internal(20)
    device_name: str = struct_internal(21)
    browser_name: str = struct_internal(22)
    last_seen_at: int = struct_internal(23)
    closed_at: int = struct_internal(24)
