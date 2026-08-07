use posvault_handler::types::Snapshot;
use std::sync::Mutex;

pub struct SnapshotCache {
    inner: Mutex<Option<Snapshot>>,
}

impl SnapshotCache {
    pub fn new() -> Self {
        SnapshotCache {
            inner: Mutex::new(None),
        }
    }

    pub fn get(&self) -> Option<Snapshot> {
        self.inner.lock().unwrap().clone()
    }

    pub fn set(&self, snapshot: Snapshot) {
        *self.inner.lock().unwrap() = Some(snapshot);
    }

    pub fn invalidate(&self) {
        *self.inner.lock().unwrap() = None;
    }
}

impl Default for SnapshotCache {
    fn default() -> Self {
        Self::new()
    }
}
