from datetime import datetime
from typing import TYPE_CHECKING

from dateutil.rrule import rrule

from bench.language.core import (
    BuiltinEnum,
    Day,
    EnumType,
    Month,
    Struct,
    StructType,
    enum_,
    property_,
    struct_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SCHEDULE_FREQUENCY)
class ScheduleFrequency(BuiltinEnum):
    YEAR = 1
    MONTH = 2
    WEEK = 3
    DAY = 4
    HOUR = 5
    MINUTE = 6


@struct_(StructType.SCHEDULE)
class Schedule(Struct):
    """The time-based schedule of something (compatible with rrule)."""

    frequency: ScheduleFrequency = property_(30)
    interval: int = property_(31, default=1)
    start: datetime | None = property_(32)
    end: datetime | None = property_(33)
    count: int | None = property_(34)
    week_start: Day | None = property_(35)
    by_set_pos: list[int] = property_(36)
    by_month: list[Month] = property_(37)
    by_month_day: list[int] = property_(38)
    by_year_day: list[int] = property_(39)
    by_easter: list[int] = property_(40)
    by_week_no: list[int] = property_(41)
    by_week_day: list[Day] = property_(42)
    by_hour: list[int] = property_(43)
    by_minute: list[int] = property_(44)
    by_second: list[int] = property_(45)

    @staticmethod
    def from_rrule(rrule: rrule) -> "Schedule":
        """Converts an rrule to a Schedule."""
        return schedule_from_rrule(rrule)

    @staticmethod
    def every(
        interval: int = 1,
        frequency: ScheduleFrequency = ScheduleFrequency.DAY,
        *,
        start: datetime | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        return Schedule(frequency=frequency, interval=interval, start=start, end=end, count=count)

    @staticmethod
    def yearly(
        *,
        start: datetime | None = None,
        months: list[Month] | None = None,
        days: list[int] | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        schedule = Schedule(
            frequency=ScheduleFrequency.YEAR,
            start=start,
            end=end,
            count=count,
            by_month=months or [],
            by_month_day=days or [],
        )
        return schedule

    @staticmethod
    def monthly(
        *,
        start: datetime | None = None,
        days: list[int] | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        schedule = Schedule(
            frequency=ScheduleFrequency.MONTH,
            start=start,
            end=end,
            count=count,
            by_month_day=days or [],
        )
        return schedule

    @staticmethod
    def weekly(
        *,
        start: datetime | None = None,
        weekdays: list[Day] | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        schedule = Schedule(
            frequency=ScheduleFrequency.WEEK,
            start=start,
            end=end,
            count=count,
            by_week_day=weekdays or [],
        )
        return schedule

    @staticmethod
    def daily(
        *,
        start: datetime | None = None,
        hours: list[int] | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        schedule = Schedule(
            frequency=ScheduleFrequency.DAY,
            start=start,
            end=end,
            count=count,
            by_hour=hours or [],
        )
        return schedule

    @staticmethod
    def hourly(
        *,
        start: datetime | None = None,
        minutes: list[int] | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        schedule = Schedule(
            frequency=ScheduleFrequency.HOUR,
            start=start,
            end=end,
            count=count,
            by_minute=minutes or [],
        )
        return schedule

    @staticmethod
    def minutely(
        *,
        start: datetime | None = None,
        seconds: list[int] | None = None,
        end: datetime | None = None,
        count: int | None = None,
    ) -> "Schedule":
        schedule = Schedule(
            frequency=ScheduleFrequency.MINUTE,
            start=start,
            end=end,
            count=count,
            by_second=seconds or [],
        )
        return schedule


def schedule_to_rrule(schedule: Schedule) -> rrule:
    """Converts a Schedule to an rrule."""
    return rrule(
        freq=schedule.frequency.value,
        interval=schedule.interval,
        dtstart=schedule.start,
        until=schedule.end,
        count=schedule.count,
        wkst=schedule.week_start,
        bysetpos=schedule.by_set_pos,
        bymonth=schedule.by_month,
        bymonthday=schedule.by_month_day,
        byyearday=schedule.by_year_day,
        byeaster=schedule.by_easter,
        byweekno=schedule.by_week_no,
        byweekday=schedule.by_week_day,
        byhour=schedule.by_hour,
        byminute=schedule.by_minute,
        bysecond=schedule.by_second,
    )


def schedule_from_rrule(rrule: rrule) -> Schedule:
    """Converts an rrule to a Schedule."""
    return Schedule(
        frequency=ScheduleFrequency(rrule._freq),  # type: ignore
        interval=rrule._interval,  # type: ignore
        start=rrule._dtstart,  # type: ignore
        end=rrule._until,  # type: ignore
        count=rrule._count,  # type: ignore
        week_start=rrule._wkst,  # type: ignore
        by_set_pos=rrule._bysetpos,  # type: ignore
        by_month=rrule._bymonth,  # type: ignore
        by_month_day=rrule._bymonthday,  # type: ignore
        by_year_day=rrule._byyearday,  # type: ignore
        by_easter=rrule._byeaster,  # type: ignore
        by_week_no=rrule._byweekno,  # type: ignore
        by_week_day=rrule._byweekday,  # type: ignore
        by_hour=rrule._byhour,  # type: ignore
        by_minute=rrule._byminute,  # type: ignore
        by_second=rrule._bysecond,  # type: ignore
    )


def render_schedule_as_rrule(schedule: Schedule) -> str:
    """Converts a Schedule to an RFC5545 RRULE string."""
    output = []
    if schedule.start:
        output.append(schedule.start.strftime("DTSTART:%Y%m%dT%H%M%S"))

    parts = [f"FREQ={schedule.frequency.name}"]
    if schedule.interval != 1:
        parts.append(f"INTERVAL={schedule.interval}")

    if schedule.week_start:
        parts.append(f"WKST={schedule.week_start.name[:2]}")

    if schedule.count is not None:
        parts.append(f"COUNT={schedule.count}")

    if schedule.end:
        parts.append(schedule.end.strftime("UNTIL=%Y%m%dT%H%M%S"))

    partfmt = "{name}={vals}"
    for name, values in [
        ("BYSETPOS", schedule.by_set_pos),
        ("BYMONTH", [m.value for m in schedule.by_month]),
        ("BYMONTHDAY", schedule.by_month_day),
        ("BYYEARDAY", schedule.by_year_day),
        ("BYWEEKNO", schedule.by_week_no),
        ("BYDAY", schedule.by_week_day),
        ("BYHOUR", schedule.by_hour),
        ("BYMINUTE", schedule.by_minute),
        ("BYSECOND", schedule.by_second),
        ("BYEASTER", schedule.by_easter),
    ]:
        if values:
            parts.append(partfmt.format(name=name, vals=",".join(str(v) for v in values)))

    output.append("RRULE:" + ";".join(parts))
    return "\n".join(output)
