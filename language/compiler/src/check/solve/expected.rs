use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, Solution, TypeRelation, TypeTerm, VariableId, VariableOrigin,
};

use super::Decision;
use super::queue::Progress;

impl CheckComponentState<'_> {
    /// Apply expected type context to one expression variable.
    pub(super) fn expect_type(
        &mut self,
        variable: VariableId,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let TypeTerm::Literal(dir::Type::Literal(_)) = source else {
            return Ok(Progress::Unchanged);
        };
        let TypeTerm::Literal(target_type) = target else {
            return Ok(Progress::Unchanged);
        };
        if !Self::can_expect_literal(target_type) {
            return Ok(Progress::Unchanged);
        }
        if self.decide_type_term_relation(TypeRelation::Assignable, source, target)?
            != Decision::Yes
        {
            return Ok(Progress::Unchanged);
        }
        let check_module = self.module_mut(variable.module)?;
        let check_variable = check_module.variable_mut(variable);
        if !matches!(check_variable.origin, VariableOrigin::Node(_)) {
            return Ok(Progress::Unchanged);
        }

        check_variable.solution = Some(Solution::Type(target.clone()));

        Ok(Progress::changed(variable))
    }

    /// Widen a literal type inferred through assignability.
    pub(super) fn widen_inferred_type(term: TypeTerm) -> TypeTerm {
        let TypeTerm::Literal(dir::Type::Literal(literal)) = term else {
            return term;
        };
        let ty = match literal {
            dir::ScalarLiteral::Integer(_) => Self::int32_type(),
            dir::ScalarLiteral::Float(_) => {
                dir::Type::Primitive(dir::PrimitiveType::Float(dir::FloatType::Float64))
            }
            dir::ScalarLiteral::Bigint(_) => dir::Type::Primitive(dir::PrimitiveType::Bigint),
            dir::ScalarLiteral::String(_) => dir::Type::Primitive(dir::PrimitiveType::String),
            dir::ScalarLiteral::Null => dir::Type::Null,
            dir::ScalarLiteral::Boolean(_) => dir::Type::Primitive(dir::PrimitiveType::Boolean),
            dir::ScalarLiteral::Character(_) => dir::Type::Primitive(dir::PrimitiveType::Character),
            dir::ScalarLiteral::RegexString { .. } => dir::Type::Object,
        };

        TypeTerm::Literal(ty)
    }

    /// Return whether a literal can take one expected DIR type.
    fn can_expect_literal(target: &dir::Type) -> bool {
        matches!(
            target,
            dir::Type::Null | dir::Type::Object | dir::Type::Primitive(_)
        )
    }
}
