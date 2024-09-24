from datetime import timedelta
from typing import TYPE_CHECKING, Collection, Optional, Union

import pytz
from croniter import croniter

from bench.language.const import (
    BlockType,
    NodeType,
    ScheduleType,
    StructType,
    TimeInterval,
    TriggerType,
)
from bench.language.node import (
    BuiltinObject,
    SourceNode,
    Struct,
    local_node_,
    object_,
    struct_,
)
from bench.language.property import Property, p_node_parent, p_regular
from bench.language.validation import (
    NAME_CONSTRAINT,
    TypeConstraintIn,
    ValidationHandler,
    constraint,
)
from bench.proto.wire import TriggerData

if TYPE_CHECKING:
    from bench.language import Block, Expression

# pyright: reportIncompatibleVariableOverride=false

# :TriggerSchedule


@struct_(StructType.SCHEDULE)
class Schedule(Struct):
    """The time-based schedule of something."""

    type: ScheduleType = p_regular(30, require=True)
    timezone: str = p_regular(31, default=pytz.utc.zone)

    # interval
    every: int = p_regular(40, default=1, constraint=TypeConstraintIn(min_value=1, max_value=60))
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
            return "<unknown>"

    def _validate_component(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        if self.type == ScheduleType.CRON:
            if not self.cron or not croniter.is_valid(self.cron):
                invalid(self, f"invalid cron ('{self.cron}')", (Schedule.cron,))
        if self.offset is not None:
            if self.offset.days < 0:
                invalid(self, "negative offset ('{self.offset}')", (Schedule.offset,))
            elif self.offset.days > 7:
                invalid(self, "offset too large ('{self.offset}')", (Schedule.offset,))


@object_()
class TriggerBase(BuiltinObject):
    """A trigger to something."""

    # state
    processed_epoch: Optional[int] = p_regular(40, default=None)

    # trigger
    schedule: Optional[Schedule] = p_regular(
        50, default=None, require=False, array=False, struct=StructType.SCHEDULE
    )
    signal: Optional["Block"] = p_regular(
        51,
        default=None,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.SIGNAL]),
    )
    condition: Optional["Expression"] = p_regular(
        52, default=None, require=False, array=False, struct=StructType.EXPRESSION
    )


@struct_(StructType.TRIGGER_INFO)
class TriggerInfo(Struct, TriggerBase):
    """The basic information describing a trigger."""

    pass


@local_node_(NodeType.TRIGGER)
class Trigger(SourceNode[TriggerData], TriggerBase):
    """A trigger to run the node it is attached to (like a Block)."""

    parent: Union["Block", None] = p_node_parent(4, NodeType.BLOCK)
    type: TriggerType = p_regular(30, require=True)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)

    # ...TriggerBase[40-59]

    # flags
    is_paused: bool = p_regular(60, default=False)

    def __content_str__(self):
        if self.type == TriggerType.SCHEDULE and self.schedule:
            content_str = self.schedule.__content_str__()
        elif self.type == TriggerType.SIGNAL and self.signal:
            content_str = self.signal.absolute_path
        else:
            content_str = None
        return f"{self.type} {content_str or '<none>'}"
