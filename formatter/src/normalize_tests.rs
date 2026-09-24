use iris_syntax::{Expression, PostfixPart, Program, Statement};

#[test]
fn erases_only_raise_offsets_when_source_locations_change() {
    let first = iris_parser::parse("fun f(){raise error from cause}").program;
    let second = iris_parser::parse("\nfun f() {\n  raise error from cause\n}\n").program;
    assert_ne!(first, second);
    assert_eq!(
        super::normalize::program(first),
        super::normalize::program(second)
    );
}

#[test]
fn preserves_every_semantic_difference_when_raise_offsets_are_normalized() {
    let pairs = [
        ("raise first", "raise second"),
        ("raise first from second", "raise first from third"),
        ("let x:Integer=1", "let x:String=1"),
        ("fun f(x=1){x}", "fun f(x=2){x}"),
        ("let x=(1,)", "let x=(1)"),
        ("import First\nimport Second", "import Second\nimport First"),
        (
            "try {raise x} catch first {}",
            "try {raise x} catch second {}",
        ),
        ("class A{}", "class B{}"),
    ];
    for (first, second) in pairs {
        let first = iris_parser::parse(first);
        let second = iris_parser::parse(second);
        assert!(first.program_accepted && second.program_accepted);
        assert_ne!(
            super::normalize::program(first.program),
            super::normalize::program(second.program)
        );
    }
}

#[test]
fn visits_typeof_and_decorators_when_they_contain_nested_raise_statements() {
    let raise = |offset| Expression::Closure {
        parameters: Vec::new(),
        full_parameters: Vec::new(),
        is_async: false,
        return_type: None,
        has_header: false,
        body: vec![Statement::Raise(Some(iris_syntax::Raise {
            value: Expression::Name("problem".into()),
            cause: None,
            offset,
        }))],
    };
    let program = |offset| Program {
        declarations: Vec::new(),
        entries: Vec::new(),
        statements: vec![Statement::StoredProperty {
            decorators: vec![iris_syntax::Decorator {
                name: "tag".into(),
                arguments: vec![raise(offset)],
            }],
            visibility: iris_syntax::Visibility::Private,
            accessors: None,
            shared: false,
            class_level: false,
            name: "value".into(),
            annotation: iris_syntax::TypeExpression::Typeof(Box::new(raise(offset))),
            initializer: raise(offset),
        }],
    };
    assert_eq!(
        super::normalize::program(program(1)),
        super::normalize::program(program(99))
    );
}

#[test]
fn normalizes_raise_offsets_inside_new_expression_wrappers() {
    let raise = |offset| Expression::Closure {
        parameters: Vec::new(),
        full_parameters: Vec::new(),
        is_async: false,
        return_type: None,
        has_header: false,
        body: vec![Statement::Raise(Some(iris_syntax::Raise {
            value: Expression::Name("problem".into()),
            cause: None,
            offset,
        }))],
    };
    let program = |offset| Program {
        declarations: Vec::new(),
        entries: Vec::new(),
        statements: vec![Statement::Expression(Expression::SafeNavigation {
            receiver: Box::new(Expression::NonNull(Box::new(raise(offset)))),
            parts: vec![
                PostfixPart::Member {
                    selector: "method".into(),
                    safe: true,
                },
                PostfixPart::Call {
                    type_arguments: Vec::new(),
                    arguments: vec![
                        Expression::PositionalSpread {
                            value: Box::new(raise(offset)),
                        },
                        Expression::KeywordSpread {
                            value: Box::new(raise(offset)),
                        },
                        Expression::BlockArgument {
                            value: Box::new(raise(offset)),
                        },
                    ],
                },
                PostfixPart::Index(Box::new(raise(offset))),
                PostfixPart::TrailingBlock(Box::new(raise(offset))),
            ],
        })],
    };

    assert_eq!(
        super::normalize::program(program(1)),
        super::normalize::program(program(99))
    );
}

#[test]
fn normalizes_raise_offsets_inside_impl_methods_and_instance_fields() {
    let raise = |offset| Expression::Closure {
        parameters: Vec::new(),
        full_parameters: Vec::new(),
        is_async: false,
        return_type: None,
        has_header: false,
        body: vec![Statement::Raise(Some(iris_syntax::Raise {
            value: Expression::Name("problem".into()),
            cause: None,
            offset,
        }))],
    };
    let program = |offset| Program {
        declarations: vec![iris_syntax::Declaration::Impl(
            iris_syntax::ImplDeclaration {
                target: iris_syntax::TypeExpression::Typeof(Box::new(raise(offset))),
                contract: iris_syntax::TypeExpression::Typeof(Box::new(raise(offset))),
                constraints: vec![iris_syntax::Constraint {
                    parameter: "T".into(),
                    bound: iris_syntax::TypeExpression::Typeof(Box::new(raise(offset))),
                }],
                methods: vec![iris_syntax::MethodDeclaration {
                    decorators: Vec::new(),
                    is_async: false,
                    is_override: false,
                    impl_contract: None,
                    kind: iris_syntax::MethodKind::Instance,
                    selector: "run".into(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: Some(iris_syntax::TypeExpression::Typeof(Box::new(raise(offset)))),
                    visibility: iris_syntax::Visibility::Private,
                    body: Some(vec![Statement::InstanceField {
                        mutable: false,
                        name: "field".into(),
                        annotation: Some(iris_syntax::TypeExpression::Typeof(Box::new(raise(
                            offset,
                        )))),
                        value: raise(offset),
                    }]),
                }],
            },
        )],
        entries: Vec::new(),
        statements: Vec::new(),
    };

    assert_eq!(
        super::normalize::program(program(1)),
        super::normalize::program(program(99))
    );
}
