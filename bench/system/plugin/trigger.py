from typing import Sequence, override

import structlog
from more_itertools import first
from opentelemetry import trace

from bench.language import (
    Identity,
    Message,
    NodeType,
    Run,
    Session,
    Trigger,
    bittuple,
)
from bench.language.source.action import ActionType
from bench.language.source.trigger import TriggerEffect, TriggerType
from bench.proto import EditData
from bench.runtime import create_run
from bench.system.host import Commit, HostPlugin

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
    async def on_commit_prepare(
        self, session: Session, commit: Commit[Trigger | Message]
    ) -> Sequence[EditData] | None:
        # collect fired triggers
        for message in commit.added:
            if not isinstance(message, Message):
                continue
            runs: list[Run] = []
            # fire trigger for each member
            thread = message.thread
            assert thread is not None, f"message {message!r} has no thread"
            for membership in thread.memberships:
                if membership.member_id == message.created_by_id:
                    continue  # skip if we're the author
                member = membership.member
                if not isinstance(member, Identity):
                    continue  # not an Identity or deleted
                flow = member.default_flow
                if flow is None:
                    continue  # no Flow to trigger
                for action in flow.actions:
                    if action.type != ActionType.START:
                        continue
                    trigger = first(
                        (
                            trigger
                            for trigger in action.triggers
                            if trigger.type == TriggerType.MESSAGE
                            and trigger.effect == TriggerEffect.RUN
                            and trigger.is_open
                        ),
                        None,
                    )
                    if trigger is None:
                        continue  # no matching trigger
                    run, _ = create_run(
                        action,
                        parent=thread,
                        trigger=trigger,
                        message=message,
                        thread=thread,
                        identity=member,
                    )
                    runs.append(run)
            logger.info("message_trigger_plugin.trigger", message=message, runs=runs)
