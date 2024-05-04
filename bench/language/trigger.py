from collections import deque
from datetime import datetime
from typing import TYPE_CHECKING, Any, Deque, Optional, cast

import pytz
from croniter import croniter

from bench.language.const import NodeType, ScheduleType, StructType, TriggerType
from bench.language.graph import NodeList
from bench.language.node import Node, Struct, node, struct
from bench.language.notice import Notice
from bench.language.property import Property, p_node_child, p_node_parent, p_regular
from bench.language.validation import NAME_CONSTRAINT, ValidationHandler
from bench.proto.wire import TriggerData

if TYPE_CHECKING:
    from bench.language.block import Block

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false

# :TriggerSchedule
TRIGGER_INTERVAL_ORIGIN = datetime(2022, 1, 1, 0, 0, 0, 0).replace(tzinfo=pytz.utc)
TRIGGER_INTERVAL_ORIGIN_TIMESTAMP = TRIGGER_INTERVAL_ORIGIN.timestamp()


@struct(StructType.SCHEDULE)
class Schedule(Struct):
    """The time-based schedule of something."""

    type: ScheduleType = p_regular(30, require=True)
    timezone: Optional[str] = p_regular(31, default=pytz.utc.zone)
    interval: Optional[int] = p_regular(32, default=None)
    cron: Optional[str] = p_regular(33, default=None)

    def __content_str__(self) -> str:
        return f"{self.type} {self.timezone} {self.interval or self.cron}"

    def _validate_component(
        self, properties: tuple[Property, ...], invalid: "ValidationHandler"
    ) -> None:
        if self.type == ScheduleType.CRON:
            if not self.cron or not croniter.is_valid(self.cron):
                invalid(self, f"cron: invalid expression ('{self.cron}')", (Schedule.cron,))


@node(NodeType.TRIGGER)
class Trigger(Node[TriggerData]):
    parent: "Block" = p_node_parent(4, NodeType.BLOCK)  # type: ignore
    type: TriggerType = p_regular(30, require=True)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    active: bool = p_regular(32, default=True)
    schedule: Optional[Schedule] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.SCHEDULE
    )
    signal: Optional["Block"] = p_regular(
        34, default=None, require=False, array=False, references=NodeType.BLOCK
    )
    # cursor, filter, ...

    notices: NodeList["Notice"] = p_node_child(NodeType.NOTICE)

    def __content_str__(self):
        if self.type == TriggerType.SCHEDULE and self.schedule:
            content_str = self.schedule.__content_str__()
        elif self.type == TriggerType.SIGNAL and self.signal:
            content_str = self.signal.absolute_path
        else:
            content_str = None
        return f"{self.type} {content_str or '<none>'}"


class ScheduleIterator:
    """Iterator through a Schedule."""

    def __init__(self, schedule: Schedule, initial_now: datetime, keep: int = 10):
        self.schedule = schedule
        self.initial_now = initial_now.astimezone(pytz.timezone(cast(Any, schedule.timezone)))
        self.offset = 0
        self.last_occurrence_initial: Optional[datetime] = None
        self.next_occurrences_buffer: Deque[datetime] = deque(maxlen=keep)
        # iter state
        self._next: float | None = None
        self._croniter: croniter | None = None
        self._init()

    @property
    def type(self) -> ScheduleType:
        return self.schedule.type

    def _init(self):
        """Reset the iterator to its initial now."""

        self.offset = 0
        self.last_occurrence_initial = None
        self.next_occurrences_buffer.clear()

        # :TriggerSchedule
        if self.type == ScheduleType.INTERVAL:
            assert self.schedule.interval is not None, f"interval is None in {self.schedule}"
            self._next = TRIGGER_INTERVAL_ORIGIN_TIMESTAMP
            previous = self._next
            initial_timestamp = self.initial_now.timestamp()
            while self._next < initial_timestamp:
                previous = self._next
                self._next += self.schedule.interval
            self.last_occurrence_initial = datetime.fromtimestamp(previous, tz=pytz.utc)
        elif self.type == ScheduleType.CRON:
            assert self.schedule.cron is not None, f"cron is None in {self.schedule}"
            if not croniter.is_valid(self.schedule.cron):
                raise ValueError(
                    f"invalid cron expression in {self.schedule}: {self.schedule.cron}"
                )
            self._croniter = croniter(
                self.schedule.cron, self.initial_now, max_years_between_matches=2
            )
            self.last_occurrence_initial = croniter(self.schedule.cron, self.initial_now).get_prev(
                datetime
            )
        else:
            raise ValueError(f"unexpected schedule type in {self.schedule}: {self.type}")

    def advance(self, n: int) -> list[datetime]:
        """Advance the iterator by n steps and return the next n occurrences."""
        # :TriggerSchedule

        if self.type == ScheduleType.INTERVAL:
            assert self._next is not None, "not init"
            assert self.schedule.interval is not None, f"interval is None in {self.schedule}"
            next_occurrences = [
                datetime.fromtimestamp(self._next + self.schedule.interval * i, tz=pytz.utc)
                for i in range(n)
            ]
            self._next += self.schedule.interval * n
            # timezone doesn't matter here since we use a common origin time
            # will matter once we support in-interval offsets (e.g. every 3 days at 10:00)
        elif self.type == ScheduleType.CRON:
            assert self._croniter is not None
            next_occurrences = [self._croniter.get_next(datetime) for _ in range(n)]
        else:
            raise ValueError(f"unexpected schedule type in {self.schedule}: {self.type}")

        self.offset += n
        for occurrence in next_occurrences:
            self.next_occurrences_buffer.append(occurrence)

        return next_occurrences

    def next(self) -> datetime:
        """Return the next occurrence."""
        return self.advance(n=1)[0]
