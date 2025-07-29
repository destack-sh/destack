from datetime import datetime
from typing import TYPE_CHECKING, final

from destack.language.core import (
    Enum,
    EnumType,
    Struct,
    StructType,
    UInt8,
    UInt16,
    UInt32,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.DAY_OF_WEEK)
class DayOfWeek(Enum):
    MONDAY = 1
    TUESDAY = 2
    WEDNESDAY = 3
    THURSDAY = 4
    FRIDAY = 5
    SATURDAY = 6
    SUNDAY = 7


@builtin_enum(EnumType.MONTH)
class Month(Enum):
    JANUARY = 1
    FEBRUARY = 2
    MARCH = 3
    APRIL = 4
    MAY = 5
    JUNE = 6
    JULY = 7
    AUGUST = 8
    SEPTEMBER = 9
    OCTOBER = 10
    NOVEMBER = 11
    DECEMBER = 12


@builtin_enum(EnumType.SCHEDULE_FREQUENCY)
class ScheduleFrequency(Enum):
    YEAR = 1
    MONTH = 2
    WEEK = 3
    DAY = 4
    HOUR = 5
    MINUTE = 6


@builtin_struct(StructType.SCHEDULE, is_final=True)
@final
class Schedule(Struct):
    """The time-based schedule of something (compatible with rrule)."""

    frequency: ScheduleFrequency = builtin_property(101)
    interval: UInt32 = builtin_property(102, default=1)
    start: datetime | None = builtin_property(110)
    end: datetime | None = builtin_property(111)
    count: UInt32 | None = builtin_property(112)
    week_start: DayOfWeek | None = builtin_property(113)
    by_set_pos: list[UInt32] | None = builtin_property(114)
    by_month: list[Month] | None = builtin_property(115)
    by_month_day: list[UInt8] | None = builtin_property(116)
    by_year_day: list[UInt16] | None = builtin_property(117)
    by_easter: list[UInt8] | None = builtin_property(118)
    by_week_no: list[UInt8] | None = builtin_property(119)
    by_week_day: list[DayOfWeek] | None = builtin_property(120)
    by_hour: list[UInt8] | None = builtin_property(121)
    by_minute: list[UInt8] | None = builtin_property(122)
    by_second: list[UInt8] | None = builtin_property(123)

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
        weekdays: list[DayOfWeek] | None = None,
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
