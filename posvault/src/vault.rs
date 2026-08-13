use posvault_auth::Session;
use posvault_crypto::{decrypt_event, encrypt_event};
use posvault_handler::Transport as _;
use posvault_handler::errors::{PosVaultError, Result};
use posvault_handler::traits::{EventStore, Journal, SnapshotStore};
use posvault_handler::types::{Event, JournalEntry};
use posvault_query::engine::QueryEngine;
use posvault_sign::ed25519::Ed25519Signer;
use posvault_sign::signed_journal::SignedJournal;
use posvault_sign::signed_store::SignedEventStore;
use posvault_store::event_store::VctrlEventStore;
use posvault_store::journal::VctrlJournal;
use posvault_store::posvault::PosVault as Store;
use posvault_store::snapshot_store::VctrlSnapshotStore;
use std::path::{Path, PathBuf};

/// High-level facade for the PosVault system.
///
/// Combines authentication, encryption, signing, storage, journal, query, and
/// sync into a single interface. A session must be established via [`login`]
/// before operations that require authorization can be executed.
pub struct PosVault {
    store: Store,
    path: PathBuf,
    session: Option<Session>,
    recipients: Vec<String>,
    signer: Option<Ed25519Signer>,
}

impl PosVault {
    /// Opens (or creates) the vault at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let store = Store::open(&path)?;
        Ok(PosVault {
            store,
            path,
            session: None,
            recipients: Vec::new(),
            signer: None,
        })
    }

    /// Sets the recipients used for encrypting event payloads.
    pub fn set_recipients(&mut self, recipients: Vec<String>) {
        self.recipients = recipients;
    }

    /// Sets the signer used for signing events and journal entries.
    pub fn set_signer(&mut self, signer: Ed25519Signer) {
        self.signer = Some(signer);
    }

    /// Authenticates against an account backend and stores a session.
    ///
    /// Returns a reference to the newly created session.
    pub fn login(
        &mut self,
        backend: &dyn age_credentials::backend::traits::AccountBackend,
        email: &str,
        passphrase: &str,
        otp_code: &str,
        totp_secret_base32: &str,
    ) -> Result<&Session> {
        let session =
            posvault_auth::login(backend, email, passphrase, otp_code, totp_secret_base32)?;
        self.session = Some(session);
        Ok(self
            .session
            .as_ref()
            .expect("session was just set and therefore must be present"))
    }

    /// Returns a reference to the active session, or an error if not logged in.
    pub fn session(&self) -> Result<&Session> {
        self.session
            .as_ref()
            .ok_or_else(|| PosVaultError::Auth("not logged in".into()))
    }

    /// Appends an event to the event store.
    ///
    /// The event is encrypted if recipients have been configured, and signed
    /// if a signer has been configured.
    pub fn transact(&mut self, mut event: Event) -> Result<()> {
        let _session = self.session()?;

        if !self.recipients.is_empty() {
            encrypt_event(&mut event, &self.recipients)?;
        }

        let store = self.store.store_arc();
        let event_store = VctrlEventStore::new(store);

        if let Some(ref signer) = self.signer {
            let mut signed_store = SignedEventStore::new(event_store, signer.clone());
            signed_store.append_event(event)?;
        } else {
            let mut plain_store = event_store;
            plain_store.append_event(event)?;
        }

        Ok(())
    }

    /// Records a journal entry, signing it if a signer is configured.
    pub fn journal(&mut self, entry: JournalEntry) -> Result<()> {
        let _session = self.session()?;
        let store = self.store.store_arc();
        let journal = VctrlJournal::new(store);

        if let Some(ref signer) = self.signer {
            let mut signed_journal = SignedJournal::new(journal, signer.clone());
            signed_journal.record(entry)?;
        } else {
            let mut plain_journal = journal;
            plain_journal.record(entry)?;
        }

        Ok(())
    }

    /// Creates a query engine backed by this vault's event and snapshot stores.
    pub fn query_engine(&self) -> QueryEngine<CombinedStore> {
        let store1 = self.store.store_arc();
        let store2 = self.store.store_arc();
        let event_store = VctrlEventStore::new(store1);
        let snapshot_store = VctrlSnapshotStore::new(store2);
        QueryEngine::new(CombinedStore {
            event_store,
            snapshot_store,
        })
    }

    /// Decrypts an event payload using an identity key.
    pub fn decrypt_payload(&self, event: &mut Event, identity: &str) -> Result<()> {
        decrypt_event(event, identity)
    }

    /// Synchronizes this vault to a remote directory using the file transport.
    pub fn sync_to_remote(&self, remote_path: impl AsRef<Path>) -> Result<()> {
        let mut transport = posvault_sync::FileTransport::new(&self.path, remote_path);
        transport.push(&[])
    }

    /// Returns the filesystem path of this vault.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a reference to the underlying low-level store.
    pub fn store(&self) -> &Store {
        &self.store
    }
}

/// Combines an event store and a snapshot store into one query engine backend.
#[derive(Debug)]
pub struct CombinedStore {
    event_store: VctrlEventStore,
    snapshot_store: VctrlSnapshotStore,
}

impl EventStore for CombinedStore {
    fn append_event(&mut self, event: Event) -> Result<()> {
        self.event_store.append_event(event)
    }

    fn get_events_since(&self, checkpoint: u64) -> Result<Vec<Event>> {
        self.event_store.get_events_since(checkpoint)
    }

    fn latest_checkpoint(&self) -> Result<u64> {
        self.event_store.latest_checkpoint()
    }
}

impl SnapshotStore for CombinedStore {
    fn save_snapshot(&mut self, snapshot: posvault_handler::types::Snapshot) -> Result<()> {
        self.snapshot_store.save_snapshot(snapshot)
    }

    fn load_snapshot(&self) -> Result<Option<posvault_handler::types::Snapshot>> {
        self.snapshot_store.load_snapshot()
    }
}
