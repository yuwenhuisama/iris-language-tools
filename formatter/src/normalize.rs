use iris_syntax::{Declaration, ExportDeclaration, MatchBody, Program, ProgramEntry, Statement};

use crate::normalize_expression::{decorators, expression, optional_type, types};

pub fn program(mut program: Program) -> Program {
    for value in &mut program.declarations {
        declaration(value);
    }
    statements(&mut program.statements);
    for entry in &mut program.entries {
        match entry {
            ProgramEntry::Declaration(value) => declaration(value),
            ProgramEntry::Statement(value) => statement(value),
        }
    }
    program
}

fn declaration(value: &mut Declaration) {
    match value {
        Declaration::Class(value) => {
            decorators(&mut value.decorators);
            optional_type(&mut value.extends);
            types(&mut value.implements);
            for mixin in &mut value.mixins {
                crate::normalize_expression::annotation(&mut mixin.target);
            }
            for constraint in &mut value.constraints {
                crate::normalize_expression::annotation(&mut constraint.bound);
            }
            statements(&mut value.body);
        }
        Declaration::Module(value) => {
            decorators(&mut value.decorators);
            types(&mut value.contract_for);
            for mixin in &mut value.mixins {
                crate::normalize_expression::annotation(&mut mixin.target);
            }
            for constraint in &mut value.constraints {
                crate::normalize_expression::annotation(&mut constraint.bound);
            }
            statements(&mut value.body);
        }
        Declaration::Contract(value) => {
            decorators(&mut value.decorators);
            types(&mut value.parents);
            for constraint in &mut value.constraints {
                crate::normalize_expression::annotation(&mut constraint.bound);
            }
            statements(&mut value.body);
        }
        Declaration::Import(_) => {}
        Declaration::Export(value) => match value.as_mut() {
            ExportDeclaration::Declaration(value) => declaration(value),
            ExportDeclaration::Names(_) => {}
        },
        Declaration::TypeAlias(value) => crate::normalize_expression::annotation(&mut value.target),
    }
}

pub fn statements(values: &mut [Statement]) {
    for value in values {
        statement(value);
    }
}

fn statement(value: &mut Statement) {
    match value {
        Statement::GlobalBinding {
            annotation, value, ..
        }
        | Statement::SharedBinding {
            annotation, value, ..
        }
        | Statement::Binding {
            annotation, value, ..
        } => {
            optional_type(annotation);
            expression(value);
        }
        Statement::DeferredBinding { .. } | Statement::Continue(_) => {}
        Statement::StoredProperty {
            decorators: attached,
            annotation,
            initializer,
            ..
        } => {
            decorators(attached);
            crate::normalize_expression::annotation(annotation);
            expression(initializer);
        }
        Statement::Method(value) => {
            decorators(&mut value.decorators);
            for parameter in &mut value.parameters {
                optional_type(&mut parameter.annotation);
                optional_expression(&mut parameter.default);
            }
            optional_type(&mut value.return_type);
            optional_body(&mut value.body);
        }
        Statement::Expression(value) => expression(value),
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            expression(condition);
            statements(then_body);
            optional_body(else_body);
        }
        Statement::Return(value) | Statement::Break { value, .. } => optional_expression(value),
        Statement::While {
            condition, body, ..
        } => {
            expression(condition);
            statements(body);
        }
        Statement::For { iterable, body, .. } => {
            expression(iterable);
            statements(body);
        }
        Statement::Match {
            subject,
            arms,
            fallback,
        } => {
            expression(subject);
            for arm in arms {
                optional_expression(&mut arm.guard);
                match_body(&mut arm.body);
            }
            if let Some(body) = fallback {
                match_body(body);
            }
        }
        Statement::Raise(value) => {
            if let Some(value) = value {
                value.offset = 0;
                expression(&mut value.value);
                optional_expression(&mut value.cause);
            }
        }
        Statement::Try {
            body,
            catches,
            finally,
        } => {
            statements(body);
            handlers(catches);
            optional_body(finally);
        }
    }
}

pub fn handlers(catches: &mut [iris_syntax::CatchClause]) {
    for clause in catches {
        optional_type(&mut clause.filter);
        statements(&mut clause.body);
    }
}

fn match_body(body: &mut MatchBody) {
    match body {
        MatchBody::Expression(value) => expression(value),
        MatchBody::Block(body) => statements(body),
    }
}

pub fn optional_body(body: &mut Option<Vec<Statement>>) {
    if let Some(body) = body {
        statements(body);
    }
}

fn optional_expression(value: &mut Option<iris_syntax::Expression>) {
    if let Some(value) = value {
        expression(value);
    }
}
