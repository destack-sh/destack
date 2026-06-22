use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, Dependency, Origin, Relation, Substitution};

/// How one class member participates in override assignability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberForm {
    /// Field members use regular assignability.
    Field,
    /// Method members ignore receiver assignability.
    Method,
}

/// One instance member inherited from a base, closest base first.
struct BaseMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member type folded into the subclass's view.
    ty: dir::GlobalTypeId,
    /// How the member participates in override assignability.
    form: MemberForm,
    /// Whether subclasses may override the member.
    overridable: bool,
    /// Whether subclasses must provide the member.
    is_abstract: bool,
}

/// One member declared on the checked class itself.
struct DeclaredMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member type.
    ty: dir::GlobalTypeId,
    /// The member declaration node.
    source: dir::GlobalNodeIdAny,
    /// How the member participates in override assignability.
    form: MemberForm,
    /// Whether the member declares an override.
    is_override: bool,
    /// Whether the member is abstract.
    is_abstract: bool,
}

impl BaseMember {
    /// Return the inherited class member represented by one definition member.
    fn from_definition(member: &dir::DefinitionMember) -> Option<Self> {
        match member {
            dir::DefinitionMember::Field(field) if field.space == dir::MemberSpace::Instance => {
                Some(Self {
                    key: field.key,
                    ty: field.ty,
                    form: MemberForm::Field,
                    overridable: field.is_abstract,
                    is_abstract: field.is_abstract,
                })
            }
            dir::DefinitionMember::Method(method) if method.space == dir::MemberSpace::Instance => {
                let dir::MemberSlot::Key(key) = method.slot else {
                    return None;
                };
                let is_abstract = method.abstraction == dir::MethodAbstraction::Abstract;
                let is_virtual = method.abstraction == dir::MethodAbstraction::Virtual;
                let overridable = is_abstract || is_virtual;

                Some(Self {
                    key,
                    ty: method.ty,
                    form: MemberForm::Method,
                    overridable,
                    is_abstract,
                })
            }
            _ => None,
        }
    }
}

impl DeclaredMember {
    /// Return the declared class member represented by one definition member.
    fn from_definition(member: &dir::DefinitionMember) -> Option<Self> {
        match member {
            dir::DefinitionMember::Field(field) if field.space == dir::MemberSpace::Instance => {
                Some(Self {
                    key: field.key,
                    ty: field.ty,
                    source: field.source,
                    form: MemberForm::Field,
                    is_override: field.is_override,
                    is_abstract: field.is_abstract,
                })
            }
            dir::DefinitionMember::Method(method) if method.space == dir::MemberSpace::Instance => {
                let dir::MemberSlot::Key(key) = method.slot else {
                    return None;
                };

                Some(Self {
                    key,
                    ty: method.ty,
                    source: method.source,
                    form: MemberForm::Method,
                    is_override: method.is_override,
                    is_abstract: method.abstraction == dir::MethodAbstraction::Abstract,
                })
            }
            _ => None,
        }
    }
}

impl CheckState<'_> {
    /// Check one declaration against its heritage rules.
    pub(in crate::check) fn check_heritage(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let instance = self.declaration_instance(source, symbol)?;
        let closure = match self.heritage_closure(origin, &instance)? {
            Answer::Ready(closure) => closure,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

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
        let own = class
            .members
            .iter()
            .filter_map(DeclaredMember::from_definition)
            .collect::<Vec<_>>();

        // collect inherited members walking up the extends chain
        let (inherited, final_base) = self.class_base_members(origin, extends)?;

        // decide every rule before reporting anything
        // (so pending re-runs never duplicate diagnostics)
        let mut errors = Vec::<DiagnosticBuilder<CheckError>>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for member in &own {
            let base = inherited
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
                    if !base.overridable {
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
                        let assignment = if member.form == MemberForm::Method
                            && base.form == MemberForm::Method
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
            for member in &inherited {
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
        if let Some(base) = final_base {
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

    /// Return inherited class members and the direct final base, if present.
    fn class_base_members(
        &mut self,
        origin: Origin,
        extends: Option<dir::NominalHeritage>,
    ) -> CompilerResult<(Vec<BaseMember>, Option<dir::GlobalSymbolId>)> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut inherited = Vec::<BaseMember>::new();
        let mut final_base = None;
        let mut substitution = Substitution::default();
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
            let members = base
                .members
                .iter()
                .filter_map(BaseMember::from_definition)
                .collect::<Vec<_>>();
            extends = base.extends.clone();

            // only the direct base can reject extension
            if depth == 1 && base.is_final {
                final_base = Some(instance.symbol);
            }

            // apply this base's parameters to its inherited member types
            substitution = self.parameter_substitution(&instance)?;
            for mut member in members {
                if !substitution.is_empty() {
                    member.ty =
                        self.fold_type(module, source, member.ty, substitution.rewrite())?;
                }
                inherited.push(member);
            }
        }

        Ok((inherited, final_base))
    }

    /// Return one declaration's own generic application.
    fn declaration_instance(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GenericInstance> {
        let parameters = self
            .generics
            .template_by_symbol(symbol)
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

    /// Return the first heritage error and report the rest directly.
    fn report_heritage_errors(
        &mut self,
        source: dir::GlobalNodeIdAny,
        errors: Vec<DiagnosticBuilder<CheckError>>,
    ) -> Option<DiagnosticBuilder<CheckError>> {
        let mut errors = errors.into_iter();
        let first = errors.next();
        for error in errors {
            self.module_mut(source.module_id).diagnostics.push(error);
        }

        first
    }
}
