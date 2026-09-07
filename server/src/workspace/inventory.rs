use super::{
    CoverageIssue, FileSource, GroupIdentity, IssueKind, MAX_FILES, MAX_TOTAL_BYTES, OverlaySet,
    Revision, Snapshot, SourceOrigin, WorkspaceError, claims, paths,
};
use lsp_types::Uri;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct Workspace {
    roots: Vec<Uri>,
    snapshot: Snapshot,
}

impl Workspace {
    pub(crate) fn new(roots: Vec<Uri>) -> Self {
        Self {
            roots,
            snapshot: Snapshot {
                revision: Revision(0),
                files: Arc::from([]),
                issues: Arc::from([CoverageIssue {
                    uri: None,
                    path: None,
                    kind: IssueKind::NotLoaded,
                }]),
            },
        }
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }

    pub(crate) fn replace_roots(
        &mut self,
        roots: Vec<Uri>,
        overlays: &OverlaySet,
    ) -> Result<Revision, WorkspaceError> {
        let changed = self.roots != roots;
        self.roots = roots;
        self.refresh(overlays, changed)
    }

    pub(crate) fn reload(&mut self, overlays: &OverlaySet) -> Result<Revision, WorkspaceError> {
        self.refresh(overlays, false)
    }

    fn refresh(
        &mut self,
        overlays: &OverlaySet,
        roots_changed: bool,
    ) -> Result<Revision, WorkspaceError> {
        let (files, issues) = load(&self.roots, overlays);
        if roots_changed
            || self.snapshot.files.as_ref() != files
            || self.snapshot.issues.as_ref() != issues
        {
            self.snapshot = Snapshot {
                revision: Revision(
                    self.snapshot
                        .revision
                        .0
                        .checked_add(1)
                        .ok_or(WorkspaceError::RevisionExhausted)?,
                ),
                files: files.into(),
                issues: issues.into(),
            };
        }
        Ok(self.snapshot.revision)
    }
}

fn load(roots: &[Uri], overlays: &OverlaySet) -> (Vec<FileSource>, Vec<CoverageIssue>) {
    let mut issues = overlays.issues.clone();
    let mut remaining = MAX_TOTAL_BYTES
        - overlays
            .files
            .values()
            .map(|file| file.text.len())
            .sum::<usize>();
    let claims = claims::collect(roots, &mut remaining);
    issues.extend(claims.issues);
    let excluded: BTreeSet<_> = overlays
        .excluded
        .iter()
        .filter_map(|uri| paths::file_path(uri).ok())
        .collect();
    let buffers: BTreeMap<_, _> = overlays
        .files
        .values()
        .filter_map(|overlay| {
            paths::file_path(&overlay.uri)
                .ok()
                .map(|path| (path, overlay))
        })
        .collect();
    let mut files: BTreeMap<String, FileSource> = overlays
        .files
        .values()
        .map(|overlay| {
            (
                overlay.uri.as_str().to_owned(),
                FileSource {
                    uri: overlay.uri.clone(),
                    text: Arc::clone(&overlay.text),
                    group: GroupIdentity::Standalone(overlay.uri.clone()),
                    origin: SourceOrigin::Buffer {
                        version: overlay.version,
                        generation: overlay.generation.clone(),
                    },
                },
            )
        })
        .collect();
    for claim in claims.files.into_values() {
        let uri = claim.uri;
        let group = if claim.ambiguous || claims.duplicates.contains(&claim.package) {
            GroupIdentity::Ambiguous(uri.clone())
        } else {
            GroupIdentity::Package(claim.package)
        };
        if excluded.contains(&claim.path) {
            continue;
        }
        if let Some(overlay) = buffers.get(&claim.path) {
            if let Some(file) = files.get_mut(overlay.uri.as_str()) {
                file.group = group;
            }
            continue;
        }
        if files.len() == MAX_FILES {
            issues.push(CoverageIssue {
                uri: Some(uri),
                path: Some(claim.path),
                kind: IssueKind::FileCount,
            });
            continue;
        }
        match paths::read_text(&claim.path, &mut remaining) {
            Ok(text) => {
                files.insert(
                    uri.as_str().to_owned(),
                    FileSource {
                        uri,
                        text: Arc::from(text),
                        group,
                        origin: SourceOrigin::Disk,
                    },
                );
            }
            Err(kind) => issues.push(CoverageIssue {
                uri: Some(uri),
                path: Some(claim.path),
                kind,
            }),
        }
    }
    (files.into_values().collect(), issues)
}
