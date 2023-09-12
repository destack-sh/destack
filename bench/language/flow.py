import itertools
import typing
from collections import deque
from dataclasses import field
from datetime import datetime
from typing import Optional, Union, cast
from uuid import UUID

import pytz
from croniter import croniter

from bench.language.const import ScheduleType, StatementType, TriggerType, TypeTag
from bench.language.core import (
    MNT,
    HasCrud,
    HasSession,
    ModuleNode,
    ModuleVisitor,
    Scope,
    Statement,
    StatementBase,
    node,
)
from bench.language.type import HasTags, HasType, IssueType, Mapping
from bench.language.utils import Runnable
from bench.utils.utils import required_field


@node(mnt=MNT.Trigger)
class Trigger(ModuleNode, HasCrud, HasSession):
    """A trigger for a runnable, possibly inside a flow."""

    type: TriggerType = required_field()
    parent: Statement | None = None
    active: bool = True
    mapping: Optional[Mapping] = None
    schedule_type: Optional[ScheduleType] = None
    timezone: Optional[str] = None
    interval: Optional[int] = None
    cron: Optional[str] = None
    runnable: Union[Runnable, UUID, None] = None
    scope: Union["HasFlow", UUID, None] = None

    def __str__(self):
        if self.type == TriggerType.TIME:
            schedule_str = (
                self.interval if self.schedule_type == ScheduleType.INTERVAL else self.cron
            )
            content_str = f"{self.schedule_type} {self.timezone} {schedule_str}"
        else:
            content_str = None
        return (
            f"{self.type} {content_str or '<none>'} on {self.parent} in {self.scope or '<global>'}"
        )

    def __repr__(self):
        return f"<Trigger {self}>"

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass


@node
class IsFlowNode(Runnable, StatementBase):
    """A symbol that can participate in a flow."""

    triggers: list[Trigger] = field(default_factory=list)

    def _clear(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        # resolve triggers
        for trigger in self.triggers:
            # resolve runnable
            if trigger.runnable is not None and not isinstance(trigger.runnable, Runnable):
                resolved = scope.lookup(trigger.runnable)
                if resolved is None:
                    self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
                else:
                    trigger.runnable = resolved
            # resolve scope
            if trigger.scope is not None and not isinstance(trigger.scope, HasFlow):
                resolved = scope.lookup(trigger.scope)
                if resolved is None:
                    self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
                else:
                    trigger.scope = resolved

    def add_trigger(self, trigger: Trigger) -> None:
        raise NotImplementedError

    def remove_trigger(self, trigger: Trigger | UUID) -> None:
        raise NotImplementedError


@node
class HasFlow(Runnable, StatementBase):
    """A symbol that has a flow of flow nodes."""

    def _clear(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        pass


# avoid circular import because Reference -> IsFlowNode
from bench.language.basic import HasText  # noqa: E402


@node
class Flow(HasType, HasFlow, HasTags, HasText, Statement):
    """An orchestrated flow of triggered runs."""

    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.CODE
    _is_async: bool = False

    def _clear(self) -> None:
        HasText._clear(self)
        HasType._clear(self)
        HasFlow._clear(self)
        HasTags._clear(self)
        Statement._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasText._interp(self, scope)
        HasType._interp(self, scope)
        HasFlow._interp(self, scope)
        HasTags._interp(self, scope)
        Statement._interp(self, scope)

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.children, self.fields, self.tags, self.triggers):
            visitor.visit_child(n)
        HasText._visit(self, visitor)

    def to_sync(self) -> "Flow":
        if not self._is_async:
            return self
        return FlowProxy.to_sync(self)

    def to_async(self) -> "Flow":
        if self._is_async:
            return self
        return FlowProxy.to_async(self)


class FlowRunner:
    def __init__(self, flow: Flow):
        self.flow = flow

    def __call__(self, *args, **kwargs):
        raise NotImplementedError


class FlowProxy:  # :SyncProxy
    """
    A simple proxy for Flow to enable to_sync/to_async while keeping the original Flow object.
    """

    def __init__(self, flow: Flow, is_async: bool):
        self._flow = flow
        self._is_async = is_async

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self._flow.__call_async__(*args, **kwargs)
        else:
            return self._flow.__call_sync__(*args, **kwargs)

    def __getattr__(self, item):
        return getattr(self._flow, item)

    @classmethod
    def to_sync(cls, flow: Flow) -> Flow:
        proxy = cls(flow, is_async=False)
        proxy.__call_sync__ = flow.session.async_to_sync(flow.__call_async__)
        return cast(Flow, proxy)

    @classmethod
    def to_async(cls, flow: Flow) -> Flow:
        proxy = cls(flow, is_async=True)
        proxy.__call_async__ = flow.session.sync_to_async(flow.__call_sync__)
        return cast(Flow, proxy)


# :TriggerSchedule
TRIGGER_INTERVAL_ORIGIN = datetime(2022, 1, 1, 0, 0, 0, 0).replace(tzinfo=pytz.utc)
TRIGGER_INTERVAL_ORIGIN_TIMESTAMP = TRIGGER_INTERVAL_ORIGIN.timestamp()
TRIGGER_INTERVAL_USR_MIN = 300  # seconds :MinTriggerInterval
TRIGGER_INTERVAL_ABS_MIN = 60  # seconds :MinTriggerInterval


class TriggerScheduleIterator:
    """Iterator for a time trigger schedule."""

    def __init__(self, trigger: Trigger, initial_now: datetime, keep: int = 10):
        self.trigger = trigger
        self.initial_now = initial_now.astimezone(pytz.timezone(trigger.timezone))
        self.offset = 0
        self.last_occurrence_initial: Optional[datetime] = None
        self.next_occurrences_buffer: typing.Deque[datetime] = deque(maxlen=keep)
        # iter state
        self._next: int | None = None
        self._croniter: croniter | None = None
        self._init()

    @property
    def type(self) -> ScheduleType:
        return self.trigger.schedule_type

    def _init(self):
        """Reset the iterator to its initial now."""

        self.offset = 0
        self.last_occurrence_initial = None
        self.next_occurrences_buffer.clear()

        # :TriggerSchedule
        if self.type == ScheduleType.INTERVAL:
            assert (self.trigger.interval or 0) >= TRIGGER_INTERVAL_ABS_MIN, "interval too small"
            self._next = TRIGGER_INTERVAL_ORIGIN_TIMESTAMP
            previous = self._next
            initial_timestamp = self.initial_now.timestamp()
            while self._next < initial_timestamp:
                previous = self._next
                self._next += self.trigger.interval
            self.last_occurrence_initial = datetime.fromtimestamp(previous, tz=pytz.utc)
        elif self.type == ScheduleType.CRON:
            if not croniter.is_valid(self.trigger.cron):
                raise ValueError(f"invalid cron expression in {self.trigger}: {self.trigger.cron}")
            self._croniter = croniter(
                self.trigger.cron, self.initial_now, max_years_between_matches=2
            )
            self.last_occurrence_initial = croniter(self.trigger.cron, self.initial_now).get_prev(
                datetime
            )
        else:
            raise ValueError(f"unexpected schedule type in {self.trigger}: {self.type}")

    def advance(self, n: int = 1) -> list[datetime]:
        """Advance the iterator by n steps and return the next n occurrences."""
        # :TriggerSchedule

        if self.type == ScheduleType.INTERVAL:
            next_occurrences = [
                datetime.fromtimestamp(self._next + self.trigger.interval * i, tz=pytz.utc)
                for i in range(n)
            ]
            self._next += self.trigger.interval * n
            # timezone doesn't matter here since we use a common origin time
            # will matter once we support in-interval offsets (e.g. every 3 days at 10:00)
        elif self.type == ScheduleType.CRON:
            next_occurrences = [self._croniter.get_next(datetime) for _ in range(n)]
        else:
            raise ValueError(f"unexpected schedule type in {self.trigger}: {self.type}")

        self.offset += n
        for occurrence in next_occurrences:
            self.next_occurrences_buffer.append(occurrence)

        return next_occurrences

    def next(self) -> datetime:
        """Return the next occurrence."""
        return self.advance(n=1)[0]


def is_time_trigger_equal(a: Trigger, b: Trigger) -> bool:
    """Checks if two time triggers are identical (as pertaining to their schedule)."""

    if a.schedule_type != b.schedule_type:
        return False
    if a.timezone != b.timezone:
        return False
    if a.schedule_type == ScheduleType.INTERVAL:
        return a.interval == b.interval
    if a.schedule_type == ScheduleType.CRON:
        return a.cron == b.cron
    return False
