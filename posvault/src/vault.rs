use posvault_auth::Session;
use posvault_crypto::{decrypt_event, encrypt_event};
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

pub struct PosVault {
    pub store: Store,
    pub path: PathBuf,
    session: Option<Session>,
    recipients: Vec<String>,
    signer: Option<Ed25519Signer>,
}

impl PosVault {
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

    pub fn set_recipients(&mut self, recipients: Vec<String>) {
        self.recipients = recipients;
    }

    pub fn set_signer(&mut self, signer: Ed25519Signer) {
        self.signer = Some(signer);
    }

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
        Ok(self.session.as_ref().unwrap())
    }

    pub fn session(&self) -> Result<&Session> {
        self.session
            .as_ref()
            .ok_or_else(|| PosVaultError::Auth("not logged in".into()))
    }

    pub fn transact(&mut self, mut event: Event) -> Result<()> {
        let _session = self.session()?;

        if !self.recipients.is_empty() {
            encrypt_event(&mut event, &self.recipients)?;
        }

        let store = self.store.clone_store()?;
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

    pub fn journal(&mut self, entry: JournalEntry) -> Result<()> {
        let _session = self.session()?;
        let store = self.store.clone_store()?;
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

    pub fn query_engine(&self) -> Result<QueryEngine<CombinedStore>> {
        let store1 = self.store.clone_store()?;
        let store2 = self.store.clone_store()?;
        let event_store = VctrlEventStore::new(store1);
        let snapshot_store = VctrlSnapshotStore::new(store2);
        Ok(QueryEngine::new(CombinedStore {
            event_store,
            snapshot_store,
        }))
    }

    pub fn decrypt_payload(&self, event: &mut Event, identity: &str) -> Result<()> {
        decrypt_event(event, identity)
    }

    pub fn sync_to_remote(&self, remote_path: impl AsRef<Path>) -> Result<()> {
        let author =
            libvctrl::domain::user::UserID::new("sync".into(), "sync@posvault.internal".into())?;
        posvault_sync::sync::pull_and_merge(&self.path, remote_path.as_ref(), author)
    }
}

#[derive(Debug)]
pub struct CombinedStore {
    pub event_store: VctrlEventStore,
    pub snapshot_store: VctrlSnapshotStore,
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
