from datetime import datetime
from typing import TYPE_CHECKING, final

from destack.core import (
    EnumType,
    OptionEnum,
    Struct,
    StructType,
    UInt8,
    UInt16,
    UInt32,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.DAY_OF_WEEK)
class DayOfWeek(OptionEnum):
    MONDAY = declare_option(1, "Monday", description="Monday")
    TUESDAY = declare_option(2, "Tuesday", description="Tuesday")
    WEDNESDAY = declare_option(3, "Wednesday", description="Wednesday")
    THURSDAY = declare_option(4, "Thursday", description="Thursday")
    FRIDAY = declare_option(5, "Friday", description="Friday")
    SATURDAY = declare_option(6, "Saturday", description="Saturday")
    SUNDAY = declare_option(7, "Sunday", description="Sunday")


@declare_enum(EnumType.MONTH)
class Month(OptionEnum):
    JANUARY = declare_option(1, "January", description="January")
    FEBRUARY = declare_option(2, "February", description="February")
    MARCH = declare_option(3, "March", description="March")
    APRIL = declare_option(4, "April", description="April")
    MAY = declare_option(5, "May", description="May")
    JUNE = declare_option(6, "June", description="June")
    JULY = declare_option(7, "July", description="July")
    AUGUST = declare_option(8, "August", description="August")
    SEPTEMBER = declare_option(9, "September", description="September")
    OCTOBER = declare_option(10, "October", description="October")
    NOVEMBER = declare_option(11, "November", description="November")
    DECEMBER = declare_option(12, "December", description="December")


@declare_enum(EnumType.SCHEDULE_FREQUENCY)
class ScheduleFrequency(OptionEnum):
    YEAR = declare_option(1, "Year", description="Yearly")
    MONTH = declare_option(2, "Month", description="Monthly")
    WEEK = declare_option(3, "Week", description="Weekly")
    DAY = declare_option(4, "Day", description="Daily")
    HOUR = declare_option(5, "Hour", description="Hourly")
    MINUTE = declare_option(6, "Minute", description="Minutely")


@declare_struct(StructType.SCHEDULE, is_final=True)
@final
class Schedule(Struct):
    """The time-based schedule of something (compatible with rrule)."""

    frequency: ScheduleFrequency = declare_property(101)
    interval: UInt32 = declare_property(102, default=1)
    start: datetime | None = declare_property(110)
    end: datetime | None = declare_property(111)
    count: UInt32 | None = declare_property(112)
    week_start: DayOfWeek | None = declare_property(113)
    by_set_pos: list[UInt32] | None = declare_property(114)
    by_month: list[Month] | None = declare_property(115)
    by_month_day: list[UInt8] | None = declare_property(116)
    by_year_day: list[UInt16] | None = declare_property(117)
    by_easter: list[UInt8] | None = declare_property(118)
    by_week_no: list[UInt8] | None = declare_property(119)
    by_week_day: list[DayOfWeek] | None = declare_property(120)
    by_hour: list[UInt8] | None = declare_property(121)
    by_minute: list[UInt8] | None = declare_property(122)
    by_second: list[UInt8] | None = declare_property(123)

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
