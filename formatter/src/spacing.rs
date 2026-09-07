use crate::tree::{Node, ends_expression};

#[derive(Default)]
enum PreviousRole {
    #[default]
    Plain,
    Selector,
    Operator,
}

#[derive(Default)]
pub struct Spacing {
    previous: String,
    tight_after: bool,
    generic_depth: usize,
    closure_header: bool,
    previous_role: PreviousRole,
}

impl Spacing {
    pub const fn continuation(&self) -> bool {
        matches!(self.previous_role, PreviousRole::Operator) && !self.tight_after
    }

    fn is_operator(text: &str) -> bool {
        matches!(
            text,
            "=" | "+"
                | "-"
                | "*"
                | "/"
                | "**"
                | "=="
                | "!="
                | "&&"
                | "||"
                | "&"
                | "|"
                | "^"
                | "<="
                | ">="
                | "<"
                | ">"
                | "<=>"
                | "=~"
                | "!~"
                | "<<"
                | ">>"
                | "+="
                | "-="
                | "*="
                | "/="
                | "**="
                | "&="
                | "|="
                | "^="
                | "<<="
                | ">>="
                | "&&="
                | "||="
                | "is"
                | "as"
                | "as?"
                | "..="
                | "..<"
        )
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn complete_expression(&self) -> bool {
        self.generic_depth == 0
            && !self.tight_after
            && !self.continuation()
            && ends_expression(&self.previous)
    }

    pub fn comment_boundary(&mut self) {
        self.reset();
        self.previous.push('\n');
    }

    pub fn gap(&mut self, text: &str, following: &str) -> bool {
        let previous = self.previous.as_str();
        let prefix = matches!(text, "!" | "~" | "+" | "-")
            && (!ends_expression(previous) || self.continuation());
        let suffix = text == "!"
            && !self.continuation()
            && ends_expression(previous)
            && previous
                .chars()
                .next()
                .is_some_and(|first| first.is_alphabetic() || first == '_');
        let channel = matches!(text, "*" | "**" | "&") && matches!(previous, "" | "(" | ",");
        let symbol = text == ":" && (!ends_expression(previous) || previous == ":");
        let arrow_tail = text == ">" && matches!(previous, "-" | "=");
        let generic_open = text == "<" && (following == "<generic>" || self.generic_depth > 0);
        let generic_close = matches!(text, ">" | ">>") && self.generic_depth > 0 && !arrow_tail;
        let header_bar = text == "|" && (self.closure_header || previous == "{");
        let gap = !previous.is_empty()
            && !self.tight_after
            && !matches!(text, "," | ";" | ")" | "]" | "::" | "." | ".." | "?")
            && (text != ":" || symbol)
            && !arrow_tail
            && !generic_open
            && !generic_close
            && !suffix
            && !(header_bar && self.closure_header)
            && !(matches!(text, "(" | "[")
                && (ends_expression(previous)
                    || matches!(self.previous_role, PreviousRole::Selector)));
        if generic_open {
            self.generic_depth += 1;
        }
        if generic_close {
            self.generic_depth = self.generic_depth.saturating_sub(text.len());
        }
        if header_bar {
            self.closure_header = !self.closure_header;
        }
        self.tight_after = matches!(text, "." | ".." | "::" | "@" | "@@" | "$" | "(" | "[")
            || prefix
            || channel
            || symbol
            || generic_open
            || (header_bar && self.closure_header);
        if arrow_tail {
            self.tight_after = false;
        }
        self.previous_role = if matches!(previous, "." | ".." | "fun") {
            PreviousRole::Selector
        } else if (Self::is_operator(text) || (text == "?" && previous == "as"))
            && !generic_open
            && !generic_close
            && !arrow_tail
        {
            PreviousRole::Operator
        } else {
            PreviousRole::Plain
        };
        self.previous = text.into();
        gap
    }

    pub fn group_end(&mut self, close: &str) {
        self.previous = close.into();
        self.tight_after = false;
        self.previous_role = PreviousRole::Plain;
    }
}

pub fn generic_start(nodes: &[Node<'_>], index: usize) -> bool {
    if nodes[index].text() != "<" || index == 0 {
        return false;
    }
    let mut depth = 0_usize;
    let mut end = None;
    for (offset, node) in nodes[index..].iter().enumerate() {
        match node.text() {
            "<" => depth += 1,
            ">" if offset > 0 && nodes[index + offset - 1].text() == "-" => {}
            ">" | ">>" => {
                depth = depth.saturating_sub(node.text().len());
                if depth == 0 {
                    end = Some(index + offset);
                    break;
                }
            }
            ";" | "=" | "{" => break,
            _ => {}
        }
    }
    let Some(end) = end else {
        return false;
    };
    let following = nodes[end + 1..]
        .iter()
        .find_map(|node| {
            if node.comment() {
                node.text().contains(['\r', '\n']).then_some("\n")
            } else {
                Some(node.text())
            }
        })
        .unwrap_or("");
    if !matches!(
        following,
        "" | "\n"
            | ","
            | "("
            | ")"
            | "]"
            | ":"
            | "="
            | "{"
            | "."
            | ".."
            | ";"
            | "?"
            | "where"
            | "extends"
            | "implements"
    ) {
        return false;
    }
    let mut probe = format!("let value: {}", nodes[index - 1].text());
    append_source(&nodes[index..=end], &mut probe);
    let parsed = iris_parser::parse(&probe);
    parsed.program_accepted && parsed.is_clean()
}

pub fn commas(nodes: &[Node<'_>]) -> Vec<usize> {
    let mut depth = 0_usize;
    let mut result = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        match node.text() {
            "<" if depth > 0 || generic_start(nodes, index) => depth += 1,
            ">" if index > 0 && nodes[index - 1].text() == "-" => {}
            ">" | ">>" => depth = depth.saturating_sub(node.text().len()),
            "," if depth == 0 => result.push(index),
            _ => {}
        }
    }
    result
}

pub fn append_source(nodes: &[Node<'_>], output: &mut String) {
    for (index, node) in nodes.iter().enumerate() {
        match node {
            Node::Token(piece) => {
                output.push_str(piece.text);
                let adjacent_colon = piece.text == ":"
                    && matches!(nodes.get(index + 1), Some(Node::Token(next)) if next.text == ":" && piece.end == next.start);
                if !adjacent_colon {
                    output.push(' ');
                }
            }
            Node::Group(group) => {
                output.push_str(group.open);
                append_source(&group.children, output);
                output.push_str(group.close);
            }
        }
    }
}
