use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn main() {
    let cases = [
        "module Main { let text = 'abc'; text.",
        "module Main { 'abc'.replace('a', ",
        "module Main { [1].reduce(0, ",
        "module Main { Encoding::UTF_8.decode(b'abc', errors: ",
        "module Main { let value: Dynamic<String> = 'abc'; value.replace(",
    ];
    for text in cases {
        let snapshot = AnalysisSnapshot::new([SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: text.into(),
        }]);
        println!("source: {text}");
        println!(
            "completion: {:?}",
            snapshot.completions(FileId(1), text.len()).items
        );
        println!(
            "signature: {:#?}",
            snapshot.signature_help(FileId(1), text.len())
        );
        if let Some(byte) = text.rfind("replace") {
            println!("hover: {:#?}", snapshot.hover(FileId(1), byte));
            println!("definitions: {:?}", snapshot.definitions(FileId(1), byte));
        }
    }
}
