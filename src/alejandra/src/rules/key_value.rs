use std::collections::LinkedList;

use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;

pub(crate) fn rule(build_ctx: &BuildCtx, node: &rnix::SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let mut children = crate::children::Children::new(build_ctx, node);

    let vertical = build_ctx.vertical || children.has_comments() || children.has_newlines();

    // a
    let child = children.get_next().unwrap();
    if vertical {
        steps.push_back(BuildStep::FormatWider(child));
    } else {
        steps.push_back(BuildStep::Format(child));
    }

    // /**/
    let mut comment = false;
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => {
            comment = true;
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
            steps.push_back(BuildStep::Comment(text));
        }
        crate::children::Trivia::Whitespace(_) => {}
    });
    if comment {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    } else {
        steps.push_back(BuildStep::Whitespace);
    }

    // peek: =
    let child_equal = children.get_next().unwrap();

    // peek: /**/
    let mut comments_before = LinkedList::new();
    let mut newlines = false;
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => comments_before.push_back(BuildStep::Comment(text)),
        crate::children::Trivia::Whitespace(text) => {
            if crate::utils::count_newlines(&text) > 0 {
                newlines = true;
            }
        }
    });

    // peek: expr
    let child_expr = children.get_next().unwrap();

    // Superfluous parens can be removed: `a = (x);` -> `a = x;`
    let child_expr = if matches!(child_expr.kind(), rnix::SyntaxKind::NODE_PAREN) {
        let mut children: Vec<rnix::SyntaxElement> = child_expr
            .as_node()
            .unwrap()
            .children_with_tokens()
            .collect();

        if children.len() == 3 {
            children.swap_remove(1)
        } else {
            child_expr
        }
    } else {
        child_expr
    };

    // peek: /**/
    let mut comments_after = LinkedList::new();
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => comments_after.push_back(BuildStep::Comment(text)),
        crate::children::Trivia::Whitespace(_) => {}
    });

    // =
    let mut dedent = false;
    steps.push_back(BuildStep::Format(child_equal));

    if vertical {
        use rnix::SyntaxKind::*;

        let node_gets_whitespace = matches!(
            child_expr.kind(),
            NODE_ASSERT |
                NODE_ATTR_SET |
                NODE_PAREN |
                NODE_LAMBDA |
                NODE_LET_IN |
                NODE_LIST |
                NODE_STRING |
                NODE_WITH
        );

        let node_is_apply = matches!(child_expr.kind(), NODE_APPLY);

        let snd_thru_penult_indented = crate::utils::second_through_penultimate_line_are_indented(
            build_ctx,
            child_expr.clone(),
            false,
        );

        // FIXME: how the FUCK does this entire if chain work. what the fuck.
        if !comments_before.is_empty() || !comments_after.is_empty() {
            dedent = true;
            // For expanded values, allow starting the value on the same line
            // *if* it is a function. This allows constructs like
            // foo = { some, arguments }:
            //   lambda_body
            if child_expr.kind() != rnix::SyntaxKind::NODE_LAMBDA {
                steps.push_back(BuildStep::Indent);
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
            }
            steps.push_back(BuildStep::Whitespace);
        } else if matches!(child_expr.kind(), NODE_LET_IN) {
            steps.push_back(BuildStep::Indent);
        } else if node_gets_whitespace || (node_is_apply && snd_thru_penult_indented) {
            steps.push_back(BuildStep::Whitespace);
            //steps.push_back(BuildStep::Indent);
        } else {
            dedent = true;
            steps.push_back(BuildStep::Indent);
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }
    } else {
        steps.push_back(BuildStep::Whitespace);
    }

    // /**/
    for comment in comments_before {
        steps.push_back(comment);
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }

    // expr
    if vertical {
        steps.push_back(BuildStep::FormatWider(child_expr));
        if !comments_after.is_empty() {
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }
    } else {
        steps.push_back(BuildStep::Format(child_expr));
    }

    // /**/
    for comment in comments_after {
        steps.push_back(comment);
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }

    // ;
    let child = children.get_next().unwrap();
    steps.push_back(BuildStep::Format(child));
    if dedent {
        steps.push_back(BuildStep::Dedent);
    }

    steps
}
