use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, Dependency, Origin, Relation, Substitution};

/// One instance member inherited from a base, closest base first.
struct BaseMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member type folded into the subclass's view.
    ty: dir::GlobalTypeId,
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
    /// Whether the member declares an override.
    is_override: bool,
    /// Whether the member is abstract.
    is_abstract: bool,
}

impl CheckState<'_> {
    /// Check one class declaration against its heritage rules.
    pub(in crate::check) fn check_class_heritage(
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
        let mut own = Vec::new();
        for member in &class.members {
            match member {
                dir::DefinitionMember::Field(field)
                    if field.space == dir::MemberSpace::Instance =>
                {
                    own.push(DeclaredMember {
                        key: field.key,
                        ty: field.ty,
                        source: field.source,
                        is_override: field.is_override,
                        is_abstract: field.is_abstract,
                    });
                }
                dir::DefinitionMember::Method(method)
                    if method.space == dir::MemberSpace::Instance =>
                {
                    let dir::MemberSlot::Key(key) = method.slot else {
                        continue;
                    };
                    own.push(DeclaredMember {
                        key,
                        ty: method.ty,
                        source: method.source,
                        is_override: method.is_override,
                        is_abstract: method.abstraction == dir::MethodAbstraction::Abstract,
                    });
                }
                _ => {}
            }
        }

        // collect inherited members walking up the extends chain
        let module = origin.module();
        let source_node = self.origin_source_node(origin)?;
        let mut inherited = Vec::<BaseMember>::new();
        let mut final_base = None;
        let mut substitution = Substitution::default();
        let mut extends = extends;
        let mut depth = 0usize;
        while let Some(heritage) = extends.take() {
            depth += 1;

            // fold this level's arguments into the subclass's view
            let mut arguments = heritage.arguments.clone();
            if !substitution.is_empty() {
                for argument in &mut arguments {
                    *argument =
                        self.fold_type(module, source_node, *argument, substitution.rewrite())?;
                }
            }
            let instance = dir::GenericInstance {
                symbol: heritage.symbol,
                arguments,
            };
            let Some(dir::Definition::Class(base)) = self.definition(instance.symbol) else {
                break;
            };

            // only the direct base can reject extension
            if depth == 1 && base.is_final {
                final_base = Some(instance.symbol);
            }

            let members = base
                .members
                .iter()
                .filter_map(|member| match member {
                    dir::DefinitionMember::Field(field)
                        if field.space == dir::MemberSpace::Instance =>
                    {
                        Some((field.key, field.ty, field.is_abstract, field.is_abstract))
                    }
                    dir::DefinitionMember::Method(method)
                        if method.space == dir::MemberSpace::Instance =>
                    {
                        let dir::MemberSlot::Key(key) = method.slot else {
                            return None;
                        };
                        let is_abstract = method.abstraction == dir::MethodAbstraction::Abstract;
                        let overridable =
                            is_abstract || method.abstraction == dir::MethodAbstraction::Virtual;

                        Some((key, method.ty, overridable, is_abstract))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            extends = base.extends.clone();

            // write inherited member types in the subclass's view
            substitution = self.parameter_substitution(&instance)?;
            for (key, ty, overridable, is_abstract) in members {
                let ty = if substitution.is_empty() {
                    ty
                } else {
                    self.fold_type(module, source_node, ty, substitution.rewrite())?
                };
                inherited.push(BaseMember {
                    key,
                    ty,
                    overridable,
                    is_abstract,
                });
            }
        }

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
                        match self.decide_relation(
                            origin,
                            Relation::Assignable,
                            member.ty,
                            base.ty,
                        )? {
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

        // hand the first error to the obligation, report the rest directly
        let mut errors = errors.into_iter();
        let first = errors.next();
        for error in errors {
            let module = source.module_id;
            self.module_mut(module).diagnostics.push(error);
        }

        Ok(Answer::Ready(first))
    }
}
