from typing import Any, Sequence, override
from uuid import UUID

from bench.language import (
    Action,
    Message,
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
                if trigger.type == TriggerType.MESSAGE:
                    # TODO :Broken: better MessageTrigger filtering / is_involved check
                    #  (consider reply_to, what about multiple Runs of same Flow,
                    #   ideally should route automatically somehow (when none mentioned?)?, ...)
                    trigger_parent = trigger.parent
                    if isinstance(trigger_parent, Run):
                        trigger_parent = trigger_parent.action
                    assert isinstance(trigger_parent, Action), f"unexpected parent for {trigger!r}"
                    flow = trigger_parent.flow
                    assert flow is not None, f"trigger {trigger!r} has no flow"
                    # if we're in the scope of the flow, always trigger, otherwise only if mentioned
                    is_involved = (message.text is not None and flow in message.text) or (
                        message.thread is not None and message.thread.scope_id == flow.id
                    )
                    is_source = message.created_by_id == flow.id  # don't react to self
                    if is_involved and not is_source:
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
            if trigger_run is not None:
                run.machine = trigger_run.machine
            session._create(run)

    @override
    async def on_commit_failed(self, session: Session, error: BaseException) -> None:
        self._collect_triggers()
