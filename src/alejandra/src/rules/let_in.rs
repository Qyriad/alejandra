use std::collections::LinkedList;

use crate::builder::{BuildCtx, Step as BuildStep};

pub(crate) fn rule(build_ctx: &BuildCtx, node: &rnix::SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let mut children = crate::children::Children::new(build_ctx, node);

    let items_count = node
        .children()
        .filter(|element| {
            use rnix::SyntaxKind::*;
            matches!(
                element.kind(),
                NODE_KEY_VALUE | NODE_INHERIT | NODE_INHERIT_FROM
            )
        })
        .count();

    let vertical =
        items_count > 1 || children.has_comments() || children.has_newlines() || build_ctx.vertical;

    // let
    let child = children.get_next().unwrap();
    if vertical {
        // For expanded `let`s, put the `let` on a new line.
        // FIXME: what will this do for a file that starts immediately with a `let`?
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }
    steps.push_back(BuildStep::Format(child));
    if vertical {
        steps.push_back(BuildStep::Indent);
    }

    let mut item_index: usize = 0;
    let mut inline_next_comment = false;

    loop {
        // /**/
        children.drain_trivia(|element| match element {
            crate::children::Trivia::Comment(text) => {
                if inline_next_comment && text.starts_with('#') {
                    steps.push_back(BuildStep::Whitespace);
                } else {
                    steps.push_back(BuildStep::NewLine);
                    steps.push_back(BuildStep::Pad);
                }
                steps.push_back(BuildStep::Comment(text));
                inline_next_comment = false;
            }
            crate::children::Trivia::Whitespace(text) => {
                let newlines = crate::utils::count_newlines(&text);

                if newlines > 1 && item_index > 0 && item_index < items_count {
                    steps.push_back(BuildStep::NewLine);
                }

                inline_next_comment = newlines == 0;
            }
        });

        if let Some(child) = children.peek_next() {
            if let rnix::SyntaxKind::TOKEN_IN = child.kind() {
                break;
            }

            // expr
            item_index += 1;
            if vertical {
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
                steps.push_back(BuildStep::FormatWider(child));
            } else {
                steps.push_back(BuildStep::Whitespace);
                steps.push_back(BuildStep::Format(child));
            }

            children.move_next();
            inline_next_comment = true;
        }
    }

    if vertical {
        steps.push_back(BuildStep::Dedent);
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    } else {
        steps.push_back(BuildStep::Whitespace);
    }

    // in
    let child_in = children.get_next().unwrap();

    // /**/
    let mut child_comments = LinkedList::new();
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => {
            child_comments.push_back(BuildStep::Comment(text))
        }
        crate::children::Trivia::Whitespace(_) => {}
    });

    // expr
    let child_expr = children.get_next().unwrap();

    // in
    let mut dedent = false;
    steps.push_back(BuildStep::Format(child_in));
    if vertical {
        if child_comments.is_empty()
            && matches!(
                child_expr.kind(),
                rnix::SyntaxKind::NODE_ATTR_SET
                    | rnix::SyntaxKind::NODE_LET_IN
                    | rnix::SyntaxKind::NODE_LIST
                    | rnix::SyntaxKind::NODE_PAREN
                    | rnix::SyntaxKind::NODE_STRING
            )
        {
            steps.push_back(BuildStep::Whitespace);
        } else {
            dedent = true;
            steps.push_back(BuildStep::Indent);
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }
    }

    // /**/
    for comment in child_comments {
        steps.push_back(comment);
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }

    // expr
    if vertical {
        steps.push_back(BuildStep::FormatWider(child_expr));
        if dedent {
            steps.push_back(BuildStep::Dedent);
        }
    } else {
        steps.push_back(BuildStep::Whitespace);
        steps.push_back(BuildStep::Format(child_expr));
    }

    steps
}
