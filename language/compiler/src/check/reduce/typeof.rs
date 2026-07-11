use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, OperationReduction, Origin, answer};

impl CheckState<'_> {
    /// Reduce one `typeof` type query from its stable value-reference path.
    pub(super) fn reduce_typeof(
        &mut self,
        origin: Origin,
        value: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
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
                if let Some(ty) = answer!(self.reduce_typeof_reference(value)?) {
                    return Ok(Answer::Ready(Some(ty)));
                }

                let owner = left.into_global_any(node.module_id);
                let Some(owner) = answer!(self.reduce_typeof(origin, owner)?) else {
                    return Ok(Answer::Ready(None));
                };

                let key = self.intern_type(
                    origin.module(),
                    dir::Type::Literal(dir::ScalarLiteral::String(name)),
                )?;
                let projection = answer!(self.reduce_static_member_projection(
                    origin,
                    owner,
                    dir::StaticKey::Name(name),
                    key,
                )?);

                // query paths stay symbolic without a unique projection
                match projection {
                    OperationReduction::Projected(ty) => Ok(Answer::Ready(Some(ty))),
                    OperationReduction::Rigid | OperationReduction::Invalid(_) => {
                        Ok(Answer::Ready(None))
                    }
                }
            }

            // rejected query operands stay symbolic after diagnostics
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Reduce a type query path that resolver bound directly.
    fn reduce_typeof_reference(
        &mut self,
        value: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
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
                let [symbol] = symbols.as_slice() else {
                    return Ok(Answer::Ready(None));
                };
                if let Some(value) = self.static_value(*symbol) {
                    return Ok(Answer::Ready(Some(value)));
                }

                match self.symbol_type(*symbol)? {
                    Answer::Ready(ty) => Ok(Answer::Ready(Some(ty))),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            }

            // projected paths reduce through their expression shape
            Some(dir::Reference::Projected { .. }) | None => Ok(Answer::Ready(None)),

            // invalid paths were already reported by walk
            Some(
                dir::Reference::Ambiguous(_)
                | dir::Reference::Missing
                | dir::Reference::Namespace(_),
            ) => Ok(Answer::Ready(None)),
        }
    }
}
