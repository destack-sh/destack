//! destack.basics.script.schedule

#![destack::partial(destack.basics.script.schedule, file)]

use crate::Timestamp;

#[destack::generated(Schedule, -, block)]
/// The time-based schedule of something (compatible with rrule).
pub struct Schedule {
    pub frequency: ScheduleFrequency,
    pub interval: u32,
    pub start: Option<Timestamp>,
    pub end: Option<Timestamp>,
    pub count: Option<u32>,
    pub week_start: Option<DayOfWeek>,
    pub by_set_pos: Option<Vec<u32>>,
    pub by_month: Option<Vec<Month>>,
    pub by_month_day: Option<Vec<u8>>,
    pub by_year_day: Option<Vec<u16>>,
    pub by_easter: Option<Vec<u8>>,
    pub by_week_no: Option<Vec<u8>>,
    pub by_week_day: Option<Vec<DayOfWeek>>,
    pub by_hour: Option<Vec<u8>>,
    pub by_minute: Option<Vec<u8>>,
    pub by_second: Option<Vec<u8>>,
}

#[destack::generated(DayOfWeek, -, block)]
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

#[destack::generated(Month, -, block)]
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

#[destack::generated(ScheduleFrequency, -, block)]
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
