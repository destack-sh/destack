use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CheckState, MemberRole, ObligationCheck, ObligationFailure, Origin, Relation, TypeSubstitution,
};

/// One class instance member that participates in heritage checks.
struct ClassMember {
    /// The member symbol.
    symbol: dir::GlobalSymbolId,
    /// The member key.
    key: dir::StaticKey,
    /// The member type rewritten into the subclass's view.
    ty: dir::GlobalTypeId,
    /// The member declaration node.
    source: dir::GlobalNodeIdAny,
    /// How the member participates in override assignability.
    role: MemberRole,
    /// Whether subclasses may override the member.
    is_overridable: bool,
    /// Whether the member declares an override.
    is_override: bool,
    /// Whether the member is abstract.
    is_abstract: bool,
}

/// One class's inherited member view.
struct ClassHeritage {
    /// Inherited members, closest base first.
    members: Vec<ClassMember>,
    /// The direct final base class, when extension is forbidden.
    final_base: Option<dir::GlobalSymbolId>,
}

impl CheckState<'_> {
    /// Return the class member represented by one definition member.
    fn class_member(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<ClassMember>> {
        match member {
            dir::DefinitionMember::Field(field) if field.space == dir::MemberSpace::Instance => {
                let Some(ty) = self.definition_member_type(member)? else {
                    return Ok(None);
                };

                Ok(Some(ClassMember {
                    key: field.key,
                    ty,
                    source: field.source,
                    role: MemberRole::Field,
                    is_overridable: field.is_abstract,
                    is_override: field.is_override,
                    symbol: field.symbol,
                    is_abstract: field.is_abstract,
                }))
            }
            dir::DefinitionMember::Method(method) if method.space == dir::MemberSpace::Instance => {
                let dir::MemberSlot::Key(key) = method.slot else {
                    return Ok(None);
                };
                let Some(ty) = self.definition_member_type(member)? else {
                    return Ok(None);
                };
                let is_abstract = method.abstraction == dir::MethodAbstraction::Abstract;
                let is_virtual = method.abstraction == dir::MethodAbstraction::Virtual;
                let is_overridable = is_abstract || is_virtual;

                Ok(Some(ClassMember {
                    key,
                    ty,
                    source: method.source,
                    role: MemberRole::Method,
                    is_overridable,
                    is_override: method.is_override,
                    symbol: method.symbol,
                    is_abstract,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Check one declaration against its heritage rules.
    pub(in crate::sema) fn check_declaration_heritage(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ObligationCheck> {
        let source = self.origin_source(origin)?;
        let instance = self.declaration_instance(symbol)?;
        let ty = self.intern_type(dir::Type::Application(instance))?;
        let closure = self.instance_heritage_closure(origin, ty, ty)?;

        // report graph errors before class member rules
        let mut failures = Vec::new();
        for conflict in closure.conflicts {
            let (_, current) = self.nominal_application(conflict.current)?;
            failures.push(ObligationFailure::ConflictingHeritage {
                source: conflict.source,
                symbol,
                target: current.symbol,
            });
        }
        for cycle in closure.cycles {
            failures.push(ObligationFailure::CircularHeritage {
                source: cycle.source,
                symbol,
            });
        }
        if !failures.is_empty() {
            return Ok(ObligationCheck::from_failures(failures));
        }

        // require one concrete space across the declaration and its heritage
        let mut space_source = self
            .definition(symbol)?
            .as_deref()
            .and_then(dir::Definition::space)
            .map(|space| (source, symbol, space));
        for application in &closure.applications {
            let (_, instance) = self.nominal_application(application.ty)?;
            let Some(space) = self.nominal_space(instance.symbol)? else {
                continue;
            };
            match space_source {
                None => space_source = Some((application.source, instance.symbol, space)),
                Some((_, _, current)) if current == space => {}
                Some((first_source, first_symbol, _)) => {
                    let failure = ObligationFailure::ConflictingHeritageSpace {
                        source: first_source,
                        symbol: first_symbol,
                        conflict_source: application.source,
                        conflict: instance.symbol,
                    };

                    return Ok(ObligationCheck::fail(failure));
                }
            }
        }

        self.check_class_member_heritage(origin, symbol)
    }

    /// Check one class declaration against its member heritage rules.
    fn check_class_member_heritage(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ObligationCheck> {
        let source = self.origin_source(origin)?;
        let definition = self.definition(symbol)?;
        let Some(dir::Definition::Class(class)) = definition.as_deref() else {
            return Ok(ObligationCheck::holds());
        };
        let is_abstract = class.is_abstract;
        let extends = class.extends.clone();
        let extends_source = extends.as_ref().map(|heritage| heritage.source);

        // collect the declared instance members heritage rules check
        let members = class.members.clone();
        let mut own = Vec::new();
        for member in &members {
            if let Some(member) = self.class_member(member)? {
                own.push(member);
            }
        }

        // collect inherited members walking up the extends chain
        let heritage = self.class_heritage(origin, extends)?;

        // decide every rule before reporting anything, keeping each override as the member
        // implementing its base
        let mut failures = Vec::new();
        let mut overrides = Vec::new();
        for member in &own {
            let base = heritage
                .members
                .iter()
                .find(|inherited| inherited.key == member.key);
            match (member.is_override, base) {
                // overrides need an inherited member to override
                (true, None) => {
                    failures.push(ObligationFailure::InvalidOverride {
                        source: member.source,
                        member: member.key,
                    });
                }
                (true, Some(base)) => {
                    // overrides need virtual or abstract inherited members
                    if !base.is_overridable {
                        failures.push(ObligationFailure::OverrideNotVirtual {
                            source: member.source,
                            member: member.key,
                        });
                    } else {
                        // overrides must remain assignable to the base member
                        let assignment = self.relate_member(
                            origin,
                            Relation::Subtype,
                            member.role,
                            member.ty,
                            base.ty,
                            None,
                            None,
                        )?;
                        if !assignment.holds() {
                            failures.push(ObligationFailure::IncompatibleOverride {
                                source: member.source,
                                member: member.key,
                                source_ty: member.ty,
                                target_ty: base.ty,
                            });
                        } else {
                            overrides.push(dir::MemberConformance {
                                member: member.symbol,
                                requirement: base.symbol,
                            });
                        }
                    }
                }
                // shadows need the override modifier
                (false, Some(_)) => {
                    failures.push(ObligationFailure::MissingOverride {
                        source: member.source,
                        member: member.key,
                    });
                }
                (false, None) => {}
            }

            // abstract members need an abstract class
            if member.is_abstract && !is_abstract {
                failures.push(ObligationFailure::AbstractMemberInConcreteClass {
                    source: member.source,
                    member: member.key,
                });
            }
        }

        // record the overrides on the extends edge
        if let Some(source) = extends_source {
            self.module_mut(symbol.module_id)
                .members_tail
                .set_conformance_members(source, overrides);
        }

        // concrete classes must provide every inherited abstract member
        if !is_abstract {
            let mut required = Vec::new();
            for member in &heritage.members {
                if required.iter().any(|(key, _)| *key == member.key) {
                    continue;
                }

                // let the closest occurrence decide whether the key is abstract
                required.push((member.key, member.is_abstract));
            }

            for (key, abstract_required) in required {
                if !abstract_required {
                    continue;
                }
                let provided = own
                    .iter()
                    .any(|member| member.key == key && !member.is_abstract);
                if !provided {
                    failures.push(ObligationFailure::UnimplementedAbstractMember {
                        source,
                        member: key,
                    });
                }
            }
        }

        // final base classes reject the extension
        if let Some(base) = heritage.final_base {
            failures.push(ObligationFailure::FinalClassExtended { source, base });
        }

        let check = ObligationCheck::from_failures(failures);

        Ok(check)
    }

    /// Return the inherited class member view.
    fn class_heritage(
        &mut self,
        _origin: Origin,
        extends: Option<dir::NominalHeritage>,
    ) -> CompilerResult<ClassHeritage> {
        let mut members = Vec::<ClassMember>::new();
        let mut final_base = None;
        let mut substitution = TypeSubstitution::default();
        let mut extends = extends;
        let mut depth = 0usize;

        // walk the heritage chain, applying each base's parameters
        while let Some(heritage) = extends.take() {
            depth += 1;

            // apply the previous base's parameters to this extends clause
            let ty = self.substitute_type(heritage.ty, &substitution)?;
            let (instance_module, instance) = self.nominal_application(ty)?;

            let definition = self.definition(instance.symbol)?;
            let Some(dir::Definition::Class(base)) = definition.as_deref() else {
                break;
            };
            let base_members = base.members.clone();
            extends = base.extends.clone();

            // check extension rejection on the direct base only
            if depth == 1 && base.is_final {
                final_base = Some(instance.symbol);
            }

            // apply this base's parameters to its inherited member types
            substitution = self.instance_substitution(instance_module, &instance)?;
            for member in &base_members {
                let Some(mut member) = self.class_member(member)? else {
                    continue;
                };
                member.ty = self.substitute_type(member.ty, &substitution)?;
                members.push(member);
            }
        }

        Ok(ClassHeritage {
            members,
            final_base,
        })
    }

    /// Return one declaration's application over its declared parameters.
    pub(in crate::sema) fn declaration_instance(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GenericApplication> {
        let parameters = match self.symbol_template(symbol)? {
            Some(template) => Some(self.generic_template_parameters(template)?),
            None => None,
        }
        .unwrap_or_default();
        let mut arguments = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let ty = dir::Type::Parameter(parameter);
            let argument = self.intern_type(ty)?;
            arguments.push(argument);
        }
        let arguments = self.intern_type_ids(&arguments)?;

        Ok(dir::GenericApplication { symbol, arguments })
    }
}
