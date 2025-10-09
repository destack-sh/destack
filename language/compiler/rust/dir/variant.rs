use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{NodeId, Type, Variant};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower an AST struct to a DIR variant.
    pub fn lower_struct_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        style: ast::StructStyle,
        representation_type: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::StructField>],
    ) -> NodeId<Variant> {
        todo!("Compiler::lower_struct_to_variant")
    }

    /// Lower an AST enum to a DIR variant.
    pub fn lower_enum_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        tag_type: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::EnumField>],
    ) -> NodeId<Variant> {
        todo!("Compiler::lower_enum_to_variant")
    }

    /// Lower an AST union to DIR variants.
    pub fn lower_union_to_variants(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        tag_type: Option<NodeId<Type>>,
        representation_type: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::UnionField>],
    ) -> Vec<NodeId<Variant>> {
        todo!("Compiler::lower_union_to_variants")
    }
}
