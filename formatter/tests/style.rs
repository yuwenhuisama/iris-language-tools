use iris_formatter::{FormatOutcome, format_document};

fn golden(source: &str, expected: &str) {
    let actual = format_document(source);
    if source == expected {
        assert_eq!(actual, FormatOutcome::Unchanged, "{source}");
    } else {
        assert_eq!(actual, FormatOutcome::Changed(expected.into()), "{source}");
    }
    assert_eq!(format_document(expected), FormatOutcome::Unchanged);
}

#[test]
fn expands_named_and_control_bodies_when_compact() {
    golden(
        "fun f(){if ready{print(1);}else{print(2);}}",
        "fun f() {\n  if ready {\n    print(1)\n  }\n  else {\n    print(2)\n  }\n}\n",
    );
    golden("class A{}\nclass B{}", "class A {\n}\n\nclass B {\n}\n");
}

#[test]
fn joins_braces_and_handlers_when_headers_are_split() {
    golden(
        "fun f()\n{\ntry {raise problem}\ncatch error {raise}\nfinally {cleanup()}\n}",
        "fun f() {\n  try {\n    raise problem\n  } catch error {\n    raise\n  } finally {\n    cleanup()\n  }\n}\n",
    );
}

#[test]
fn normalizes_spacing_when_operators_have_distinct_roles() {
    golden(
        "let x= - 2+3*4;let y=a< b && ! ready;let z=foo (x: :ok,y : - 1);pkg :: Thing . ready?;view .. Contract . call()",
        "let x = -2 + 3 * 4\nlet y = a < b && !ready\nlet z = foo(x: :ok, y: -1)\npkg::Thing.ready?\nview..Contract.call()\n",
    );
    golden(
        "let value : Box< Array< String >> =item;left>>right;let span=1..=10",
        "let value: Box<Array<String>> = item\nleft >> right\nlet span = 1 ..= 10\n",
    );
}

#[test]
fn retains_required_semicolons_when_closures_and_accessors_are_inline() {
    golden(
        "let f={|x:Integer|->Integer;x+1;};property name:String{get;private set;}",
        "let f = { |x: Integer| -> Integer; x + 1 }\nproperty name: String { get; private set; }\n",
    );
    golden(
        "let f={|x|;let y=x+1;y*2}",
        "let f = { |x|;\n  let y = x + 1\n  y * 2\n}\n",
    );
}

#[test]
fn formats_multiline_lists_when_source_is_already_broken() {
    golden(
        "let a=[1,\n2];let h=%{\n:ok:1,:bad:2\n};let t=(1,);foo(\n1,2\n)",
        "let a = [\n  1,\n  2,\n]\nlet h = %{\n  :ok: 1,\n  :bad: 2,\n}\nlet t = (1,)\nfoo(\n  1,\n  2,\n)\n",
    );
    golden(
        "let a=[1,2,];foo(1,);let t=(1,2,)",
        "let a = [1, 2]\nfoo(1)\nlet t = (1, 2)\n",
    );
}

#[test]
fn wraps_long_lists_when_the_line_exceeds_120_columns() {
    let arguments = ["alpha".repeat(8), "beta".repeat(10), "gamma".repeat(8)];
    let source = format!("call({})", arguments.join(","));
    let expected = format!(
        "call(\n  {},\n  {},\n  {},\n)\n",
        arguments[0], arguments[1], arguments[2]
    );
    golden(&source, &expected);
}

#[test]
fn attaches_docs_and_decorators_when_declarations_need_blank_lines() {
    golden(
        "import Z\nimport A\n/// docs\n@sealed()\nclass A {property x:Integer\nproperty y:Integer\nfun a(){1}\n/// next\n@trace()\nfun b(){2}}",
        "import Z\nimport A\n\n/// docs\n@sealed()\nclass A {\n  property x: Integer\n  property y: Integer\n\n  fun a() {\n    1\n  }\n\n  /// next\n  @trace()\n  fun b() {\n    2\n  }\n}\n",
    );
}

#[test]
fn preserves_comments_when_delimiters_inside_them_look_like_code() {
    golden(
        "fun f(){\r\n//fake {\r\n/* outer\r\nprint(9) } /* nested */\r\n*/\r\nprint(1);//tail\r\n}",
        "fun f() {\n  // fake {\n  /* outer\r\nprint(9) } /* nested */\r\n*/\n  print(1)  // tail\n}\n",
    );
}

#[test]
fn normalizes_exterior_newlines_when_bom_shebang_and_mixed_endings_exist() {
    golden(
        "\u{feff}#!/usr/bin/env iris\r\nfun f(){\rprint(1)\n}\r\n",
        "#!/usr/bin/env iris\nfun f() {\n  print(1)\n}\n",
    );
    golden("\u{feff}  fun f(){1}", "fun f() {\n  1\n}\n");
    golden(" \t\r\n", "");
    golden("", "");
}
