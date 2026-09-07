use super::{CoverageIssue, IssueKind, MAX_DISCOVERY_ENTRIES, paths};
use lsp_types::Uri;
use std::{collections::BTreeSet, fs, path::PathBuf};

pub(super) struct Discovery {
    pub(super) manifests: Vec<(PathBuf, PathBuf)>,
    pub(super) issues: Vec<CoverageIssue>,
}

pub(super) fn discover(roots: &[Uri]) -> Discovery {
    let mut result = Discovery {
        manifests: Vec::new(),
        issues: Vec::new(),
    };
    let mut pending = Vec::new();
    for uri in roots {
        let root = paths::file_path(uri).and_then(|path| {
            let metadata =
                fs::symlink_metadata(&path).map_err(|error| IssueKind::Io(error.kind()))?;
            if metadata.file_type().is_symlink() {
                return Err(IssueKind::UnsafePath);
            }
            if !metadata.is_dir() {
                return Err(IssueKind::NonRegular);
            }
            Ok(path)
        });
        match root {
            Ok(root) => pending.push((root.clone(), root)),
            Err(kind) => result.issues.push(CoverageIssue {
                uri: Some(uri.clone()),
                path: None,
                kind,
            }),
        }
    }
    pending.sort();
    let mut visited = BTreeSet::new();
    let mut entries = 0;
    while let Some((root, directory)) = pending.pop() {
        if !visited.insert(directory.clone()) {
            continue;
        }
        let read = match fs::read_dir(&directory) {
            Ok(read) => read,
            Err(error) => {
                result.issues.push(CoverageIssue {
                    uri: None,
                    path: Some(directory),
                    kind: IssueKind::Io(error.kind()),
                });
                continue;
            }
        };
        for entry in read {
            if entries == MAX_DISCOVERY_ENTRIES {
                result.issues.push(CoverageIssue {
                    uri: None,
                    path: Some(directory),
                    kind: IssueKind::DiscoveryEntries,
                });
                result.manifests.sort();
                return result;
            }
            entries += 1;
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    result.issues.push(CoverageIssue {
                        uri: None,
                        path: Some(directory.clone()),
                        kind: IssueKind::Io(error.kind()),
                    });
                    continue;
                }
            };
            let path = entry.path();
            if paths::excluded(&path) {
                continue;
            }
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    result.issues.push(CoverageIssue {
                        uri: None,
                        path: Some(path),
                        kind: IssueKind::Io(error.kind()),
                    });
                    continue;
                }
            };
            if file_type.is_dir() {
                pending.push((root.clone(), path));
            } else if entry.file_name() == "iris.toml" {
                if file_type.is_symlink() || !file_type.is_file() {
                    result.issues.push(CoverageIssue {
                        uri: None,
                        path: Some(path),
                        kind: IssueKind::NonRegular,
                    });
                } else {
                    result.manifests.push((root.clone(), path));
                }
            }
        }
    }
    result.manifests.sort();
    result
}
