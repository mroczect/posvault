pub const DEFAULT_RECIPIENTS_COUNT: usize = 2;

pub const SIGNATURE_ALGORITHM: &str = "ed25519";

pub const JOURNAL_COMPACTION_THRESHOLD: u64 = 100_000;

pub const SNAPSHOT_INTERVAL: u64 = 10_000;

pub const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;

const _: () = {
    assert!(JOURNAL_COMPACTION_THRESHOLD > 0);
    assert!(SNAPSHOT_INTERVAL > 0);
    assert!(MAX_PAYLOAD_SIZE > 0);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_valid() {
        assert!(!SIGNATURE_ALGORITHM.is_empty());
    }
}
