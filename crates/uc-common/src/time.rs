use chrono::Utc;

/// Epoch milliseconds, matching the `int64` shape used throughout the upstream
/// spec for `created_at` / `updated_at`.
pub type EpochMillis = i64;

pub fn now_epoch_millis() -> EpochMillis {
    Utc::now().timestamp_millis()
}
