use crate::engine::QueryEngine;
use posvault_handler::errors::Result;
use posvault_handler::traits::{EventStore, SnapshotStore};
use std::collections::HashMap;

pub type StockState = HashMap<String, u64>;

pub fn apply_stock_event(state: &mut Vec<u8>, payload: &[u8]) -> Result<()> {
    let mut current: StockState = if state.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_slice(state)
            .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?
    };

    let delta: (String, i64) = serde_json::from_slice(payload)
        .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;

    let entry = current.entry(delta.0).or_insert(0);
    if delta.1 >= 0 {
        *entry += delta.1 as u64;
    } else {
        let abs = (-delta.1) as u64;
        if *entry < abs {
            return Err(posvault_handler::errors::PosVaultError::InvalidInput(
                "insufficient stock".into(),
            ));
        }
        *entry -= abs;
    }

    *state = serde_json::to_vec(&current)
        .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
    Ok(())
}

pub fn get_stock<S: EventStore + SnapshotStore>(
    engine: &mut QueryEngine<S>,
    item: &str,
) -> Result<u64> {
    let snapshot = engine.rebuild_snapshot(apply_stock_event)?;
    let state: StockState = serde_json::from_slice(snapshot.data.as_bytes())
        .map_err(|e| posvault_handler::errors::PosVaultError::Serialization(e.to_string()))?;
    Ok(state.get(item).copied().unwrap_or(0))
}

pub fn daily_sales<S: EventStore + SnapshotStore>(
    _engine: &mut QueryEngine<S>,
    _date: &str,
) -> Result<u64> {
    Ok(0)
}
