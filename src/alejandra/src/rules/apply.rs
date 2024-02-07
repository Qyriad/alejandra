use std::collections::LinkedList;

use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;

pub(crate) fn rule(build_ctx: &BuildCtx, node: &rnix::SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let mut children = crate::children2::new(build_ctx, node);

    let first = children.next().unwrap();
    let second = children.next().unwrap();

    let vertical = build_ctx.vertical
        || first.has_inline_comment
        || first.has_trivialities
        || second.has_inline_comment
        || second.has_trivialities;

    // first
    if vertical {
        steps.push_back(BuildStep::FormatWider(first.element));
    } else {
        steps.push_back(BuildStep::Format(first.element));
    }

    if let Some(text) = first.inline_comment {
        steps.push_back(BuildStep::Whitespace);
        steps.push_back(BuildStep::Comment(text));
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }

    for trivia in first.trivialities {
        match trivia {
            crate::children2::Trivia::Comment(text) => {
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
                steps.push_back(BuildStep::Comment(text));
            }
            crate::children2::Trivia::Newlines(_) => {}
        }
    }

    // second
    if vertical {
        if !first.has_inline_comment
            && !first.has_trivialities
            && matches!(
                second.element.kind(),
                rnix::SyntaxKind::NODE_ATTR_SET
                    | rnix::SyntaxKind::NODE_LIST
                    | rnix::SyntaxKind::NODE_PAREN
                    | rnix::SyntaxKind::NODE_STRING
            )
        {
            steps.push_back(BuildStep::Whitespace);
        } else {
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        };
        steps.push_back(BuildStep::FormatWider(second.element));
    } else {
        steps.push_back(BuildStep::Whitespace);
        steps.push_back(BuildStep::Format(second.element));
    }

    steps
}
