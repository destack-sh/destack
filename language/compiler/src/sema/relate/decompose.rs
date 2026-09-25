use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;
use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{
    ActiveGoal, CheckState, GenericParameterId, Origin, Relation, TypeSubstitution, Verdict,
};

impl CheckState<'_> {
    /// Decompose two same-constructor types into pairs of their fixed children.
    pub(in crate::sema) fn decompose_type_pair(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        // decompose by the heads standing on both sides
        let pair_lists = match (self.ty(source)?, self.ty(target)?) {
            // nominal applications decompose by declaration with kind-aware pairing
            (dir::Type::Application(source_type), dir::Type::Application(target_type))
                if source_type.symbol == target_type.symbol =>
            {
                let source = self.type_ids(source.module_id, source_type.arguments)?;
                let target = self.type_ids(target.module_id, target_type.arguments)?;
                let Some(pairs) = self.pair_application_arguments(source, target)? else {
                    return Ok(None);
                };

                return Ok(Some(pairs));
            }

            // member projections decompose by key, owner, arguments, and qualifier declaration
            (dir::Type::Member(source_type), dir::Type::Member(target_type))
                if let source_type = self.type_member(source.module_id, source_type)?
                    && let target_type = self.type_member(target.module_id, target_type)?
                    && source_type.key == target_type.key
                    && self
                        .qualifier_roots_agree(source_type.qualifier, target_type.qualifier)? =>
            {
                let mut source_slots = SmallVec::from_slice(&[source_type.owner]);
                source_slots
                    .extend_from_slice(self.type_ids(source.module_id, source_type.arguments)?);

                let mut target_slots = SmallVec::from_slice(&[target_type.owner]);
                target_slots
                    .extend_from_slice(self.type_ids(target.module_id, target_type.arguments)?);

                (source_slots, target_slots)
            }

            // variants decompose by member and owner
            (dir::Type::Variant(source_type), dir::Type::Variant(target_type))
                if source_type.variant == target_type.variant =>
            {
                (
                    SmallVec::from_slice(&[source_type.owner]),
                    SmallVec::from_slice(&[target_type.owner]),
                )
            }

            // decompose memory forms by constructor into their fixed type children
            (dir::Type::Form(source_type), dir::Type::Form(target_type)) => {
                match (source_type.form, target_type.form) {
                    // borrows decompose over region, access, and value
                    (dir::Form::Borrowed(source_borrow), dir::Form::Borrowed(target_borrow)) => {
                        let source_borrow = self.type_borrow(source.module_id, source_borrow)?;
                        let target_borrow = self.type_borrow(target.module_id, target_borrow)?;

                        (
                            SmallVec::from_slice(&[
                                source_borrow.region,
                                source_borrow.access,
                                source_type.value,
                            ]),
                            SmallVec::from_slice(&[
                                target_borrow.region,
                                target_borrow.access,
                                target_type.value,
                            ]),
                        )
                    }
                    // every other matching form decomposes over its value alone
                    (source_form, target_form) if source_form == target_form => (
                        SmallVec::from_slice(&[source_type.value]),
                        SmallVec::from_slice(&[target_type.value]),
                    ),
                    // reject two differing form constructors
                    _ => return Ok(None),
                }
            }

            // dynamic representations decompose over their constraints
            (dir::Type::Dynamic(source_type), dir::Type::Dynamic(target_type)) => (
                SmallVec::from_slice(&[source_type.constraint]),
                SmallVec::from_slice(&[target_type.constraint]),
            ),

            // callables decompose over their signatures
            (dir::Type::Function(source_type), dir::Type::Function(target_type)) => (
                SmallVec::from_slice(&[source_type.signature]),
                SmallVec::from_slice(&[target_type.signature]),
            ),
            // function pointers decompose over their signature alone
            (dir::Type::FunctionPointer(source_type), dir::Type::FunctionPointer(target_type)) => (
                SmallVec::from_slice(&[source_type.signature]),
                SmallVec::from_slice(&[target_type.signature]),
            ),

            // type operations decompose after non-type payloads agree
            (dir::Type::Operation(source_type), dir::Type::Operation(target_type)) => {
                let source_type = self.type_operation(source.module_id, source_type)?;
                let target_type = self.type_operation(target.module_id, target_type)?;

                return self.decompose_operation_pair(
                    source.module_id,
                    &source_type,
                    target.module_id,
                    &target_type,
                );
            }

            // slices decompose over their element
            (dir::Type::Slice(source_type), dir::Type::Slice(target_type)) => (
                SmallVec::from_slice(&[source_type.element]),
                SmallVec::from_slice(&[target_type.element]),
            ),
            // fixed arrays decompose over their element and length
            (dir::Type::FixedArray(source_type), dir::Type::FixedArray(target_type)) => (
                SmallVec::from_slice(&[source_type.element, source_type.count]),
                SmallVec::from_slice(&[target_type.element, target_type.count]),
            ),

            // tuples decompose element-wise when their element shapes agree
            (dir::Type::Tuple(source_type), dir::Type::Tuple(target_type))
                if source_type.form == target_type.form
                    && source_type.elements.len() == target_type.elements.len() =>
            {
                let source_elements =
                    self.tuple_elements(source.module_id, source_type.elements)?;
                let target_elements =
                    self.tuple_elements(target.module_id, target_type.elements)?;

                let mut source_slots = SmallVec::new();
                let mut target_slots = SmallVec::new();
                for (source, target) in source_elements.iter().zip(target_elements) {
                    if source.label != target.label
                        || source.is_optional != target.is_optional
                        || source.is_readonly != target.is_readonly
                        || source.is_rest != target.is_rest
                    {
                        return Ok(None);
                    }

                    source_slots.push(source.ty);
                    target_slots.push(target.ty);
                }

                (source_slots, target_slots)
            }

            // signatures decompose inputs and outputs when their shapes agree
            (
                dir::Type::FunctionSignature(source_type),
                dir::Type::FunctionSignature(target_type),
            ) if let source_type = self.type_signature(source.module_id, source_type)?
                && let target_type = self.type_signature(target.module_id, target_type)?
                && source_type.is_construct == target_type.is_construct
                && source_type.this_parameter.is_some() == target_type.this_parameter.is_some()
                && source_type.return_type.is_some() == target_type.return_type.is_some() =>
            {
                let source_parameters =
                    self.signature_parameters(source.module_id, source_type.parameters)?;
                let target_parameters =
                    self.signature_parameters(target.module_id, target_type.parameters)?;
                if source_parameters.len() != target_parameters.len() {
                    return Ok(None);
                }

                let mut source_slots = SmallVec::new();
                let mut target_slots = SmallVec::new();
                source_slots.extend(source_type.this_parameter);
                target_slots.extend(target_type.this_parameter);

                for (source, target) in source_parameters.iter().zip(target_parameters) {
                    if source.is_optional != target.is_optional || source.is_rest != target.is_rest
                    {
                        return Ok(None);
                    }

                    source_slots.push(source.ty);
                    target_slots.push(target.ty);
                }

                source_slots.extend(source_type.return_type);
                target_slots.extend(target_type.return_type);

                (source_slots, target_slots)
            }

            // reject every other pair of constructors
            _ => return Ok(None),
        };

        Ok(type_pairs(pair_lists.0, pair_lists.1))
    }

    /// Decompose two fixed-arity type operations under one shared constructor.
    fn decompose_operation_pair(
        &self,
        source_module: ModuleId,
        source: &dir::TypeOperation,
        target_module: ModuleId,
        target: &dir::TypeOperation,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        let pair_lists =
            match (source, target) {
                // string mappings decompose over their mapped target
                (
                    dir::TypeOperation::StringMapping {
                        mapping: source_mapping,
                        target: source_target,
                    },
                    dir::TypeOperation::StringMapping {
                        mapping: target_mapping,
                        target: target_target,
                    },
                ) if source_mapping == target_mapping => (
                    SmallVec::from_slice(&[*source_target]),
                    SmallVec::from_slice(&[*target_target]),
                ),

                // conditionals decompose operands and branches
                (
                    dir::TypeOperation::Conditional(source),
                    dir::TypeOperation::Conditional(target),
                ) if source.is_distributive == target.is_distributive => (
                    SmallVec::from_slice(&[
                        source.left,
                        source.right,
                        source.then_type,
                        source.else_type,
                    ]),
                    SmallVec::from_slice(&[
                        target.left,
                        target.right,
                        target.then_type,
                        target.else_type,
                    ]),
                ),

                // narrows decompose source and target under one polarity
                (dir::TypeOperation::Narrow(source), dir::TypeOperation::Narrow(target))
                    if source.is_positive == target.is_positive =>
                {
                    (
                        SmallVec::from_slice(&[source.source, source.target]),
                        SmallVec::from_slice(&[target.source, target.target]),
                    )
                }

                // mapped types decompose constraint, key remap, and value under one binder
                (dir::TypeOperation::Mapped(source), dir::TypeOperation::Mapped(target))
                    if source.parameter.name == target.parameter.name
                        && source.parameter.parameter == target.parameter.parameter
                        && source.modifiers == target.modifiers
                        && source.parameter.key_remap.is_some()
                            == target.parameter.key_remap.is_some() =>
                {
                    let mut source_slots = SmallVec::from_slice(&[source.parameter.constraint]);
                    source_slots.extend(source.parameter.key_remap);
                    source_slots.push(source.value);
                    let mut target_slots = SmallVec::from_slice(&[target.parameter.constraint]);
                    target_slots.extend(target.parameter.key_remap);
                    target_slots.push(target.value);

                    (source_slots, target_slots)
                }

                // indexed accesses decompose receiver and index
                (dir::TypeOperation::Index(source), dir::TypeOperation::Index(target)) => (
                    SmallVec::from_slice(&[source.left, source.index]),
                    SmallVec::from_slice(&[target.left, target.index]),
                ),

                // type queries compare by declaration
                (dir::TypeOperation::TypeOf(source), dir::TypeOperation::TypeOf(target))
                    if source.symbol == target.symbol =>
                {
                    (SmallVec::new(), SmallVec::new())
                }

                // applications compare their targets and written arguments
                (
                    dir::TypeOperation::Instantiation(source),
                    dir::TypeOperation::Instantiation(target),
                ) => {
                    let mut sources = SmallVec::from_slice(&[source.target]);
                    sources.extend_from_slice(self.type_ids(source_module, source.arguments)?);
                    let mut targets = SmallVec::from_slice(&[target.target]);
                    targets.extend_from_slice(self.type_ids(target_module, target.arguments)?);

                    (sources, targets)
                }

                // template literals decompose spans under equal strings
                (
                    dir::TypeOperation::TemplateLiteral(source),
                    dir::TypeOperation::TemplateLiteral(target),
                ) if {
                    let source_strings = self.template_strings(source_module, source.strings)?;
                    source_strings == self.template_strings(target_module, target.strings)?
                } =>
                {
                    (
                        SmallVec::from_slice(self.type_ids(source_module, source.spans)?),
                        SmallVec::from_slice(self.type_ids(target_module, target.spans)?),
                    )
                }

                // infer binders decompose their optional constraint under one name
                (dir::TypeOperation::Infer(source), dir::TypeOperation::Infer(target))
                    if source.name == target.name
                        && source.constraint.is_some() == target.constraint.is_some() =>
                {
                    (
                        SmallVec::from_iter(source.constraint),
                        SmallVec::from_iter(target.constraint),
                    )
                }

                // unary operations decompose their targets
                (dir::TypeOperation::KeyOf(source), dir::TypeOperation::KeyOf(target))
                | (dir::TypeOperation::NoInfer(source), dir::TypeOperation::NoInfer(target))
                | (dir::TypeOperation::Awaited(source), dir::TypeOperation::Awaited(target))
                | (dir::TypeOperation::SpaceOf(source), dir::TypeOperation::SpaceOf(target)) => (
                    SmallVec::from_slice(&[source.target]),
                    SmallVec::from_slice(&[target.target]),
                ),

                // try projections decompose their projected value
                (
                    dir::TypeOperation::TryOutput { value: source },
                    dir::TypeOperation::TryOutput { value: target },
                )
                | (
                    dir::TypeOperation::TryResidual { value: source },
                    dir::TypeOperation::TryResidual { value: target },
                )
                | (
                    dir::TypeOperation::TryFailure { value: source },
                    dir::TypeOperation::TryFailure { value: target },
                ) => (
                    SmallVec::from_slice(&[*source]),
                    SmallVec::from_slice(&[*target]),
                ),

                // static binary operations decompose operands under one operator
                (
                    dir::TypeOperation::StaticBinary(source),
                    dir::TypeOperation::StaticBinary(target),
                ) if source.operator == target.operator => (
                    SmallVec::from_slice(&[source.left, source.right]),
                    SmallVec::from_slice(&[target.left, target.right]),
                ),

                // static unary operations decompose targets under one operator
                (
                    dir::TypeOperation::StaticUnary(source),
                    dir::TypeOperation::StaticUnary(target),
                ) if source.operator == target.operator => (
                    SmallVec::from_slice(&[source.target]),
                    SmallVec::from_slice(&[target.target]),
                ),

                // reject every other pair of operations
                _ => return Ok(None),
            };

        Ok(type_pairs(pair_lists.0, pair_lists.1))
    }

    /// Extend one substitution by structurally matching positional type pairs.
    pub(in crate::sema) fn extend_generic_substitution(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<bool> {
        for (pattern, actual) in pairs.iter().copied() {
            // matching binds parameters; assignability decides the rest later
            if !self.type_flags(pattern)?.has_parameter() {
                continue;
            }

            if !self.match_generic_type(origin, parameters, substitution, pattern, actual)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether two projection qualifiers name the same declaration.
    fn qualifier_roots_agree(
        &self,
        source: Option<dir::GlobalTypeId>,
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        let (Some(source), Some(target)) = (source, target) else {
            return Ok(source.is_none() && target.is_none());
        };

        Ok(self.qualifier_root(source)? == self.qualifier_root(target)?)
    }

    /// Return the declaration one projection qualifier names.
    fn qualifier_root(
        &self,
        qualifier: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let qualifier = self.shallow_resolve(qualifier)?;
        // name the declaration the qualifier applies
        match self.ty(qualifier)? {
            dir::Type::Reference(reference) => Ok(Some(reference.symbol)),
            dir::Type::Application(instance) => Ok(Some(instance.symbol)),
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(qualifier.module_id, refined)?;

                self.qualifier_root(refined.base)
            }
            _ => Ok(None),
        }
    }

    /// Match one generic type pattern without opening inference variables.
    fn match_generic_type(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // reject a pair that recurses into the question it answers
        let question = ActiveGoal::Match(pattern, actual, substitution_fingerprint(substitution));
        if !self.active.insert(question) {
            return Ok(false);
        }

        // hold the question on the matching path across the step
        let matched =
            self.match_generic_type_step(origin, parameters, substitution, pattern, actual);
        self.active.swap_remove(&question);

        matched
    }

    /// Match one pattern against one actual, a step beneath the inductive guard.
    fn match_generic_type_step(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // bind an open extension parameter before reducing the authored argument
        let pattern = self.shallow_resolve(pattern)?;
        let actual = self.shallow_resolve(actual)?;
        if let dir::Type::Parameter(parameter) = self.ty(pattern)? {
            if parameters.contains(&parameter) {
                return self.bind_generic_argument(origin, substitution, parameter, actual);
            }

            // defer an open actual under a rigid parameter to the relation
            if self.root_variable(actual)?.is_some() {
                return Ok(true);
            }
        }

        // leave open pattern children to the relation, which binds them
        if self.root_variable(pattern)?.is_some() {
            return Ok(true);
        }

        // leave a pattern binding nothing further over an open actual to the relation
        if self.root_variable(actual)?.is_some()
            && self.type_parameters(pattern)?.iter().all(|parameter| {
                !parameters.contains(parameter) || substitution.argument(*parameter).is_some()
            })
        {
            return Ok(true);
        }

        // accept unknown as an actual, which every pattern matches
        if matches!(self.ty(actual)?, dir::Type::Unknown) {
            return Ok(true);
        }

        // match region terms by the region rule, binding free extents and spaces
        if let Some(matched) =
            self.match_region_terms(origin, parameters, substitution, pattern, actual)?
        {
            return Ok(matched);
        }

        // accept one identical parameter-free type
        if pattern == actual && !self.type_flags(pattern)?.has_parameter() {
            return Ok(true);
        }

        // match two closed access rungs, the rung lattice deciding the relation afterwards
        if self.access_of(pattern)?.is_some() && self.access_of(actual)?.is_some() {
            return Ok(true);
        }

        // unions and intersections match as unordered type sets
        let sets = match (self.ty(pattern)?, self.ty(actual)?) {
            (dir::Type::Union(pattern_set), dir::Type::Union(actual_set)) => {
                Some((pattern_set.elements, actual_set.elements))
            }
            (dir::Type::Intersection(pattern_set), dir::Type::Intersection(actual_set)) => {
                Some((pattern_set.elements, actual_set.elements))
            }
            _ => None,
        };

        // match two composite sets element by element
        if let Some((pattern_elements, actual_elements)) = sets {
            let pattern_elements =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(pattern.module_id, pattern_elements)?);
            let actual_elements =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(actual.module_id, actual_elements)?);

            return self.match_generic_type_sets(
                origin,
                parameters,
                substitution,
                pattern_elements,
                actual_elements,
            );
        }

        // match by the heads standing on both sides
        match (self.ty(pattern)?, self.ty(actual)?) {
            // match a literal actual against the primitive pattern of its domain
            (dir::Type::Primitive(primitive), dir::Type::Literal(literal))
                if literal.widens_to_primitive(primitive) =>
            {
                Ok(true)
            }
            // bind a union actual through whichever arm the pattern matches
            (_, dir::Type::Union(actual_union)) => {
                let arms = SmallVec::<[_; 4]>::from_slice(
                    self.type_ids(actual.module_id, actual_union.elements)?,
                );
                for arm in arms {
                    let mut scratch = substitution.clone();
                    if self.match_generic_type(origin, parameters, &mut scratch, pattern, arm)? {
                        *substitution = scratch;

                        return Ok(true);
                    }
                }

                Ok(false)
            }

            // match an owned pattern against a bare value-family actual through the payload
            (dir::Type::Form(form), actual_type)
                if form.form == dir::Form::Owned
                    && !matches!(actual_type, dir::Type::Form(_))
                    && self.default_ownership(origin, actual)? == Some(dir::Ownership::Owned) =>
            {
                self.match_generic_type(origin, parameters, substitution, form.value, actual)
            }

            // match fixed children under one shared constructor first
            (pattern_type, actual_type) => {
                if let Some(pairs) = self.decompose_type_pair(pattern, actual)? {
                    if self.match_generic_arguments(origin, parameters, substitution, &pairs)? {
                        return Ok(true);
                    }

                    // relate two stuck computations once reduced
                    let computes = |ty: &dir::Type| {
                        matches!(ty, dir::Type::Member(_) | dir::Type::Operation(_))
                    };
                    if !computes(&pattern_type) && !computes(&actual_type) {
                        return Ok(false);
                    }
                }
                // accept two heads that already name the same constructor
                else if pattern_type == actual_type {
                    return Ok(true);
                }

                // reduce stuck heads once and rematch, or reject a fully reduced mismatch
                let reduced_pattern = self.match_normalize(origin, pattern)?;
                let reduced_actual = self.match_normalize(origin, actual)?;
                if reduced_pattern == pattern && reduced_actual == actual {
                    return Ok(false);
                }

                self.match_generic_type(
                    origin,
                    parameters,
                    substitution,
                    reduced_pattern,
                    reduced_actual,
                )
            }
        }
    }

    /// Reduce one stuck head for a rematch.
    fn match_normalize(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let reduced = self.structurally_normalize(origin, id)?;

        // decide which heads still fold under a full normalization
        let folds = match self.ty(reduced)? {
            dir::Type::Form(_) => true,
            dir::Type::Application(instance) => matches!(
                self.language_item(instance.symbol)?,
                Some(dir::LanguageItem::AccessOf | dir::LanguageItem::WithAccess)
            ),
            _ => false,
        };

        // normalize a head that folds into a plainer value
        if folds {
            return self.normalize(origin, reduced);
        }

        Ok(reduced)
    }

    /// Match two unordered type sets wherever each pattern has one viable target.
    fn match_generic_type_sets(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        mut patterns: SmallVec<[dir::GlobalTypeId; 4]>,
        mut actuals: SmallVec<[dir::GlobalTypeId; 4]>,
    ) -> CompilerResult<bool> {
        if patterns.len() != actuals.len() {
            return Ok(false);
        }

        // commit only matches whose target is unambiguous under current bindings
        while !patterns.is_empty() {
            let mut selected = None;
            for (pattern_index, pattern) in patterns.iter().copied().enumerate() {
                let mut candidate = None;
                let mut is_ambiguous = false;

                // try this pattern against every remaining actual
                for (actual_index, actual) in actuals.iter().copied().enumerate() {
                    let mut matched = substitution.clone();
                    match self.match_generic_type(
                        origin,
                        parameters,
                        &mut matched,
                        pattern,
                        actual,
                    )? {
                        true if candidate.is_some() => is_ambiguous = true,
                        true => candidate = Some((actual_index, matched)),
                        false => {}
                    }
                }

                if !is_ambiguous && let Some((actual_index, matched)) = candidate {
                    selected = Some((pattern_index, actual_index, matched));

                    break;
                }
            }

            // consume one unambiguous pair, or reject the whole set
            let Some((pattern_index, actual_index, matched)) = selected else {
                return Ok(false);
            };

            *substitution = matched;
            patterns.remove(pattern_index);
            actuals.remove(actual_index);
        }

        Ok(true)
    }

    /// Pair two applied argument lists by kind, dropping regions when the arities differ.
    pub(in crate::sema) fn pair_application_arguments(
        &self,
        source: &[dir::GlobalTypeId],
        target: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        // pair positionally whenever both lists carry the same arity
        if source.len() == target.len() {
            return Ok(Some(
                source.iter().copied().zip(target.iter().copied()).collect(),
            ));
        }

        // collect the source's type arguments, dropping its lifetimes
        let mut source_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in source {
            if self.memory_kind(*argument)? != Some(dir::MemoryParameter::Region) {
                source_types.push(*argument);
            }
        }

        // mirror that partition over the target arguments
        let mut target_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in target {
            if self.memory_kind(*argument)? != Some(dir::MemoryParameter::Region) {
                target_types.push(*argument);
            }
        }

        // reject lists whose type arguments fail to pair one for one
        if source_types.len() != target_types.len() {
            return Ok(None);
        }

        // pair the arguments one for one
        Ok(Some(
            source_types
                .iter()
                .copied()
                .zip(target_types.iter().copied())
                .collect(),
        ))
    }

    /// Bind one generic parameter argument in a direct substitution.
    pub(in crate::sema) fn bind_generic_argument(
        &mut self,
        origin: Origin,
        substitution: &mut TypeSubstitution,
        parameter: GenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let Some(bound) = substitution.argument(parameter) else {
            substitution.bind(parameter, argument)?;

            return Ok(true);
        };

        // keep the existing bound unless it compares unequal
        let verdict = self.decide_relation(origin, Relation::Equal, bound, argument)?;

        Ok(verdict != Verdict::Fails)
    }

    /// Match fixed positional type pairs.
    fn match_generic_arguments(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<bool> {
        for (pattern, actual) in pairs.iter().copied() {
            if !self.match_generic_type(origin, parameters, substitution, pattern, actual)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Hash one substitution's bindings into a stable matching fingerprint.
fn substitution_fingerprint(substitution: &TypeSubstitution) -> u64 {
    let mut hasher = FxHasher::default();
    for binding in &substitution.bindings {
        binding.parameter.hash(&mut hasher);
        binding.argument.hash(&mut hasher);
    }

    hasher.finish()
}

/// Zip two fixed child lists into relation pairs.
fn type_pairs(
    source: SmallVec<[dir::GlobalTypeId; 4]>,
    target: SmallVec<[dir::GlobalTypeId; 4]>,
) -> Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>> {
    if source.len() != target.len() {
        return None;
    }

    Some(source.into_iter().zip(target).collect())
}
