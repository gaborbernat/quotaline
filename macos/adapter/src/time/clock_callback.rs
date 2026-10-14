//! Declares the Swift formatter for locale-dependent calendar values.

#[uniffi::export(foreign)]
pub trait ClockFormatter: Send + Sync {
    fn utc_day(&self, at: i64) -> String;
    fn time(&self, at: i64) -> String;
    fn weekday_time(&self, at: i64) -> String;
    fn date_time(&self, at: i64) -> String;
    fn same_local_day(&self, at: i64, now: i64) -> bool;
}
