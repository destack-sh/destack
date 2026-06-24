use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::check::{Answer, CheckState, Origin, Relation, answer};
use crate::{CheckError, CompilerResult};

impl CheckState<'_> {
    /// Check whether one try expression can propagate through its return type.
    pub(in crate::check) fn check_try_propagates(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        return_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let Some(return_type) = return_type else {
            let (module, anchor) = self.source_anchor(source);
            let diagnostic = CheckError::TryOutsideFunction { anchor, module };

            return Ok(Answer::Ready(Some(diagnostic.into())));
        };

        // reject incompatible failure propagation
        if answer!(self.decide_try_propagation(source, value, return_type)?) {
            return Ok(Answer::Ready(None));
        }

        let source_text = self.format_type(return_type);
        let target_text = format!("FromResidual<{}>", self.format_type(value));
        let (module, anchor) = self.source_anchor(source);

        let error = CheckError::InterfaceNotImplemented {
            anchor,
            module,
            source: source_text,
            target: target_text,
        };

        Ok(Answer::Ready(Some(error.note(
            "the '?' operator propagates failures into the enclosing return type",
        ))))
    }

    /// Decide whether a propagated try failure fits an enclosing return type.
    fn decide_try_propagation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let origin = Origin::Node(source);
        let module = source.module_id;

        // project the propagated failure channel
        let operation = dir::TypeOperation::TryResidual { value };
        let residual = self.push_type(module, dir::Type::Operation(operation), source.local_id)?;
        let residual = answer!(self.evaluate_root(origin, residual)?);

        // require the return type to accept the residual
        let symbol = self.language_symbol(dir::LanguageItem::FromResidual);
        let target = self.push_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: vec![residual],
            }),
            source.local_id,
        )?;

        self.decide_relation(origin, Relation::Implements, return_type, target)
    }
}
