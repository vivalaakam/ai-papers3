use chrono::Timelike;
use embedded_sdmmc::{TimeSource, Timestamp};

#[derive(Debug)]
pub struct Clock;

impl TimeSource for Clock {
    fn get_timestamp(&self) -> Timestamp {
        use chrono::Datelike;
        let local: chrono::DateTime<chrono::Local> = chrono::Local::now();
        Timestamp {
            year_since_1970: (local.year() - 1970) as u8,
            zero_indexed_month: local.month0() as u8,
            zero_indexed_day: local.day0() as u8,
            hours: local.hour() as u8,
            minutes: local.minute() as u8,
            seconds: local.second() as u8,
        }
    }
}
