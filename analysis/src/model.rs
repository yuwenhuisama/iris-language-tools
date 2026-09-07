use std::sync::Arc;

pub use iris_parser::source::Span;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroupId(pub u32);

#[derive(Clone, Debug)]
pub struct SourceInput {
    pub id: FileId,
    pub text: Arc<str>,
    pub group: GroupId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Target {
    pub file: FileId,
    pub span: Span,
    pub name_span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Location {
    pub file: FileId,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionKind {
    Variable,
    Constant,
    Parameter,
    TypeParameter,
    Class,
    Module,
    Contract,
    TypeAlias,
    Method,
    Property,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: CompletionKind,
    pub replace: Span,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompletionResult {
    pub items: Vec<CompletionItem>,
    pub is_incomplete: bool,
    pub allow_keywords: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeHint {
    pub offset: usize,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoverInfo {
    pub span: Span,
    pub signature: String,
    pub type_label: Option<String>,
}
