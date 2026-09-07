use crate::{
    SkipReason,
    safety::OUTPUT_LIMIT,
    spacing::{Spacing, generic_start},
    tokens::Kind,
    tree::{Group, Node, Role},
};

mod comments;
mod groups;

#[derive(Default)]
pub struct Writer {
    output: String,
    indent: usize,
    spacing: Spacing,
}

pub fn render(nodes: &[Node<'_>]) -> Result<String, SkipReason> {
    let mut writer = Writer::default();
    writer.sequence(nodes, true)?;
    while writer.output.ends_with('\n') {
        writer.output.pop();
    }
    if !writer.output.is_empty() {
        writer.output.push('\n');
    }
    Ok(writer.output)
}

impl Writer {
    fn column(&self) -> usize {
        self.output
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .chars()
            .count()
    }

    fn line(&mut self) {
        self.physical_line();
        self.spacing.reset();
    }

    fn physical_line(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn blank(&mut self) {
        self.line();
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }

    fn raw(&mut self, text: &str) -> Result<(), SkipReason> {
        if self.output.len() + text.len() + self.indent * 2 > OUTPUT_LIMIT {
            return Err(SkipReason::OutputLimit);
        }
        if self.output.is_empty() || self.output.ends_with('\n') {
            for _ in 0..self.indent {
                self.output.push_str("  ");
            }
        }
        self.output.push_str(text);
        Ok(())
    }

    fn token(&mut self, text: &str, following: &str) -> Result<(), SkipReason> {
        if self.spacing.gap(text, following) && !self.output.ends_with('\n') {
            self.raw(" ")?;
        }
        self.raw(text)
    }

    fn sequence(&mut self, nodes: &[Node<'_>], statements: bool) -> Result<(), SkipReason> {
        let mut index = 0;
        let mut boundary = true;
        let mut attachment = false;
        let mut pending_continuation = false;
        let (sequence_start, base_indent) = (self.output.len(), self.indent);
        while index < nodes.len() {
            let node = &nodes[index];
            let next = nodes.get(index + 1).map_or("", Node::text);
            pending_continuation = (node.newline() || node.comment())
                && (pending_continuation || self.spacing.continuation());
            if node.newline() || node.text() == ";" {
                if !statements && comments::adjacent_newline(nodes, index) {
                    self.physical_line();
                }
                let next_code = nodes[index + 1..].iter().find(|node| !node.newline());
                let joins = node.newline() && (crate::tree::header_continues(nodes, index) || next_code.is_some_and(|node| matches!(node.text(), "catch" | "finally") || matches!(node, Node::Group(group) if matches!(group.role, Role::Body | Role::Accessors))));
                if statements && !joins {
                    let trailing =
                        node.text() == ";" && nodes.get(index + 1).is_some_and(Node::comment);
                    if !trailing {
                        self.indent = base_indent + usize::from(pending_continuation);
                        self.line();
                    }
                    boundary = !trailing;
                }
                index += 1;
                continue;
            }
            if statements && boundary {
                let named = named_ahead(&nodes[index..]);
                if named && !attachment && self.output.len() > sequence_start {
                    self.blank();
                }
                attachment = node.comment() || node.text() == "@";
                boundary = false;
            }
            match node {
                Node::Token(piece) => match piece.kind {
                    Kind::Comment => {
                        if self.comment(piece.text, statements)? {
                            boundary = true;
                        } else if self.apply_comment_boundary(
                            nodes,
                            index,
                            statements,
                            pending_continuation,
                            base_indent,
                        ) {
                            (boundary, attachment) = (true, false);
                        }
                    }
                    Kind::Code | Kind::Literal => {
                        if self.column() + piece.text.chars().count() + 1 > 120
                            && self.spacing.continuation()
                            && piece.kind == Kind::Code
                        {
                            self.indent = base_indent + 1;
                            self.line();
                        }
                        if piece.text == ":"
                            && next == ":"
                            && matches!(nodes.get(index + 1), Some(Node::Token(next)) if piece.end == next.start)
                        {
                            self.token("::", "")?;
                            index += 1;
                        } else {
                            let following = if generic_start(nodes, index) {
                                "<generic>"
                            } else {
                                next
                            };
                            self.token(piece.text, following)?;
                        }
                    }
                    Kind::Newline => {}
                },
                Node::Group(group) => {
                    self.group(group)?;
                    if statements
                        && matches!(group.role, Role::Body | Role::Closure | Role::Accessors)
                    {
                        attachment = false;
                        let following = nodes[index + 1..].iter().find(|node| !node.newline());
                        if following.is_some_and(|node| {
                            !node.comment()
                                && !matches!(
                                    node.text(),
                                    "catch" | "finally" | "," | ";" | "." | ".." | "(" | "[" | ")"
                                )
                        }) {
                            self.line();
                            boundary = true;
                        }
                    }
                }
            }
            index += 1;
        }
        self.indent = base_indent;
        Ok(())
    }
}

fn named_ahead(nodes: &[Node<'_>]) -> bool {
    let mut property = false;
    for node in nodes {
        if node.comment() || node.newline() || node.text() == "@" {
            continue;
        }
        if matches!(node, Node::Group(_)) {
            continue;
        }
        match node.text() {
            "property" => property = true,
            "fun" => return true,
            "class" | "module" | "contract" | "type" if !property => return true,
            "public" | "private" | "protected" | "override" | "impl" | "async" | "open"
            | "export" => {}
            _ if node.text().starts_with(char::is_alphabetic)
                && nodes
                    .first()
                    .is_some_and(|node| node.text() == "@" || node.comment()) => {}
            _ => return false,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn attaches_docs_and_decorators_to_named_declarations() {
        let source = "import Z\nimport A\n/// docs\n@sealed()\nclass A {property x:Integer\nproperty y:Integer\nfun a(){1}\n/// next\n@trace()\nfun b(){2}}";
        let pieces = crate::tokens::scan(source).unwrap();
        let tree = crate::tree::build(&pieces).unwrap();
        assert_eq!(
            super::render(&tree).unwrap(),
            "import Z\nimport A\n\n/// docs\n@sealed()\nclass A {\n  property x: Integer\n  property y: Integer\n\n  fun a() {\n    1\n  }\n\n  /// next\n  @trace()\n  fun b() {\n    2\n  }\n}\n"
        );
    }

    #[test]
    fn spaces_unary_operands_when_following_colons_and_binary_operators() {
        let source = "let x= - 2+3*4;let y=a< b && ! ready;let z=foo (x: :ok,y : - 1);pkg :: Thing . ready?;view .. Contract . call()";
        let pieces = crate::tokens::scan(source).unwrap();
        let tree = crate::tree::build(&pieces).unwrap();
        let actual = super::render(&tree).unwrap();
        assert_eq!(
            actual,
            "let x = -2 + 3 * 4\nlet y = a < b && !ready\nlet z = foo(x: :ok, y: -1)\npkg::Thing.ready?\nview..Contract.call()\n"
        );
    }
}
