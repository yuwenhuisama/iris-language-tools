use super::{CoverageIssue, IssueKind, MAX_FILES, PackageIdentity, discovery, paths};
use iris_package::Manifest;
use lsp_types::Uri;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) struct Claim {
    pub(super) uri: Uri,
    pub(super) path: PathBuf,
    pub(super) package: PackageIdentity,
    pub(super) ambiguous: bool,
}

pub(super) struct Claims {
    pub(super) files: BTreeMap<PathBuf, Claim>,
    pub(super) duplicates: BTreeSet<PackageIdentity>,
    pub(super) issues: Vec<CoverageIssue>,
}

pub(super) fn collect(roots: &[Uri], remaining: &mut usize) -> Claims {
    let discovery = discovery::discover(roots);
    let mut result = Claims {
        files: BTreeMap::new(),
        duplicates: BTreeSet::new(),
        issues: discovery.issues,
    };
    let mut packages = BTreeSet::new();
    for (root, path) in discovery.manifests {
        let manifest = paths::check_path(&root, &path)
            .and_then(|()| paths::read_text(&path, remaining))
            .and_then(|text| Manifest::parse(&text).map_err(|_| IssueKind::InvalidManifest));
        let manifest = match manifest {
            Ok(manifest) => manifest,
            Err(kind) => {
                result.issues.push(CoverageIssue {
                    uri: None,
                    path: Some(path),
                    kind,
                });
                continue;
            }
        };
        let identity = PackageIdentity {
            package_id: manifest.package_id,
            api_major: manifest.api_major,
        };
        if !packages.insert(identity.clone()) {
            result.duplicates.insert(identity.clone());
            result.issues.push(CoverageIssue {
                uri: None,
                path: Some(path.clone()),
                kind: IssueKind::DuplicatePackage(identity.clone()),
            });
        }
        let Some(directory) = path.parent() else {
            continue;
        };
        for source in manifest.sources {
            let source_path = directory.join(source.as_str());
            let uri = match paths::check_path(&root, &source_path)
                .and_then(|()| paths::file_uri(&source_path))
            {
                Ok(uri) => uri,
                Err(kind) => {
                    result.issues.push(CoverageIssue {
                        uri: None,
                        path: Some(source_path),
                        kind,
                    });
                    continue;
                }
            };
            if let Some(claim) = result.files.get_mut(&source_path) {
                claim.ambiguous = true;
                result.issues.push(CoverageIssue {
                    uri: Some(uri),
                    path: Some(source_path),
                    kind: IssueKind::DuplicateSource,
                });
            } else if result.files.len() == MAX_FILES {
                result.issues.push(CoverageIssue {
                    uri: None,
                    path: Some(path.clone()),
                    kind: IssueKind::FileCount,
                });
                break;
            } else {
                result.files.insert(
                    source_path.clone(),
                    Claim {
                        uri,
                        path: source_path,
                        package: identity.clone(),
                        ambiguous: false,
                    },
                );
            }
        }
    }
    result
}
