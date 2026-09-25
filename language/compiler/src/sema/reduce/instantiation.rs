use smallvec::SmallVec;
use tspp_dir as dir;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CheckError, CompilerResult};

impl CheckState<'_> {
    /// Apply explicit arguments and defaults to a constructor or callable type.
    pub(in crate::sema) fn instantiate_type(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(dir::GlobalTypeId, TypeSubstitution)>> {
        // wait for a queried declaration or projected member to become available
        let target = self.normalize(origin, target)?;
        if matches!(self.ty(target)?, dir::Type::Operation(_)) {
            return Ok(None);
        }
        if self.has_error_operand(&[target])? {
            return Ok(Some((target, TypeSubstitution::default())));
        }

        // read generic parameters from the checked value type
        let mut substitution = TypeSubstitution::default();
        let (template, parameters) = match self.ty(target)? {
            dir::Type::Reference(reference) if reference.arguments.is_empty() => {
                let template = self.symbol_template(reference.symbol)?;
                let parameters = match template {
                    Some(template) => self.generic_template_parameters(template)?,
                    None => SmallVec::new(),
                };

                (template, parameters)
            }
            dir::Type::Reference(_) => (None, SmallVec::new()),
            _ => match self.callable_signature_type(origin, target)? {
                Some((declared, signature)) => {
                    let arguments =
                        self.signature_arguments(declared.module_id, signature.arguments)?;
                    substitution = substitution.with_carried(arguments)?;
                    let parameters =
                        self.signature_generic_parameters(declared.module_id, &signature)?;

                    (signature.template, parameters)
                }
                None => (None, SmallVec::new()),
            },
        };
        let substitution = self.bind_explicit_arguments(&parameters, written, substitution)?;

        // diagnose arguments that the selected type cannot accept
        let Some(substitution) = substitution else {
            let expected = self.writable_parameter_count(&parameters)?;
            let name = self.format_type(target);
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            self.report(
                module,
                CheckError::WrongGenericArity {
                    anchor,
                    module,
                    name,
                    expected,
                    supplied: written.len(),
                },
            );
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(Some((error, TypeSubstitution::default())));
        };

        // require the bounds selected by the written arguments
        if let Some(template) = template {
            for constraint in self.substitute_constraint_checks(origin, template, &substitution)? {
                self.push_relation(constraint)?;
            }
        }

        // retain declaration arguments and substitute callable parameters and results
        let ty = match self.ty(target)? {
            dir::Type::Reference(mut reference) if template.is_some() => {
                let arguments = substitution.arguments().collect::<SmallVec<[_; 4]>>();
                reference.arguments = self.intern_type_ids(&arguments)?;

                self.intern_type(dir::Type::Reference(reference))?
            }
            _ => {
                let ty = self.substitute_type(target, &substitution)?;

                // preserve fixed arguments and leave other parameters generic
                if let Some((declared, mut signature)) = self.callable_signature_type(origin, ty)? {
                    signature.arguments = self.intern_generic_arguments(&substitution.bindings)?;
                    let signature = self.intern_signature(signature)?;

                    self.replace_type(ty, declared, signature)?
                } else {
                    ty
                }
            }
        };

        Ok(Some((ty, substitution)))
    }
}
