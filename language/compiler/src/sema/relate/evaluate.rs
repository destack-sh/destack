use crate::CompilerResult;
use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, Cycle, InferSubstitution, Origin, Relation,
};
use smallvec::SmallVec;
use tspp_core::ensure_sufficient_stack;
use tspp_dir as dir;

/// The outcome of deciding one relation, keeping ambiguity apart from failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Verdict {
    /// The relation holds.
    Holds,
    /// The relation fails on closed operands.
    Fails,
    /// An open variable decided the outcome: retry once it solves.
    Ambiguous,
}

impl Verdict {
    /// Classify one decided outcome.
    pub(in crate::sema) fn decided(is_holds: bool) -> Self {
        match is_holds {
            true => Self::Holds,
            false => Self::Fails,
        }
    }

    /// Return whether the relation decidedly holds.
    pub(in crate::sema) fn holds(self) -> bool {
        self == Self::Holds
    }

    /// Keep a failure ambiguous while another undecided path may still hold.
    pub(in crate::sema) fn join_undecided(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fails, Self::Ambiguous) => Self::Ambiguous,
            _ => self,
        }
    }

    /// Join one conjunct, where failure dominates and ambiguity taints.
    pub(in crate::sema) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fails, _) | (_, Self::Fails) => Self::Fails,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            _ => Self::Holds,
        }
    }

    /// Join one disjunct, where success dominates and ambiguity taints.
    pub(in crate::sema) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Holds, _) | (_, Self::Holds) => Self::Holds,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            _ => Self::Fails,
        }
    }

    /// Join one disjunct evaluated only while this one leaves the outcome open.
    pub(in crate::sema) fn or_else(
        self,
        other: impl FnOnce() -> CompilerResult<Self>,
    ) -> CompilerResult<Self> {
        match self {
            Self::Holds => Ok(Self::Holds),
            verdict => Ok(verdict.or(other()?)),
        }
    }
}

impl CheckState<'_> {
    /// Evaluate one relation between type roots as a pure query, growing the stack.
    pub(in crate::sema) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // bring both operands to their comparison roots
        let (source, source_variable) = self.relate_root(origin, source)?;
        let (target, target_variable) = self.relate_root(origin, target)?;

        // retry an open head once it solves
        if source_variable.is_some() || target_variable.is_some() {
            return Ok(Verdict::Ambiguous);
        }

        // decide the pair in isolation
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let (related, decided) = self.decide(|state| {
            ensure_sufficient_stack(|| state.relate_pair(origin, cause, relation, source, target))
        })?;
        Ok(match related.and(decided) {
            Verdict::Holds => Verdict::Holds,
            Verdict::Ambiguous => Verdict::Ambiguous,
            Verdict::Fails => self.decide_stuck_relation(source, target)?,
        })
    }

    /// Decide one failed stuck relation, undecided while either head stays open.
    pub(in crate::sema) fn decide_stuck_relation(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let is_undecided = self.is_open_head(source)? || self.is_open_head(target)?;

        Ok(match is_undecided {
            true => Verdict::Ambiguous,
            false => Verdict::Fails,
        })
    }

    /// Return whether one head stays open: a variable, or a non-template projection over one.
    pub(in crate::sema) fn is_open_head(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        if self.root_variable(ty)?.is_some() {
            return Ok(true);
        }

        // template literal verdicts decide through matching
        let projects = match self.ty(ty)? {
            dir::Type::Member(_) => true,
            dir::Type::Operation(operation) => !matches!(
                self.type_operation(ty.module_id, operation)?,
                dir::TypeOperation::TemplateLiteral(_)
            ),
            _ => false,
        };
        if !projects {
            return Ok(false);
        }

        Ok(!self.type_variables(ty)?.is_empty())
    }

    /// Classify one relation outcome, where failure over an open head is ambiguity.
    pub(in crate::sema) fn decide_outcome(
        &mut self,
        is_holds: bool,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        if is_holds {
            return Ok(Verdict::Holds);
        }

        // retry the decision once an open head solves
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;
        if self.is_open_head(source)? || self.is_open_head(target)? {
            return Ok(Verdict::Ambiguous);
        }

        Ok(Verdict::Fails)
    }

    /// Relate one resolved pair through the structural matrix, binding the open children.
    pub(in crate::sema) fn relate_pair(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // identity mapped targets over an open variable bind it whole
        if let Some(variable) = self.reverse_mapped_variable(target)? {
            return self.constrain_type(origin, cause, relation, source, variable);
        }

        // identical roots relate under every relation
        if source == target {
            return Ok(Verdict::Holds);
        }

        // accept inference barrier targets outright
        if let Some(inner) = self.no_infer_target(target)? {
            let source_value = self.strip_form(origin, source)?;
            if matches!(
                self.ty(source_value)?,
                dir::Type::Function(_) | dir::Type::FunctionSignature(_)
            ) {
                return self.constrain_type(origin, cause, relation, source, inner);
            }

            return Ok(Verdict::Holds);
        }

        // decide a pair over open variables afresh
        if self.type_flags(source)?.has_variable() || self.type_flags(target)?.has_variable() {
            return self.evaluate_relation(origin, cause, relation, source, target);
        }

        // reuse decided relations, in flight through their cycle or memoized once closed
        let interface = self.conformance_target_symbol(target)?;
        let is_auto = match interface {
            Some(symbol) => self
                .language_item(symbol)?
                .and_then(dir::AutoInterface::from_language_item)
                .is_some(),
            None => false,
        };
        let cycle = match interface.is_some() && !is_auto {
            true => Cycle::Inductive,
            false => Cycle::Coinductive,
        };
        let key = (relation, source, target);
        if let Some(is_holds) = self.decided_relations.get(&key) {
            self.counters.relation_reuses += 1;

            return Ok(Verdict::decided(*is_holds));
        }
        if let Some(is_holds) = self.infer.relations.lookup(&key, cycle) {
            self.counters.relation_reuses += 1;

            return Ok(Verdict::decided(is_holds));
        }

        // decide the pair as an active attempt, memoizing resolved decisions
        self.counters.relation_decisions += 1;
        let attempt = self.infer.relations.enter(key);
        let decision = self.evaluate_relation(origin, cause, relation, source, target);
        match &decision {
            Ok(verdict) if *verdict != Verdict::Ambiguous => {
                // remember closed pairs alone
                let flags = self.type_flags(source)? | self.type_flags(target)?;
                let is_open = flags.has_parameter() || flags.has_this() || flags.has_variable();
                if let Some(is_holds) = self.infer.relations.finish(attempt, verdict.holds())
                    && !is_open
                {
                    self.decided_relations.insert(key, is_holds);
                }
            }
            _ => self.infer.relations.cancel(attempt),
        }

        decision
    }

    /// Return one stuck conditional's then branch with each infer binder at its constraint.
    fn substitute_infer_binders(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // capture each named binder at the constraint it declares, an open binder at unknown
        let mut captures = SmallVec::<[InferSubstitution; 2]>::new();
        for binder in self.collect_infer_binders(conditional.right)? {
            let Some(symbol) = binder.symbol else {
                continue;
            };
            let ty = match binder.constraint {
                Some(constraint) => constraint,
                None => self.intern_type(dir::Type::Unknown)?,
            };
            captures.push(InferSubstitution { symbol, ty });
        }

        self.substitute_infer_captures(Some(origin), conditional.then_type, &captures)
    }

    /// Relate the type operator heads shared by every relation.
    fn relate_heads(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        // refined targets require the base and the refined member equality
        if let Some(refined) = self.refined_head(target)? {
            let base = self.constrain_type(origin, cause, relation, source, refined.base)?;
            if base == Verdict::Fails {
                return Ok(Some(Verdict::Fails));
            }

            // project the refined key out of the source through the refined interface
            let arguments = self.intern_type_ids(&[])?;
            let projected = self.intern_member(dir::MemberType {
                owner: source,
                key: refined.key,
                arguments,
                qualifier: Some(refined.base),
            })?;
            let value =
                self.constrain_type(origin, cause, Relation::Equal, projected, refined.value)?;

            return Ok(Some(base.and(value)));
        }

        // refined sources imply their base application
        if let Some(refined) = self.refined_head(source)? {
            return self
                .constrain_type(origin, cause, relation, refined.base, target)
                .map(Some);
        }

        // leave the relation undecided while a conditional blocks on open variables
        for side in [source, target] {
            if let Some(dir::TypeOperation::Conditional(conditional)) = self.operation_head(side)?
                && !self
                    .collect_open_variables([conditional.left, conditional.right])?
                    .is_empty()
            {
                return Ok(Some(Verdict::Ambiguous));
            }
        }

        // decide a stuck narrow through its source and its narrowing target
        if let Some(dir::TypeOperation::Narrow(narrow)) = self.operation_head(source)? {
            let through_source =
                self.constrain_type(origin, cause, relation, narrow.source, target)?;
            if through_source == Verdict::Holds {
                return Ok(Some(Verdict::Holds));
            }

            if narrow.is_positive {
                let through_target =
                    self.constrain_type(origin, cause, relation, narrow.target, target)?;
                if through_target == Verdict::Holds {
                    return Ok(Some(Verdict::Holds));
                }
            }
        }

        // relate an irreducible conditional through both of its branches
        if relation.is_directed()
            && let Some(dir::TypeOperation::Conditional(conditional)) =
                self.operation_head(source)?
        {
            let then_type = self.substitute_infer_binders(origin, conditional)?;
            let then_branch = self.constrain_type(origin, cause, relation, then_type, target)?;
            if then_branch == Verdict::Fails {
                return Ok(Some(Verdict::Fails));
            }

            let else_branch =
                self.constrain_type(origin, cause, relation, conditional.else_type, target)?;

            return Ok(Some(then_branch.and(else_branch)));
        }

        // relate region terms by the region rule
        if let Some(verdict) = self.relate_region_terms(origin, cause, relation, source, target)? {
            return Ok(Some(verdict));
        }

        // relate access terms by their ladder
        if let Some(verdict) = self.relate_access_terms(relation, source, target)? {
            return Ok(Some(verdict));
        }

        // exact property keys compare by key identity
        if let (Some(source_key), Some(target_key)) = (
            self.static_key_from_type(source)?,
            self.static_key_from_type(target)?,
        ) {
            return Ok(Some(Verdict::decided(source_key == target_key)));
        }
        // decide region kind inhabitants before canonicalization erases parameter kinds
        if let Some(symbol) = self.type_symbol(target)?
            && matches!(self.language_item(symbol)?, Some(dir::LanguageItem::Region))
            && self.memory_kind(source)? == Some(dir::MemoryParameter::Region)
        {
            return Ok(Some(Verdict::Holds));
        }

        Ok(None)
    }

    /// Return whether one relation target asks interface conformance.
    pub(in crate::sema) fn is_conformance_target(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        Ok(self.conformance_target_symbol(target)?.is_some())
    }

    /// Return the interface symbol one conformance target names.
    fn conformance_target_symbol(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // read the nominal symbol the target names
        let symbol = match self.ty(target)? {
            dir::Type::Application(application) => application.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(None),
        };
        let symbol = self.resolve_symbol_alias(symbol)?;

        // read whether the symbol declares an interface
        let is_interface = matches!(
            self.definition(symbol)?.as_deref(),
            Some(dir::Definition::Interface(_))
        );

        Ok(is_interface.then_some(symbol))
    }

    /// Evaluate one relation over the shared operator heads, then its specific rule.
    fn evaluate_relation(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        if let Some(verdict) = self.relate_heads(origin, cause, relation, source, target)? {
            return Ok(verdict);
        }

        // decide equality on closed pairs before the broader relation
        let closed = !(self.type_flags(source)? | self.type_flags(target)?).has_variable();
        let equal = match relation {
            Relation::Equal => false,
            _ => closed && self.relate_equal(origin, cause, source, target)?.holds(),
        };
        if equal {
            return Ok(Verdict::Holds);
        }

        // decide the relation itself when equality left it open
        let decision = match relation {
            Relation::Equal => self.relate_equal(origin, cause, source, target)?,
            Relation::Subtype | Relation::Storable => {
                self.relate_directed(origin, cause, relation, source, target)?
            }
        };

        Ok(decision)
    }

    /// Return the open variable behind one identity mapped target.
    pub(in crate::sema) fn reverse_mapped_variable(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let dir::Type::Operation(operation) = self.ty(target)? else {
            return Ok(None);
        };
        let dir::TypeOperation::Mapped(mapped) =
            self.type_operation(target.module_id, operation)?
        else {
            return Ok(None);
        };

        // reject remaps and modifiers that reshape the source
        let is_plain = mapped.parameter.key_remap.is_none()
            && mapped.modifiers.readonly == dir::MappedTypeModifier::None
            && mapped.modifiers.optional == dir::MappedTypeModifier::None;
        if !is_plain {
            return Ok(None);
        }

        // iterate an open variable's keys
        let constraint = self.shallow_resolve(mapped.parameter.constraint)?;
        let dir::Type::Operation(constraint) = self.ty(constraint)? else {
            return Ok(None);
        };
        let dir::TypeOperation::KeyOf(keys) = self.type_operation(target.module_id, constraint)?
        else {
            return Ok(None);
        };
        let variable = self.shallow_resolve(keys.target)?;
        if !matches!(self.ty(variable)?, dir::Type::Variable(_)) {
            return Ok(None);
        }

        // project the iterated key back out of the variable
        let value = self.shallow_resolve(mapped.value)?;
        let dir::Type::Operation(value) = self.ty(value)? else {
            return Ok(None);
        };
        let dir::TypeOperation::Index(index) = self.type_operation(target.module_id, value)? else {
            return Ok(None);
        };
        let is_identity = self.shallow_resolve(index.left)? == variable
            && matches!(
                self.resolved_ty(index.index)?,
                dir::Type::Parameter(parameter) if parameter == mapped.parameter.parameter
            );

        Ok(is_identity.then_some(variable))
    }

    /// Relate every pair in one pair list.
    pub(in crate::sema) fn relate_each(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Verdict> {
        // stop at the first decided failure
        let mut verdict = Verdict::Holds;
        for (source, target) in pairs.iter().copied() {
            verdict = verdict.and(self.constrain_type(origin, cause, relation, source, target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }
}
