from datetime import timedelta
from typing import TYPE_CHECKING, Optional

import pytz

from bench.language.core import (
    ScheduleType,
    Struct,
    StructType,
    TimeInterval,
    TypeConstraintIn,
    p_regular,
    struct_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

# NOTE :UX :Architecture: check out iCalendar spec / dateutil for Schedule


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
