use destack_dir as dir;
use destack_repository::ProviderError;

use super::Dir;

impl Dir<'_> {
    /// Return the runtime ownership one persisted checked type defaults to.
    ///
    /// The checked type table stores reduced memory forms, so a bare nominal
    /// carries its declaration family's ownership.
    pub fn default_ownership(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::Ownership>, ProviderError> {
        let ownership = match self.get_type(type_id)? {
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Object(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Slice(_)
            | dir::Type::Function(_) => Some(dir::Ownership::Managed),
            dir::Type::Never
            | dir::Type::Void
            | dir::Type::Undefined
            | dir::Type::Null
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Key(_)
            | dir::Type::Range(_)
            | dir::Type::Tuple(_)
            | dir::Type::FixedArray(_)
            | dir::Type::FunctionPointer(_) => Some(dir::Ownership::Owned),
            dir::Type::Primitive(primitive) => Some(primitive_ownership(primitive)),
            // literals share the ownership of their runtime scalar carrier
            dir::Type::Literal(literal) => match literal.widen() {
                dir::Type::Primitive(primitive) => Some(primitive_ownership(primitive)),
                _ => Some(dir::Ownership::Owned),
            },
            dir::Type::Application(instance) => {
                return self.nominal_ownership(instance.symbol);
            }
            dir::Type::Variant(variant) => {
                return self.default_ownership(variant.owner);
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly | dir::Form::Placed { .. } => {
                    return self.default_ownership(form.value);
                }
                // take an explicit form as the value's own ownership
                dir::Form::Managed | dir::Form::Owned | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    form.form.ownership()
                }
            },
            dir::Type::Reference(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Variable(_)
            | dir::Type::Hole(_)
            | dir::Type::Rigid(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Intersection(_)
            | dir::Type::Union(_)
            // refined bases live in the owning module's refinement table
            | dir::Type::Refined(_)
            | dir::Type::Error => None,
        };

        Ok(ownership)
    }

    /// Return the ownership one nominal declaration's family defaults to.
    fn nominal_ownership(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::Ownership>, ProviderError> {
        let definition = self.read_declaration_tables(symbol.module_id, |_, definitions| {
            Ok(definitions.definition(symbol).cloned())
        })?;

        match definition {
            Some(dir::Definition::Class(_) | dir::Definition::Interface(_)) => {
                Ok(Some(dir::Ownership::Managed))
            }
            Some(dir::Definition::Struct(_) | dir::Definition::Enum(_)) => {
                Ok(Some(dir::Ownership::Owned))
            }
            // newtypes share their backing representation's ownership
            Some(dir::Definition::Newtype(definition)) => {
                self.default_ownership(definition.backing)
            }
            _ => Ok(None),
        }
    }
}

/// Return the ownership one primitive scalar defaults to.
fn primitive_ownership(primitive: dir::PrimitiveType) -> dir::Ownership {
    if primitive.representation_item().is_some() {
        dir::Ownership::Managed
    } else {
        dir::Ownership::Owned
    }
}
