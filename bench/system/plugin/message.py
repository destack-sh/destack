from typing import override

from bench.language import (
    Membership,
    Message,
    NodeType,
    Session,
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
        # NOTE: ideally we would auto-create JOIN/LEAVE messages when a Membership is created/deleted
        #  BUT that's 1) confused if we archive/restore a Thread (since that would create more Messages)
        #  AND also it doesn't totally work when a Thread is created together with initial Messages,
        #  because then the JOIN/LEAVE Messages would appear *after* the initial Messages (which is weird).
        #  (Right now we just do it in the frontend, but this speaks to a larger problem..)
        #  :BadAutoMessages
        pass
