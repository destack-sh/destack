use destack_source::FileId;

use crate::parse::{ParseOptions, Parser};
use crate::{Function, LocalNodeId, NodeTree};

/// Parse one MIR test module and return its single function.
pub(crate) fn parse_test_function(source: &str) -> (NodeTree, LocalNodeId<Function>) {
    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");

    let function_id = tree
        .iter_nodes::<Function>()
        .next()
        .map(|(function_id, _)| function_id)
        .expect("missing function");

    (tree, function_id)
}
