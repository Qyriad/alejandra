use std::collections::LinkedList;


use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;
#[allow(unused_imports)] // These are used for debugging.
use crate::utils::{FormatSyntax, FormatSyntaxOptions};

#[allow(unused_imports)]
use log::{trace, debug, info, warn, error};

pub(crate) fn rule(build_ctx: &BuildCtx, node: &rnix::SyntaxNode) -> LinkedList<BuildStep> {
    let mut steps = LinkedList::new();

    let mut children = crate::children2::new(build_ctx, node);

    let opener = children.next().unwrap();
    let expression = children.next().unwrap();
    let expr_kind = expression.element.kind();
    let closer = children.next().unwrap();

    // wtf is loose
    let loose = {
        use rnix::SyntaxKind::*;
        let nodes_to_check = [&opener, &expression, &closer];

        let any_have_trivial = nodes_to_check.iter().any(|node| node.has_trivialities);
        let any_have_inline_comment = nodes_to_check.iter().any(|node| node.has_inline_comment);
        let any_have_comments = nodes_to_check.iter().any(|node| node.has_comments);

        let node_should_loose_if_has_trivial = !matches!(
            expression.element.kind(),
            NODE_ATTR_SET | NODE_LITERAL | NODE_LIST | NODE_STRING | NODE_UNARY_OP,
        );

        if expr_kind == NODE_LAMBDA {

            //expression.element.log_syn(log::Level::Trace, Default::default());

            // A curried function will be a bunch of NODE_LAMBDAs and NODE_IDENTs;
            // if we see a node other than one of those two, we know we've reached the end
            // of the curried function definition.
            // If there are newlines in the *definition*, then we want to format "loose".
            // So we have to specifically search for newlines until we hit the
            // function body.
            // So we'll keep traversing subchildren to find a node that's not NODE_IDENT
            // or NODE_LAMBDA. Meanwhile, if we see a TOKEN_WHITESPACE that contains a
            // newline character, then we still need to check the next node, as that whitespace
            // could be the newline at the very end of the argument list.
            // FIXME: refactor to somewhere else; this doesn't need to be a nested function.

            #[derive(Debug, Copy, Clone, PartialEq)]
            enum LookState {
                KeepGoing(bool),
                Almost,
                Done(bool),
            }


            fn look_for_newline_until_func_end(element: &rnix::SyntaxElement, found_newline: bool) -> LookState
            {
                use LookState::*;
                let mut found_newline = found_newline;

                if matches!(element.kind(), TOKEN_WHITESPACE) {
                    if element.as_token().unwrap().text().contains("\n") {
                        return Almost;
                    }
                }

                if let Some(node) = element.as_node() {
                    let part_of_func_def = matches!(
                        node.kind(),
                        NODE_IDENT | NODE_LAMBDA,
                    );
                    if !part_of_func_def {
                        return Done(found_newline);
                    }

                    let node_children = node.children_with_tokens();
                    for child in node_children {
                        match look_for_newline_until_func_end(&child, found_newline) {
                            KeepGoing(true) => {
                                found_newline = true;
                            },
                            Done(result) => {
                                return Done(result);
                            },
                            Almost => {
                                return Almost;
                            },
                            _ => (),
                        }
                    }
                }

                KeepGoing(found_newline)
            }

            match look_for_newline_until_func_end(&expression.element, false) {
                LookState::Almost => false,
                LookState::Done(found_newline) => {
                    dbg!(found_newline);
                    found_newline
                },
                LookState::KeepGoing(found_newline) => {
                    dbg!(&found_newline);
                    error!("lambda definition `{}` seems incomplete?", &expression.element);
                    // I'm pretty sure this case is unreachable, but if it isn't, just fallback
                    // to the logic that's used for things that aren't lambdas.
                    any_have_inline_comment || any_have_comments || matches!(expr_kind, NODE_IF_ELSE) || (
                        any_have_trivial && node_should_loose_if_has_trivial
                    )
                },
            }
            //if let LookState::Done(true) = res {
            //    //dbg!(has_newline)
            //    true
            //} else {
            //    error!("lambda definition `{}` seems incomplete?", &expression.element);
            //    // I'm pretty sure this case is unreachable, but if it isn't, just fallback
            //    // to the logic that's used for things that aren't lambdas.
            //    any_have_inline_comment || any_have_comments || matches!(expr_kind, NODE_IF_ELSE) || (
            //        any_have_trivial && node_should_loose_if_has_trivial
            //    )
            //}

        } else {
            any_have_inline_comment || any_have_comments || matches!(expr_kind, NODE_IF_ELSE) || (
                any_have_trivial && node_should_loose_if_has_trivial
            )
        }
    };

    let should_indent = {
        use rnix::SyntaxKind::*;

        let node_can_indent = matches!(
            expr_kind,
            NODE_APPLY | NODE_ASSERT | NODE_BIN_OP | NODE_OR_DEFAULT | NODE_LAMBDA | NODE_SELECT | NODE_WITH,
        );

        node_can_indent && !crate::utils::second_through_penultimate_line_are_indented(
            build_ctx,
            expression.element.clone(),
            matches!(expr_kind, NODE_LAMBDA),
        )
    };

    // opener
    steps.push_back(BuildStep::Format(opener.element));
    if should_indent {
        steps.push_back(BuildStep::Indent);
    }

    if let Some(text) = opener.inline_comment {
        steps.push_back(BuildStep::Whitespace);
        steps.push_back(BuildStep::Comment(text));
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    } else if loose {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }

    for trivia in opener.trivialities {
        match trivia {
            crate::children2::Trivia::Comment(text) => {
                steps.push_back(BuildStep::Comment(text));
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
            }
            crate::children2::Trivia::Newlines(_) => {}
        }
    }

    // expression
    if loose {
        steps.push_back(BuildStep::FormatWider(expression.element));
    } else {
        steps.push_back(BuildStep::Format(expression.element));
    }

    if let Some(text) = expression.inline_comment {
        steps.push_back(BuildStep::Whitespace);
        steps.push_back(BuildStep::Comment(text));
    }

    for trivia in expression.trivialities {
        match trivia {
            crate::children2::Trivia::Comment(text) => {
                steps.push_back(BuildStep::NewLine);
                steps.push_back(BuildStep::Pad);
                steps.push_back(BuildStep::Comment(text));
            }
            crate::children2::Trivia::Newlines(_) => {}
        }
    }

    // closer
    if should_indent {
        steps.push_back(BuildStep::Dedent);
    }

    if loose {
        steps.push_back(BuildStep::NewLine);
        steps.push_back(BuildStep::Pad);
    }
    steps.push_back(BuildStep::Format(closer.element));

    steps
}
