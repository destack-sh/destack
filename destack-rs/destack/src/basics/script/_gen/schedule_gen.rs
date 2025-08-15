//! destack.basics.script.schedule@2025.08.15.1

#![destack::generated(destack.basics.script.schedule, file)]

use crate::DayOfWeek;
use crate::Month;
use crate::Schedule;
use crate::ScheduleFrequency;

#[destack::generated(Schedule, Debug, block)]
impl std::fmt::Debug for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Schedule")
    }
}

#[destack::generated(DayOfWeek, Debug, block)]
impl std::fmt::Debug for DayOfWeek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DayOfWeek::Monday => write!(f, "MONDAY"),
            DayOfWeek::Tuesday => write!(f, "TUESDAY"),
            DayOfWeek::Wednesday => write!(f, "WEDNESDAY"),
            DayOfWeek::Thursday => write!(f, "THURSDAY"),
            DayOfWeek::Friday => write!(f, "FRIDAY"),
            DayOfWeek::Saturday => write!(f, "SATURDAY"),
            DayOfWeek::Sunday => write!(f, "SUNDAY"),
        }
    }
}

#[destack::generated(Month, Debug, block)]
impl std::fmt::Debug for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Month::January => write!(f, "JANUARY"),
            Month::February => write!(f, "FEBRUARY"),
            Month::March => write!(f, "MARCH"),
            Month::April => write!(f, "APRIL"),
            Month::May => write!(f, "MAY"),
            Month::June => write!(f, "JUNE"),
            Month::July => write!(f, "JULY"),
            Month::August => write!(f, "AUGUST"),
            Month::September => write!(f, "SEPTEMBER"),
            Month::October => write!(f, "OCTOBER"),
            Month::November => write!(f, "NOVEMBER"),
            Month::December => write!(f, "DECEMBER"),
        }
    }
}

#[destack::generated(ScheduleFrequency, Debug, block)]
impl std::fmt::Debug for ScheduleFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleFrequency::Year => write!(f, "YEAR"),
            ScheduleFrequency::Month => write!(f, "MONTH"),
            ScheduleFrequency::Week => write!(f, "WEEK"),
            ScheduleFrequency::Day => write!(f, "DAY"),
            ScheduleFrequency::Hour => write!(f, "HOUR"),
            ScheduleFrequency::Minute => write!(f, "MINUTE"),
        }
    }
}
