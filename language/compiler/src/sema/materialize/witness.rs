use smallvec::SmallVec;
use std::mem::take;

use tspp_core::FxIndexMap;
use tspp_dir as dir;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

use super::conformance::ConformanceSource;
use super::instance::InstanceWorklist;

/// One member an interface requires of its implementers.
#[derive(Clone)]
pub(super) enum Requirement {
    /// One method requirement, with the key and space implementers declare it under.
    Function(dir::GlobalSymbolId, dir::StaticKey, dir::MemberSpace),
    /// One associated type.
    Type(dir::StaticKey),
    /// One associated const, as the interface declares it.
    Const(dir::AssociatedConstDefinition),
}

impl CheckState<'_> {
    /// Return the application one instance closes its nominal template at.
    pub(in crate::sema) fn instance_application(
        &mut self,
        key: &dir::InstanceKey,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // apply the declaration at the written arguments, the key binding implicit parameters
        let mut arguments = Vec::with_capacity(key.arguments.len());
        for binding in &key.arguments {
            let is_written = self
                .generic_parameter(binding.parameter)?
                .is_some_and(|parameter| {
                    parameter.is_writable() && parameter.induced_memory_parameter().is_none()
                });
            if is_written {
                arguments.push(binding.argument);
            }
        }
        let arguments = self.intern_type_ids(&arguments)?;

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: key.symbol,
            arguments,
        }))
    }

    /// Queue the declared conformances of one declaration.
    pub(super) fn queue_declared_conformances(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: &dir::Definition,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // read the conforming type
        let ty = match definition {
            dir::Definition::Extension(dir::ExtensionDefinition {
                target: dir::ExtensionTarget::Rooted { ty, .. },
                ..
            }) => *ty,
            dir::Definition::Struct(_)
            | dir::Definition::Class(_)
            | dir::Definition::Enum(_)
            | dir::Definition::Newtype(_) => {
                let application = self.declaration_instance(symbol)?;
                self.intern_type(dir::Type::Application(application))?
            }
            _ => return Ok(()),
        };

        // skip templates
        if let Some(template) = definition.template()
            && !self
                .generic_template_parameters(template.into_global(self.module_id))?
                .is_empty()
        {
            return Ok(());
        }

        // queue each implemented interface
        let conformances = match definition {
            dir::Definition::Class(_) => self.class_conformances(ty)?,
            _ => definition
                .implementations()
                .map(|conformance| (conformance.interface, conformance.source))
                .collect(),
        };
        for (interface, source) in conformances {
            worklist.conformances.push((ty, interface, source));
        }

        // queue Drop for a nominal with a drop hook
        if !matches!(definition, dir::Definition::Extension(_))
            && self.drop_hook_member(symbol)?.is_some()
        {
            let source = self.committed_definition_source(symbol)?;
            let drop = self.language_type(dir::LanguageItem::Drop, &[])?;
            worklist.conformances.push((ty, drop, source));
        }

        Ok(())
    }

    /// Return every interface one class and its bases implement.
    pub(super) fn class_conformances(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(dir::GlobalTypeId, dir::GlobalNodeIdAny)>> {
        let mut conformances = Vec::new();
        let mut class = Some(ty);
        while let Some(current) = class.take() {
            let (module, instance) = self.nominal_application(current)?;
            let definition = self.definition(instance.symbol)?;
            let Some(declared @ dir::Definition::Class(definition)) = definition.as_deref() else {
                return Err(CompilerError::Internal {
                    message: "a class conformance chain through a non-class".to_string(),
                });
            };
            let extends = definition.extends.clone();
            let implements = declared.implementations().cloned().collect::<Vec<_>>();

            // read the interfaces and the base under this class's arguments
            let substitution = self.instance_substitution(module, &instance)?;
            for conformance in implements {
                let interface = self.substitute_type(conformance.interface, &substitution)?;
                conformances.push((interface, conformance.source));
            }
            class = match extends {
                Some(heritage) => Some(self.substitute_type(heritage.ty, &substitution)?),
                None => None,
            };
        }

        Ok(conformances)
    }

    /// Record the witnesses of the queued conformances.
    pub(in crate::sema) fn record_declared_witnesses(
        &mut self,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        for (ty, interface, source) in take(&mut worklist.conformances) {
            self.write_witness(ty, interface, source, worklist)?;
        }

        Ok(())
    }

    /// Record the witness of each non-object value one coercion erases.
    pub(super) fn write_erasure_witnesses(
        &mut self,
        coercion: &dir::Coercion,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        for (value, erased) in coercion.erasures() {
            let Some(constraint) = self.erased_constraint(erased)? else {
                continue;
            };
            let shape = self.shallow_strip_forms(value)?;
            let is_object = matches!(self.ty(shape)?, dir::Type::Object(_));
            let is_interface = matches!(self.ty(constraint)?, dir::Type::Application(_));
            if is_interface && !is_object {
                self.reject_consuming_erasure(value, constraint, source)?;
                self.write_witness(value, constraint, source, worklist)?;
            }
        }

        Ok(())
    }

    /// Reject boxing a non-copy value whose member consumes its receiver.
    fn reject_consuming_erasure(
        &mut self,
        value: dir::GlobalTypeId,
        constraint: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // skip handles, callables and copies
        let origin = self.anchored_origin(source)?;
        let is_boxed = !self.is_class_type(value)?
            && self.invoked_key(value)?.is_none()
            && !matches!(self.ty(value)?, dir::Type::Form(_));
        if !is_boxed || self.decide_copy(origin, value, &mut SmallVec::new())? == Verdict::Holds {
            return Ok(());
        }

        // report each requirement taking a boxed receiver by value
        for (_, requirement) in self.witness_members(constraint, value)? {
            let Requirement::Function(member, _, dir::MemberSpace::Instance) = requirement else {
                continue;
            };
            let signature = self.symbol_type(member)?;
            let this = self
                .signature_head(signature)?
                .and_then(|signature| signature.this_parameter);
            let Some(this) = this else {
                continue;
            };
            let is_plain = matches!(self.ty(this)?, dir::Type::This);
            let is_owned = self
                .receiver_form(this)?
                .is_some_and(|form| form.ownership == dir::Ownership::Owned);
            if is_plain || is_owned {
                self.report_consumed_receiver(origin, value, constraint, member)?;
            }
        }

        Ok(())
    }

    /// Record the witness one closed type answers one interface application with, once.
    pub(super) fn write_witness(
        &mut self,
        ty: dir::GlobalTypeId,
        interface: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        // resolve witness identities into this module's type table
        let origin = self.anchored_origin(source)?;
        let ty = self.deeply_resolve(origin, ty)?;
        let interface = self.deeply_resolve(origin, interface)?;

        // read the interface at its full parameter list
        let interface = match self.ty(interface)? {
            dir::Type::Application(application) => self
                .fill_elided_application(interface.module_id, &application)?
                .unwrap_or(interface),
            _ => interface,
        };

        // open pairs close under their instances, recorded pairs stay
        let flags = self.type_flags(ty)? | self.type_flags(interface)?;
        if flags.has_parameter() || flags.has_this() || flags.has_variable() || flags.has_infer() {
            return Ok(());
        }
        if self.module.generics_tail.witness(ty, interface).is_some() {
            return Ok(());
        }

        // answer an erased value's own interface through its table
        let dir::Type::Application(application) = self.ty(interface)? else {
            return Err(CompilerError::Internal {
                message: "a witness interface outside an application".to_string(),
            });
        };
        if self.dispatches_dynamically(origin, ty, application.symbol)? {
            if self
                .erased_constraint(ty)?
                .and_then(|constraint| self.ty(constraint).ok()?.symbol())
                == Some(application.symbol)
            {
                self.write_dynamic_witness(origin, ty, interface)?;
            }
            return Ok(());
        }

        // decide the marker's conformance from its set
        let conformance = self.resolve_conformance(origin, ty, interface)?;
        if self.is_marker_interface(application.symbol)? {
            let closure = self.heritage_closure(origin, interface)?;
            for base in closure.applications {
                let interface = self.deeply_resolve(origin, base.ty)?;
                let Some(symbol) = self.ty(interface)?.symbol() else {
                    continue;
                };
                if self.is_marker_interface(symbol)?
                    || self.module.generics_tail.witness(ty, interface).is_some()
                {
                    continue;
                }
                self.write_conformance_witness(ty, interface, &conformance, source, worklist)?;
            }

            return Ok(());
        }

        self.write_conformance_witness(ty, interface, &conformance, source, worklist)
    }

    /// Record the witness a dynamic value answers its own interface with.
    fn write_dynamic_witness(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let mut functions = Vec::new();
        for (declaring, requirement) in self.witness_members(interface, ty)? {
            let Requirement::Function(requirement, _, dir::MemberSpace::Instance) = requirement
            else {
                continue;
            };
            // dispatch through the table, a generic member through its default body
            let is_generic = self.is_generic_member(requirement)?;
            let source = match (is_generic, self.requirement_has_default(requirement)?) {
                (false, _) => dir::WitnessSource::Dynamic,
                (true, true) => dir::WitnessSource::Default,
                // report a generic member without a default
                (true, false) => {
                    self.report_not_dynamic_member(origin, requirement)?;
                    continue;
                }
            };
            let (module, owner) = self.nominal_application(declaring)?;
            let bindings = self.instance_substitution(module, &owner)?.bindings;
            functions.push(dir::WitnessFunction {
                member: requirement,
                function: dir::InstanceKey::new(requirement, bindings.to_vec()),
                source,
                dispatch: dir::FunctionDispatch::Direct,
            });
        }
        self.module.generics_tail.bind_witness(
            ty,
            interface,
            dir::Witness {
                functions,
                types: Vec::new(),
                constants: Vec::new(),
            },
        );

        Ok(())
    }

    /// Record the witness one closed type answers one member interface with under one conformance.
    fn write_conformance_witness(
        &mut self,
        ty: dir::GlobalTypeId,
        interface: dir::GlobalTypeId,
        conformance: &ConformanceSource,
        source: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let origin = self.anchored_origin(source)?;
        let dir::Type::Application(application) = self.ty(interface)? else {
            return Err(CompilerError::Internal {
                message: "a witness interface outside an application".to_string(),
            });
        };

        // reserve the pair before following recursive types and resolving its members
        self.module.generics_tail.bind_witness(
            ty,
            interface,
            dir::Witness {
                functions: Vec::new(),
                types: Vec::new(),
                constants: Vec::new(),
            },
        );

        // admit the applications the answering type names
        self.walk_type_graph(ty, source, worklist)?;

        // resolve each member at its declaring interface's arguments
        let mut functions = Vec::new();
        let mut types = Vec::new();
        let mut constants = Vec::new();
        for (declaring, requirement) in self.witness_members(interface, ty)? {
            let (module, owner) = self.nominal_application(declaring)?;
            let owner_bindings = self.instance_substitution(module, &owner)?.bindings;
            match requirement {
                Requirement::Function(requirement, key, space) => {
                    let answer = self.resolve_requirement(
                        requirement,
                        key,
                        space,
                        ty,
                        conformance,
                        &owner_bindings,
                        source,
                        worklist,
                    )?;
                    if let Some((implementer, witness_source)) = answer {
                        let dispatch = self.slot_dispatch(implementer.symbol, ty)?;
                        functions.push(dir::WitnessFunction {
                            member: requirement,
                            function: implementer,
                            source: witness_source,
                            dispatch,
                        });
                    }
                }
                Requirement::Type(key) => {
                    let answer = self.resolve_associated_type(
                        owner.symbol,
                        key,
                        ty,
                        conformance,
                        &owner_bindings,
                    )?;
                    self.walk_type_graph(answer, source, worklist)?;
                    types.push(dir::WitnessType {
                        member: key,
                        ty: answer,
                    });
                }
                Requirement::Const(constant) => {
                    let value = self.resolve_associated_const(&constant, ty, conformance)?;
                    constants.push(dir::WitnessConst {
                        member: constant.key,
                        value,
                    });
                }
            }
        }

        // bind the resolved members onto the reserved pair
        let witness = dir::Witness {
            functions,
            types,
            constants,
        };
        self.module
            .generics_tail
            .bind_witness(ty, interface, witness.clone());

        // answer each interface the interface extends by the members it inherits
        let closure = self.heritage_closure(origin, interface)?;
        for base in closure.applications {
            let interface = self.deeply_resolve(origin, base.ty)?;
            let Some(symbol) = self.ty(interface)?.symbol() else {
                continue;
            };
            if symbol == application.symbol
                || self.is_marker_interface(symbol)?
                || self.module.generics_tail.witness(ty, interface).is_some()
            {
                continue;
            }
            let members = self.witness_members(interface, ty)?;
            let inherited = Self::witness_restricted_to(&witness, &members);
            self.module
                .generics_tail
                .bind_witness(ty, interface, inherited);
        }

        Ok(())
    }

    /// Return the part of one witness answering the members one interface declares.
    fn witness_restricted_to(
        witness: &dir::Witness,
        members: &[(dir::GlobalTypeId, Requirement)],
    ) -> dir::Witness {
        let functions = witness
            .functions
            .iter()
            .filter(|function| {
                members.iter().any(|(_, member)| {
                    matches!(member, Requirement::Function(symbol, ..) if *symbol == function.member)
                })
            })
            .cloned()
            .collect();
        let types = witness
            .types
            .iter()
            .filter(|witness_type| {
                members
                    .iter()
                    .any(|(_, member)| matches!(member, Requirement::Type(key) if *key == witness_type.member))
            })
            .copied()
            .collect();
        let constants = witness
            .constants
            .iter()
            .filter(|constant| {
                members.iter().any(|(_, member)| {
                    matches!(member, Requirement::Const(declared) if declared.key == constant.member)
                })
            })
            .copied()
            .collect();

        dir::Witness {
            functions,
            types,
            constants,
        }
    }

    /// Return the members one interface witness answers, inherited first, in declaration order.
    fn witness_members(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<(dir::GlobalTypeId, Requirement)>> {
        let mut requirements = FxIndexMap::default();
        self.collect_witness_members(interface, receiver, &mut requirements)?;

        Ok(requirements.into_values().collect())
    }

    /// Collect one interface's witness members keyed by member key, bases ahead of own members.
    fn collect_witness_members(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        requirements: &mut FxIndexMap<dir::StaticKey, (dir::GlobalTypeId, Requirement)>,
    ) -> CompilerResult<()> {
        let (_, application) = self.nominal_application(interface)?;
        let declared = self.definition(application.symbol)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a witness over the non-interface '{}'",
                    self.format_type(interface)
                ),
            });
        };

        // inherit the base requirements first
        for heritage in &definition.extends {
            let base = self.instantiate_interface_type(heritage.ty, interface, receiver)?;
            self.collect_witness_members(base, receiver, requirements)?;
        }

        // add the own methods and associated types by key
        for member in &definition.members {
            let Some(key) = member.key() else {
                continue;
            };
            let requirement = match member {
                dir::DefinitionMember::Method(method) => {
                    Requirement::Function(method.symbol, key, method.space)
                }
                dir::DefinitionMember::AssociatedType(associated) => {
                    Requirement::Type(associated.key)
                }
                dir::DefinitionMember::AssociatedConst(constant) => {
                    Requirement::Const(constant.clone())
                }
                _ => continue,
            };
            requirements.insert(key, (interface, requirement));
        }

        Ok(())
    }
}
