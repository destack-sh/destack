use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, Cause, CauseId, CauseKind, CheckState, Dependency, Origin, Relation, answer,
};

/// One applied heritage edge in a nominal declaration closure.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageApplication {
    /// The source clause that introduced the application.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The applied nominal or interface instance.
    pub(in crate::check) instance: dir::GenericInstance,
}

/// One duplicate heritage application with incompatible arguments.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageConflict {
    /// The source clause that introduced the conflicting application.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The conflicting inherited application.
    pub(in crate::check) current: dir::GenericInstance,
}

/// One heritage branch that exposes a cycle.
#[derive(Debug, Clone)]
pub(in crate::check) struct HeritageCycle {
    /// The source branch that exposes the cycle.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

/// Complete heritage closure for one nominal or interface application.
#[derive(Debug, Clone, Default)]
pub(in crate::check) struct HeritageClosure {
    /// The inherited applications in traversal order.
    pub(in crate::check) applications: SmallVec<[HeritageApplication; 8]>,
    /// Duplicate applications with different arguments.
    pub(in crate::check) conflicts: SmallVec<[HeritageConflict; 2]>,
    /// Cycles found while walking heritage edges.
    pub(in crate::check) cycles: SmallVec<[HeritageCycle; 2]>,
}

impl HeritageClosure {
    /// Return the first application naming one symbol.
    fn application(&self, symbol: dir::GlobalSymbolId) -> Option<&HeritageApplication> {
        self.applications
            .iter()
            .find(|application| application.instance.symbol == symbol)
    }
}

impl CheckState<'_> {
    /// Return the substituted backing type for one newtype instance.
    pub(in crate::check) fn newtype_backing_type(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol)? else {
            return Ok(None);
        };
        let backing = definition.backing;
        let substitution = self.instance_substitution(instance_module, instance)?;
        let backing = self.substitute_type(origin.module(), backing, &substitution)?;

        Ok(Some(backing))
    }

    /// Re-root one instance's argument list into the origin module.
    pub(in crate::check) fn origin_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericInstance,
    ) -> CompilerResult<dir::GenericInstance> {
        let module = origin.module();
        if instance_module == module || instance.arguments.is_empty() {
            return Ok(instance);
        }

        let arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
            self.type_ids(instance_module, instance.arguments)?,
        );
        let arguments = self.intern_type_ids(module, &arguments)?;

        Ok(dir::GenericInstance {
            symbol: instance.symbol,
            arguments,
        })
    }

    /// Decide one check-only constraint relation between closed roots.
    pub(in crate::check) fn decide_satisfies(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_equal(origin, source, target)?.is_ready_true() {
            return Ok(Answer::Ready(true));
        }

        // memory forms decide like closed form assignability
        if (matches!(self.ty(source)?, dir::Type::Form(_))
            || matches!(self.ty(target)?, dir::Type::Form(_)))
            && let Some(decision) =
                self.constrain_form_assignable_rooted(origin, relation, source, target)?
        {
            return Ok(decision);
        }

        // memory singletons inhabit their stdlib singleton kind
        let memory_kind = match self.ty(source)? {
            dir::Type::Memory(source) => Some(source.kind_language_item()),
            _ => None,
        };
        let target_item = self
            .type_symbol(target)?
            .map(|symbol| self.language_item(symbol))
            .transpose()?
            .flatten();
        if let Some(memory_kind) = memory_kind
            && target_item == Some(memory_kind)
        {
            return Ok(Answer::Ready(true));
        }

        // enum members satisfy constraints through their owner
        if let dir::Type::EnumMember(member) = self.ty(source)? {
            return self.decide_relation(origin, relation, member.owner, target);
        }

        // generic parameters prove relations through their active bounds
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = self.ty(source)? {
            let decision = self.decide_parameter_relation(origin, relation, parameter, target)?;

            return self.decide_union_membership(origin, relation, decision, source, target);
        }

        // union sources must satisfy the target through every element
        if let dir::Type::Union(union) = self.ty(source)? {
            let elements = self.type_ids(source.module_id, union.elements)?.to_vec();

            return self.decide_all_sources(origin, relation, &elements, target);
        }

        // union targets accept when any element accepts the source
        if let dir::Type::Union(union) = self.ty(target)? {
            let elements = self.type_ids(target.module_id, union.elements)?.to_vec();

            return self.decide_any_target(origin, relation, source, &elements);
        }

        // comptime scalars inhabit closed enums by member value
        if let dir::Type::Literal(literal) = self.ty(source)?
            && let Some(symbol) = self.type_symbol(target)?
            && let Some(dir::Definition::Enum(definition)) = self.definition(symbol)?
        {
            let members = definition.members.clone();
            for member in &members {
                let dir::DefinitionMember::EnumVariant(variant) = member else {
                    continue;
                };
                if dir::ScalarLiteral::from(variant.value) == literal {
                    return Ok(Answer::Ready(true));
                }
            }

            return Ok(Answer::Ready(false));
        }

        // static scalar operations relate through their result type
        if matches!(
            self.operation_head(source)?,
            Some(dir::TypeOperation::StaticBinary(_) | dir::TypeOperation::StaticUnary(_))
        ) && let Some(result) = answer!(self.static_operation_type(origin, source)?)
        {
            let decision = self.decide_relation(origin, relation, result, target)?;
            if !matches!(decision, Answer::Ready(false)) {
                return Ok(decision);
            }
        }

        // intersection targets require every element under the same relation
        if let dir::Type::Intersection(intersection) = self.ty(target)? {
            let elements = self
                .type_ids(target.module_id, intersection.elements)?
                .to_vec();

            return self.decide_all_targets(origin, relation, source, &elements);
        }

        // intersection sources satisfy through any element
        if let dir::Type::Intersection(intersection) = self.ty(source)? {
            let elements = self
                .type_ids(source.module_id, intersection.elements)?
                .to_vec();

            return self.decide_any_source(origin, relation, &elements, target);
        }

        // nominal sources meet nominal constraints through their declarations
        let instances = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Instance(source), dir::Type::Instance(target)) => Some((source, target)),
            _ => None,
        };
        let target_instance = match self.ty(target)? {
            dir::Type::Instance(target) => Some(target),
            _ => None,
        };

        match (instances, relation) {
            (Some((source_instance, target_instance)), _) => self.decide_nominal_satisfies(
                origin,
                source,
                &source_instance,
                target,
                &target_instance,
            ),

            // auto interfaces decide by their derivation rules
            (None, _)
                if let Some(interface) = target_instance
                    .as_ref()
                    .and_then(|instance| self.language_item(instance.symbol).ok().flatten())
                    .and_then(dir::AutoInterface::from_language_item)
                    .filter(|interface| interface.has_auto_conformance()) =>
            {
                self.satisfies_auto_interface(origin, source, interface)
            }

            // check extension implementations over any receiver form
            (None, _) if self.is_interface_instance(target_instance.as_ref())? => {
                let Some(target_instance) = target_instance.as_ref() else {
                    return Ok(Answer::Ready(false));
                };
                let module = origin.module();
                let implemented = self.body(module).decide_extension_implementation(
                    origin,
                    module,
                    target.module_id,
                    source,
                    target_instance,
                    None,
                )?;
                if !matches!(implemented, Answer::Ready(false)) {
                    return Ok(implemented);
                }

                self.decide_assignable(origin, Relation::Assignable, source, target)
            }

            // explicit implements requires the heritage relation
            (None, Relation::Implements) => Ok(Answer::Ready(false)),

            // everything else satisfies through assignability,
            //  or sits inside a union target as a member
            (None, _) => {
                // check-only relations read structural pairs covariantly
                if let (dir::Type::Shape(_), dir::Type::Shape(_)) =
                    (self.ty(source)?, self.ty(target)?)
                {
                    return self.decide_shape_relation(origin, relation, source, target);
                }
                let assignable = self.decide_assignable(origin, relation, source, target)?;

                self.decide_union_membership(origin, relation, assignable, source, target)
            }
        }
    }

    /// Constrain one open nominal application into a required interface.
    pub(in crate::check) fn constrain_nominal_satisfies(
        &mut self,
        cause: CauseId,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.cause_origin(cause);

        // declared heritage carries the relation and binds open arguments
        if let Some(heritage) = answer!(self.heritage_instance(
            origin,
            source.module_id,
            source_instance,
            target_instance.symbol
        )?) {
            let heritage_arguments = self.type_ids(origin.module(), heritage.arguments)?.to_vec();
            let target_arguments = self
                .type_ids(target.module_id, target_instance.arguments)?
                .to_vec();

            let context = self.default_symbol_context(target_instance.symbol);

            return self.relate_type_arguments(
                cause,
                target_instance.symbol,
                context,
                Relation::Assignable,
                &heritage_arguments,
                &target_arguments,
            );
        }

        // a sole conforming extension binds open arguments through its clause
        let module = origin.module();
        let extensions = self
            .body(module)
            .visible_receiver_extensions(module, source)?;
        let mut sole = None;
        for extension_symbol in extensions {
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }
            let names_target = extension
                .implements
                .iter()
                .any(|heritage| heritage.symbol == target_instance.symbol);
            if !names_target {
                continue;
            }
            if sole.is_some() {
                return Ok(Answer::Ready(false));
            }
            sole = Some((
                extension_symbol,
                extension.target.r#type(),
                extension.implements.clone(),
            ));
        }
        let Some((extension_symbol, target_type, implements)) = sole else {
            return Ok(Answer::Ready(false));
        };

        // bind the extension pattern to the open source
        let template = self.symbol_template(extension_symbol)?;
        let Some(substitution) = answer!(self.body(module).match_extension_target(
            origin,
            source,
            template,
            target_type
        )?) else {
            return Ok(Answer::Ready(false));
        };

        // relate the substituted clause against the required application
        for heritage in implements {
            if heritage.symbol != target_instance.symbol {
                continue;
            }
            let applied =
                answer!(self.substituted_heritage(origin, module, &substitution, &heritage)?);
            let applied_arguments = self.type_ids(module, applied.arguments)?.to_vec();
            let target_arguments = self
                .type_ids(target.module_id, target_instance.arguments)?
                .to_vec();

            let context = self.default_symbol_context(target_instance.symbol);

            return self.relate_type_arguments(
                cause,
                target_instance.symbol,
                context,
                Relation::Assignable,
                &applied_arguments,
                &target_arguments,
            );
        }

        Ok(Answer::Ready(false))
    }

    /// Decide whether one nominal application satisfies another.
    fn decide_nominal_satisfies(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // same-symbol interface applications compare by declared variance
        if source_instance.symbol == target_instance.symbol {
            if matches!(
                self.definition(target_instance.symbol)?,
                Some(dir::Definition::Interface(_))
            ) {
                let source_arguments = self
                    .type_ids(source.module_id, source_instance.arguments)?
                    .to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();

                let context = self.default_symbol_context(target_instance.symbol);

                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                return self.relate_type_arguments(
                    cause,
                    target_instance.symbol,
                    context,
                    Relation::Assignable,
                    &source_arguments,
                    &target_arguments,
                );
            }

            return self.constrain_instance_arguments(
                origin,
                source.module_id,
                source_instance,
                target.module_id,
                target_instance,
            );
        }

        // heritage carries the relation when it names the target;
        //  heritage argument list always land in origin.module()
        if let Some(heritage) = answer!(self.heritage_instance(
            origin,
            source.module_id,
            source_instance,
            target_instance.symbol
        )?) {
            let is_interface = matches!(
                self.definition(target_instance.symbol)?,
                Some(dir::Definition::Interface(_))
            );
            let arguments = if is_interface {
                let heritage_arguments =
                    self.type_ids(origin.module(), heritage.arguments)?.to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();

                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                self.relate_type_arguments(
                    cause,
                    target_instance.symbol,
                    self.default_symbol_context(target_instance.symbol),
                    Relation::Assignable,
                    &heritage_arguments,
                    &target_arguments,
                )?
            } else {
                self.constrain_instance_arguments(
                    origin,
                    origin.module(),
                    &heritage,
                    target.module_id,
                    target_instance,
                )?
            };
            if !is_interface {
                return Ok(arguments);
            }

            let members =
                self.decide_interface_satisfied(origin, source, target.module_id, target_instance)?;

            return Ok(arguments.and(members));
        }

        // check visible extension implementations
        if matches!(
            self.definition(target_instance.symbol)?,
            Some(dir::Definition::Interface(_))
        ) {
            let module = origin.module();
            let implemented = self.body(module).decide_extension_implementation(
                origin,
                module,
                target.module_id,
                source,
                target_instance,
                None,
            )?;
            if !matches!(implemented, Answer::Ready(false)) {
                return Ok(implemented);
            }
        }

        // structural interfaces satisfy member-wise
        let target_definition = self.definition(target_instance.symbol)?;
        let is_structural_interface = matches!(
            target_definition,
            Some(dir::Definition::Interface(interface)) if !interface.is_nominal
        );
        if is_structural_interface {
            return self.decide_interface_satisfied(
                origin,
                source,
                target.module_id,
                target_instance,
            );
        }

        Ok(Answer::Ready(false))
    }

    /// Return whether one instance target names an interface.
    pub(in crate::check) fn is_interface_instance(
        &mut self,
        instance: Option<&dir::GenericInstance>,
    ) -> CompilerResult<bool> {
        let Some(instance) = instance else {
            return Ok(false);
        };

        Ok(matches!(
            self.definition(instance.symbol)?,
            Some(dir::Definition::Interface(_))
        ))
    }

    /// Decide assignability between different nominal applications.
    pub(in crate::check) fn decide_nominal_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (source_instance, target_instance) = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Instance(source), dir::Type::Instance(target)) => (source, target),
            _ => return Ok(Answer::Ready(false)),
        };

        self.decide_nominal_satisfies(origin, source, &source_instance, target, &target_instance)
    }

    /// Decide whether one structural shape satisfies one reference target.
    pub(in crate::check) fn decide_source_against_reference(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_module: ModuleId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // shapes satisfy structural interfaces member-wise
        let is_structural_interface = matches!(
            self.definition(target_instance.symbol)?,
            Some(dir::Definition::Interface(interface)) if !interface.is_nominal
        );
        if is_structural_interface {
            return self.decide_interface_satisfied(origin, source, target_module, target_instance);
        }

        Ok(Answer::Ready(false))
    }

    /// Decide whether one literal shape constructs one struct.
    pub(in crate::check) fn decide_struct_construction(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // collect the written literal fields
        let written = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_fields(source.module_id, shape.fields)?
                .iter()
                .map(|field| (field.key, field.ty))
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(Answer::Ready(false)),
        };

        // require every declared instance field from the literal
        let module = origin.module();
        let mut decision = Answer::Ready(true);
        for (key, has_initializer) in self.nominal_instance_fields(target_instance.symbol)? {
            let lookup = answer!(self.body(module).lookup_member(
                origin,
                module,
                target,
                dir::MemberSpace::Instance,
                key
            )?);
            let declared = match lookup.field_type() {
                Some(ty) => ty,
                None => continue,
            };

            // omitted fields fill from their initializers or undefined
            let supplied = written.iter().find(|(written, _)| *written == key);
            let Some((_, supplied)) = supplied else {
                if answer!(self.field_may_be_omitted(origin, declared, has_initializer)?) {
                    continue;
                }

                return Ok(Answer::Ready(false));
            };

            // written fields write into fresh storage
            decision = decision.and(self.decide_relation(
                origin,
                Relation::Writable,
                *supplied,
                declared,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // reject written fields that are not declared construction fields
        let fields = self.nominal_field_keys(target_instance.symbol)?;
        for (key, _) in written {
            if !fields.contains(&key) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(decision)
    }

    /// Return the first declared field missing from one struct construction.
    pub(in crate::check) fn first_missing_struct_field(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source_fields = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_fields(source.module_id, shape.fields)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        let dir::Type::Instance(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol)? else {
            return Ok(None);
        };

        // scan declared fields in construction order
        let module = origin.module();
        for (key, has_initializer) in self.nominal_instance_fields(target_instance.symbol)? {
            if source_fields.contains(&key) {
                continue;
            }

            let lookup = match self.body(module).lookup_member(
                origin,
                module,
                target,
                dir::MemberSpace::Instance,
                key,
            )? {
                Answer::Ready(lookup) => lookup,
                Answer::Pending(_) => return Ok(None),
            };
            let Some(declared) = lookup.field_type() else {
                continue;
            };

            let may_omit = match self.field_may_be_omitted(origin, declared, has_initializer)? {
                Answer::Ready(may_omit) => may_omit,
                Answer::Pending(_) => return Ok(None),
            };
            if may_omit {
                continue;
            }

            return Ok(Some(key));
        }

        Ok(None)
    }

    /// Return the first undeclared field supplied to one struct construction.
    pub(in crate::check) fn first_excess_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let source_fields = match self.ty(source)? {
            dir::Type::Shape(shape) => self
                .shape_fields(source.module_id, shape.fields)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        let dir::Type::Instance(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol)? else {
            return Ok(None);
        };

        // compare against declared construction fields
        let fields = self.nominal_field_keys(target_instance.symbol)?;
        for key in source_fields {
            if !fields.contains(&key) {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }

    /// Return substituted direct instance field types for one constructed struct.
    pub(in crate::check) fn struct_field_types(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[dir::TypeField; 8]>>> {
        let receiver = answer!(self.reduce_type_head(origin, target)?);
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Instance(instance) = self.ty(target)? else {
            return Ok(Answer::Ready(SmallVec::new()));
        };

        self.struct_instance_field_types(origin, receiver, target.module_id, &instance)
    }

    /// Return substituted direct fields for one struct instance.
    fn struct_instance_field_types(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<SmallVec<[dir::TypeField; 8]>>> {
        let Some(dir::Definition::Struct(definition)) = self.definition(instance.symbol)?.cloned()
        else {
            return Ok(Answer::Ready(SmallVec::new()));
        };
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(receiver);
        let mut fields = SmallVec::new();

        // collect direct instance fields through the selected target
        for member in definition.members {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };
            if field.space != dir::MemberSpace::Instance {
                continue;
            }
            let Some(ty) = self.symbol_type_maybe(field.symbol) else {
                return Ok(Answer::pending([Dependency::SymbolType(field.symbol)]));
            };
            let ty = self.substitute_type(origin.module(), ty, &substitution)?;
            fields.push(dir::TypeField {
                key: field.key,
                ty,
                is_optional: field.is_optional || field.initializer.is_some(),
                is_readonly: false,
            });
        }

        Ok(Answer::Ready(fields))
    }

    /// Return whether one literal may omit one declared field.
    pub(in crate::check) fn field_may_be_omitted(
        &mut self,
        origin: Origin,
        declared: dir::GlobalTypeId,
        has_initializer: bool,
    ) -> CompilerResult<Answer<bool>> {
        if has_initializer {
            return Ok(Answer::Ready(true));
        }
        let undefined = self.intern_type(origin.module(), dir::Type::Undefined)?;

        self.decide_relation(origin, Relation::Assignable, undefined, declared)
    }

    /// Bound open construction arguments from written literal fields.
    pub(in crate::check) fn constrain_struct_construction(
        &mut self,
        origin: Origin,
        source_fields: &[dir::TypeField],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let declared_fields = answer!(self.struct_field_types(origin, target)?);
        for declared in declared_fields {
            let Some(field) = source_fields.iter().find(|field| field.key == declared.key) else {
                continue;
            };

            let field_cause =
                self.intern_cause(Cause::root(origin, CauseKind::Field { key: field.key }));
            answer!(self.constrain_type(field_cause, Relation::Writable, field.ty, declared.ty)?);
        }

        Ok(Answer::Ready(()))
    }

    /// Collect one definition's instance field keys through heritage.
    pub(in crate::check) fn nominal_field_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        Ok(self
            .nominal_instance_fields(symbol)?
            .into_iter()
            .map(|(key, _)| key)
            .collect())
    }

    /// Collect one definition's instance fields with their initializer presence.
    pub(in crate::check) fn nominal_instance_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::StaticKey, bool); 8]>> {
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);

            let Some(definition) = self.definition(symbol)? else {
                continue;
            };
            for member in definition.members() {
                if let dir::DefinitionMember::Field(field) = member
                    && field.space == dir::MemberSpace::Instance
                {
                    keys.push((field.key, field.initializer.is_some()));
                }
            }
            for heritage in definition.bases() {
                pending.push(heritage.symbol);
            }
        }

        Ok(keys)
    }

    /// Collect one definition's member keys through its heritage chain.
    pub(in crate::check) fn nominal_member_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        let mut keys = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);

            let Some(definition) = self.definition(symbol)? else {
                continue;
            };
            for member in definition.members() {
                if let Some(key) = member.key() {
                    keys.push(key);
                }
            }
            for heritage in definition.bases() {
                pending.push(heritage.symbol);
            }
        }

        Ok(keys)
    }

    /// Decide whether one reference source satisfies one structural shape target.
    pub(in crate::check) fn decide_reference_against_target(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source_instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // require each target field from the source fields
        let (fields, index_signatures) = match self.ty(target)? {
            dir::Type::Shape(shape) => (
                self.shape_fields(target.module_id, shape.fields)?
                    .iter()
                    .map(|field| (field.key, field.ty, field.is_optional))
                    .collect::<SmallVec<[_; 4]>>(),
                self.shape_index_signatures(target.module_id, shape.index_signatures)?
                    .to_vec(),
            ),
            _ => return Ok(Answer::Ready(false)),
        };
        let module = origin.module();
        let source = self.reference_type(origin, source_module, source_instance)?;
        let mut decision = Answer::Ready(true);
        for (key, field_type, is_optional) in fields {
            let lookup = answer!(self.body(module).lookup_member(
                origin,
                module,
                source,
                dir::MemberSpace::Instance,
                key
            )?);

            let member = lookup.value_type();

            match member {
                // missing members satisfy optional targets only
                None => {
                    if !is_optional {
                        return Ok(Answer::Ready(false));
                    }
                }
                Some(member) => {
                    decision = decision.and(self.decide_relation(
                        origin,
                        Relation::Assignable,
                        member,
                        field_type,
                    )?);
                    if decision.is_ready_false() {
                        return Ok(decision);
                    }
                }
            }
        }
        for signature in index_signatures {
            decision =
                decision.and(self.decide_index_signature_satisfied(origin, source, &signature)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the full heritage closure for one nominal application.
    pub(in crate::check) fn heritage_closure(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<HeritageClosure>> {
        let mut closure = HeritageClosure::default();
        let mut active = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // extensions implement each application independently, so
        //  same-symbol instantiations dispatch by form instead of conflicting
        let independent = matches!(
            self.definition(instance.symbol)?,
            Some(dir::Definition::Extension(_))
        );

        active.push(instance.symbol);
        self.collect_heritage(
            origin,
            instance_module,
            instance,
            None,
            independent,
            &mut active,
            &mut closure,
            &mut blockers,
        )?;

        Ok(Answer::ready_unless_blocked(closure, blockers))
    }

    /// Collect inherited applications from one nominal application.
    fn collect_heritage(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        branch_source: Option<dir::GlobalNodeIdAny>,
        independent: bool,
        active: &mut SmallVec<[dir::GlobalSymbolId; 8]>,
        closure: &mut HeritageClosure,
        blockers: &mut SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<()> {
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(());
        };
        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (*heritage).clone())
            .collect::<SmallVec<[_; 2]>>();
        let substitution = self.instance_substitution(instance_module, instance)?;
        let module = origin.module();

        // walk direct heritage edges with applied arguments
        for heritage in heritages {
            let instance =
                match self.substituted_heritage(origin, module, &substitution, &heritage)? {
                    Answer::Ready(instance) => instance,
                    Answer::Pending(pending) => {
                        blockers.extend(pending);
                        continue;
                    }
                };
            let application = HeritageApplication {
                source: branch_source.unwrap_or(heritage.source),
                instance,
            };

            // cycles are reported at the branch that exposed the cycle
            if active.contains(&application.instance.symbol) {
                closure.cycles.push(HeritageCycle {
                    source: application.source,
                });
                continue;
            }

            // duplicate applications must use the same arguments,
            //  except under declarations that implement each independently
            if let Some(previous) = closure.application(application.instance.symbol) {
                if independent {
                    continue;
                }
                match self.constrain_instance_arguments(
                    origin,
                    module,
                    &previous.instance,
                    module,
                    &application.instance,
                )? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => closure.conflicts.push(HeritageConflict {
                        source: application.source,
                        current: application.instance,
                    }),
                    Answer::Pending(pending) => blockers.extend(pending),
                }
                continue;
            }

            // recurse through newly reached applications
            closure.applications.push(application.clone());
            active.push(application.instance.symbol);
            self.collect_heritage(
                origin,
                module,
                &application.instance,
                Some(application.source),
                independent,
                active,
                closure,
                blockers,
            )?;
            active.pop();
        }

        Ok(())
    }

    /// Find one heritage application naming a target symbol, transitively.
    pub(in crate::check) fn heritage_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GenericInstance>>> {
        let closure = answer!(self.heritage_closure(origin, instance_module, instance)?);
        if let Some(application) = closure.application(target) {
            return Ok(Answer::Ready(Some(application.instance)));
        }

        Ok(Answer::Ready(None))
    }

    /// Relate arguments of two same-template applications.
    pub(in crate::check) fn constrain_instance_arguments(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: &dir::GenericInstance,
        target_module: ModuleId,
        target: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        if source.arguments.len() != target.arguments.len() {
            return Ok(Answer::Ready(false));
        }

        let pairs = self
            .type_ids(source_module, source.arguments)?
            .iter()
            .copied()
            .zip(
                self.type_ids(target_module, target.arguments)?
                    .iter()
                    .copied(),
            )
            .collect::<SmallVec<[_; 4]>>();

        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let mut decision = Answer::Ready(true);
        for (source_argument, target_argument) in pairs {
            decision = decision.and(self.constrain_type(
                cause,
                Relation::Equal,
                source_argument,
                target_argument,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Allocate one reference type for a nominal application.
    fn reference_type(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let instance = self.origin_instance(origin, instance_module, *instance)?;

        self.intern_type(origin.module(), dir::Type::Instance(instance))
    }
}
