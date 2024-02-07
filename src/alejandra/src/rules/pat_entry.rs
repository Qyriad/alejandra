use std::collections::LinkedList;

use rnix::SyntaxNode;

use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;

pub(crate) fn rule(build_ctx: &BuildCtx, node: &SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let mut children = crate::children::Children::new(build_ctx, node);
    let vertical = children.has_comments() || children.has_newlines() || build_ctx.vertical;

    // expr
    let child = children.get_next().unwrap();
    if vertical {
        steps.push_back(BuildStep::FormatWider(child));
    } else {
        steps.push_back(crate::builder::Step::Format(child));
    }

    if children.has_next() {
        // /**/
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

        // operator
        let child = children.get_next().unwrap();
        steps.push_back(BuildStep::Format(child));

        // /**/
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

        // expr
        let child = children.get_next().unwrap();
        let mut dedent = false;

        if comment {
            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
        } else if {
            use rnix::SyntaxKind::*;
            matches!(
                child.kind(),
                NODE_ATTR_SET
                    | NODE_IDENT
                    | NODE_PAREN
                    | NODE_LAMBDA
                    | NODE_LET_IN
                    | NODE_LIST
                    | NODE_LITERAL
                    | NODE_STRING,
            )
        } || crate::builder::fits_in_single_line(build_ctx, child.clone())
        {
            steps.push_back(BuildStep::Whitespace);
        } else {
            dedent = true;
            steps.push_back(BuildStep::Indent);
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }

        if vertical {
            steps.push_back(BuildStep::FormatWider(child));
        } else {
            steps.push_back(BuildStep::Format(child));
        }
        if dedent {
            steps.push_back(BuildStep::Dedent);
        }
    }

    steps
}
