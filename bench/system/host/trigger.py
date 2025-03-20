from typing import Sequence, override
from uuid import UUID

import structlog
from opentelemetry import trace

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
from bench.runtime import create_run

from .core import Commit, HostPlugin

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

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
        for message in commit.added:
            if not isinstance(message, Message):
                continue
            for trigger in self._active_triggers_by_id.values():
                if trigger.type != TriggerType.MESSAGE:
                    continue

                # nocheckin: check Channel/Thread.memberships
                thread = message.thread
                assert thread is not None, f"message {message!r} has no thread"
                is_involved = ...
                is_author = ...

                # fire trigger
                if is_involved and not is_author:
                    target = trigger.parent
                    if isinstance(target, Run):
                        target = target.runnable
                    if not isinstance(target, Action):
                        raise RuntimeError(
                            f"trigger {trigger!r} has unsupported target: {target!r}"
                        )
                    run, thread = create_run(
                        target,
                        parent=thread,
                        trigger=trigger,
                        message=message,
                        thread=message.thread,
                    )
                    logger.info(
                        "message_trigger_plugin.trigger",
                        message=message,
                        thread=thread,
                        run=run,
                    )

    @override
    async def on_commit_failed(self, session: Session, error: BaseException) -> None:
        self._collect_triggers()
