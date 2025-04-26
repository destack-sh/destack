from typing import override

from bench.language import (
    Membership,
    Message,
    MessageType,
    NodeType,
    Session,
    Thread,
    bittuple,
)
from bench.system.host import Commit, HostPlugin


class MessagePlugin(HostPlugin[Message | Membership]):
    """
    Create and react to Messages at appropriate times.
    """

    watch_types = bittuple(NodeType.MESSAGE, NodeType.MEMBERSHIP)

    @override
    async def pre_commit(self, session: Session, commit: Commit[Message | Membership]) -> None:
        """
        Pre-commit hook for Messages.
        """
        for node in commit.added:
            if isinstance(node, Membership) and isinstance(thread := node.parent, Thread):
                message = Message.new(type=MessageType.JOIN, nodes_ptr=[node.member_ptr])
                thread.messages.append(message)
