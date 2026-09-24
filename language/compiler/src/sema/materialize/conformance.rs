use destack_dir as dir;

use super::instance::InstanceWorklist;
use crate::sema::{CheckState, Origin, TypeSubstitution, Verdict};
use crate::{CompilerError, CompilerResult};

/// The declaration one closed type conforms to one interface through.
pub(super) enum ConformanceSource {
    /// A nominal or extension declaring the conformance, at its template's arguments.
    Declared {
        /// The declaring nominal or extension.
        declaration: dir::GlobalSymbolId,
        /// The declaration's template arguments at the receiver.
        bindings: Vec<dir::GenericArgumentBinding>,
    },
    /// The compiler implements the interface for the type itself.
    Intrinsic,
    /// A structural interface the receiver's own members answer by key.
    Structural {
        /// The receiver with its ownership forms peeled.
        receiver: dir::GlobalTypeId,
    },
}

impl CheckState<'_> {
    /// Resolve the declaration one closed type conforms to one interface through.
    pub(super) fn resolve_conformance(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        interface: dir::GlobalTypeId,
    ) -> CompilerResult<ConformanceSource> {
        let dir::Type::Application(application) = self.ty(interface)? else {
            return Err(CompilerError::Internal {
                message: "a conformance resolved outside an interface application".to_string(),
            });
        };

        // conform ownership forms through their value, extensions matching the written form
        let written = self.evaluate_type(origin, receiver)?;
        let receiver = self.ownership_payload(origin, written)?;

        // the receiver's nominal or one of its bases declares the interface
        if let Some(conformance) =
            self.declared_conformance(origin, receiver, receiver, application.symbol)?
        {
            let (module, implementer) = self.nominal_application(conformance.implementer)?;
            let bindings = self
                .instance_substitution(module, &implementer)?
                .bindings
                .to_vec();

            return Ok(ConformanceSource::Declared {
                declaration: implementer.symbol,
                bindings,
            });
        }

        // decide the extension implementation
        let decision = self.decide_extension_implementation(
            origin,
            interface.module_id,
            written,
            &application,
            None,
        )?;
        if let Some(winner) = decision.winner {
            let mut bindings = Vec::new();
            if let Some(template) = self.symbol_template(winner.extension)? {
                let parameters = self.generic_template_parameters(template)?;
                for (parameter, argument) in parameters.iter().zip(winner.arguments.iter()) {
                    let argument = self.shallow_resolve(*argument)?;
                    bindings.push(dir::GenericArgumentBinding::new(*parameter, argument));
                }
            }

            return Ok(ConformanceSource::Declared {
                declaration: winner.extension,
                bindings,
            });
        }

        // decide the auto interface the compiler implements
        let symbol = self.resolve_symbol_alias(application.symbol)?;
        let auto = self
            .language_item(symbol)?
            .and_then(dir::AutoInterface::from_language_item);
        if let Some(auto) = auto
            && self.decide_intrinsic_interface(origin, receiver, interface, auto)? == Verdict::Holds
        {
            return Ok(ConformanceSource::Intrinsic);
        }

        // decide an interface a builtin-implemented interface extends
        if self.inherits_intrinsic_interface(origin, receiver, symbol)? {
            return Ok(ConformanceSource::Intrinsic);
        }

        // a structural interface is answered by the receiver's own members
        let is_nominal = match self.definition(symbol)?.as_deref() {
            Some(dir::Definition::Interface(definition)) => definition.is_nominal,
            _ => true,
        };
        if !is_nominal {
            return Ok(ConformanceSource::Structural { receiver });
        }

        Err(CompilerError::Internal {
            message: format!(
                "no conformance of '{}' to '{}'",
                self.format_type(receiver),
                self.format_type(interface)
            ),
        })
    }

    /// Resolve the implementer one closed type answers a requirement with, interning its instance.
    pub(super) fn resolve_requirement(
        &mut self,
        requirement: dir::GlobalSymbolId,
        key: dir::StaticKey,
        space: dir::MemberSpace,
        receiver: dir::GlobalTypeId,
        conformance: &ConformanceSource,
        owner_bindings: &[dir::GenericArgumentBinding],
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<(dir::InstanceKey, dir::WitnessSource)>> {
        match conformance {
            // instantiate the member the conforming declaration selected for the requirement
            ConformanceSource::Declared {
                declaration,
                bindings,
            } => {
                let member = self
                    .conformance_member(*declaration, requirement)?
                    .filter(|member| *member != requirement);

                // leave a default body to the instance that calls it
                let Some(member) = member else {
                    return Ok(None);
                };
                let key = dir::InstanceKey::new(member, bindings.clone());
                self.intern_instance(
                    member,
                    None,
                    bindings.clone(),
                    source,
                    dir::InstanceOrigin::Instantiation,
                    worklist,
                )?;

                Ok(Some((key, dir::WitnessSource::Declared)))
            }
            // a structural interface takes the receiver's own member under the requirement's key
            ConformanceSource::Structural { receiver } => {
                let origin = self.anchored_origin(source)?;
                let module = self.module_id;
                let candidates =
                    self.lookup_inherent_member(origin, module, *receiver, space, key)?;
                let declared = candidates
                    .candidates
                    .iter()
                    .find_map(|candidate| candidate.declaration())
                    .map(|declared| (declared.symbol, declared.generic_arguments.clone()));
                let Some((member, bindings)) = declared else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "no member '{}' on '{}' for a structural requirement",
                            self.format_static_key(&key),
                            self.format_type(*receiver)
                        ),
                    });
                };
                let key = dir::InstanceKey::new(member, bindings.clone());
                self.intern_instance(
                    member,
                    None,
                    bindings,
                    source,
                    dir::InstanceOrigin::Instantiation,
                    worklist,
                )?;

                Ok(Some((key, dir::WitnessSource::Declared)))
            }
            // synthesize the member a derivable interface supplies at a composite receiver
            ConformanceSource::Intrinsic => {
                let synthesized =
                    self.build_derived_member(requirement, receiver, owner_bindings, source)?;
                let Some(synthesized) = synthesized else {
                    return Ok(None);
                };
                let key = dir::InstanceKey::new(synthesized.symbol, Vec::new())
                    .with_receiver(Some(receiver));
                let redirected = self.allocate_instance(
                    key.clone(),
                    source,
                    dir::InstanceOrigin::Instantiation,
                    worklist,
                )?;

                // bind the types a synthesized member's instance reads, closed already
                if let Some(redirected) = redirected {
                    let generics = &mut self.module.generics_tail;
                    generics.bind_instance_symbol(
                        redirected,
                        synthesized.symbol,
                        synthesized.callable,
                    );
                    for (parameter, ty) in synthesized.parameters() {
                        generics.bind_instance_symbol(redirected, parameter, ty);
                    }

                    // close each recorded call's instance under the body's own template
                    let Some(body) = synthesized.body else {
                        return Err(CompilerError::Internal {
                            message: "a derived member without its body".to_owned(),
                        });
                    };
                    for call in synthesized.calls {
                        let dir::CallableTarget::Symbol { function, .. } = call.target else {
                            continue;
                        };
                        self.intern_instance(
                            function.key.symbol,
                            function.key.receiver,
                            function.key.arguments,
                            body,
                            dir::InstanceOrigin::Instantiation,
                            worklist,
                        )?;
                    }
                }

                Ok(Some((key, dir::WitnessSource::Derived)))
            }
        }
    }

    /// Resolve the type one closed type answers one associated type with, from the declared value.
    pub(super) fn resolve_associated_type(
        &mut self,
        interface: dir::GlobalSymbolId,
        key: dir::StaticKey,
        receiver: dir::GlobalTypeId,
        conformance: &ConformanceSource,
        owner_bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the value the conforming declaration writes for the associated type
        if let ConformanceSource::Declared {
            declaration,
            bindings,
        } = conformance
            && let Some(value) = self.associated_type_value(*declaration, key)?
        {
            let substitution = TypeSubstitution {
                bindings: bindings.iter().copied().collect(),
                receiver: Some(receiver),
            };
            let value = self.substitute_type(value, &substitution)?;

            return self.shallow_resolve(value);
        }

        // read the default the interface itself writes
        if let Some(value) = self.associated_type_value(interface, key)? {
            let substitution = TypeSubstitution {
                bindings: owner_bindings.iter().copied().collect(),
                receiver: Some(receiver),
            };
            let value = self.substitute_type(value, &substitution)?;

            return self.shallow_resolve(value);
        }

        Err(CompilerError::Internal {
            message: format!(
                "no value for the associated type '{}' of '{}'",
                self.format_static_key(&key),
                self.format_type(receiver)
            ),
        })
    }

    /// Return the value one declaration writes for an associated type key.
    fn associated_type_value(
        &mut self,
        declaration: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let value = self.definition(declaration)?.and_then(|definition| {
            definition.members().iter().find_map(|member| match member {
                dir::DefinitionMember::AssociatedType(associated) if associated.key == key => {
                    associated.value
                }
                _ => None,
            })
        });

        Ok(value)
    }

    /// Return whether one interface is a memberless marker, answered by the satisfied set alone.
    pub(super) fn is_marker_interface(
        &self,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        Ok(self
            .language_item(interface)?
            .and_then(dir::AutoInterface::from_language_item)
            .is_some_and(dir::AutoInterface::is_marker))
    }

    /// Return whether one receiver dispatches one interface through its own erased constraint.
    pub(super) fn dispatches_dynamically(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let receiver = self.ownership_payload(origin, receiver)?;
        let constraint = match self.ty(receiver)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            _ if self.is_conformance_target(receiver)? => receiver,
            _ => return Ok(false),
        };

        Ok(self.ty(constraint)?.symbol() == Some(interface)
            || self
                .declared_conformance(origin, constraint, constraint, interface)?
                .is_some())
    }

    /// Resolve the const member one closed type answers one associated const with.
    pub(super) fn resolve_associated_const(
        &mut self,
        constant: &dir::AssociatedConstDefinition,
        receiver: dir::GlobalTypeId,
        conformance: &ConformanceSource,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        // take the member the conforming declaration selected for the requirement
        if let ConformanceSource::Declared { declaration, .. } = conformance
            && let Some(member) = self.conformance_member(*declaration, constant.symbol)?
        {
            return Ok(member);
        }

        // take the default value the interface itself writes
        if !matches!(constant.implementation, dir::MemberImplementation::Required) {
            return Ok(constant.symbol);
        }

        Err(CompilerError::Internal {
            message: format!(
                "no const '{}' on '{}' for an associated const requirement",
                self.format_static_key(&constant.key),
                self.format_type(receiver)
            ),
        })
    }
}
