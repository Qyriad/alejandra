use std::fmt::Display;

#[derive(PartialEq)]
pub(crate) enum Step {
    Comment(String),
    Dedent,
    Format(rnix::SyntaxElement),
    FormatWider(rnix::SyntaxElement),
    Indent,
    NewLine,
    Pad,
    Token(rnix::SyntaxKind, String),
    Whitespace,
}

#[derive(Clone)]
pub(crate) struct BuildCtx {
    pub force_wide:         bool,
    pub force_wide_success: bool,
    pub indentation:        usize,
    pub pos_old:            crate::position::Position,
    pub path:               String,
    pub vertical:           bool,
}

pub(crate) fn build(build_ctx: &mut BuildCtx, element: rnix::SyntaxElement) -> Option<rowan::GreenNode> {
    let mut builder = rowan::GreenNodeBuilder::new();

    build_step(&mut builder, build_ctx, &crate::builder::Step::Format(element));

    if build_ctx.force_wide {
        if build_ctx.force_wide_success {
            Some(builder.finish())
        } else {
            None
        }
    } else {
        Some(builder.finish())
    }
}

fn build_step(builder: &mut rowan::GreenNodeBuilder, build_ctx: &mut BuildCtx, step: &crate::builder::Step) {
    if build_ctx.force_wide && !build_ctx.force_wide_success {
        return;
    }

    use crate::builder::Step::*;
    match step {
        Comment(text) => {
            let mut lines: Vec<String> = text
                .lines()
                .map(&str::trim_end)
                .map(&str::to_string)
                .collect();

            lines = lines
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    if index == 0 || line.is_empty() {
                        line.to_string()
                    } else {
                        format!("{0:<1$}{2}", "", 2 * build_ctx.indentation, line,)
                    }
                })
                .collect();

            add_token(builder, rnix::SyntaxKind::TOKEN_COMMENT, &lines.join("\n"));
        }
        Dedent => {
            build_ctx.indentation -= 1;
        }
        Format(element) => {
            format(builder, build_ctx, element);
        }
        FormatWider(element) => {
            format_wider(builder, build_ctx, element);
        }
        Indent => {
            build_ctx.indentation += 1;
        }
        NewLine => {
            build_ctx.force_wide_success = false;

            add_token(builder, rnix::SyntaxKind::TOKEN_WHITESPACE, "\n");
        }
        Pad => {
            if build_ctx.indentation > 0 {
                add_token(
                    builder,
                    rnix::SyntaxKind::TOKEN_WHITESPACE,
                    &format!("{0:<1$}", "", 2 * build_ctx.indentation),
                );
            }
        }
        Token(kind, text) => {
            add_token(builder, *kind, text);
        }
        Whitespace => {
            add_token(builder, rnix::SyntaxKind::TOKEN_WHITESPACE, " ");
        }
    }
}

fn add_token(builder: &mut rowan::GreenNodeBuilder, kind: rnix::SyntaxKind, text: &str) {
    builder.token(rowan::SyntaxKind(kind as u16), text);
}

fn format(builder: &mut rowan::GreenNodeBuilder, build_ctx: &mut BuildCtx, element: &rnix::SyntaxElement) {
    let kind = element.kind();

    match element {
        rnix::SyntaxElement::Node(node) => {
            builder.start_node(rowan::SyntaxKind(kind as u16));

            use rnix::SyntaxKind::*;
            let rule = match kind {
                // a b
                NODE_APPLY => crate::rules::apply::rule,

                // assert a; b
                NODE_ASSERT => crate::rules::scoped::rule,

                // { }
                NODE_ATTR_SET => crate::rules::attr_set::rule,

                // a $op b
                NODE_BIN_OP => crate::rules::bin_op::rule,

                // ${a} (interpolation but for NODE_SELECT)
                NODE_DYNAMIC => crate::rules::dynamic::rule,

                // $identifier
                NODE_IDENT => crate::rules::default,

                // if a then b else c
                NODE_IF_ELSE => crate::rules::if_else::rule,

                // inherit NODE_INHERIT_FROM? b+ ;
                NODE_INHERIT => crate::rules::inherit::rule,

                // ( a )
                NODE_INHERIT_FROM => crate::rules::paren::rule,

                NODE_KEY => crate::rules::default,

                // a = b;
                NODE_KEY_VALUE => crate::rules::key_value::rule,

                // a: b
                NODE_LAMBDA => crate::rules::lambda::rule,

                // let NODE_KEY_VALUE* in b;
                NODE_LET_IN => crate::rules::let_in::rule,

                // [ ... ]
                NODE_LIST => crate::rules::list::rule,

                // 1 | true | null
                NODE_LITERAL => crate::rules::default,

                // let { }
                NODE_LEGACY_LET => crate::rules::default,

                // a or b
                NODE_OR_DEFAULT => crate::rules::bin_op::rule,

                // ( a )
                NODE_PAREN => crate::rules::paren::rule,

                // a | a ? b
                NODE_PAT_BIND => crate::rules::pat_bind::rule,

                // { NODE_PAT_ENTRY* }
                NODE_PATTERN => crate::rules::pattern::rule,

                // NODE_PAT_BIND | TOKEN_ELLIPSIS
                NODE_PAT_ENTRY => crate::rules::pat_entry::rule,
                // /path/to/${a}
                NODE_PATH_WITH_INTERPOL => crate::rules::default,
                // implementation detail of rowan
                NODE_ROOT => crate::rules::root::rule,
                // a.b | a.NODE_DYNAMIC
                NODE_SELECT => crate::rules::select::rule,
                // "..." || ''...''
                NODE_STRING => crate::rules::string::rule,
                // ${a}
                NODE_STRING_INTERPOL => crate::rules::paren::rule,
                // !a
                NODE_UNARY_OP => crate::rules::default,
                // with a; b
                NODE_WITH => crate::rules::scoped::rule,
                kind => {
                    panic!("Missing rule for {:?} at: {}", kind, build_ctx.path);
                }
            };

            for step in rule(build_ctx, node) {
                build_step(builder, build_ctx, &step);
            }

            builder.finish_node();
        }
        rnix::SyntaxElement::Token(token) => {
            let text = token.text();
            add_token(builder, kind, text);
            build_ctx.pos_old.update(text);
        }
    }
}

#[rustfmt::skip]
fn format_wider(builder: &mut rowan::GreenNodeBuilder, build_ctx: &mut BuildCtx, element: &rnix::SyntaxElement) {
    match element {
        rnix::SyntaxElement::Node(node) => {
            let mut build_ctx_clone = build_ctx.clone();
            build_ctx_clone.vertical = !fits_in_single_line(build_ctx, node.clone().into());

            format(builder, &mut build_ctx_clone, element);
        }
        rnix::SyntaxElement::Token(_) => {
            format(builder, build_ctx, element);
        }
    };
}

pub(crate) fn fits_in_single_line(build_ctx_old: &BuildCtx, element: rnix::SyntaxElement) -> bool {
    // eprintln!("Checking if {} fits on a single line", element);
    let mut build_ctx = BuildCtx {
        force_wide: true,
        force_wide_success: true,
        vertical: false,
        ..build_ctx_old.clone()
    };

    build(&mut build_ctx, element).is_some()
}

pub(crate) fn make_isolated_token(kind: rnix::SyntaxKind, text: &str) -> rnix::SyntaxToken {
    use rowan::Language;

    let mut builder = rowan::GreenNodeBuilder::new();
    builder.start_node(rnix::NixLanguage::kind_to_raw(rnix::SyntaxKind::NODE_ROOT));
    builder.token(rnix::NixLanguage::kind_to_raw(kind), text);
    builder.finish_node();

    let node = builder.finish();

    rnix::SyntaxNode::new_root(node).first_token().unwrap()
}
