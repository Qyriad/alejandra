pub(crate) fn has_newlines(string: &str) -> bool {
    string.chars().any(|c| c == '\n')
}

pub(crate) fn count_newlines(string: &str) -> usize {
    string.chars().filter(|c| *c == '\n').count()
}

pub(crate) fn second_through_penultimate_line_are_indented(
    build_ctx: &crate::builder::BuildCtx,
    element: rnix::SyntaxElement,
    if_leq_than_two_lines: bool,
) -> bool {
    let mut build_ctx = crate::builder::BuildCtx {
        force_wide: false,
        ..build_ctx.clone()
    };

    let formatted = crate::builder::build(&mut build_ctx, element)
        .unwrap()
        .to_string();

    let formatted_lines: Vec<&str> = formatted.split('\n').collect();

    if formatted_lines.len() <= 2 {
        return if_leq_than_two_lines;
    }

    let whitespace = format!("{0:<1$}  ", "", 2 * build_ctx.indentation);
    let lambda = format!("{0:<1$}}}:", "", 2 * build_ctx.indentation);
    let in_ = format!("{0:<1$}in", "", 2 * build_ctx.indentation);

    formatted_lines.iter().skip(1).rev().skip(1).all(|line| {
        line.is_empty()
            || line.starts_with(&lambda)
            || line.starts_with(&in_)
            || line.starts_with(&whitespace)
    })
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FormatSyntaxOptions {
    pub short: bool,
    pub recursive: bool,
}

impl Default for FormatSyntaxOptions {
    fn default() -> Self {
        Self {
            short: true,
            recursive: true,
        }
    }
}

impl FormatSyntaxOptions {
    pub fn log_for<T: FormatSyntax>(self, element: &T, level: log::Level) {
        element.log_syn(level, self)
    }
}

pub trait FormatSyntax {
    type Options;

    fn fmt_syn(&self) -> String;
    fn log_syn(&self, log_level: log::Level, options: FormatSyntaxOptions) {
        self.log_syn_indent(log_level, options, 0)
    }
    fn log_syn_indent(&self, log_level: log::Level, options: FormatSyntaxOptions, indent_level: usize);
}

impl FormatSyntax for rnix::SyntaxElement {
    type Options = FormatSyntaxOptions;

    fn fmt_syn(&self) -> String {
        let element_text = format!("{}", self);
        let multiline = element_text.contains("\n");
        if multiline {
            format!("{:?} `\n{}\n`", self.kind(), element_text)
        } else {
            format!("{:?} `{}`", self.kind(), element_text)
        }
    }

    fn log_syn_indent(&self, log_level: log::Level, options: FormatSyntaxOptions, indent_level: usize) {
        match self {
            rnix::SyntaxElement::Node(node) => node.log_syn_indent(log_level, options, indent_level),
            rnix::SyntaxElement::Token(token) => token.log_syn_indent(log_level, options, indent_level),
        };
        //let indent: String = vec!["  "; indent_level].into_iter().collect();
        //let element_text = self.to_string();
        //let is_multiline = element_text.contains("\n");
        //
        //let kind_fmt = format!("{:?}", self.kind());
        //
        //let element_fmt = {
        //    if is_multiline {
        //        if options.short {
        //            let first_line = element_text.lines().next().unwrap();
        //            format!("{first_line} ⟨…⟩")
        //        } else {
        //            format!("`\n{}\n`", element_text)
        //        }
        //    } else {
        //        format!("{element_text}")
        //    }
        //};
        //
        //log::log!(log_level, "{}{} {}", indent, kind_fmt, element_fmt);
        //
        //if options.recursive {
        //    // Only nodes can have children.
        //    if let Some(node) = self.as_node() {
        //        let children = node.children_with_tokens();
        //        for child in children {
        //            child.log_syn_indent(log_level, options, indent_level + 1);
        //        }
        //    }
        //}
    }
}

impl FormatSyntax for rnix::SyntaxNode {
    type Options = FormatSyntaxOptions;

    fn fmt_syn(&self) -> String {
        rnix::SyntaxElement::from(self.clone()).fmt_syn()
    }

    fn log_syn_indent(&self, log_level: log::Level, options: FormatSyntaxOptions, indent_level: usize) {
        let indent: String = vec!["  "; indent_level].into_iter().collect();
        let element_text = self.to_string();
        let is_multiline = element_text.contains("\n");

        let kind_fmt = format!("{:?}", self.kind());

        let element_fmt = {
            if is_multiline {
                if options.short {

                    // If this element is multi-line, but we're logging in "short" mode,
                    // then log the entire first line of this element, then a unicode ␍
                    // to indicate that there's a newline, and then we'll give a short
                    // "preview" of the next line, skipping whitespace.
                    // We'll try not to go over 80 characters for the overall line length,
                    // but we'll always show at minimum 4 characters of the "preview";

                    const MAX_LINE_LEN: i32 = 80;
                    const MIN_PREVIEW_LEN: i32 = 4;

                    let mut lines = element_text.lines();
                    let first_line = lines.next().unwrap();
                    let current_len = (indent_level * 2) + first_line.len() + 1; // +1 for the ␍

                    let second_line_preview: String = lines
                        .next()
                        .unwrap()
                        .chars()
                        .into_iter()
                        .skip_while(|ch| ch.is_whitespace())
                        .take(Ord::max(MIN_PREVIEW_LEN, MAX_LINE_LEN - current_len as i32) as usize)
                        .collect();

                    // FIXME: color properly.
                    // For now, bold the ␍ and the ellipsis.
                    format!("{first_line} \x1b[1m␍\x1b[22m {second_line_preview}\x1b[1m…\x1b[22m")
                } else {
                    format!("`\n{element_text}\n`")
                }
            } else {
                format!("{element_text}")
            }
        };

        log::log!(log_level, "{}{} {}", indent, kind_fmt, element_fmt);

        if options.recursive {
            let children = self.children_with_tokens();
            for child in children {
                child.log_syn_indent(log_level, options, indent_level + 1);
            }
        }
    }
}

impl FormatSyntax for rnix::SyntaxToken {
    type Options = FormatSyntaxOptions;

    fn fmt_syn(&self) -> String {
        rnix::SyntaxElement::from(self.clone()).fmt_syn()
    }

    fn log_syn_indent(&self, log_level: log::Level, _options: FormatSyntaxOptions, indent_level: usize) {
        let indent: String = vec!["  "; indent_level].into_iter().collect();

        log::log!(log_level, "{}{:?}", indent, self.kind());

        // Tokens can't have children, so we're done here.
    }
}
