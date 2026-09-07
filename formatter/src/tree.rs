use crate::{
    SkipReason,
    safety::NESTING_LIMIT,
    tokens::{Kind, Piece},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Body,
    Closure,
    Accessors,
    List,
    Parens,
    Index,
}

#[derive(Debug)]
pub struct Group<'a> {
    pub open: &'a str,
    pub close: &'a str,
    pub role: Role,
    pub children: Vec<Node<'a>>,
}

#[derive(Debug)]
pub enum Node<'a> {
    Token(Piece<'a>),
    Group(Group<'a>),
}

impl Node<'_> {
    pub const fn text(&self) -> &str {
        match self {
            Self::Token(piece) => piece.text,
            Self::Group(group) => group.open,
        }
    }

    pub const fn newline(&self) -> bool {
        matches!(
            self,
            Self::Token(Piece {
                kind: Kind::Newline,
                ..
            })
        )
    }

    pub const fn comment(&self) -> bool {
        matches!(
            self,
            Self::Token(Piece {
                kind: Kind::Comment,
                ..
            })
        )
    }
}

pub fn build<'a>(pieces: &[Piece<'a>]) -> Result<Vec<Node<'a>>, SkipReason> {
    let mut stack: Vec<Group<'a>> = Vec::new();
    let mut root = Vec::new();
    for &piece in pieces {
        let target = match stack.last_mut() {
            Some(group) => &mut group.children,
            None => &mut root,
        };
        let closer = match (piece.kind, piece.text) {
            (Kind::Code, "{" | "%{") => Some("}"),
            (Kind::Code, "(") => Some(")"),
            (Kind::Code, "[") => Some("]"),
            _ => None,
        };
        if let Some(close) = closer {
            let role = role(piece.text, target);
            if stack.len() == NESTING_LIMIT {
                return Err(SkipReason::NestingLimit);
            }
            stack.push(Group {
                open: piece.text,
                close,
                role,
                children: Vec::new(),
            });
        } else if piece.kind == Kind::Code && matches!(piece.text, "}" | ")" | "]") {
            let group = stack.pop().ok_or(SkipReason::MismatchedDelimiter)?;
            if group.close != piece.text {
                return Err(SkipReason::MismatchedDelimiter);
            }
            match stack.last_mut() {
                Some(parent) => parent.children.push(Node::Group(group)),
                None => root.push(Node::Group(group)),
            }
        } else {
            target.push(Node::Token(piece));
        }
    }
    if !stack.is_empty() {
        return Err(SkipReason::UnclosedDelimiter);
    }
    Ok(root)
}

fn role(open: &str, before: &[Node<'_>]) -> Role {
    let previous = before.last().map_or("", |node| match node {
        Node::Token(piece) => piece.text,
        Node::Group(group) => group.close,
    });
    match open {
        "[" if ends_expression(previous) => Role::Index,
        "%{" | "[" => Role::List,
        "(" if ends_expression(previous)
            && !matches!(previous, "if" | "while" | "catch" | "match" | "return") =>
        {
            Role::List
        }
        "{" => {
            let header = before.iter().enumerate().rev().skip_while(|(_, node)| node.newline()).take_while(|(index, node)| {
                (!node.newline() || header_continues(before, *index)) && !matches!(node.text(), ";" | "{") && !matches!(node, Node::Group(group) if matches!(group.role, Role::Body | Role::Closure | Role::Accessors))
            }).map(|(_, node)| node).collect::<Vec<_>>();
            if header.iter().any(|node| node.text() == "property")
                && !header.iter().any(|node| node.text() == "fun")
            {
                Role::Accessors
            } else if header.iter().any(|node| {
                matches!(
                    node.text(),
                    "fun"
                        | "class"
                        | "module"
                        | "contract"
                        | "if"
                        | "else"
                        | "while"
                        | "for"
                        | "match"
                        | "try"
                        | "catch"
                        | "finally"
                )
            }) {
                Role::Body
            } else {
                Role::Closure
            }
        }
        _ => Role::Parens,
    }
}

pub fn header_continues(nodes: &[Node<'_>], index: usize) -> bool {
    let before = &nodes[..index];
    let mut preceding = before
        .iter()
        .rev()
        .filter(|node| !node.newline() && !node.comment());
    let previous = preceding.next().map_or("", Node::text);
    let next = nodes[index + 1..]
        .iter()
        .find(|node| !node.newline() && !node.comment())
        .map_or("", Node::text);
    let clause = |text| matches!(text, "extends" | "for" | "mixin" | "where" | "meta");
    let arrow = previous == ">" && preceding.next().is_some_and(|node| node.text() == "-");
    if !(clause(next)
        || clause(previous)
        || arrow
        || matches!(previous, ":" | "&" | "|" | "<" | ">" | ">>" | ",")
        || matches!(next, ">" | ">>" | "," | "&" | "|"))
    {
        return false;
    }
    for (start, node) in before.iter().enumerate().rev() {
        match node.text() {
            "fun" | "class" | "module" | "contract" | "property" => {
                let mut depth = 0_usize;
                for (offset, node) in before[start..].iter().enumerate() {
                    match node.text() {
                        "<" if depth > 0
                            || crate::spacing::generic_start(nodes, start + offset) =>
                        {
                            depth += 1;
                        }
                        ">" if start + offset > 0 && nodes[start + offset - 1].text() == "-" => {}
                        ">" | ">>" => depth = depth.saturating_sub(node.text().len()),
                        _ => {}
                    }
                }
                return clause(next)
                    || clause(previous)
                    || arrow
                    || depth > 0
                    || matches!(previous, ":" | "&" | "|")
                    || (previous == "," && before[start..].iter().any(|node| clause(node.text())));
            }
            ";" | "{" | "let" | "return" | "raise" | "yield" | "=" => return false,
            _ => {}
        }
    }
    false
}

pub fn ends_expression(text: &str) -> bool {
    text.chars().last().is_some_and(|last| {
        last.is_alphanumeric()
            || matches!(last, '_' | '?' | '!' | ')' | ']' | '}' | '"' | '\'' | '>')
    }) && !matches!(
        text,
        "let" | "mut" | "const" | "return" | "raise" | "yield" | "in" | "is" | "as" | "await"
    )
}
