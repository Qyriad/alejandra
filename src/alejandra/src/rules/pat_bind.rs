use std::collections::LinkedList;

use rnix::SyntaxNode;

use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;

pub(crate) fn rule(build_ctx: &BuildCtx, node: &SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let mut children = crate::children::Children::new(build_ctx, node);
    dbg!(&children);

    let vertical = children.has_comments() || children.has_newlines() || build_ctx.vertical;

    let child = children.get_next().unwrap();
    if vertical {
        steps.push_back(BuildStep::FormatWider(child));
    } else {
        steps.push_back(BuildStep::Format(child));
    }

    let mut comment = false;
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => {
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
            steps.push_back(BuildStep::Comment(text));
            comment = true;
        }
        crate::children::Trivia::Whitespace(_) => {}
    });

    if comment {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    } else {
        steps.push_back(BuildStep::Whitespace);
    }

    let child = children.get_next().unwrap();
    if vertical {
        steps.push_back(BuildStep::FormatWider(child));
    } else {
        steps.push_back(BuildStep::Format(child));
    }
    children.move_prev();

    steps
}
