//! The bounded audit-event channel and its drainer task.
//!
//! [`audit_layer`] records one event per mutating request without blocking
//! the response: one long-lived drainer task, fed by a bounded mpsc channel,
//! writes the events, so a stalled audit DB cannot pile up pending tasks.
//! It takes all waiting events at once, up to [`BATCH`], and writes them in
//! one statement. On overflow the event is dropped, with a warning in the
//! log.
//!
//! [`audit_layer`]: crate::observability::audit::layer

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use tokio::sync::mpsc;
use uc_api::AuditEvent;
use uc_common::now_epoch_millis;
use uc_metastore::DynMetastore;

/// Default channel capacity: enough for a short audit-DB stall, small
/// enough to cap memory in a long one.
pub const DEFAULT_CAPACITY: usize = 1024;

/// The most events in one write.
pub const BATCH: usize = 128;

/// Minimum gap between warnings about dropped audit events: at most one
/// line, with the drop count, per interval.
const DROP_WARN_INTERVAL_MS: i64 = 1000;

/// Producer-side handle for the audit drainer. It keeps tokio's channel
/// type out of `AppState`.
#[derive(Clone)]
pub struct AuditHandle {
    tx: mpsc::Sender<AuditEvent>,
}

impl AuditHandle {
    /// Submit without waiting. A full or closed channel drops the event and
    /// counts it for the next throttled warning.
    pub fn try_record(&self, ev: AuditEvent) {
        use mpsc::error::TrySendError;
        match self.tx.try_send(ev) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => self.record_drop("channel full"),
            Err(TrySendError::Closed(_)) => self.record_drop("channel closed (drainer panicked?)"),
        }
    }

    fn record_drop(&self, reason: &'static str) {
        DROPS_SINCE_LAST_LOG.fetch_add(1, Ordering::Relaxed);
        let now = now_epoch_millis();
        let last = LAST_DROP_LOG_MS.load(Ordering::Relaxed);
        if now - last >= DROP_WARN_INTERVAL_MS
            && LAST_DROP_LOG_MS
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            let count = DROPS_SINCE_LAST_LOG.swap(0, Ordering::Relaxed);
            tracing::warn!(target: "uc::audit", dropped = count, reason,
                "dropped audit events — audit DB may be slow");
        }
    }
}

static DROPS_SINCE_LAST_LOG: AtomicU64 = AtomicU64::new(0);
static LAST_DROP_LOG_MS: AtomicI64 = AtomicI64::new(0);

/// Spawn the audit drainer and return the producer handle.
///
/// A failed write is logged as a warning, and the drainer goes on.
pub fn spawn_drainer(metastore: DynMetastore, capacity: usize) -> AuditHandle {
    let (tx, mut rx) = mpsc::channel::<AuditEvent>(capacity);
    tokio::spawn(async move {
        let mut batch = Vec::with_capacity(BATCH);
        while rx.recv_many(&mut batch, BATCH).await > 0 {
            let events = std::mem::replace(&mut batch, Vec::with_capacity(BATCH));
            if let Err(e) = metastore.audit_log().record(events).await {
                tracing::warn!(target: "uc::audit", error = %e, "audit record failed");
            }
        }
    });
    AuditHandle { tx }
}
