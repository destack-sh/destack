from typing import Any, Sequence, cast, override
from uuid import UUID

from bench.language import (
    Action,
    Message,
    MessageTrigger,
    NodeType,
    Run,
    Session,
    Trigger,
    TriggerType,
    bittuple,
)
from bench.proto import EditData
from bench.runtime import make_run_from_node

from .core import Commit, HostPlugin

# NOTE :Incomplete: consider runtime Triggers (i.e., those not in source)
# NOTE :Architecture: how will Triggers work with hibernated Runs? :HibernateRuns


class ScheduleTriggerPlugin(HostPlugin[Trigger]):
    """Process ScheduleTriggers."""

    watch_types = bittuple(NodeType.TRIGGER)


class MessageTriggerPlugin(HostPlugin[Trigger | Message]):
    """Process MessageTriggers."""

    watch_types = bittuple(NodeType.TRIGGER, NodeType.MESSAGE)

    @override
    async def start(self) -> None:
        self._collect_triggers()

    def _collect_triggers(self) -> None:
        """Collects all active Triggers."""
        self._active_triggers_by_id: dict[UUID, Trigger] = {}
        for trigger in self.main_package._graph.nodes_of_type(Trigger):
            if trigger.is_open:
                self._active_triggers_by_id[trigger.id] = trigger

    @override
    async def on_commit_prepare(
        self, session: Session, commit: Commit[Trigger | Message]
    ) -> Sequence[EditData] | None:
        # update active triggers
        for trigger in commit.added:
            if isinstance(trigger, Trigger) and trigger.is_open:
                self._active_triggers_by_id[trigger.id] = trigger
        for trigger in commit.updated:
            if isinstance(trigger, Trigger):
                if trigger.is_open:
                    self._active_triggers_by_id[trigger.id] = trigger
                else:
                    del self._active_triggers_by_id[trigger.id]
        for trigger in commit.removed:
            if isinstance(trigger, Trigger) and trigger.id in self._active_triggers_by_id:
                del self._active_triggers_by_id[trigger.id]

        # collect fired triggers
        fired_triggers: list[tuple[Run | None, dict[str, Any], str, Trigger]] = []
        for message in commit.added:
            if not isinstance(message, Message):
                continue
            for trigger in self._active_triggers_by_id.values():
                if trigger.type == TriggerType.MESSAGE and message_trigger_matches(
                    cast(MessageTrigger, trigger), message
                ):
                    inputs = {"message": message}
                    fired_triggers.append((message.run, inputs, str(message.id), trigger))

        # fire triggers
        for trigger_run, inputs, trigger_key, trigger in fired_triggers:
            target = trigger.parent
            if isinstance(target, Run):
                target = target.runnable
            if not isinstance(target, Action):
                raise RuntimeError(f"trigger {trigger!r} has unsupported target: {target!r}")
            run = make_run_from_node(target, inputs=inputs)
            run.trigger = trigger
            run.trigger_key = trigger_key
            run.trigger_run = trigger_run
            if trigger_run is not None:
                run.machine = trigger_run.machine
            session._create(run)

    @override
    async def on_commit_failed(self, session: Session, error: BaseException) -> None:
        self._collect_triggers()


def message_trigger_matches(trigger: MessageTrigger, message: Message) -> bool:
    """
    Checks if a MessageTrigger matches a Message.
    """
    if (channel := trigger.channel) is not None and channel.id != message.channel_id:
        return False
    if (thread := trigger.thread) is not None and thread.id != message.thread_id:  # noqa: SIM103
        return False
    return True
