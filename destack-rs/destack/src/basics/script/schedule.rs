//! destack.basics.script.schedule@2025.08.15.1

#![destack::partial(destack.basics.script.schedule, file)]

#[destack::generated(Schedule, struct, block)]
/// The time-based schedule of something (compatible with rrule).
pub struct Schedule {

}

#[destack::generated(DayOfWeek, enum, block)]
/// DayOfWeek
pub enum DayOfWeek {
    /// Monday
    MONDAY = 1,
    /// Tuesday
    TUESDAY = 2,
    /// Wednesday
    WEDNESDAY = 3,
    /// Thursday
    THURSDAY = 4,
    /// Friday
    FRIDAY = 5,
    /// Saturday
    SATURDAY = 6,
    /// Sunday
    SUNDAY = 7
}

#[destack::generated(Month, enum, block)]
/// Month
pub enum Month {
    /// January
    JANUARY = 1,
    /// February
    FEBRUARY = 2,
    /// March
    MARCH = 3,
    /// April
    APRIL = 4,
    /// May
    MAY = 5,
    /// June
    JUNE = 6,
    /// July
    JULY = 7,
    /// August
    AUGUST = 8,
    /// September
    SEPTEMBER = 9,
    /// October
    OCTOBER = 10,
    /// November
    NOVEMBER = 11,
    /// December
    DECEMBER = 12
}

#[destack::generated(ScheduleFrequency, enum, block)]
/// ScheduleFrequency
pub enum ScheduleFrequency {
    /// Yearly
    YEAR = 1,
    /// Monthly
    MONTH = 2,
    /// Weekly
    WEEK = 3,
    /// Daily
    DAY = 4,
    /// Hourly
    HOUR = 5,
    /// Minutely
    MINUTE = 6
}