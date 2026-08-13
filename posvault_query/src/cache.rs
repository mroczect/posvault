use posvault_handler::types::Snapshot;
use std::sync::Mutex;

/// Thread-safe in-memory cache for the latest materialized snapshot.
///
/// The cache stores an optional [`Snapshot`] behind a mutex. It is used by
/// [`QueryEngine`](crate::QueryEngine) to avoid recomputing state on every
/// query.
pub struct SnapshotCache {
    inner: Mutex<Option<Snapshot>>,
}

impl SnapshotCache {
    /// Creates an empty cache.
    pub fn new() -> Self {
        SnapshotCache {
            inner: Mutex::new(None),
        }
    }

    /// Returns a clone of the cached snapshot, if any.
    ///
    /// Cloning is cheap because [`Snapshot`] uses heap-allocated bytes but is
    /// still reasonably small. If callers need to avoid cloning, they should
    /// use the mutex directly.
    pub fn get(&self) -> Option<Snapshot> {
        self.inner.lock().unwrap().clone()
    }

    /// Stores a snapshot in the cache.
    pub fn set(&self, snapshot: Snapshot) {
        *self.inner.lock().unwrap() = Some(snapshot);
    }

    /// Removes the cached snapshot.
    pub fn invalidate(&self) {
        *self.inner.lock().unwrap() = None;
    }
}

impl Default for SnapshotCache {
    fn default() -> Self {
        Self::new()
    }
}
