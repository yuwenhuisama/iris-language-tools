use super::{Node, Role, SkipReason, Writer};

pub(super) fn adjacent_newline(nodes: &[Node<'_>], index: usize) -> bool {
    nodes[index].newline()
        && (index
            .checked_sub(1)
            .is_some_and(|previous| nodes[previous].comment())
            || nodes.get(index + 1).is_some_and(Node::comment))
}

impl Writer {
    pub(super) fn comment(&mut self, text: &str, statements: bool) -> Result<bool, SkipReason> {
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.raw("  ")?;
        }
        self.raw(&crate::tokens::comment(text))?;
        let line_comment = text.starts_with("//") || text.starts_with("#!");
        if line_comment {
            if statements {
                self.line();
            } else {
                self.physical_line();
            }
        }
        Ok(line_comment)
    }

    pub(super) fn apply_comment_boundary(
        &mut self,
        nodes: &[Node<'_>],
        index: usize,
        statements: bool,
        pending_continuation: bool,
        base_indent: usize,
    ) -> bool {
        if !statements || pending_continuation || !self.comment_ends_statement(nodes, index) {
            return false;
        }
        self.spacing.comment_boundary();
        self.indent = base_indent;
        true
    }

    fn comment_ends_statement(&self, nodes: &[Node<'_>], index: usize) -> bool {
        nodes[index].text().contains(['\r', '\n'])
            && self.spacing.complete_expression()
            && !crate::tree::header_continues(nodes, index)
            && !nodes[index + 1..]
                .iter()
                .find(|node| !node.newline() && !node.comment())
                .is_some_and(|node| {
                    matches!(node.text(), "catch" | "finally")
                        || matches!(node, Node::Group(group) if matches!(group.role, Role::Body | Role::Accessors))
                })
    }
}
