use crate::resolve::binding::cache::ResolveScopeIndexCache;
use crate::{Compiler, ResolveResult};
use destack_artifact::ExportedSymbolTable;
use destack_dir::{
    FloatType, GlobalSymbolId, IntType, LocalNodeId, LocalScopeId, LocalSymbolId, PrimitiveType,
    SymbolTable, TypeExpression, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Builtin type name resolution.
impl Compiler {
    /// Whether the token string encodes a type literal with an explicit width.
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Resolve an identifier string to a builtin type literal.
    pub(crate) fn resolve_string_to_type(&self, string: &str) -> Option<TypeLiteral> {
        match string {
            // undefined
            "undefined" => Some(TypeLiteral::Undefined),
            // unknown
            "unknown" => Some(TypeLiteral::Unknown),
            // object
            "object" => Some(TypeLiteral::Object),
            // void
            "void" => Some(TypeLiteral::Void),
            // null
            "null" => Some(TypeLiteral::Null),
            // any
            "any" => Some(TypeLiteral::Any),
            // never
            "never" => Some(TypeLiteral::Never),
            // boolean
            "boolean" => Some(TypeLiteral::Primitive(PrimitiveType::Boolean)),
            // character
            "character" => Some(TypeLiteral::Primitive(PrimitiveType::Character)),
            // string
            "string" => Some(TypeLiteral::Primitive(PrimitiveType::String)),
            // bigint
            "bigint" => Some(TypeLiteral::Primitive(PrimitiveType::Bigint)),
            // number
            "number" => Some(TypeLiteral::Primitive(PrimitiveType::Number)),
            // int (followed by number or nothing)
            "int" => Some(TypeLiteral::Primitive(PrimitiveType::Int(
                IntType::Arbitrary {
                    width: self.options.default_int_width,
                    is_signed: true,
                }
                .simplify(),
            ))),
            "isize" => Some(TypeLiteral::Primitive(PrimitiveType::Int(IntType::Isize))),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: true,
                    }
                    .simplify(),
                )))
            }
            // uint (followed by number or nothing)
            "uint" => Some(TypeLiteral::Primitive(PrimitiveType::Int(
                IntType::Arbitrary {
                    width: self.options.default_int_width,
                    is_signed: false,
                }
                .simplify(),
            ))),
            "usize" => Some(TypeLiteral::Primitive(PrimitiveType::Int(IntType::Usize))),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: false,
                    }
                    .simplify(),
                )))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: false,
                    }
                    .simplify(),
                )))
            }
            // float (followed by number or nothing)
            "float" => Some(TypeLiteral::Primitive(PrimitiveType::Float(
                FloatType::Arbitrary {
                    width: self.options.default_float_width,
                }
                .simplify(),
            ))),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Float(
                    FloatType::Arbitrary { width }.simplify(),
                )))
            }
            // composite type
            _ => None,
        }
    }

    /// Build one resolved type reference for the target symbol.
    fn resolve_symbol_to_type_expression(
        &self,
        module: &Module,
        target_symbol: GlobalSymbolId,
        path: &destack_dir::Path,
        generic_arguments: Vec<LocalNodeId<destack_dir::GenericArgument>>,
        symbols: &SymbolTable,
    ) -> TypeExpression {
        // keep remote targets global
        if target_symbol.module_id != module.id {
            return TypeExpression::GlobalReference {
                path: path.clone(),
                generic_arguments,
                target_symbol,
            };
        }

        // choose local vs module reference from the symbol scope
        let symbol = symbols.get_symbol(target_symbol.local_id);
        let scope = symbols.get_scope_by_id(symbol.scope.0);
        if scope.kind == destack_dir::ScopeKind::Block {
            TypeExpression::LocalReference {
                path: path.clone(),
                generic_arguments,
                target_symbol,
            }
        } else {
            TypeExpression::ModuleReference {
                path: path.clone(),
                generic_arguments,
                target_symbol,
            }
        }
    }

    /// Resolve one type reference expression in place.
    pub(crate) fn resolve_type_reference_expression(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        tree: &mut destack_dir::Tree,
        symbols: &mut SymbolTable,
        _types: &mut TypeTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        exported_symbols: &mut ExportedSymbolTable,
        expression_id: LocalNodeId<TypeExpression>,
        _scope_cache: &mut ResolveScopeIndexCache,
    ) -> ResolveResult<()> {
        let expression = tree.get(expression_id).clone();

        // only type references need a resolve rewrite here
        let TypeExpression::Reference {
            path,
            generic_arguments,
            space_order,
        } = expression
        else {
            return Ok(());
        };

        // resolve the bound path to one final symbol
        let resolved_target_symbol = self.resolve_path_target_symbol(
            revision,
            module,
            profile,
            tree,
            symbols,
            namespace_symbol,
            namespace_scope,
            global_augmentation_scope,
            exported_symbols,
            expression_id,
            &path,
            space_order,
        )?;
        let Some(resolved_target_symbol) = resolved_target_symbol else {
            return Ok(());
        };

        // rewrite the node into one resolved reference form
        let resolved_expression = self.resolve_symbol_to_type_expression(
            module,
            resolved_target_symbol,
            &path,
            generic_arguments,
            symbols,
        );
        *tree.get_mut(expression_id) = resolved_expression;

        Ok(())
    }
}
