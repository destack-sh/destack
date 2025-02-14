from typing import Sequence, override
from uuid import UUID

from bench.language import Action, Message, NodeType, Session, Trigger, TriggerType, bittuple
from bench.proto import EditData

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
        fired_triggers: list[Trigger] = []
        for message in commit.added:
            if not isinstance(message, Message):
                continue
            # nocheckin: filter trigger matches properly
            for trigger in self.active_triggers_by_id.values():
                if trigger.type == TriggerType.MESSAGE:
                    fired_triggers.append(trigger)

        # fire triggers
        for trigger in fired_triggers:
            target = trigger.parent
            if not isinstance(target, Action):
                # not yet supported, see above
                raise RuntimeError(f"trigger {trigger!r} has non-Action parent {target!r}")

            raise NotImplementedError(f"nocheckin: trigger {trigger!r}")
            # run = Run(...)
