use std::collections::HashMap;

use chrono::{DateTime, Datelike, Local, Timelike, Weekday};
use serde_json::{to_value, Value};

/// Extract fields related to time.
pub fn time_fields(time: &DateTime<Local>) -> HashMap<String, Value> {
    let mut map = HashMap::new();

    let weekday = match time.weekday() {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    };

    let month = match time.month() {
        1 => "january",
        2 => "february",
        3 => "march",
        4 => "april",
        5 => "may",
        6 => "june",
        7 => "july",
        8 => "august",
        9 => "september",
        10 => "october",
        11 => "november",
        _ => "december",
    };

    let daypart = match time.hour() {
        6..=11 => "morning",
        12..=13 => "noon",
        14..=17 => "afternoon",
        18..=22 => "evening",
        _ => "night",
    };

    let season = match time.month() {
        3 | 4 | 5 => "spring",
        6 | 7 | 8 => "summer",
        9 | 10 | 11 => "autumn",
        _ => "winter",
    };

    map.insert("daypart".to_string(), to_value(daypart).unwrap());
    map.insert("month".to_string(), to_value(month).unwrap());
    map.insert("weekday".to_string(), to_value(weekday).unwrap());
    map.insert("season".to_string(), to_value(season).unwrap());

    map
}
