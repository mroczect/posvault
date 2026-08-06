pub mod vault;

pub use posvault_auth::{Session, login, require_role};
pub use posvault_crypto::{decrypt_event, encrypt_event};
pub use posvault_handler::{
    constants, enums,
    errors::{PosVaultError, Result},
    traits, types,
};
pub use posvault_query::{engine::QueryEngine, examples as query_examples};
pub use posvault_sign::{
    ed25519::{Ed25519Signer, generate_keypair},
    signed_journal::SignedJournal,
    signed_store::SignedEventStore,
};
pub use posvault_store::{VctrlEventStore, VctrlJournal, VctrlSnapshotStore};
pub use posvault_sync::{
    FileTransport, UnionCsvResolver,
    branch::{checkout_branch, create_store_branch, current_branch},
    pull_and_merge,
};
pub use vault::PosVault;
