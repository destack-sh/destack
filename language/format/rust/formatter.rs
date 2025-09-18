use dyst_language_ast::{NodeTree, PathPool, StringPool};

#[derive(Debug)]
pub struct Formatter<'a> {
    ast: &'a NodeTree,
    strings: &'a StringPool,
    paths: &'a PathPool,
}
