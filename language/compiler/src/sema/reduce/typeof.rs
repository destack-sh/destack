use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, OperationReduction, Origin};

impl CheckState<'_> {
    /// Reduce one `typeof` type query from its stable value-reference path.
    pub(super) fn reduce_typeof(
        &mut self,
        origin: Origin,
        value: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let node = value.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        match expression {
            // reduce direct value references through resolver output
            dir::Expression::Identifier { .. } => self.reduce_typeof_reference(value),

            // reduce dotted paths through static member projection
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                if let Some(ty) = self.reduce_typeof_reference(value)? {
                    return Ok(Some(ty));
                }

                let owner = left.into_global_any(node.module_id);
                let Some(owner) = self.reduce_typeof(origin, owner)? else {
                    return Ok(None);
                };

                let key = self.intern_type(dir::Type::Literal(dir::Literal::String(name)))?;
                let projection = self.reduce_static_member_projection(
                    origin,
                    owner,
                    dir::StaticKey::Name(name),
                    key,
                )?;

                // query paths stay symbolic without a unique projection
                match projection {
                    OperationReduction::Projected(ty) => Ok(Some(ty)),
                    OperationReduction::Rigid | OperationReduction::Invalid(_) => Ok(None),
                }
            }

            // rejected query operands stay symbolic after diagnostics
            _ => Ok(None),
        }
    }

    /// Reduce a type query path that resolver bound directly.
    fn reduce_typeof_reference(
        &mut self,
        value: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let reference = self
            .module(value.module_id)
            .resolved
            .references
            .get(value)
            .cloned();

        match reference {
            // use a single value declaration's static value or checked type
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.present_symbols(&symbols);

                // intersect an overload group's signatures into one type
                if symbols.len() > 1 {
                    let mut elements = Vec::new();
                    for symbol in &symbols {
                        elements.push(self.symbol_type(*symbol)?);
                    }
                    let elements = self.intern_type_ids(&elements)?;

                    return Ok(Some(self.intern_type(dir::Type::Intersection(
                        dir::IntersectionType { elements },
                    ))?));
                }
                let [symbol] = symbols.as_slice() else {
                    return Ok(None);
                };

                // lift a class used as a value to its declared static side
                let is_class = matches!(self.definition(*symbol)?, Some(dir::Definition::Class(_)));
                if is_class && let Some(id) = self.symbol_static_id(*symbol) {
                    return Ok(Some(self.intern_type(dir::Type::Static(id))?));
                }

                if let Some(value) = self.static_value(*symbol) {
                    return Ok(Some(value));
                }

                let ty = self.symbol_type(*symbol)?;

                Ok(Some(ty))
            }

            // projected paths reduce through their expression shape
            Some(dir::Reference::Projected { .. }) | None => Ok(None),

            // invalid paths were already reported by walk
            Some(
                dir::Reference::Ambiguous(_)
                | dir::Reference::Missing
                | dir::Reference::Namespace { .. },
            ) => Ok(None),
        }
    }
}
