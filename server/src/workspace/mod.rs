//! Read-only inventory for a future worker. Capture overlays on the coordinator;
//! call `reload`/`replace_roots` off the main loop. No runtime or dependency loading.
mod claims;
mod details;
mod discovery;
mod inventory;
mod overlay;
mod paths;

pub use details::InventoryDetails;
pub use inventory::Workspace;
pub use overlay::{DocumentGeneration, DocumentVersion, OverlaySet};

use iris_package::PackageId;
use lsp_types::Uri;
use std::{path::PathBuf, sync::Arc};

pub const MAX_FILE_BYTES: usize = 256 * 1024;
pub const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILES: usize = 1024;
pub const MAX_DISCOVERY_ENTRIES: usize = 20_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Revision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageIdentity {
    pub(crate) package_id: PackageId,
    pub(crate) api_major: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupIdentity {
    Package(PackageIdentity),
    Standalone(Uri),
    Ambiguous(Uri),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceOrigin {
    Disk,
    Buffer {
        version: DocumentVersion,
        generation: DocumentGeneration,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSource {
    pub(crate) uri: Uri,
    pub(crate) text: Arc<str>,
    pub(crate) group: GroupIdentity,
    pub(crate) origin: SourceOrigin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IssueKind {
    NotLoaded,
    InvalidUri,
    Io(std::io::ErrorKind),
    InvalidManifest,
    UnsafePath,
    NonRegular,
    InvalidUtf8,
    FileBytes,
    TotalBytes,
    FileCount,
    DiscoveryEntries,
    DuplicatePackage(PackageIdentity),
    DuplicateSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageIssue {
    pub(crate) uri: Option<Uri>,
    pub(crate) path: Option<PathBuf>,
    pub(crate) kind: IssueKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub(crate) revision: Revision,
    pub(crate) files: Arc<[FileSource]>,
    pub(crate) issues: Arc<[CoverageIssue]>,
}

impl Snapshot {
    pub(crate) fn is_complete(&self) -> bool {
        self.issues.is_empty()
    }

    pub(crate) fn file(&self, uri: &Uri) -> Option<&FileSource> {
        self.files.iter().find(|file| &file.uri == uri)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace revision exhausted")]
    RevisionExhausted,
}

#[cfg(test)]
mod discovery_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod limit_tests;
#[cfg(test)]
mod tests;
