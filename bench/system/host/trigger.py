from typing import Sequence, override

from bench.language import Message, NodeType, Session, Trigger, bittuple
from bench.proto import EditData

from .core import Commit, HostPlugin


class ScheduleTriggerPlugin(HostPlugin[Trigger]):
    """Process ScheduleTriggers."""

    watch_types = bittuple(NodeType.TRIGGER)


class MessageTriggerPlugin(HostPlugin[Trigger | Message]):
    """Process MessageTriggers."""

    watch_types = bittuple(NodeType.TRIGGER, NodeType.MESSAGE)

    @override
    async def start(self) -> None:
        # fetch active triggers (from runtime, source is already loaded)
        ...

    @override
    async def on_commit_prepare(
        self, session: Session, commit: Commit[Trigger | Message]
    ) -> Sequence[EditData] | None:
        # update active triggers
        ...

        # fire triggers
        ...
