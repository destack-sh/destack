from collections import deque
from datetime import datetime
from typing import TYPE_CHECKING, Collection, Deque, Optional

import pytz
from croniter import croniter

from bench.language.const import NodeType, ScheduleType, TriggerType
from bench.language.node import (
    Node,
    NodeList,
    node,
    p_child,
    node_component,
    p_parent,
    p_tracked,
)
from bench.language.validation import ValidationHandler, enum_validator, int_range_validator
from bench.utils.func import dict_minus

if TYPE_CHECKING:
    from bench.language.block import Block

# :TriggerSchedule
TRIGGER_INTERVAL_ORIGIN = datetime(2022, 1, 1, 0, 0, 0, 0).replace(tzinfo=pytz.utc)
TRIGGER_INTERVAL_ORIGIN_TIMESTAMP = TRIGGER_INTERVAL_ORIGIN.timestamp()
TRIGGER_INTERVAL_USR_MIN = 60  # seconds :MinTriggerInterval
TRIGGER_INTERVAL_ABS_MAX = 60 * 60 * 24 * 365  # seconds :MaxTriggerInterval
TRIGGER_INTERVAL_ABS_MIN = 60  # seconds :MinTriggerInterval


@node(NodeType.TRIGGER)
class Trigger(Node):
    """A trigger for a block to run."""

    parent: "Block" = p_parent(4, NodeType.BLOCK)
    type: TriggerType = p_tracked(30, require=True, validate=enum_validator(TriggerType))
    name: str | None = p_tracked(31, default=None)
    active: bool = p_tracked(32, default=True)
    schedule_type: Optional[ScheduleType] = p_tracked(
        33, default=None, validate=enum_validator(ScheduleType)
    )
    timezone: Optional[str] = p_tracked(34, default=pytz.utc.zone)
    interval: Optional[int] = p_tracked(
        35,
        default=None,
        validate=int_range_validator(TRIGGER_INTERVAL_USR_MIN, TRIGGER_INTERVAL_ABS_MAX),
    )
    cron: Optional[str] = p_tracked(36, default=None)

    @staticmethod
    def new(
        type: TriggerType = TriggerType.TIME, *args, for_parent: "Block" = None, **kwargs
    ) -> "Trigger":
        if type == TriggerType.TIME:
            return Trigger.time(*args, **kwargs)
        else:
            return Trigger(type=type, *args, **kwargs)

    @staticmethod
    def time(schedule: str | int, **kwargs) -> "Trigger":
        if isinstance(schedule, str):
            return Trigger(
                type=TriggerType.TIME, schedule_type=ScheduleType.CRON, cron=schedule, **kwargs
            )
        elif isinstance(schedule, int):
            return Trigger(
                type=TriggerType.TIME,
                schedule_type=ScheduleType.INTERVAL,
                interval=schedule,
                **kwargs,
            )
        else:
            raise ValueError(f"invalid schedule: {schedule}")

    @staticmethod
    def to_python(node, props: dict, for_parent: "Block" = None) -> tuple[str, dict, dict]:
        if node.type == TriggerType.TIME:
            return (
                "Trigger.time",
                {"schedule": node.cron or node.interval},
                dict_minus(props, "type", "cron", "interval"),
            )
        else:
            raise ValueError(f"unexpected trigger type: {node!r}")

    def __content_str__(self):
        if self.type == TriggerType.TIME:
            schedule_str = (
                self.interval if self.schedule_type == ScheduleType.INTERVAL else self.cron
            )
            content_str = f"{self.schedule_type} {self.timezone} {schedule_str}"
        else:
            content_str = None
        return f"{self.type} {content_str or '<none>'}"

    # ignore scope and block for now

    def _validate_inner(self, properties: Collection[str], on_invalid: "ValidationHandler") -> None:
        if self.type == TriggerType.TIME:
            if self.schedule_type == ScheduleType.CRON:
                if not croniter.is_valid(self.cron):
                    on_invalid(self, f"cron: invalid expression ('{self.cron}')", ["cron"])
            elif self.schedule_type == ScheduleType.INTERVAL:
                interval = self.interval or 0
                if interval < TRIGGER_INTERVAL_USR_MIN or interval > TRIGGER_INTERVAL_ABS_MAX:
                    on_invalid(
                        self,
                        f"interval: invalid ({interval} not in [{TRIGGER_INTERVAL_USR_MIN}, {TRIGGER_INTERVAL_ABS_MAX}])",
                        ["interval"],
                    )


@node_component
class HasTriggers(Node):
    triggers: NodeList[Trigger] = p_child(NodeType.TRIGGER)


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

    def advance(self, n: int) -> list[datetime]:
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
