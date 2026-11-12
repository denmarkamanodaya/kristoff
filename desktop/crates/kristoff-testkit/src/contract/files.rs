//! `FileStore` contract.

use super::assert_code;
use kristoff_core::error::ErrorCode;
use kristoff_core::ports::FileStore;

const A: &str = "0000000000000001";

/// Runs every `FileStore` promise against fresh stores from `make`. `stored` reads the committed
/// PDF of a doc back (`None` when absent), so the suite can check commit and purge.
pub fn file_store_contract<S: FileStore>(
    make: impl Fn() -> S,
    stored: impl Fn(&S, &str) -> Option<Vec<u8>>,
) {
    writes_chunks_at_explicit_offsets(&make());
    commits_the_upload(&make(), &stored);
    stores_parts_and_replaces_them(&make());
    purges_everything_and_is_repeatable(&make(), &stored);
}

fn writes_chunks_at_explicit_offsets(files: &impl FileStore) {
    assert_eq!(files.partial_len(A).unwrap(), None);

    assert_eq!(files.write_partial(A, 0, b"%PDF-1.7").unwrap(), 8);
    assert_eq!(files.write_partial(A, 8, b" body").unwrap(), 13);
    assert_eq!(files.write_partial(A, 8, b" BODY").unwrap(), 13);

    assert_eq!(files.partial_len(A).unwrap(), Some(13));
    assert_eq!(files.partial_head(A, 5).unwrap(), b"%PDF-");
    assert_eq!(files.partial_head(A, 100).unwrap(), b"%PDF-1.7 BODY");
}

fn stores_parts_and_replaces_them(files: &impl FileStore) {
    files.write_part(A, 2, b"%PDF-old").unwrap();
    files.write_part(A, 2, b"%PDF-new").unwrap();

    assert_eq!(files.read_stored(A, Some(2)).unwrap(), b"%PDF-new");
    assert_code(files.read_stored(A, Some(1)), ErrorCode::DocNotFound);
    files.purge(A).unwrap();
    assert_code(files.read_stored(A, Some(2)), ErrorCode::DocNotFound);
}

fn commits_the_upload<S: FileStore>(files: &S, stored: &impl Fn(&S, &str) -> Option<Vec<u8>>) {
    assert_code(files.commit_partial(A), ErrorCode::Internal);
    files.write_partial(A, 0, b"%PDF-1.7").unwrap();

    files.commit_partial(A).unwrap();

    assert_eq!(files.partial_len(A).unwrap(), None);
    assert_eq!(stored(files, A), Some(b"%PDF-1.7".to_vec()));
    assert_eq!(files.read_stored(A, None).unwrap(), b"%PDF-1.7");
    assert_code(files.read_stored(A, Some(1)), ErrorCode::DocNotFound);
    assert_code(
        files.read_stored("0000000000000009", None),
        ErrorCode::DocNotFound,
    );
}

fn purges_everything_and_is_repeatable<S: FileStore>(
    files: &S,
    stored: &impl Fn(&S, &str) -> Option<Vec<u8>>,
) {
    files.write_partial(A, 0, b"%PDF-1").unwrap();
    files.commit_partial(A).unwrap();
    files.write_partial(A, 0, b"%PDF-2").unwrap();

    files.purge(A).unwrap();
    files.purge(A).unwrap();

    assert_eq!(files.partial_len(A).unwrap(), None);
    assert_eq!(stored(files, A), None);
}
