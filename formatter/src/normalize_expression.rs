use iris_syntax::{Decorator, Expression, TypeExpression};

use crate::normalize::{handlers, optional_body, statements};

pub fn expression(value: &mut Expression) {
    match value {
        Expression::ReifiedType(value) => annotation(value),
        Expression::ClosedGeneric { arguments, .. } => types(arguments),
        Expression::Await(value)
        | Expression::Grouped(value)
        | Expression::Unary { operand: value, .. }
        | Expression::Member {
            receiver: value, ..
        }
        | Expression::ContractView {
            receiver: value, ..
        }
        | Expression::KeywordArgument { value, .. } => expression(value),
        Expression::Yield(value) => {
            if let Some(value) = value {
                expression(value);
            }
        }
        Expression::Name(_)
        | Expression::Literal(_)
        | Expression::Symbol(_)
        | Expression::RawIvar(_)
        | Expression::ClassVar(_)
        | Expression::GlobalVar(_) => {}
        Expression::Array(values) | Expression::Tuple(values) => expressions(values),
        Expression::Hash(entries) => {
            for (key, value) in entries {
                expression(key);
                expression(value);
            }
        }
        Expression::Closure {
            return_type, body, ..
        } => {
            optional_type(return_type);
            statements(body);
        }
        Expression::Call {
            callee,
            type_arguments,
            arguments,
        } => {
            expression(callee);
            types(type_arguments);
            expressions(arguments);
        }
        Expression::Index { receiver, index } => {
            expression(receiver);
            expression(index);
        }
        Expression::Binary { left, right, .. } | Expression::Assignment { left, right, .. } => {
            expression(left);
            expression(right);
        }
        Expression::If {
            condition,
            then_body,
            else_body,
        } => {
            expression(condition);
            statements(then_body);
            optional_body(else_body);
        }
        Expression::While {
            condition, body, ..
        } => {
            expression(condition);
            statements(body);
        }
        Expression::Try {
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

fn expressions(values: &mut [Expression]) {
    for value in values {
        expression(value);
    }
}

pub fn decorators(values: &mut [Decorator]) {
    for value in values {
        expressions(&mut value.arguments);
    }
}

pub fn types(values: &mut [TypeExpression]) {
    for value in values {
        annotation(value);
    }
}

pub fn optional_type(value: &mut Option<TypeExpression>) {
    if let Some(value) = value {
        annotation(value);
    }
}

pub fn annotation(value: &mut TypeExpression) {
    match value {
        TypeExpression::Name(_) => {}
        TypeExpression::Typeof(value) => expression(value),
        TypeExpression::Intersection(values)
        | TypeExpression::Union(values)
        | TypeExpression::Generic {
            arguments: values, ..
        } => types(values),
        TypeExpression::Function { parameters, result } => {
            types(parameters);
            annotation(result);
        }
    }
}
