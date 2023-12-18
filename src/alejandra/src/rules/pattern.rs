use std::collections::LinkedList;

use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;

pub(crate) fn rule(build_ctx: &BuildCtx, node: &rnix::SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let children = crate::children::Children::new(build_ctx, node);
    let pattern = crate::parsers::pattern::parse(build_ctx, node);

    let has_comments_between_curly_b = pattern
        .arguments
        .iter()
        .any(|arg| arg.comment_after.is_some() || !arg.comments_before.is_empty());

    let has_comments = has_comments_between_curly_b
        || !pattern.comments_after_initial_at.is_empty()
        || !pattern.comments_before_end_at.is_empty();

    let soft_len: u32 = node.text_range().len().into();
    let indentation_size = 2; // FIXME: detect
    let hard_len = (build_ctx.indentation as u32 * indentation_size as u32) + soft_len;

    let arguments_count = pattern.arguments.len();

    let vertical = has_comments
        // If the pattern is already formatted with newlines,
        // then keep it that way.
        || children.has_newlines()
        || (soft_len > 80)
        || (hard_len > 120)
        || (arguments_count > 0 && has_comments_between_curly_b)
        || (arguments_count > 6) // FIXME: why 5?
        || build_ctx.vertical;

    // x @
    if let Some(element) = &pattern.initial_at {
        let element = element.clone();
        if vertical {
            steps.push_back(BuildStep::FormatWider(element));
        } else {
            steps.push_back(BuildStep::Format(element));
        }
    }

    // /**/
    if !pattern.comments_after_initial_at.is_empty() {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
        for text in pattern.comments_after_initial_at {
            steps.push_back(BuildStep::Comment(text));
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }
    } else if pattern.initial_at.is_some() {
        steps.push_back(BuildStep::Whitespace);
    }

    // {
    steps.push_back(BuildStep::Token(
        rnix::SyntaxKind::TOKEN_CURLY_B_OPEN,
        "{".to_string(),
    ));
    if vertical {
        steps.push_back(BuildStep::Indent);
    }

    // arguments
    for (index, argument) in pattern.arguments.into_iter().enumerate() {
        if vertical {
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        } else {
            // For collapsed patterns, add a space before each argument
            // to make { ... } instead of {...}. This includes the first argument.
            steps.push_back(BuildStep::Whitespace);
        }

        // /**/
        if !argument.comments_before.is_empty() {
            for text in argument.comments_before {
                steps.push_back(BuildStep::Comment(text));
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
            }
        }

        // argument
        let element = argument.item.unwrap();
        let element_kind = element.kind();
        if vertical {
            steps.push_back(BuildStep::FormatWider(element));
        } else {
            steps.push_back(BuildStep::Format(element));
        };

        // ,
        if vertical {
            if !matches!(element_kind, rnix::SyntaxKind::TOKEN_ELLIPSIS) {
                steps.push_back(BuildStep::Token(rnix::SyntaxKind::TOKEN_COMMA, ",".to_string()));
            }
        } else if index + 1 < arguments_count {
            steps.push_back(BuildStep::Token(rnix::SyntaxKind::TOKEN_COMMA, ",".to_string()));
        };

        // possible inline comment
        if let Some(text) = argument.comment_after {
            if text.starts_with('#') {
                steps.push_back(BuildStep::Whitespace);
            } else {
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
            }
            steps.push_back(BuildStep::Comment(text));
        }
    }

    // /**/
    let has_comments_before_curly_b_close = !pattern.comments_before_curly_b_close.is_empty();
    for text in pattern.comments_before_curly_b_close {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
        steps.push_back(BuildStep::Comment(text));
    }

    // }
    if vertical {
        steps.push_back(BuildStep::Dedent);
        if arguments_count > 0 || has_comments_before_curly_b_close {
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }
    } else {
        // Add a space after the last argument, to make { ... } instead of {...}.
        steps.push_back(BuildStep::Whitespace);
    }
    steps.push_back(BuildStep::Token(
        rnix::SyntaxKind::TOKEN_CURLY_B_OPEN,
        "}".to_string(),
    ));

    // /**/
    if pattern.comments_before_end_at.is_empty() {
        if pattern.end_at.is_some() {
            steps.push_back(BuildStep::Whitespace);
        }
    } else {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
        for text in pattern.comments_before_end_at {
            steps.push_back(BuildStep::Comment(text));
            steps.push_back(BuildStep::NewLine);
            steps.push_back(BuildStep::Pad);
        }
    }

    // @ x
    if let Some(element) = pattern.end_at {
        if vertical {
            steps.push_back(BuildStep::FormatWider(element));
        } else {
            steps.push_back(BuildStep::Format(element));
        }
    }

    steps
}
