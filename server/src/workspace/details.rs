use super::{CoverageIssue, IssueKind};
use lsp_types::Uri;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryDetails {
    total_count: usize,
    issues: Vec<IssueDetail>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IssueDetail {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    io_kind: Option<String>,
    uri: Option<Uri>,
    path: Option<String>,
}

impl InventoryDetails {
    pub(crate) fn from_issues(issues: &[CoverageIssue]) -> Self {
        Self {
            total_count: issues.len(),
            issues: issues.iter().take(8).map(IssueDetail::from).collect(),
        }
    }
}

impl From<&CoverageIssue> for IssueDetail {
    fn from(issue: &CoverageIssue) -> Self {
        let (kind, io_kind) = match &issue.kind {
            IssueKind::NotLoaded => ("notLoaded", None),
            IssueKind::InvalidUri => ("invalidUri", None),
            IssueKind::Io(kind) => ("io", Some(format!("{kind:?}"))),
            IssueKind::InvalidManifest => ("invalidManifest", None),
            IssueKind::UnsafePath => ("unsafePath", None),
            IssueKind::NonRegular => ("nonRegular", None),
            IssueKind::InvalidUtf8 => ("invalidUtf8", None),
            IssueKind::FileBytes => ("fileBytes", None),
            IssueKind::TotalBytes => ("totalBytes", None),
            IssueKind::FileCount => ("fileCount", None),
            IssueKind::DiscoveryEntries => ("discoveryEntries", None),
            IssueKind::DuplicatePackage(_) => ("duplicatePackage", None),
            IssueKind::DuplicateSource => ("duplicateSource", None),
        };
        Self {
            kind,
            io_kind,
            uri: issue.uri.clone(),
            path: issue
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retains_issue_count_when_inventory_details_are_truncated() {
        for count in [0, 1, 8, 9] {
            let given = vec![
                CoverageIssue {
                    kind: IssueKind::InvalidManifest,
                    uri: None,
                    path: Some("iris.toml".into()),
                };
                count
            ];

            let when = serde_json::to_value(InventoryDetails::from_issues(&given)).unwrap();

            assert_eq!(when["totalCount"], count);
            assert_eq!(
                when["issues"],
                json!(vec![
                    json!({
                        "kind":"invalidManifest","uri":null,"path":"iris.toml"
                    });
                    count.min(8)
                ])
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn serializes_path_when_inventory_contains_non_utf8_filename() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let given = [CoverageIssue {
            kind: IssueKind::InvalidUtf8,
            uri: None,
            path: Some(OsString::from_vec(b"source\xff.ir".to_vec()).into()),
        }];

        let when = serde_json::to_value(InventoryDetails::from_issues(&given)).unwrap();

        assert_eq!(
            when["issues"][0],
            json!({
                "kind":"invalidUtf8","uri":null,"path":"source\u{fffd}.ir"
            })
        );
    }
}
