//! destack.basics.script.schedule@2025.08.15.1

#![destack::partial(destack.basics.script.schedule, file)]

#[destack::generated(Schedule, struct, block)]
/// The time-based schedule of something (compatible with rrule).
pub struct Schedule {}

#[destack::generated(DayOfWeek, enum, block)]
/// DayOfWeek
pub enum DayOfWeek {
    /// Monday
    Monday = 1,
    /// Tuesday
    Tuesday = 2,
    /// Wednesday
    Wednesday = 3,
    /// Thursday
    Thursday = 4,
    /// Friday
    Friday = 5,
    /// Saturday
    Saturday = 6,
    /// Sunday
    Sunday = 7,
}

#[destack::generated(Month, enum, block)]
/// Month
pub enum Month {
    /// January
    January = 1,
    /// February
    February = 2,
    /// March
    March = 3,
    /// April
    April = 4,
    /// May
    May = 5,
    /// June
    June = 6,
    /// July
    July = 7,
    /// August
    August = 8,
    /// September
    September = 9,
    /// October
    October = 10,
    /// November
    November = 11,
    /// December
    December = 12,
}

#[destack::generated(ScheduleFrequency, enum, block)]
/// ScheduleFrequency
pub enum ScheduleFrequency {
    /// Yearly
    Year = 1,
    /// Monthly
    Month = 2,
    /// Weekly
    Week = 3,
    /// Daily
    Day = 4,
    /// Hourly
    Hour = 5,
    /// Minutely
    Minute = 6,
}
