use posvault_handler::types::Snapshot;
use std::cell::RefCell;

pub struct SnapshotCache {
    inner: RefCell<Option<Snapshot>>,
}

impl SnapshotCache {
    pub fn new() -> Self {
        SnapshotCache {
            inner: RefCell::new(None),
        }
    }

    pub fn get(&self) -> Option<Snapshot> {
        self.inner.borrow().clone()
    }

    pub fn set(&self, snapshot: Snapshot) {
        *self.inner.borrow_mut() = Some(snapshot);
    }

    pub fn invalidate(&self) {
        *self.inner.borrow_mut() = None;
    }
}

impl Default for SnapshotCache {
    fn default() -> Self {
        Self::new()
    }
}
