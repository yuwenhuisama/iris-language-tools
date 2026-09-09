use std::sync::Arc;

pub use iris_parser::source::DeclarationKind as HoverKind;
pub use iris_parser::source::Span;
pub use iris_syntax::ParameterCategory;

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
    pub kind: HoverKind,
    pub owner: Option<String>,
    pub details: Vec<HoverDetail>,
    pub docs: Option<DocumentationInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HoverDetail {
    ReturnType(String),
    ValueType(String),
    ParameterCategory(ParameterCategory),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentationInfo {
    pub text: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureHelpInfo {
    pub signatures: Vec<SignatureInfo>,
    pub active_signature: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureInfo {
    pub active_parameter: Option<usize>,
    pub label: String,
    pub parameters: Vec<SignatureParameterInfo>,
    pub docs: Option<DocumentationInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureParameterInfo {
    pub label: Span,
    pub name: String,
    pub category: ParameterCategory,
    pub docs: Option<DocumentationInfo>,
}
