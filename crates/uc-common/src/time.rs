/// Epoch milliseconds, matching the `int64` shape used throughout the upstream
/// spec for `created_at` / `updated_at`.
pub type EpochMillis = i64;

pub fn now_epoch_millis() -> EpochMillis {
    jiff::Timestamp::now().as_millisecond()
}
