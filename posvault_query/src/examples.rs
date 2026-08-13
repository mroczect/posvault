use crate::engine::QueryEngine;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::{EventStore, SnapshotStore};
use std::collections::HashMap;

/// A simple materialized view state: stock items to quantities.
pub type StockState = HashMap<String, u64>;

/// Applies a stock delta event to the in-memory state.
///
/// The payload is expected to be a JSON tuple `(item, delta)` where delta may
/// be negative. Negative deltas that would drive stock below zero are
/// rejected.
pub fn apply_stock_event(state: &mut Vec<u8>, payload: &[u8]) -> Result<()> {
    let mut current: StockState = if state.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_slice(state).map_err(|e| PosVaultError::Serialization(e.to_string()))?
    };

    let delta: (String, i64) =
        serde_json::from_slice(payload).map_err(|e| PosVaultError::Serialization(e.to_string()))?;

    let entry = current.entry(delta.0).or_insert(0);
    if delta.1 >= 0 {
        *entry += delta.1 as u64;
    } else {
        let abs = (-delta.1) as u64;
        if *entry < abs {
            return Err(PosVaultError::InvalidInput("insufficient stock".into()));
        }
        *entry -= abs;
    }

    *state =
        serde_json::to_vec(&current).map_err(|e| PosVaultError::Serialization(e.to_string()))?;
    Ok(())
}

/// Returns the current stock quantity for `item`.
///
/// If the cached snapshot is stale, this function rebuilds it.
pub fn get_stock<S: EventStore + SnapshotStore>(
    engine: &mut QueryEngine<S>,
    decrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    encrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    item: &str,
) -> Result<u64> {
    if engine.needs_rebuild()? {
        engine.rebuild_snapshot(decrypt, encrypt, apply_stock_event)?;
    }
    let snapshot = engine
        .get_cached_snapshot()
        .ok_or_else(|| PosVaultError::NotFound("no snapshot available".into()))?;
    let plain = decrypt(snapshot.data.as_bytes())?;
    let state: StockState =
        serde_json::from_slice(&plain).map_err(|e| PosVaultError::Serialization(e.to_string()))?;
    Ok(state.get(item).copied().unwrap_or(0))
}

/// Placeholder for daily sales aggregation.
///
/// This function is not yet implemented. It exists to demonstrate how
/// additional queries could extend the engine.
pub fn daily_sales<S: EventStore + SnapshotStore>(
    _engine: &mut QueryEngine<S>,
    _decrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    _encrypt: &dyn Fn(&[u8]) -> Result<Vec<u8>>,
    _date: &str,
) -> Result<u64> {
    Err(PosVaultError::NotFound(
        "daily_sales is not implemented yet".into(),
    ))
}
