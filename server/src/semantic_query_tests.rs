use super::*;
use crate::workspace::{
    CoverageIssue, FileSource, GroupIdentity, IssueKind, Revision, SourceOrigin,
};
use iris_analysis::{GroupId, SourceInput};
use std::sync::Arc;

#[test]
fn marks_keyword_response_incomplete_when_workspace_inventory_is_partial() {
    let uri: Uri = "untitled:keywords".parse().unwrap();
    let text: Arc<str> = "let value = 1".into();
    let snapshot = Snapshot {
        revision: Revision(1),
        files: vec![FileSource {
            uri: uri.clone(),
            text: Arc::clone(&text),
            group: GroupIdentity::Standalone(uri.clone()),
            origin: SourceOrigin::Disk,
        }]
        .into(),
        issues: vec![CoverageIssue {
            uri: None,
            path: None,
            kind: IssueKind::FileCount,
        }]
        .into(),
    };
    let analysis = AnalysisSnapshot::new([SourceInput {
        id: FileId(0),
        group: GroupId(0),
        text,
    }]);
    let keywords = [CompletionItem {
        label: "let".into(),
        kind: Some(lsp_types::CompletionItemKind::KEYWORD),
        ..CompletionItem::default()
    }];
    let query = Query {
        uri,
        operation: Operation::Completion(Position::new(0, 0)),
    };

    let when = query.execute((&snapshot, &analysis), &keywords).unwrap();

    assert_eq!(when["isIncomplete"], true);
    assert_eq!(when["items"], json!(keywords));
}
