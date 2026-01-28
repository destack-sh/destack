use destack_ast::{AnnotationPosition, Doc};

use crate::program::ModuleAst;

/// Collect documentation strings attached to a node.
pub(crate) fn doc_strings_for_node(ast: &ModuleAst, node_id: u32) -> Vec<String> {
    // get doc annotations attached to this AST node
    let docs = ast.tree.get_docs_for(node_id);

    // bail when there are no docs
    if docs.is_empty() {
        return Vec::new();
    }

    // filter to prefix docs and return their text
    docs.into_iter()
        .filter(|(_, pos)| {
            matches!(
                pos,
                AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
            )
        })
        .map(|(doc_id, _)| {
            let doc = ast.tree.get::<Doc>(doc_id);
            ast.strings.get(doc.string).to_string()
        })
        .collect()
}

/// Join documentation strings for display.
pub(crate) fn doc_text_for_node(ast: &ModuleAst, node_id: u32) -> Option<String> {
    // collect the doc strings for this node
    let doc_strings = doc_strings_for_node(ast, node_id);

    // bail when there are no docs
    if doc_strings.is_empty() {
        return None;
    }

    // join doc blocks with a blank line
    Some(doc_strings.join("\n\n"))
}
