use iris_formatter::{FormatOptions, FormatOutcome, format_document};

#[test]
fn dedents_closing_line_when_it_also_opens_another_block() {
    let source = "fun f() {\nif ready {\nprint(1)\n   } else {\nprint(2)\n}\n}";

    let actual = format_document(source, FormatOptions::new(2, true).unwrap());

    assert_eq!(
        actual,
        FormatOutcome::Changed(
            "fun f() {\n  if ready {\n    print(1)\n  } else {\n    print(2)\n  }\n}".into()
        )
    );
}

#[test]
fn preserves_protected_crlf_lines_when_first_comment_byte_looks_like_code() {
    let source = "fun f() {\r\n/* comment\r\nprint(9) }\r\n*/\r\nprint(1)\r\n}";

    let actual = format_document(source, FormatOptions::new(2, true).unwrap());

    assert_eq!(
        actual,
        FormatOutcome::Changed(
            "fun f() {\r\n/* comment\r\nprint(9) }\r\n*/\r\n  print(1)\r\n}".into()
        )
    );
}

#[test]
fn indents_delimiters_when_lines_start_with_multiple_closers() {
    let source = "fun f() {\nlet data = %{\nkey: [\ncall(\n1\n)]\n}\n   }";
    let options = FormatOptions::new(2, true).unwrap();

    let actual = format_document(source, options);

    assert_eq!(
        actual,
        FormatOutcome::Changed(
            "fun f() {\n  let data = %{\n    key: [\n      call(\n        1\n    )]\n  }\n}".into()
        )
    );
}

#[test]
fn preserves_line_endings_when_source_mixes_crlf_cr_and_lf() {
    let source = "fun f() {\r\nprint(1  +  2)\rprint(1 + 2)\n  }\r\n";

    let actual = format_document(source, FormatOptions::new(2, true).unwrap());

    assert_eq!(
        actual,
        FormatOutcome::Changed("fun f() {\r\n  print(1  +  2)\r  print(1 + 2)\n}\r\n".into())
    );
}

#[test]
fn preserves_comments_and_blanks_when_block_comments_contain_fake_delimiters() {
    let source = "fun f() {\n \t// { fake\n \t\n/* (\n  } [\n /* nested */\n */ print(1)\nprint(2) /* }\n   ) */\n  }";

    let actual = format_document(source, FormatOptions::new(2, true).unwrap());

    assert_eq!(actual, FormatOutcome::Changed(
        "fun f() {\n \t// { fake\n \t\n/* (\n  } [\n /* nested */\n */ print(1)\n  print(2) /* }\n   ) */\n}".into()
    ));
}

#[test]
fn preserves_bom_and_shebang_when_formatting_following_code() {
    let source = "\u{feff}#!/usr/bin/env iris\r\nfun f() {\nprint(1)\n}";

    let actual = format_document(source, FormatOptions::new(1, true).unwrap());

    assert_eq!(
        actual,
        FormatOutcome::Changed("\u{feff}#!/usr/bin/env iris\r\nfun f() {\n print(1)\n}".into())
    );
}

#[test]
fn preserves_bom_when_first_code_line_has_indentation() {
    let source = "\u{feff}  fun f() {\nprint(1)\n}";

    let actual = format_document(source, FormatOptions::new(2, true).unwrap());

    assert_eq!(
        actual,
        FormatOutcome::Changed("\u{feff}fun f() {\n  print(1)\n}".into())
    );
}

#[test]
fn preserves_inline_bytes_when_tokens_are_spacing_sensitive() {
    let lines = [
        "print(1  +  2)",
        "print(1 + 2)",
        "let n = 0x1.fp3f32",
        "pkg::Thing.ready?",
        "property fun value!=(x) {}",
        "value!==other",
        "let value: Box<Array<String>> = item",
        "left >> right",
        "let pattern = /[{}()]+/im",
        "let n = 1/2",
        "let s = \"} [ ( // /*\"",
        "let b = b\"hi\"",
        "let m = m\"text\"",
        "let bytes = mb\"data\"",
        "let quote = '\"'",
        "let escaped = \"\\\"}\"",
    ];
    for line in lines {
        let source = format!("fun f() {{\n{line}\n}}");

        let actual = format_document(&source, FormatOptions::new(2, true).unwrap());

        assert_eq!(
            actual,
            FormatOutcome::Changed(format!("fun f() {{\n  {line}\n}}")),
            "{line}"
        );
    }
}

#[test]
fn is_idempotent_when_options_and_newlines_vary() {
    for tab_size in 1..=16 {
        for insert_spaces in [true, false] {
            for newline in ["\n", "\r\n", "\r"] {
                let options = FormatOptions::new(tab_size, insert_spaces).unwrap();
                let source = format!("fun f() {{{newline}print({newline}1{newline}){newline}}}");
                let FormatOutcome::Changed(formatted) = format_document(&source, options) else {
                    panic!("fixture must change");
                };

                let actual = format_document(&formatted, options);

                assert_eq!(actual, FormatOutcome::Unchanged);
                let unit = if insert_spaces {
                    " ".repeat(usize::try_from(tab_size).unwrap())
                } else {
                    "\t".into()
                };
                assert_eq!(
                    formatted,
                    format!(
                        "fun f() {{{newline}{unit}print({newline}{unit}{unit}1{newline}{unit}){newline}}}"
                    )
                );
            }
        }
    }
}

#[test]
fn returns_unchanged_when_document_needs_no_indentation() {
    for source in ["", " \t", "// hi\r\n", "print(1 + 2)", "\u{feff}", "{}\n"] {
        let actual = format_document(source, FormatOptions::new(4, true).unwrap());

        assert_eq!(actual, FormatOutcome::Unchanged);
    }
}
