use iris_syntax::{Expression, Statement};

use super::{Group, Node, Role, SkipReason, Writer};

impl Writer {
    pub(super) fn group(&mut self, group: &Group<'_>) -> Result<(), SkipReason> {
        match group.role {
            Role::Body => self.body(group),
            Role::Accessors => {
                let multiline = group.children.iter().any(Node::comment);
                self.token(group.open, "")?;
                if multiline {
                    self.line();
                    self.indent += 1;
                }
                for node in &group.children {
                    if node.newline() {
                        if multiline {
                            self.line();
                        }
                        continue;
                    }
                    if node.text() == ";" {
                        self.token(";", "")?;
                    } else {
                        self.sequence(std::slice::from_ref(node), false)?;
                    }
                }
                if multiline {
                    self.line();
                    self.indent -= 1;
                } else {
                    self.raw(" ")?;
                }
                self.raw(group.close)?;
                self.spacing.group_end(group.close);
                Ok(())
            }
            Role::Closure => self.closure(group),
            Role::List | Role::Parens | Role::Index => self.list(group),
        }
    }

    fn body(&mut self, group: &Group<'_>) -> Result<(), SkipReason> {
        self.token(group.open, "")?;
        self.line();
        self.indent += 1;
        self.sequence(&group.children, true)?;
        self.indent -= 1;
        self.line();
        self.raw(group.close)?;
        self.spacing.group_end(group.close);
        Ok(())
    }

    fn list(&mut self, group: &Group<'_>) -> Result<(), SkipReason> {
        let commas = crate::spacing::commas(&group.children);
        let trailing = group
            .children
            .iter()
            .rev()
            .find(|node| !node.newline())
            .is_some_and(|node| node.text() == ",");
        let singleton = group.role == Role::Parens && commas.len() == 1 && trailing;
        let mut flat = Self::default();
        let contents = if trailing && !singleton {
            let end = group
                .children
                .iter()
                .rposition(|node| node.text() == ",")
                .unwrap_or(group.children.len());
            &group.children[..end]
        } else {
            &group.children
        };
        let forced = group
            .children
            .iter()
            .any(|node| node.newline() || node.comment());
        if !forced {
            flat.sequence(contents, false)?;
        }
        let multiline = forced
            || flat.output.contains('\n')
            || self.column() + flat.output.chars().count() + 2 > 120;
        self.token(group.open, "")?;
        if multiline && !group.children.is_empty() {
            self.line();
            self.indent += 1;
            self.list_items(group, &commas)?;
            self.indent -= 1;
        } else {
            self.raw(&flat.output)?;
        }
        self.raw(group.close)?;
        self.spacing.group_end(group.close);
        Ok(())
    }

    fn closure(&mut self, group: &Group<'_>) -> Result<(), SkipReason> {
        let split = crate::preflight::closure_header(&group.children)?.map_or(0, |index| index + 1);
        let mut original = String::from("{");
        crate::spacing::append_source(&group.children, &mut original);
        original.push('}');
        let parsed = iris_parser::parse(&original);
        let simple = matches!(parsed.program.statements.as_slice(), [Statement::Expression(Expression::Closure { body, .. })] if matches!(body.as_slice(), [Statement::Expression(expression)] if simple_expression(expression)));
        let comments = group.children.iter().any(Node::comment);
        if simple && !comments {
            let mut flat = Self::default();
            flat.token("{", "")?;
            if split > 0 {
                flat.sequence(&group.children[..split - 1], false)?;
                flat.token(";", "")?;
            }
            flat.sequence(&group.children[split..], false)?;
            flat.raw(" }")?;
            if !flat.output.contains('\n') && self.column() + flat.output.chars().count() < 120 {
                self.token(&flat.output, "")?;
                self.spacing.group_end("}");
                return Ok(());
            }
        }
        self.token("{", "")?;
        if split > 0 {
            self.sequence(&group.children[..split - 1], false)?;
            self.token(";", "")?;
        }
        self.line();
        self.indent += 1;
        self.sequence(&group.children[split..], true)?;
        self.indent -= 1;
        self.line();
        self.raw("}")?;
        self.spacing.group_end("}");
        Ok(())
    }
}

impl Writer {
    fn list_items(&mut self, group: &Group<'_>, commas: &[usize]) -> Result<(), SkipReason> {
        let children = &group.children;
        let mut start = 0;
        while start < children.len() {
            while start < children.len() && children[start].newline() {
                start += 1;
            }
            if start == children.len() {
                break;
            }
            let end = commas
                .iter()
                .copied()
                .find(|index| *index >= start)
                .unwrap_or(children.len());
            let content = &children[start..end];
            let Some(last_code) = content
                .iter()
                .rposition(|node| !node.newline() && !node.comment())
            else {
                self.sequence(content, false)?;
                self.line();
                start = end.saturating_add(1);
                continue;
            };
            let add_comma = end < children.len()
                || group.role == Role::List
                || (group.role == Role::Parens && !commas.is_empty());
            self.sequence(&content[..=last_code], false)?;
            if add_comma {
                self.raw(",")?;
            }
            self.sequence(&content[last_code + 1..], false)?;
            start = end.saturating_add(1);
            if start < children.len() && children[start].comment() {
                self.sequence(&children[start..=start], false)?;
                start += 1;
            }
            self.line();
        }
        Ok(())
    }
}

const fn simple_expression(expression: &Expression) -> bool {
    !matches!(
        expression,
        Expression::If { .. }
            | Expression::While { .. }
            | Expression::Try { .. }
            | Expression::Closure { .. }
    )
}
