use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckError, CheckState, Dependency, MemberRole, Origin, Relation, TypeSubstitution,
    answer,
};

/// One class instance member that participates in heritage checks.
struct ClassMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member type folded into the subclass's view.
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
    ) -> CompilerResult<Answer<Option<ClassMember>>> {
        match member {
            dir::DefinitionMember::Field(field) if field.space == dir::MemberSpace::Instance => {
                let Some(ty) = answer!(self.definition_member_type(member)?) else {
                    return Ok(Answer::Ready(None));
                };

                Ok(Answer::Ready(Some(ClassMember {
                    key: field.key,
                    ty,
                    source: field.source,
                    role: MemberRole::Field,
                    is_overridable: field.is_abstract,
                    is_override: field.is_override,
                    is_abstract: field.is_abstract,
                })))
            }
            dir::DefinitionMember::Method(method) if method.space == dir::MemberSpace::Instance => {
                let dir::MemberSlot::Key(key) = method.slot else {
                    return Ok(Answer::Ready(None));
                };
                let Some(ty) = answer!(self.definition_member_type(member)?) else {
                    return Ok(Answer::Ready(None));
                };
                let is_abstract = method.abstraction == dir::MethodAbstraction::Abstract;
                let is_virtual = method.abstraction == dir::MethodAbstraction::Virtual;
                let is_overridable = is_abstract || is_virtual;

                Ok(Answer::Ready(Some(ClassMember {
                    key,
                    ty,
                    source: method.source,
                    role: MemberRole::Method,
                    is_overridable,
                    is_override: method.is_override,
                    is_abstract,
                })))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Check one declaration against its heritage rules.
    pub(in crate::check) fn check_declaration_heritage(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let instance = self.declaration_instance(source, symbol)?;
        let closure = answer!(self.heritage_closure(origin, &instance)?);

        // report graph errors before class member rules
        let mut errors = Vec::<DiagnosticBuilder<CheckError>>::new();
        for conflict in closure.conflicts {
            let (module, anchor) = self.source_anchor(conflict.source);
            errors.push(
                CheckError::ConflictingHeritage {
                    anchor,
                    module,
                    source: self.format_symbol(symbol),
                    target: self.format_symbol(conflict.current.symbol),
                }
                .into(),
            );
        }
        for cycle in closure.cycles {
            let (module, anchor) = self.source_anchor(cycle.source);
            errors.push(
                CheckError::CircularHeritage {
                    anchor,
                    module,
                    source: self.format_symbol(symbol),
                }
                .into(),
            );
        }
        if !errors.is_empty() {
            return Ok(Answer::Ready(self.report_heritage_errors(source, errors)));
        }

        self.check_class_member_heritage(source, symbol)
    }

    /// Check one class declaration against its member heritage rules.
    fn check_class_member_heritage(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let Some(dir::Definition::Class(class)) = self.definition(symbol) else {
            return Ok(Answer::Ready(None));
        };
        let is_abstract = class.is_abstract;
        let extends = class.extends.clone();

        // collect own instance members relevant to heritage rules
        let members = class.members.clone();
        let mut own = Vec::new();
        for member in &members {
            if let Some(member) = answer!(self.class_member(member)?) {
                own.push(member);
            }
        }

        // collect inherited members walking up the extends chain
        let heritage = answer!(self.class_heritage(origin, extends)?);

        // decide every rule before reporting anything
        // (so pending re-runs never duplicate diagnostics)
        let mut errors = Vec::<DiagnosticBuilder<CheckError>>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for member in &own {
            let base = heritage
                .members
                .iter()
                .find(|inherited| inherited.key == member.key);
            let name = self.format_static_key(&member.key);
            let (member_module, anchor) = self.source_anchor(member.source);

            match (member.is_override, base) {
                // overrides need an inherited member to override
                (true, None) => {
                    errors.push(
                        CheckError::InvalidOverride {
                            anchor,
                            module: member_module,
                            member: name,
                        }
                        .into(),
                    );
                }
                (true, Some(base)) => {
                    // overrides need virtual or abstract inherited members
                    if !base.is_overridable {
                        errors.push(
                            CheckError::OverrideNotVirtual {
                                anchor,
                                module: member_module,
                                member: name,
                            }
                            .help("declare the inherited member 'virtual' or 'abstract'"),
                        );
                    } else {
                        // overrides must remain assignable to the base member
                        let assignment = if member.role == MemberRole::Method
                            && base.role == MemberRole::Method
                        {
                            self.decide_method_assignable(origin, member.ty, base.ty)?
                        } else {
                            self.decide_relation(origin, Relation::Assignable, member.ty, base.ty)?
                        };
                        match assignment {
                            Answer::Ready(true) => {}
                            Answer::Ready(false) => {
                                errors.push(
                                    CheckError::IncompatibleOverride {
                                        anchor,
                                        module: member_module,
                                        member: name,
                                        source: self.format_type(member.ty),
                                        target: self.format_type(base.ty),
                                    }
                                    .into(),
                                );
                            }
                            Answer::Pending(pending) => blockers.extend(pending),
                        }
                    }
                }
                // shadows need the override modifier
                (false, Some(_)) => {
                    errors.push(
                        CheckError::MissingOverride {
                            anchor,
                            module: member_module,
                            member: name,
                        }
                        .help("add the 'override' modifier"),
                    );
                }
                (false, None) => {}
            }

            // abstract members need an abstract class
            if member.is_abstract && !is_abstract {
                let name = self.format_static_key(&member.key);
                errors.push(
                    CheckError::AbstractMemberInConcreteClass {
                        anchor: self.source_anchor(member.source).1,
                        module: member_module,
                        member: name,
                    }
                    .into(),
                );
            }
        }

        // concrete classes must provide every inherited abstract member
        if !is_abstract {
            let mut required = Vec::new();
            for member in &heritage.members {
                if required.iter().any(|(key, _)| *key == member.key) {
                    continue;
                }
                // the closest occurrence decides whether the key is abstract
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
                    let (class_module, anchor) = self.source_anchor(source);
                    errors.push(
                        CheckError::UnimplementedAbstractMember {
                            anchor,
                            module: class_module,
                            member: self.format_static_key(&key),
                        }
                        .help("implement the member or declare the class 'abstract'"),
                    );
                }
            }
        }

        // final base classes reject the extension
        if let Some(base) = heritage.final_base {
            let (class_module, anchor) = self.source_anchor(source);
            errors.push(
                CheckError::FinalClassExtended {
                    anchor,
                    module: class_module,
                    ty: self.format_symbol(base),
                }
                .into(),
            );
        }

        // park until every override decision closes
        if !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }

        Ok(Answer::Ready(self.report_heritage_errors(source, errors)))
    }

    /// Return the inherited class member view.
    fn class_heritage(
        &mut self,
        origin: Origin,
        extends: Option<dir::NominalHeritage>,
    ) -> CompilerResult<Answer<ClassHeritage>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut members = Vec::<ClassMember>::new();
        let mut final_base = None;
        let mut substitution = TypeSubstitution::default();
        let mut extends = extends;
        let mut depth = 0usize;

        while let Some(heritage) = extends.take() {
            depth += 1;

            // apply the previous base's parameters to this next extends clause
            let mut arguments = heritage.arguments.clone();
            if !substitution.is_empty() {
                for argument in &mut arguments {
                    *argument =
                        self.fold_type(module, source, *argument, substitution.rewrite())?;
                }
            }
            let instance = dir::GenericInstance {
                symbol: heritage.symbol,
                arguments,
            };

            let Some(dir::Definition::Class(base)) = self.definition(instance.symbol) else {
                break;
            };
            let base_members = base.members.clone();
            extends = base.extends.clone();

            // only the direct base can reject extension
            if depth == 1 && base.is_final {
                final_base = Some(instance.symbol);
            }

            // apply this base's parameters to its inherited member types
            substitution = self.instance_substitution(&instance)?;
            for member in &base_members {
                let Some(mut member) = answer!(self.class_member(member)?) else {
                    continue;
                };
                if !substitution.is_empty() {
                    member.ty =
                        self.fold_type(module, source, member.ty, substitution.rewrite())?;
                }
                members.push(member);
            }
        }

        Ok(Answer::Ready(ClassHeritage {
            members,
            final_base,
        }))
    }

    /// Return one declaration's own generic application.
    fn declaration_instance(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GenericInstance> {
        let parameters = self
            .symbol_template(symbol)
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        let mut arguments = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let ty = dir::Type::Parameter(parameter);
            let argument = self.push_type(source.module_id, ty, source.local_id)?;
            arguments.push(argument);
        }

        Ok(dir::GenericInstance { symbol, arguments })
    }
}
