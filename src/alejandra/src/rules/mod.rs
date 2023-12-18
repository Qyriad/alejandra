use std::collections::LinkedList;

use crate::builder::BuildCtx;
use crate::builder::Step as BuildStep;

pub(crate) mod apply;
pub(crate) mod attr_set;
pub(crate) mod bin_op;
pub(crate) mod dynamic;
pub(crate) mod if_else;
pub(crate) mod inherit;
pub(crate) mod key_value;
pub(crate) mod lambda;
pub(crate) mod let_in;
pub(crate) mod list;
pub(crate) mod paren;
pub(crate) mod pat_bind;
pub(crate) mod pat_entry;
pub(crate) mod pattern;
pub(crate) mod root;
pub(crate) mod scoped;
pub(crate) mod select;
pub(crate) mod string;

pub(crate) fn default(_build_ctx: &BuildCtx, node: &rnix::SyntaxNode) -> LinkedList<BuildStep> {
    node.children_with_tokens().map(BuildStep::Format).collect()
}
