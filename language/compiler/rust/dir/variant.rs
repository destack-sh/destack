use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Field, NodeId, StringId, Type, Variant};
use dyst_source::FileId;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Lower an AST variant field to a DIR variant field.
    #[inline]
    pub(super) fn lower_field(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        field_id: ast::NodeId<ast::Field>,
    ) -> NodeId<Field> {
        let field = match ast.get(field_id) {
            ast::Field::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = self.intern_string(file_id, name.string());
                let ty = self.lower_expression_to_type(file_id, ast, *ty);
                let default = default.map(|default| self.lower_expression(file_id, ast, default));
                Field::Named {
                    modifiers,
                    name,
                    ty,
                    default,
                }
            }
            ast::Field::Positional {
                modifiers,
                ty,
                default,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let ty = self.lower_expression_to_type(file_id, ast, *ty);
                let default = default.map(|default| self.lower_expression(file_id, ast, default));
                Field::Positional {
                    modifiers,
                    ty,
                    default,
                }
            }
            ast::Field::Dynamic {
                modifiers,
                name,
                ty,
                key,
                default,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = name.map(|name| self.intern_string(file_id, name));
                let ty = self.lower_expression_to_type(file_id, ast, *ty);
                let key = self.lower_expression_to_type(file_id, ast, *key);
                let default = default.map(|default| self.lower_expression(file_id, ast, default));
                Field::Dynamic {
                    modifiers,
                    name,
                    ty,
                    key,
                    default,
                }
            }
        };

        self.tree.insert_from_ast(field, file_id, field_id)
    }

    /// Lower an AST struct to a DIR variant.
    pub fn lower_struct_to_variant(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        definition_id: ast::NodeId<ast::Definition>,
        name: Option<StringId>,
        format: ast::VariantFormat,
        ty: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::Field>],
    ) -> NodeId<Variant> {
        let fields = fields
            .iter()
            .map(|field| self.lower_field(file_id, ast, *field))
            .collect();
        let variant = match format {
            ast::VariantFormat::Tuple => Variant::Tuple {
                name,
                ty,
                fields,
                value: None,
            },
            ast::VariantFormat::Struct => Variant::Struct {
                name,
                ty,
                fields,
                value: None,
            },
        };
        self.tree.insert_from_ast(variant, file_id, definition_id)
    }

    /// Lower an AST enum to a DIR variant.
    pub fn lower_enum_to_variant(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        _definition_id: ast::NodeId<ast::Definition>,
        tag_type: Option<NodeId<Type>>,
        fields: &[ast::NodeId<ast::EnumField>],
    ) -> Vec<NodeId<Variant>> {
        fields
            .iter()
            .map(|field| self.lower_enum_field_to_variant(file_id, ast, tag_type, *field))
            .collect()
    }

    /// Lower an AST enum field to a DIR variant.
    #[inline]
    pub(super) fn lower_enum_field_to_variant(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        representation_type: Option<NodeId<Type>>,
        field_id: ast::NodeId<ast::EnumField>,
    ) -> NodeId<Variant> {
        let field = ast.get(field_id);
        let name = self.intern_string(file_id, field.name.string());
        let value = field
            .value
            .map(|value| self.lower_expression(file_id, ast, value));
        self.tree.insert_from_ast(
            Variant::Unit {
                name: Some(name),
                ty: representation_type,
                value,
            },
            file_id,
            field_id,
        )
    }

    /// Lower an AST union to DIR variants.
    pub fn lower_union_to_variant(
        &mut self,
        file_id: FileId,
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
                    file_id,
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
        file_id: FileId,
        ast: &ast::NodeTree,
        // NOTE #Incomplete: consider union field tag type?
        _tag_type: Option<NodeId<Type>>,
        representation_type: Option<NodeId<Type>>,
        field_id: ast::NodeId<ast::UnionField>,
    ) -> NodeId<Variant> {
        let field = ast.get(field_id);
        let variant = match field {
            ast::UnionField::Unit { name, value } => {
                let name = self.intern_string(file_id, *name);
                let value = value.map(|value| self.lower_expression(file_id, ast, value));
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
                let name = self.intern_string(file_id, *name);
                let fields = fields
                    .iter()
                    .map(|field| self.lower_field(file_id, ast, *field))
                    .collect();
                let value = value.map(|value| self.lower_expression(file_id, ast, value));
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
                let name = self.intern_string(file_id, *name);
                let fields = fields
                    .iter()
                    .map(|field| self.lower_field(file_id, ast, *field))
                    .collect();
                let value = value.map(|value| self.lower_expression(file_id, ast, value));
                Variant::Struct {
                    name: Some(name),
                    ty: representation_type,
                    fields,
                    value,
                }
            }
        };
        self.tree.insert_from_ast(variant, file_id, field_id)
    }
}
