use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn main() {
    let text = std::env::args().nth(1).unwrap_or_else(|| {
        "module Main {\n/// Reads a value.\npublic fun read(value, key option = '😀') -> String {}\nread(option:".into()
    });
    let snapshot = AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.clone().into(),
    }]);
    println!(
        "signature: {:#?}",
        snapshot.signature_help(FileId(1), text.len())
    );
    if let Some(byte) = text.find("read") {
        println!("hover: {:#?}", snapshot.hover(FileId(1), byte));
    }
}
