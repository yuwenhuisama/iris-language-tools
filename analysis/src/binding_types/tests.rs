use crate::{AnalysisSnapshot, FileId, GroupId, SourceInput, types::TypeFact};
use iris_parser::source::SourceKind;

#[test]
fn nominal_value_remains_object_when_metadata_contains_a_header_annotation() {
    for header in [
        "class Box extends Base {}",
        "module Box for Base {}",
        "contract Box extends Base {}",
    ] {
        let text = format!("class Base {{}} {header}");
        let mut given = AnalysisSnapshot::new([SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: text.into(),
        }]);
        let symbol = given
            .symbols
            .iter_mut()
            .find(|symbol| symbol.declaration.name.text == "Box")
            .unwrap();
        let key = symbol.key;
        let document = &given.documents[&key.file];
        let annotation = document
            .source
            .node(key.node)
            .children
            .iter()
            .copied()
            .find(|child| matches!(document.source.node(*child).kind, SourceKind::Type(_)))
            .unwrap();
        symbol.declaration.annotation = Some(annotation);

        let when = given.binding_type(key, 0);

        assert!(matches!(when, Some(TypeFact::Object(owner)) if owner == key));
    }
}
