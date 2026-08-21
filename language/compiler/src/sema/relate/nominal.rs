use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{Cause, CauseId, CauseKind, CheckState, Origin, Relation, Verdict};
use crate::{CompilerError, CompilerResult};

/// One applied heritage edge in a nominal declaration closure.
#[derive(Debug, Clone)]
pub(in crate::sema) struct HeritageApplication {
    /// The source clause that introduced the application.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The applied nominal or interface type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
}

/// One duplicate heritage application with incompatible arguments.
#[derive(Debug, Clone)]
pub(in crate::sema) struct HeritageConflict {
    /// The source clause that introduced the conflicting application.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The conflicting inherited type.
    pub(in crate::sema) current: dir::GlobalTypeId,
}

/// One heritage branch that exposes a cycle.
#[derive(Debug, Clone)]
pub(in crate::sema) struct HeritageCycle {
    /// The source branch that exposes the cycle.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
}

/// Complete heritage closure for one nominal or interface application.
#[derive(Debug, Clone, Default)]
pub(in crate::sema) struct HeritageClosure {
    /// The inherited applications in traversal order.
    pub(in crate::sema) applications: SmallVec<[HeritageApplication; 8]>,
    /// Duplicate applications with different arguments.
    pub(in crate::sema) conflicts: SmallVec<[HeritageConflict; 2]>,
    /// Cycles found while walking heritage edges.
    pub(in crate::sema) cycles: SmallVec<[HeritageCycle; 2]>,
}

/// The declarations one declaration's heritage reaches.
#[derive(Debug, Clone)]
pub(in crate::sema) enum HeritageReach {
    /// The heritage reaches exactly these declarations, the root included.
    Closed(SmallVec<[dir::GlobalSymbolId; 8]>),
    /// The heritage passes through a parameter or an alias, so it reaches every declaration.
    Open,
}

impl HeritageClosure {
    /// Return the first application naming one symbol.
    fn application<'a>(
        &'a self,
        symbol: dir::GlobalSymbolId,
        state: &CheckState<'_>,
    ) -> CompilerResult<Option<&'a HeritageApplication>> {
        for application in &self.applications {
            let (_, instance) = state.nominal_application(application.ty)?;
            if instance.symbol == symbol {
                return Ok(Some(application));
            }
        }

        Ok(None)
    }
}

impl CheckState<'_> {
    /// Relate two closed roots under a check-only constraint relation.
    pub(in crate::sema) fn relate_satisfies(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // formed sources prove capability and interface targets directly
        if matches!(self.ty(source)?, dir::Type::Form(_)) {
            // capability targets decide through their intrinsic rule
            if let Some(interface) = self
                .type_symbol(target)?
                .map(|symbol| self.language_item(symbol))
                .transpose()?
                .flatten()
                .and_then(dir::AutoInterface::from_language_item)
                .filter(|interface| interface.has_builtin_implementation())
            {
                return self.satisfies_auto_interface(origin, source, interface);
            }

            // applied interface targets select implementations
            if let dir::Type::Application(instance) = self.ty(target)?
                && matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(_))
                )
            {
                return self.relate_interface(origin, cause, relation, source, target);
            }
        }

        // intersection targets require every element under the same relation
        if let dir::Type::Intersection(intersection) = self.ty(target)? {
            let elements: SmallVec<[_; 8]> = self
                .type_ids(target.module_id, intersection.elements)?
                .into();

            return self.relate_all_targets(origin, cause, relation, source, &elements);
        }

        // intersection sources satisfy through any element
        if let dir::Type::Intersection(intersection) = self.ty(source)? {
            let elements: SmallVec<[_; 8]> = self
                .type_ids(source.module_id, intersection.elements)?
                .into();

            return self.relate_any_source(origin, cause, relation, &elements, target);
        }

        // generic parameters prove relations through their bounds, then their form
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = self.ty(source)? {
            let decision = self
                .relate_parameter_bounds(origin, cause, relation, parameter, target)?
                .or_else(|| self.relate_into_union(origin, cause, relation, source, target))?;
            if decision == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
            let formed = self.constrain_form_assignable(origin, cause, relation, source, target)?;

            return Ok(formed.unwrap_or(decision));
        }

        // memory forms decide like closed form assignability
        if (matches!(self.ty(source)?, dir::Type::Form(_))
            || matches!(self.ty(target)?, dir::Type::Form(_)))
            && let Some(decision) =
                self.constrain_form_assignable(origin, cause, relation, source, target)?
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
            return Ok(Verdict::Holds);
        }

        // enum members satisfy constraints through their owner
        if let dir::Type::Variant(member) = self.ty(source)? {
            return self.constrain_type(origin, cause, relation, member.owner, target);
        }

        // union sources must satisfy the target through every element
        if let dir::Type::Union(union) = self.ty(source)? {
            let elements: SmallVec<[_; 8]> =
                self.type_ids(source.module_id, union.elements)?.into();

            return self.relate_all_sources(origin, cause, relation, &elements, target);
        }

        // union targets accept when any element accepts the source
        if let dir::Type::Union(union) = self.ty(target)? {
            let elements: SmallVec<[_; 8]> =
                self.type_ids(target.module_id, union.elements)?.into();

            return self.relate_any_target(origin, cause, relation, source, &elements);
        }

        // const scalars inhabit closed enums by member value
        if let dir::Type::Literal(literal) = self.ty(source)?
            && let Some(symbol) = self.type_symbol(target)?
            && let Some(dir::Definition::Enum(definition)) = self.definition(symbol)?
        {
            let members = definition.members.clone();
            for member in &members {
                let dir::DefinitionMember::EnumVariant(variant) = member else {
                    continue;
                };
                if dir::Literal::from(variant.value) == literal {
                    return Ok(Verdict::Holds);
                }
            }

            return Ok(Verdict::Fails);
        }

        // static scalar operations relate through their result type
        if matches!(
            self.operation_head(source)?,
            Some(dir::TypeOperation::StaticBinary(_) | dir::TypeOperation::StaticUnary(_))
        ) && let Some(result) = self.static_operation_type(origin, source)?
        {
            let verdict = self.constrain_type(origin, cause, relation, result, target)?;
            if verdict != Verdict::Fails {
                return Ok(verdict);
            }
        }

        // read the nominal application the target names, if any
        let target_instance = match self.ty(target)? {
            dir::Type::Application(target) => Some(target),
            _ => None,
        };

        // route every applied interface through one implementation selection path
        if let Some(target_instance) = target_instance.as_ref()
            && matches!(
                self.definition(target_instance.symbol)?,
                Some(dir::Definition::Interface(_))
            )
        {
            return self.relate_interface(origin, cause, relation, source, target);
        }

        // nominal sources meet nominal constraints through their declarations
        let instances = match (self.ty(source)?, target_instance) {
            (dir::Type::Application(source), Some(target)) => Some((source, target)),
            _ => None,
        };

        match (instances, relation) {
            (Some((source_instance, target_instance)), _) => self.relate_application(
                origin,
                cause,
                relation,
                source,
                &source_instance,
                target,
                &target_instance,
            ),

            // satisfy other values through assignability or union membership
            (None, _) => {
                // check-only relations read structural pairs covariantly
                if let (dir::Type::Object(_), dir::Type::Object(_)) =
                    (self.ty(source)?, self.ty(target)?)
                {
                    return self.relate_shape(origin, cause, relation, source, target);
                }

                // everything else decides through assignability
                self.relate_assignable(origin, cause, relation, source, target)?
                    .or_else(|| self.relate_into_union(origin, cause, relation, source, target))
            }
        }
    }

    /// Relate two applied declarations.
    pub(in crate::sema) fn relate_application(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
        target_instance: &dir::GenericApplication,
    ) -> CompilerResult<Verdict> {
        // keep nominal applications symbolic while declaring
        let has_target_definition = self.definition(target_instance.symbol)?.is_some();
        let has_source_definition = self.definition(source_instance.symbol)?.is_some();
        if !has_target_definition || !has_source_definition {
            if self.is_declaration() {
                return Ok(Verdict::Holds);
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nominal relation has missing definitions: source = {:?}, target = {:?}",
                    source_instance.symbol, target_instance.symbol,
                ),
            });
        }

        let target_kind = self.symbol_kind(target_instance.symbol)?;

        // interface targets select one implementation path
        if target_kind.is_interface() {
            return self.relate_interface(origin, cause, relation, source, target);
        }

        // select the source application of the target declaration
        let application = if source_instance.symbol == target_instance.symbol {
            Some((source.module_id, *source_instance))
        } else {
            let heritage =
                self.heritage_instance(origin, source, source, target_instance.symbol)?;

            match heritage {
                Some(heritage) => Some(self.nominal_application(heritage)?),
                None => None,
            }
        };

        // compare every nominal path through one argument relation
        if let Some((application_module, application)) = application {
            let source_arguments: SmallVec<[_; 8]> = self
                .type_ids(application_module, application.arguments)?
                .into();
            let target_arguments: SmallVec<[_; 8]> = self
                .type_ids(target.module_id, target_instance.arguments)?
                .into();
            let form = self.default_variance_form(target_instance.symbol)?;

            let arguments = if relation == Relation::Subtype {
                self.relate_application_arguments(
                    origin,
                    cause,
                    target_instance.symbol,
                    form,
                    relation,
                    &source_arguments,
                    &target_arguments,
                )?
            } else {
                self.constrain_instance_arguments(
                    origin,
                    application_module,
                    &application,
                    target.module_id,
                    target_instance,
                )?
            };

            return Ok(arguments);
        }

        Ok(Verdict::Fails)
    }

    /// Relate two different nominal applications under assignability.
    pub(in crate::sema) fn relate_application_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let (source_instance, target_instance) = match (self.ty(source)?, self.ty(target)?) {
            (dir::Type::Application(source), dir::Type::Application(target)) => (source, target),
            _ => return Ok(Verdict::Fails),
        };

        self.relate_application(
            origin,
            cause,
            Relation::Assignable,
            source,
            &source_instance,
            target,
            &target_instance,
        )
    }

    /// Return the first declared field missing from one struct construction.
    pub(in crate::sema) fn first_missing_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the written fields of the structural source
        let source_fields = match self.ty(source)? {
            dir::Type::Object(shape) => self
                .shape_properties(source.module_id, shape.properties)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        // read construction fields from a struct application only
        let dir::Type::Application(target_instance) = self.ty(target)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Struct(_)) = self.definition(target_instance.symbol)? else {
            return Ok(None);
        };

        // scan declared fields in construction order
        for (key, is_required) in self.nominal_instance_fields(target_instance.symbol)? {
            if source_fields.contains(&key) {
                continue;
            }

            if !is_required {
                continue;
            }

            return Ok(Some(key));
        }

        Ok(None)
    }

    /// Return the first undeclared field supplied to one struct construction.
    pub(in crate::sema) fn first_excess_struct_field(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        // read the written fields of the structural source
        let source_fields = match self.ty(source)? {
            dir::Type::Object(shape) => self
                .shape_properties(source.module_id, shape.properties)?
                .iter()
                .map(|field| field.key)
                .collect::<SmallVec<[_; 8]>>(),
            _ => return Ok(None),
        };

        // read construction fields from a struct application only
        let dir::Type::Application(target_instance) = self.ty(target)? else {
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

    /// Return substituted direct instance fields for one struct.
    pub(in crate::sema) fn struct_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeProperty; 8]>> {
        // read the nominal application beneath the target's memory forms
        let receiver = target;
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(SmallVec::new());
        };

        self.instantiate_struct_fields(origin, receiver, target.module_id, &instance)
    }

    /// Return substituted fields accepted by one struct constructor.
    pub(in crate::sema) fn struct_constructor_fields(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeProperty; 8]>> {
        // read the struct declaration behind the target's instance fields
        let mut fields = self.struct_fields(origin, target)?;
        let receiver = target;
        let chain = self.form_chain(origin, receiver)?;
        let target = chain.base();
        let dir::Type::Application(instance) = self.ty(target)? else {
            return Ok(fields);
        };
        let Some(dir::Definition::Struct(definition)) = self.definition(instance.symbol)? else {
            return Ok(fields);
        };

        // initialized fields may be omitted from construction
        for field in &mut fields {
            let is_initialized = definition.members.iter().any(|member| match member {
                dir::DefinitionMember::Field(declared) => {
                    declared.space == dir::MemberSpace::Instance
                        && declared.key == field.key
                        && declared.initializer.is_some()
                }
                _ => false,
            });
            field.is_optional |= is_initialized;
        }

        Ok(fields)
    }

    /// Return substituted direct fields for one struct instance.
    fn instantiate_struct_fields(
        &mut self,
        _origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<SmallVec<[dir::TypeProperty; 8]>> {
        let Some(dir::Definition::Struct(definition)) = self.definition(instance.symbol)?.cloned()
        else {
            return Ok(SmallVec::new());
        };

        // substitute the instance arguments over the declared field types
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

            let ty = self.symbol_type(field.symbol)?;
            let ty = self.substitute_type(ty, &substitution)?;
            fields.push(dir::TypeProperty {
                key: field.key,
                access: dir::PropertyAccess::ReadWrite {
                    read: ty,
                    write: ty,
                },
                is_optional: field.is_optional,
            });
        }

        Ok(fields)
    }

    /// Bound open construction arguments from written literal fields.
    pub(in crate::sema) fn constrain_struct_construction(
        &mut self,
        origin: Origin,
        source_fields: &[dir::TypeProperty],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // relate each written field against its declared counterpart
        let declared_fields = self.struct_fields(origin, target)?;
        for declared in declared_fields {
            let Some(field) = source_fields.iter().find(|field| field.key == declared.key) else {
                continue;
            };

            let field_cause =
                self.intern_cause(Cause::root(origin, CauseKind::Field { key: field.key }));
            self.constrain_type(
                origin,
                field_cause,
                Relation::Assignable,
                field.access.store(),
                declared.access.store(),
            )?;
        }

        Ok(())
    }

    /// Collect one definition's instance field keys through heritage.
    pub(in crate::sema) fn nominal_field_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        Ok(self
            .nominal_instance_fields(symbol)?
            .into_iter()
            .map(|(key, _)| key)
            .collect())
    }

    /// Collect one definition's instance fields with their required presence.
    pub(in crate::sema) fn nominal_instance_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::StaticKey, bool); 8]>> {
        // walk the heritage chain from the declaration, visiting each symbol once
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
            let members = definition.members();
            let bases = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();

            // keep each declared instance field with its required presence
            for member in members {
                if let dir::DefinitionMember::Field(field) = member
                    && field.space == dir::MemberSpace::Instance
                {
                    let is_required = !field.is_optional && field.initializer.is_none();
                    keys.push((field.key, is_required));
                }
            }

            // queue the bases this declaration inherits from
            for heritage in bases {
                let (_, base) = self.nominal_application(heritage)?;
                pending.push(base.symbol);
            }
        }

        Ok(keys)
    }

    /// Collect one definition's member keys through its heritage chain.
    pub(in crate::sema) fn nominal_member_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::StaticKey; 8]>> {
        // walk the heritage chain from the declaration, visiting each symbol once
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
            let members = definition.members();
            let bases = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();

            // keep every keyed member of this declaration
            for member in members {
                if let Some(key) = member.key() {
                    keys.push(key);
                }
            }

            // queue the bases this declaration inherits from
            for heritage in bases {
                let (_, base) = self.nominal_application(heritage)?;
                pending.push(base.symbol);
            }
        }

        Ok(keys)
    }

    /// Relate one reference source against a structural shape target.
    pub(in crate::sema) fn relate_reference_against_target(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source_module: ModuleId,
        source_instance: &dir::GenericApplication,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // require each target field from the source fields
        let (fields, index_signatures) = match self.ty(target)? {
            dir::Type::Object(shape) => (
                self.shape_properties(target.module_id, shape.properties)?
                    .iter()
                    .map(|field| (field.key, field.access.store(), field.is_optional))
                    .collect::<SmallVec<[_; 4]>>(),
                SmallVec::<[_; 4]>::from(
                    self.shape_index_signatures(target.module_id, shape.index_signatures)?,
                ),
            ),
            _ => return Ok(Verdict::Fails),
        };
        let module = origin.module();
        let source = self.reference_type(origin, source_module, source_instance)?;

        // look each target key up on the source and relate what it finds
        let mut verdict = Verdict::Holds;
        for (key, field_type, is_optional) in fields {
            let subject = dir::MemberSubject::new(source, source, dir::MemberSpace::Instance);
            let lookup = self.body().lookup_member(origin, module, subject, key)?;
            let member = self.body().member_read_type(&lookup)?;

            match member {
                // missing members satisfy optional targets only
                None => {
                    if !is_optional {
                        return Ok(Verdict::Fails);
                    }
                }
                Some(member) => {
                    verdict = verdict
                        .and(self.constrain_type(origin, cause, relation, member, field_type)?);
                    if verdict == Verdict::Fails {
                        return Ok(Verdict::Fails);
                    }
                }
            }
        }

        // require each target index signature from the source
        for signature in index_signatures {
            verdict =
                verdict.and(self.relate_index_signature(origin, relation, source, &signature)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Return the full heritage closure for one nominal application.
    pub(in crate::sema) fn heritage_closure(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<HeritageClosure> {
        let ty = self.heritage_root(origin, ty)?;

        // close a structural entry over an empty heritage
        let Some((instance_module, instance)) = self.nominal_application_maybe(ty)? else {
            return Ok(HeritageClosure::default());
        };

        self.instance_heritage_closure(origin, ty, instance_module, instance)
    }

    /// Resolve one heritage root, reducing aliases to their nominal application.
    fn heritage_root(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let is_alias = match self.nominal_application_maybe(ty)? {
            Some((_, instance)) => matches!(
                self.definition(instance.symbol)?,
                Some(dir::Definition::TypeAlias(_))
            ),
            None => true,
        };

        Ok(match is_alias {
            true => self.normalize(origin, ty)?,
            false => ty,
        })
    }

    /// Build the heritage closure from one nominal instance under one receiver.
    pub(in crate::sema) fn instance_heritage_closure(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: dir::GenericApplication,
    ) -> CompilerResult<HeritageClosure> {
        let mut closure = HeritageClosure::default();
        let mut active = SmallVec::<[dir::GlobalSymbolId; 8]>::new();

        // dispatch same-symbol extension instantiations by form
        let independent = matches!(
            self.definition(instance.symbol)?,
            Some(dir::Definition::Extension(_))
        );

        // walk the heritage edges from the root
        active.push(instance.symbol);
        self.collect_heritage(
            origin,
            receiver,
            instance_module,
            &instance,
            None,
            independent,
            &mut active,
            &mut closure,
        )?;

        Ok(closure)
    }

    /// Collect inherited applications from one nominal application.
    fn collect_heritage(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        branch_source: Option<dir::GlobalNodeIdAny>,
        independent: bool,
        active: &mut SmallVec<[dir::GlobalSymbolId; 8]>,
        closure: &mut HeritageClosure,
    ) -> CompilerResult<()> {
        // read the declaration this application instantiates, staying symbolic while declaring
        let declaring = self.is_declaration();
        let definition = match self.definition(instance.symbol)? {
            Some(definition) => definition,
            None if declaring => return Ok(()),
            None => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "heritage application {:?} has no checked definition",
                        instance.symbol
                    ),
                });
            }
        };

        // read the bases and implemented interfaces this declaration inherits from
        let heritages = definition.heritage_edges();

        // apply this instance's arguments to every edge below
        let substitution =
            self.qualified_instance_substitution(instance_module, instance, receiver)?;

        // walk direct heritage edges with applied arguments
        for heritage in heritages {
            let ty = self.substitute_type(heritage.ty, &substitution)?;
            let (application_module, instance) = self.nominal_application(ty)?;
            let application = HeritageApplication {
                source: branch_source.unwrap_or(heritage.source),
                ty,
            };

            // cycles are reported at the branch that exposed the cycle
            if active.contains(&instance.symbol) {
                closure.cycles.push(HeritageCycle {
                    source: application.source,
                });
                continue;
            }

            // require duplicate applications to use the same arguments
            if let Some(previous) = closure.application(instance.symbol, self)? {
                if independent {
                    continue;
                }

                let (previous_module, previous_instance) = self.nominal_application(previous.ty)?;
                let arguments = self.constrain_instance_arguments(
                    origin,
                    previous_module,
                    &previous_instance,
                    application_module,
                    &instance,
                )?;
                if arguments == Verdict::Fails {
                    closure.conflicts.push(HeritageConflict {
                        source: application.source,
                        current: application.ty,
                    });
                }
                continue;
            }

            // recurse through newly reached applications, keeping this at the root receiver
            closure.applications.push(application.clone());
            active.push(instance.symbol);
            self.collect_heritage(
                origin,
                receiver,
                application_module,
                &instance,
                Some(application.source),
                independent,
                active,
                closure,
            )?;
            active.pop();
        }

        Ok(())
    }

    /// Return whether one declaration's heritage reaches a target declaration.
    ///
    /// The answer over-approximates the applied closure: heritage written through a parameter
    /// or an alias reaches every declaration.
    pub(in crate::sema) fn reaches_heritage(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        // decide the reach once per declaration
        if !self.heritages.contains_key(&symbol) {
            let reach = self.collect_heritage_reach(symbol)?;
            self.heritages.insert(symbol, reach);
        }

        Ok(match &self.heritages[&symbol] {
            HeritageReach::Closed(symbols) => symbols.contains(&target),
            HeritageReach::Open => true,
        })
    }

    /// Collect the declarations one declaration's heritage reaches, walking its edges.
    fn collect_heritage_reach(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<HeritageReach> {
        // seed the walk with the declaration itself
        let mut reached = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        let mut frontier = SmallVec::<[dir::GlobalSymbolId; 8]>::new();
        reached.push(symbol);
        frontier.push(symbol);

        while let Some(current) = frontier.pop() {
            // an unchecked or aliased declaration leaves the heritage open
            let Some(definition) = self.definition(current)? else {
                return Ok(HeritageReach::Open);
            };
            if matches!(definition, dir::Definition::TypeAlias(_)) {
                return Ok(HeritageReach::Open);
            }

            // read the bases and implemented interfaces this declaration inherits from
            let edges = definition.heritage_edges();

            // an edge written over a parameter or an alias leaves the heritage open
            for edge in edges {
                let Some((_, instance)) = self.nominal_application_maybe(edge.ty)? else {
                    return Ok(HeritageReach::Open);
                };

                // queue each newly reached declaration
                if !reached.contains(&instance.symbol) {
                    reached.push(instance.symbol);
                    frontier.push(instance.symbol);
                }
            }
        }

        Ok(HeritageReach::Closed(reached))
    }

    /// Find one heritage application naming a target symbol, transitively.
    pub(in crate::sema) fn heritage_instance(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.heritage_root(origin, ty)?;
        let Some((instance_module, instance)) = self.nominal_application_maybe(ty)? else {
            return Ok(None);
        };
        let closure =
            self.instance_heritage_closure(origin, receiver, instance_module, instance)?;
        if let Some(application) = closure.application(target, self)? {
            return Ok(Some(application.ty));
        }

        Ok(None)
    }

    /// Relate arguments of two same-template applications.
    pub(in crate::sema) fn constrain_instance_arguments(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: &dir::GenericApplication,
        target_module: ModuleId,
        target: &dir::GenericApplication,
    ) -> CompilerResult<Verdict> {
        // written applications complete their elided arguments
        let mut source = *source;
        let mut target = *target;
        let mut source_module = source_module;
        let mut target_module = target_module;
        if source.arguments.len() != target.arguments.len() {
            if let Some(filled) = self.fill_elided_application(source_module, &source)? {
                let dir::Type::Application(filled) = self.ty(filled)? else {
                    unreachable!("filled application is not an application");
                };
                source = filled;
                source_module = self.module_id;
            }

            if let Some(filled) = self.fill_elided_application(target_module, &target)? {
                let dir::Type::Application(filled) = self.ty(filled)? else {
                    unreachable!("filled application is not an application");
                };
                target = filled;
                target_module = self.module_id;
            }
        }

        // both applications must reach the same arity to pair up
        if source.arguments.len() != target.arguments.len() {
            return Ok(Verdict::Fails);
        }

        // pair the arguments positionally
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

        // erase proof-only lifetime slots from instance identity
        let lifetimes = match self.symbol_template(source.symbol)? {
            Some(template) => {
                let parameters = self.generic_template_parameters(template)?;
                parameters
                    .iter()
                    .map(|parameter| {
                        self.generic_parameter(*parameter).is_some_and(|binding| {
                            binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
                        })
                    })
                    .collect::<SmallVec<[bool; 4]>>()
            }
            None => SmallVec::new(),
        };

        // require every remaining pair to be equal
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let mut verdict = Verdict::Holds;
        for (index, (source_argument, target_argument)) in pairs.into_iter().enumerate() {
            if lifetimes.get(index).copied().unwrap_or(false) {
                continue;
            }

            let relation = Relation::Equal;
            verdict = verdict.and(self.constrain_type(
                origin,
                cause,
                relation,
                source_argument,
                target_argument,
            )?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Allocate one reference type for a nominal application.
    fn reference_type(
        &mut self,
        _origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // adopt the arguments into this module before interning
        let arguments: SmallVec<[_; 8]> =
            self.type_ids(instance_module, instance.arguments)?.into();
        let arguments = self.intern_type_ids(&arguments)?;
        let instance = dir::GenericApplication {
            symbol: instance.symbol,
            arguments,
        };

        self.intern_type(dir::Type::Application(instance))
    }
}
