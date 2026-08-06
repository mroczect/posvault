pub const DEFAULT_RECIPIENTS_COUNT: usize = 2;

#[allow(dead_code)]
pub const SIGNATURE_ALGORITHM: &str = "ed25519";

pub const JOURNAL_COMPACTION_THRESHOLD: u64 = 100_000;

pub const SNAPSHOT_INTERVAL: u64 = 10_000;

pub const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;
