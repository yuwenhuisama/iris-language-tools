use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn main() {
    let text = "class Box { public fun read(value = 'a  b') -> String { value } } module Main { let item = Box.new(); item.read() }";
    let snapshot = AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }]);
    let cursor = text.rfind("read").expect("driver fixture contains read");
    let hover = snapshot.hover(FileId(1), cursor).expect("known member");
    assert_eq!(
        hover.signature,
        "public fun read(value: Dynamic<Object> = 'a  b') -> String"
    );
    assert_eq!(hover.type_label.as_deref(), Some("String"));
    println!(
        "{}..{}: {}",
        hover.span.start, hover.span.end, hover.signature
    );
    assert_eq!(snapshot.hover(FileId(1), cursor + 4), None);
    assert_eq!(snapshot.hover(FileId(1), usize::MAX), None);
    println!("After-name and invalid offsets: no hover");
}
