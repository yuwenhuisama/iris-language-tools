use iris_formatter::{FormatOutcome, SkipReason, format_document};

#[test]
fn keeps_raise_values_and_causes_when_offsets_move_through_nested_syntax() {
    let source = "fun f(){let closure={raise problem from cause};let v=try {raise first} catch err {raise second from err};return v}";
    assert!(
        iris_parser::parse(source).program_accepted,
        "{:?}",
        iris_parser::parse(source).diagnostics
    );
    let outcome = format_document(source);
    assert!(matches!(outcome, FormatOutcome::Changed(_)), "{outcome:?}");
    if let FormatOutcome::Changed(output) = outcome {
        assert_eq!(format_document(&output), FormatOutcome::Unchanged);
    }
}

#[test]
fn formats_varied_valid_statements_when_layout_and_adjacency_change() {
    let statements = [
        "let x=1+2",
        "return",
        "return - 1",
        "raise problem from cause",
        "let x=[1,2,]",
        "let f={||;2}",
        "let f={|x|;x+1}",
        "foo(x: :ok)",
        "foo(1/2)",
        "foo(\"a\" 'b')",
        "obj.+(2)",
        "if ready{a()}else{b()}",
        "try{work()}catch err{raise err}finally{cleanup()}",
    ];
    for first in statements {
        for second in statements {
            let source = format!("fun first(){{{first}}}\nfun second(){{{second}}}");
            let outcome = format_document(&source);
            assert!(
                !matches!(outcome, FormatOutcome::Skipped(_)),
                "{source}: {outcome:?}"
            );
            if let FormatOutcome::Changed(output) = outcome {
                assert_eq!(
                    format_document(&output),
                    FormatOutcome::Unchanged,
                    "{output}"
                );
            }
        }
    }
}

#[test]
fn leaves_semicolon_sensitive_index_boundaries_unchanged_when_joining_would_change_meaning() {
    let source = "let x = a;[0]";
    let outcome = format_document(source);
    assert_eq!(outcome, FormatOutcome::Changed("let x = a\n[0]\n".into()));
}

#[test]
fn refuses_missing_closure_header_terminators_instead_of_repairing_them() {
    assert_eq!(
        format_document("let f={|x| x}"),
        FormatOutcome::Skipped(SkipReason::ParseDiagnostics)
    );
}
