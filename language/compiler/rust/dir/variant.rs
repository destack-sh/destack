use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{NodeId, Variant};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a variant to a DIR variant.
    pub fn lower_struct_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        variant_types,
    ) -> NodeId<Variant> {
        todo!("Compiler::lower_variant")
    }
}
