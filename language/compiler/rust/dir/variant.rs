use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{NodeId, StringId, Type, Variant, VariantField};
use dyst_source::SourceId;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Lower an AST variant field to a DIR variant field.
    #[inline]
    pub(super) fn lower_variant_field(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        field_id: ast::NodeId<ast::VariantField>,
    ) -> NodeId<VariantField> {
        let field = ast.get(field_id);
        let name = field.name.map(|name| self.intern_string(source_id, name));
        let ty = self.lower_expression_to_type(source_id, ast, field.ty);
        let default = field
            .default
            .map(|default| self.lower_expression(source_id, ast, default));
        let variant_field = {
            if let Some(name) = name {
                VariantField::Named { name, ty, default }
            } else {
                VariantField::Positional { ty, default }
            }
        };
        self.tree.insert(variant_field, source_id, field_id)
    }

    /// Lower an AST struct to a DIR variant.
    pub fn lower_struct_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        definition_id: ast::NodeId<ast::Definition>,
        name: Option<StringId>,
        style: ast::VariantStyle,
        ty: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::VariantField>],
    ) -> NodeId<Variant> {
        let variant_fields = fields
            .iter()
            .map(|field| self.lower_variant_field(source_id, ast, *field))
            .collect();
        let variant = match style {
            ast::VariantStyle::Tuple => Variant::Tuple {
                name,
                ty,
                fields: variant_fields,
                value: None,
            },
            ast::VariantStyle::Struct => Variant::Struct {
                name,
                ty,
                fields: variant_fields,
                value: None,
            },
        };
        self.tree.insert(variant, source_id, definition_id)
    }

    /// Lower an AST enum to a DIR variant.
    pub fn lower_enum_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        _definition_id: ast::NodeId<ast::Definition>,
        tag_type: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::EnumField>],
    ) -> Vec<NodeId<Variant>> {
        fields
            .iter()
            .map(|field| self.lower_enum_field_to_variant(source_id, ast, tag_type, *field))
            .collect()
    }

    /// Lower an AST enum field to a DIR variant.
    #[inline]
    pub(super) fn lower_enum_field_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        representation_type: Option<NodeId<Type>>,
        field_id: ast::NodeId<ast::EnumField>,
    ) -> NodeId<Variant> {
        let field = ast.get(field_id);
        let name = self.intern_string(source_id, field.name);
        let value = field
            .value
            .map(|value| self.lower_expression(source_id, ast, value));
        self.tree.insert(
            Variant::Unit {
                name: Some(name),
                ty: representation_type,
                value,
            },
            source_id,
            field_id,
        )
    }

    /// Lower an AST union to DIR variants.
    pub fn lower_union_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        _definition_id: ast::NodeId<ast::Definition>,
        tag_type: Option<NodeId<Type>>,
        representation_type: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::UnionField>],
    ) -> Vec<NodeId<Variant>> {
        fields
            .iter()
            .map(|field| {
                self.lower_union_field_to_variant(
                    source_id,
                    ast,
                    tag_type,
                    representation_type,
                    *field,
                )
            })
            .collect()
    }

    /// Lower an AST union field to a DIR variant.
    #[inline]
    pub(super) fn lower_union_field_to_variant(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        // NOTE #Incomplete: consider union field tag type?
        _tag_type: Option<NodeId<Type>>,
        representation_type: Option<NodeId<Type>>,
        field_id: ast::NodeId<ast::UnionField>,
    ) -> NodeId<Variant> {
        let field = ast.get(field_id);
        let variant = match field {
            ast::UnionField::Unit { name, value } => {
                let name = self.intern_string(source_id, *name);
                let value = value.map(|value| self.lower_expression(source_id, ast, value));
                Variant::Unit {
                    name: Some(name),
                    ty: representation_type,
                    value,
                }
            }
            ast::UnionField::Tuple {
                name,
                fields,
                value,
            } => {
                let name = self.intern_string(source_id, *name);
                let fields = fields
                    .iter()
                    .map(|field| self.lower_variant_field(source_id, ast, *field))
                    .collect();
                let value = value.map(|value| self.lower_expression(source_id, ast, value));
                Variant::Tuple {
                    name: Some(name),
                    ty: representation_type,
                    fields,
                    value,
                }
            }
            ast::UnionField::Struct {
                name,
                fields,
                value,
            } => {
                let name = self.intern_string(source_id, *name);
                let fields = fields
                    .iter()
                    .map(|field| self.lower_variant_field(source_id, ast, *field))
                    .collect();
                let value = value.map(|value| self.lower_expression(source_id, ast, value));
                Variant::Struct {
                    name: Some(name),
                    ty: representation_type,
                    fields,
                    value,
                }
            }
        };
        self.tree.insert(variant, source_id, field_id)
    }
}
