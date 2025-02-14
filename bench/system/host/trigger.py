from typing import Sequence, override
from uuid import UUID

from bench.language import (
    Action,
    Message,
    NodeType,
    Session,
    Trigger,
    TriggerStatus,
    TriggerType,
    bittuple,
)
from bench.proto import EditData
from bench.runtime import make_run_from_node

from .core import Commit, HostPlugin


class ScheduleTriggerPlugin(HostPlugin[Trigger]):
    """Process ScheduleTriggers."""

    watch_types = bittuple(NodeType.TRIGGER)


class MessageTriggerPlugin(HostPlugin[Trigger | Message]):
    """Process MessageTriggers."""

    # NOTE :Incomplete: consider remote Triggers (i.e., those not in source)

    watch_types = bittuple(NodeType.TRIGGER, NodeType.MESSAGE)

    @override
    async def start(self) -> None:
        self._collect_triggers()

    def _collect_triggers(self) -> None:
        """Collects all active Triggers."""
        self.active_triggers_by_id: dict[UUID, Trigger] = {}
        for trigger in self.main_package._graph.nodes_of_type(Trigger):
            self.active_triggers_by_id[trigger.id] = trigger

    @override
    async def on_commit_prepare(
        self, session: Session, commit: Commit[Trigger | Message]
    ) -> Sequence[EditData] | None:
        # update active triggers
        for trigger in commit.added:
            if isinstance(trigger, Trigger):
                self.active_triggers_by_id[trigger.id] = trigger
        for trigger in commit.removed:
            if isinstance(trigger, Trigger):
                del self.active_triggers_by_id[trigger.id]

        # collect fired triggers
        fired_triggers: list[tuple[str, Trigger]] = []
        for message in commit.added:
            if not isinstance(message, Message):
                continue
            # nocheckin: filter trigger matches properly
            for trigger in self.active_triggers_by_id.values():
                if trigger.status == TriggerStatus.OPEN and trigger.type == TriggerType.MESSAGE:
                    fired_triggers.append((str(message.id), trigger))

        # fire triggers
        for trigger_key, trigger in fired_triggers:
            target = trigger.parent
            if not isinstance(target, Action):
                raise NotImplementedError(f"trigger {trigger!r} has unsupported target: {target!r}")

            run = make_run_from_node(target)
            run.trigger = trigger
            run.trigger_key = trigger_key
            session._create(run)

    @override
    async def on_commit_failed(self, session: Session, error: BaseException) -> None:
        self._collect_triggers()
