pub(crate) fn rule(
    build_ctx: &crate::builder::BuildCtx,
    node: &rnix::SyntaxNode,
) -> std::collections::LinkedList<crate::builder::Step> {
    let mut steps = std::collections::LinkedList::new();

    let mut children = crate::children::Children::new(build_ctx, node);

    let vertical = children.has_comments() || children.has_newlines() || build_ctx.vertical;
    // eprintln!("node = {}", &node);
    // dbg!((children.has_comments(), children.has_newlines(), build_ctx.vertical));

    // a
    let child = children.get_next().unwrap();
    if vertical {
        steps.push_back(crate::builder::Step::FormatWider(child));
    } else {
        steps.push_back(crate::builder::Step::Format(child));
    }

    if let rnix::SyntaxKind::TOKEN_COMMENT | rnix::SyntaxKind::TOKEN_WHITESPACE =
        children.peek_next().unwrap().kind()
    {
        steps.push_back(crate::builder::Step::NewLine);
        steps.push_back(crate::builder::Step::Pad);
    }

    // /**/
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => {
            steps.push_back(crate::builder::Step::Comment(text));
            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
        }
        crate::children::Trivia::Whitespace(_) => {}
    });

    // :
    let child = children.get_next().unwrap();
    steps.push_back(crate::builder::Step::Format(child));

    // /**/
    let mut comment = false;
    children.drain_trivia(|element| match element {
        crate::children::Trivia::Comment(text) => {
            comment = true;
            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
            steps.push_back(crate::builder::Step::Comment(text));
        }
        crate::children::Trivia::Whitespace(_) => {}
    });

    // c
    let child = children.get_next().unwrap();
    if vertical {
        use rnix::SyntaxKind::*;
        let node_should_newline = !matches!(
            child.kind(),
            NODE_ATTR_SET | NODE_PAREN | NODE_LAMBDA | NODE_LET_IN | NODE_LIST | NODE_LITERAL | NODE_STRING,
        );
        if comment || node_should_newline {
            let node_should_indent = !matches!(
                child.kind(),
                NODE_ATTR_SET | NODE_PAREN | NODE_LAMBDA | NODE_LET_IN | NODE_LIST | NODE_STRING
            );
            let should_indent = node_should_indent && build_ctx.indentation > 0;

            if should_indent {
                steps.push_back(crate::builder::Step::Indent);
            }

            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
            steps.push_back(crate::builder::Step::FormatWider(child));

            if should_indent {
                steps.push_back(crate::builder::Step::Dedent);
            }
        } else {
            steps.push_back(crate::builder::Step::Whitespace);
            steps.push_back(crate::builder::Step::FormatWider(child));
        }
    } else {
        steps.push_back(crate::builder::Step::Whitespace);
        steps.push_back(crate::builder::Step::Format(child));
    }

    steps
}
