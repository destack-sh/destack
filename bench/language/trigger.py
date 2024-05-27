from collections import deque
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Collection, Deque, Optional, Union, cast

import pytz
from croniter import croniter

from bench.language.const import NodeType, ScheduleType, StructType, TimeInterval, TriggerType
from bench.language.field import TypeConstraint
from bench.language.graph import NodeList
from bench.language.node import Node, Struct, node, struct
from bench.language.notice import Notice
from bench.language.property import Property, p_node_child, p_node_parent, p_regular
from bench.language.validation import NAME_CONSTRAINT, ValidationHandler
from bench.proto.wire import TriggerData

if TYPE_CHECKING:
    from bench.language import Block, Expression, Step

# pyright: reportIncompatibleVariableOverride=false

# :TriggerSchedule


@struct(StructType.SCHEDULE)
class Schedule(Struct):
    """The time-based schedule of something."""

    type: ScheduleType = p_regular(30, require=True)
    timezone: Optional[str] = p_regular(31, default=pytz.utc.zone)

    # interval
    every: int = p_regular(40, default=1, constraint=TypeConstraint(min_value=0, max_value=60))
    interval: TimeInterval = p_regular(41, default=TimeInterval.DAY)
    offset: Optional[timedelta] = p_regular(42, default=None)

    # cron
    cron: Optional[str] = p_regular(50, default=None)

    def __content_str__(self) -> str:
        if self.type == ScheduleType.INTERVAL:
            offset_str = f" at {self.offset}" if self.offset else ""
            return f"every {self.every} {self.interval.bench_name}{offset_str}"
        elif self.type == ScheduleType.CRON:
            return self.cron or "???"
        else:
            return "???"

    def _validate_component(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        if self.type == ScheduleType.CRON:
            if not self.cron or not croniter.is_valid(self.cron):
                invalid(self, f"invalid cron ('{self.cron}')", (Schedule.cron,))


@node(NodeType.TRIGGER)
class Trigger(Node[TriggerData]):
    """A trigger to run the node it is attached to (like a Block or Step)."""

    parent: Union["Block", "Step"] = p_node_parent(4, NodeType.BLOCK, NodeType.STEP)
    type: TriggerType = p_regular(30, require=True)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    processed_epoch: Optional[int] = p_regular(32, default=None)

    # content
    schedule: Optional[Schedule] = p_regular(
        40, default=None, require=False, array=False, struct=StructType.SCHEDULE
    )
    signal: Optional["Block"] = p_regular(
        41, default=None, require=False, array=False, references=NodeType.BLOCK
    )
    condition: Optional["Expression"] = p_regular(
        42, default=None, require=False, array=False, struct=StructType.EXPRESSION
    )

    # flags
    is_paused: bool = p_regular(50, default=False)

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
    """Iterate through a Schedule."""

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
        if self.type == ScheduleType.CRON:
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

        if self.type == ScheduleType.CRON:
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
