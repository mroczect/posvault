use libvctrl::domain::user::UserID;
use posvault_handler::traits::{ConflictResolver, Transport};
use posvault_handler::types::BranchName;
use posvault_store::PosVault;
use posvault_sync::branch::{checkout_branch, create_store_branch, current_branch};
use posvault_sync::resolver::UnionCsvResolver;
use posvault_sync::sync::pull_and_merge;
use posvault_sync::transport::FileTransport;
use std::fs;
use tempfile::TempDir;

fn setup_vault() -> (TempDir, PosVault) {
    let dir = TempDir::new().unwrap();
    let vault = PosVault::open(dir.path()).unwrap();
    (dir, vault)
}

fn create_author() -> UserID {
    UserID::new("tester".into(), "test@posvault.internal".into()).unwrap()
}

#[test]
fn test_create_store_branch() {
    let (_dir, vault) = setup_vault();
    let mut guard = vault.store_ref().lock().unwrap();
    let refs: &mut dyn libvctrl::storage::traits::RefStore = &mut *guard;
    let branch = create_store_branch(refs, "tokomainan").unwrap();
    assert_eq!(branch.as_str(), "store-tokomainan");

    let current = current_branch(refs).unwrap().unwrap();
    assert_eq!(current.as_str(), "store-tokomainan");
}

#[test]
fn test_checkout_branch() {
    let (_dir, vault) = setup_vault();

    {
        let mut guard = vault.store_ref().lock().unwrap();
        let refs = &mut *guard as &mut dyn libvctrl::storage::traits::RefStore;
        create_store_branch(refs, "cabang1").unwrap();
    }

    {
        let mut guard = vault.store_ref().lock().unwrap();
        let refs = &mut *guard as &mut dyn libvctrl::storage::traits::RefStore;
        create_store_branch(refs, "cabang2").unwrap();
    }

    let branch = BranchName::new("store-cabang1").unwrap();
    let mut guard = vault.store_ref().lock().unwrap();
    let refs = &mut *guard as &mut dyn libvctrl::storage::traits::RefStore;
    checkout_branch(refs, &branch).unwrap();
    let current = current_branch(refs).unwrap().unwrap();
    assert_eq!(current.as_str(), "store-cabang1");
}

#[test]
fn test_union_csv_resolver_no_conflict() {
    let resolver = UnionCsvResolver;
    let base = b"apple\nbanana\ncherry";
    let ours = b"apple\nbanana\ncherry\ndurian";
    let theirs = b"apple\nbanana\ncherry";
    let resolved = resolver.resolve(base, ours, theirs).unwrap();
    assert_eq!(resolved, ours);
}

#[test]
fn test_union_csv_resolver_conflict() {
    let resolver = UnionCsvResolver;
    let base = b"apple\nbanana";
    let ours = b"apple\nbanana\ncherry";
    let theirs = b"apple\nbanana\ndurian";
    let resolved = resolver.resolve(base, ours, theirs).unwrap();
    let resolved_str = String::from_utf8(resolved).unwrap();
    assert!(resolved_str.contains("cherry"));
    assert!(resolved_str.contains("durian"));
    assert!(resolved_str.contains("apple"));
    assert!(resolved_str.contains("banana"));
}

#[test]
fn test_file_transport_push() {
    let src_dir = TempDir::new().unwrap();
    let dst_dir = TempDir::new().unwrap();
    let src_file = src_dir.path().join("store.vctrl");
    fs::write(&src_file, b"test data").unwrap();

    let mut transport = FileTransport::new(src_dir.path(), dst_dir.path());
    transport.push(&[]).unwrap();

    let dst_file = dst_dir.path().join("store.vctrl");
    assert!(dst_file.exists());
    let content = fs::read_to_string(&dst_file).unwrap();
    assert_eq!(content, "test data");
}

#[test]
fn test_file_transport_pull() {
    let local_dir = TempDir::new().unwrap();
    let remote_dir = TempDir::new().unwrap();
    let remote_file = remote_dir.path().join("store.vctrl");
    fs::write(&remote_file, b"remote data").unwrap();

    let mut transport = FileTransport::new(local_dir.path(), remote_dir.path());
    transport.pull(&[]).unwrap();

    let local_file = local_dir.path().join("store.vctrl");
    assert!(local_file.exists());
    let content = fs::read_to_string(&local_file).unwrap();
    assert_eq!(content, "remote data");
}

#[test]
fn test_pull_and_merge_no_conflict() {
    let local_dir = TempDir::new().unwrap();
    let local_vault = PosVault::open(local_dir.path()).unwrap();

    {
        let mut guard = local_vault.store_ref().lock().unwrap();
        let refs = &mut *guard as &mut dyn libvctrl::storage::traits::RefStore;
        create_store_branch(refs, "toko1").unwrap();
    }

    let remote_dir = TempDir::new().unwrap();
    let _remote_vault = PosVault::open(remote_dir.path()).unwrap();

    let author = create_author();
    let result = pull_and_merge(local_dir.path(), remote_dir.path(), author);
    assert!(result.is_err());
}
