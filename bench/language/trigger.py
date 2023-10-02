from collections import deque
from datetime import datetime
from typing import TYPE_CHECKING, Collection, Deque, Mapping, Optional, Union
from uuid import UUID

import pytz
from croniter import croniter

from bench.language import IssueType
from bench.language.const import MNT, ScheduleType, TriggerType
from bench.language.module import (
    Node,
    NodeList,
    ScopeNode,
    nchildren,
    node,
    node_component,
    nparent,
    nproperty,
)
from bench.language.run import HasRun
from bench.language.validation import ValidationHandler, enum_validator

if TYPE_CHECKING:
    from bench.language.statement import Statement

# :TriggerSchedule
TRIGGER_INTERVAL_ORIGIN = datetime(2022, 1, 1, 0, 0, 0, 0).replace(tzinfo=pytz.utc)
TRIGGER_INTERVAL_ORIGIN_TIMESTAMP = TRIGGER_INTERVAL_ORIGIN.timestamp()
TRIGGER_INTERVAL_USR_MIN = 300  # seconds :MinTriggerInterval
TRIGGER_INTERVAL_ABS_MAX = 60 * 60 * 24 * 365  # seconds :MaxTriggerInterval
TRIGGER_INTERVAL_ABS_MIN = 60  # seconds :MinTriggerInterval


@node(mnt=MNT.Trigger)
class Trigger(Node):
    """A trigger for a runnable, possibly inside a flow."""

    parent: "Statement" = nparent(MNT.Statement)
    type: TriggerType = nproperty(is_required=True, validate=enum_validator(TriggerType))
    active: bool = nproperty(default=True)
    mapping: Optional[Mapping] = nproperty(default=None)
    schedule_type: Optional[ScheduleType] = nproperty(
        default=None, validate=enum_validator(ScheduleType)
    )
    timezone: Optional[str] = nproperty(default=pytz.utc.zone)
    interval: Optional[int] = nproperty(default=None)
    cron: Optional[str] = nproperty(default=None)
    runnable: Union["Statement", UUID, None] = nproperty(default=None)
    scope: Union["Node", UUID, None] = nproperty(default=None)

    @staticmethod
    def new(
        type: TriggerType = TriggerType.TIME, *args, for_parent: "Statement" = None, **kwargs
    ) -> "Trigger":
        if type == TriggerType.TIME:
            return Trigger.time(*args, **kwargs)
        else:
            return Trigger(type=type, *args, **kwargs)

    @staticmethod
    def time(schedule: str | int) -> "Trigger":
        if isinstance(schedule, str):
            return Trigger(type=TriggerType.TIME, schedule_type=ScheduleType.CRON, cron=schedule)
        elif isinstance(schedule, int):
            return Trigger(
                type=TriggerType.TIME, schedule_type=ScheduleType.INTERVAL, interval=schedule
            )
        else:
            raise ValueError(f"invalid schedule: {schedule}")

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

    def _clear_inner(self) -> None:
        self.runnable = self.runnable.id if isinstance(self.runnable, Node) else self.runnable
        self.scope = self.scope.id if isinstance(self.scope, Node) else self.scope

    def _interp_inner(self, scope: "ScopeNode") -> None:
        # resolve runnable
        if self.runnable is not None and not isinstance(self.runnable, HasRun):
            resolved = scope.lookup(self.runnable)
            if resolved is None:
                self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
            else:
                self.runnable = resolved
        # resolve scope
        if self.scope is not None and not isinstance(self.scope, Node):
            resolved = scope.lookup(self.scope)
            if resolved is None:
                self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
            else:
                self.scope = resolved

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        if self.type == TriggerType.TIME:
            if self.schedule_type == ScheduleType.CRON:
                if not croniter.is_valid(self.cron):
                    on_issue(self, f"cron: invalid expression ('{self.cron}')", ["cron"])
            elif self.schedule_type == ScheduleType.INTERVAL:
                interval = self.interval or 0
                if interval < TRIGGER_INTERVAL_USR_MIN or interval > TRIGGER_INTERVAL_ABS_MAX:
                    on_issue(
                        self,
                        f"interval: invalid ({interval} not in [{TRIGGER_INTERVAL_USR_MIN}, {TRIGGER_INTERVAL_ABS_MAX}])",
                        ["interval"],
                    )


@node_component
class HasTriggers(Node):
    """A symbol that can participate in a flow."""

    triggers: NodeList[Trigger] = nchildren(MNT.Trigger)


class TriggerScheduleIterator:
    """Iterator for a time trigger schedule."""

    def __init__(self, trigger: Trigger, initial_now: datetime, keep: int = 10):
        self.trigger = trigger
        self.initial_now = initial_now.astimezone(pytz.timezone(trigger.timezone))
        self.offset = 0
        self.last_occurrence_initial: Optional[datetime] = None
        self.next_occurrences_buffer: Deque[datetime] = deque(maxlen=keep)
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
