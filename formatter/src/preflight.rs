use crate::{
    SkipReason,
    tree::{Node, Role},
};

pub fn syntax(nodes: &[Node<'_>]) -> Result<(), SkipReason> {
    for node in nodes {
        if let Node::Group(group) = node {
            if group.role == Role::Closure {
                closure_header(&group.children)?;
            }
            syntax(&group.children)?;
        }
    }
    Ok(())
}

pub fn closure_header(nodes: &[Node<'_>]) -> Result<Option<usize>, SkipReason> {
    let Some(start) = nodes
        .iter()
        .position(|node| !node.newline() && !node.comment())
    else {
        return Ok(None);
    };
    let close = match nodes[start].text() {
        "||" => start,
        "|" => nodes[start + 1..]
            .iter()
            .position(|node| node.text() == "|")
            .map(|index| start + 1 + index)
            .ok_or(SkipReason::ParseDiagnostics)?,
        _ => return Ok(None),
    };
    let after = close + 1;
    if nodes.get(after).is_some_and(|node| node.text() == "-") {
        let terminator = nodes[after..]
            .iter()
            .position(|node| node.newline() || node.text() == ";")
            .map(|index| after + index)
            .ok_or(SkipReason::ParseDiagnostics)?;
        Ok(Some(terminator))
    } else {
        let next = nodes[after..]
            .iter()
            .position(|node| !node.comment())
            .map(|index| after + index)
            .ok_or(SkipReason::ParseDiagnostics)?;
        if nodes[next].newline() || nodes[next].text() == ";" {
            Ok(Some(next))
        } else {
            Err(SkipReason::ParseDiagnostics)
        }
    }
}
