use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BoundSet, BoundSetId, Callee, CanonicalGoal, CheckState, GenericParameterId, Goal, Hole,
    Origin, Premise, PremiseParameter,
};
use crate::{CompilerError, CompilerResult};

/// One goal's operands in canonical form, shared across equal goals.
#[derive(Debug, Clone)]
pub(in crate::sema) struct Canonical {
    /// The canonical operands, in goal order.
    pub(in crate::sema) operands: SmallVec<[dir::GlobalTypeId; 2]>,
    /// The live root renamed into each hole, in hole order.
    pub(in crate::sema) holes: SmallVec<[dir::TypeVariableId; 4]>,
    /// The canonical rigid number per renamed parameter.
    pub(in crate::sema) renaming: FxIndexMap<GenericParameterId, dir::RigidIndex>,
    /// The live parameter type per rigid number, in rigid order.
    pub(in crate::sema) parameters: SmallVec<[dir::GlobalTypeId; 4]>,
    /// The assumptions the renamed parameters decide under.
    pub(in crate::sema) premise: Premise,
}

/// One memoized canonical form with the hole state it was folded under.
#[derive(Debug, Clone)]
pub(in crate::sema) struct CanonicalEntry {
    /// The memoized canonical form, absent for refused closed pairs.
    canonical: Option<Arc<Canonical>>,
    /// The open roots the fold numbered, invalidating the entry when one closes.
    holes: SmallVec<[dir::TypeVariableId; 4]>,
}

/// The renaming one canonicalization threads through its folds.
#[derive(Debug)]
pub(super) struct Renaming {
    /// The canonical hole per open root.
    pub(super) holes: FxIndexMap<dir::TypeVariableId, dir::HoleIndex>,
    /// The canonical rigid number per generic parameter.
    pub(super) parameters: FxIndexMap<GenericParameterId, dir::RigidIndex>,
    /// Whether answer-owned generics stay raw.
    pub(super) keeps_raw_parameters: bool,
}

impl Renaming {
    /// Create a renaming that numbers content as the folds reach it.
    pub(super) fn growing() -> Self {
        Self {
            holes: FxIndexMap::default(),
            parameters: FxIndexMap::default(),
            keeps_raw_parameters: false,
        }
    }

    /// Create a renaming seeded at one goal's numbering, growing answer holes.
    pub(super) fn seeded(canonical: &Canonical) -> Self {
        // number each of the goal's holes in its decided order
        let mut holes = FxIndexMap::default();
        for (index, root) in canonical.holes.iter().enumerate() {
            holes.insert(*root, dir::HoleIndex(index as u16));
        }

        Self {
            holes,
            parameters: canonical.renaming.clone(),
            keeps_raw_parameters: true,
        }
    }
}

impl CheckState<'_> {
    /// Canonicalize one subject's operands into a memoizable goal.
    pub(in crate::sema) fn canonicalize_goal(
        &mut self,
        origin: Origin,
        goal: Goal,
        operands: &[dir::GlobalTypeId],
        open: bool,
    ) -> CompilerResult<Option<(CanonicalGoal, Arc<Canonical>)>> {
        // canonicalize pairs through the closed memo, lists past it
        let canonical = match operands {
            [source, target] => self.canonicalize(origin, [*source, *target], open)?,
            operands => self.canonicalize_list(origin, operands, open)?,
        };
        let Some(canonical) = canonical else {
            return Ok(None);
        };
        let goal = self.canonical_goal(goal, &canonical)?;

        Ok(Some((goal, canonical)))
    }

    /// Mint one further goal over an already canonicalized form.
    pub(in crate::sema) fn canonical_goal(
        &mut self,
        goal: Goal,
        canonical: &Canonical,
    ) -> CompilerResult<CanonicalGoal> {
        let operands = self.intern_type_ids(&canonical.operands)?;

        Ok(CanonicalGoal {
            goal,
            operands,
            premise: canonical.premise,
        })
    }

    /// Canonicalize one selection goal over its callee and operands.
    pub(in crate::sema) fn selection_goal(
        &mut self,
        origin: Origin,
        callee: Callee,
        expected: Option<dir::GlobalTypeId>,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(CanonicalGoal, Arc<Canonical>)>> {
        // lead the operand list with the expectation when one exists
        let mut canonicalized = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        canonicalized.extend(expected);
        canonicalized.extend_from_slice(operands);
        let subject = Goal::Selection {
            callee,
            expected: expected.is_some(),
        };

        self.canonicalize_goal(origin, subject, &canonicalized, true)
    }

    /// Canonicalize one goal's operands so equal goals look equal.
    pub(in crate::sema) fn canonicalize(
        &mut self,
        origin: Origin,
        operands: [dir::GlobalTypeId; 2],
        open: bool,
    ) -> CompilerResult<Option<Arc<Canonical>>> {
        let [source, target] = operands;
        let source = self.shallow_resolve(source)?;
        let target = self.shallow_resolve(target)?;
        let flags = self.type_flags(source)? | self.type_flags(target)?;
        let resolved = [source, target];
        let is_closed = !flags.has_variable();

        // refuse an open pair the goal cannot memoize
        if !is_closed && !open {
            return Ok(None);
        }

        // serve the memo while every hole still carries the folded state
        let scope = if flags.has_parameter() || flags.has_this() {
            self.assuming_scope(origin)?
        } else {
            None
        };
        let key = (resolved, scope);
        if let Some(entry) = self.canonical_entries.get(&key)
            && self.is_entry_live(entry)?
        {
            return Ok(entry.canonical.clone());
        }

        let (canonical, cacheable) = self.canonicalize_operands(origin, &resolved)?;
        let canonical = canonical.map(Arc::new);

        // memoize closed forms freely and open forms with their numbered roots
        if cacheable && (is_closed || canonical.is_some()) {
            let holes = match &canonical {
                Some(canonical) => canonical.holes.clone(),
                None => SmallVec::new(),
            };
            self.canonical_entries.insert(
                key,
                CanonicalEntry {
                    canonical: canonical.clone(),
                    holes,
                },
            );
        }

        Ok(canonical)
    }

    /// Return whether one memoized canonical form still matches its holes' state.
    fn is_entry_live(&self, entry: &CanonicalEntry) -> CompilerResult<bool> {
        for root in &entry.holes {
            if !self.infer.variable(*root)?.state.is_open() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Canonicalize one operand list so equal goals look equal.
    pub(in crate::sema) fn canonicalize_list(
        &mut self,
        origin: Origin,
        operands: &[dir::GlobalTypeId],
        open: bool,
    ) -> CompilerResult<Option<Arc<Canonical>>> {
        let mut resolved = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut flags = dir::TypeFlags::EMPTY;
        for operand in operands {
            let operand = self.shallow_resolve(*operand)?;
            flags |= self.type_flags(operand)?;
            resolved.push(operand);
        }

        // refuse an open list the goal cannot memoize
        if !open && flags.has_variable() {
            return Ok(None);
        }
        let (canonical, _) = self.canonicalize_operands(origin, &resolved)?;

        Ok(canonical.map(Arc::new))
    }

    /// Canonicalize one goal's operands past the memo.
    fn canonicalize_operands(
        &mut self,
        origin: Origin,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<(Option<Canonical>, bool)> {
        // rename both operands under one numbering
        let mut renaming = Renaming::growing();
        let mut canonical_operands = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        let mut flags = dir::TypeFlags::EMPTY;
        for operand in operands {
            let Some(operand) = self.canonical_operand(*operand, &mut renaming)? else {
                return Ok((None, true));
            };
            flags |= self.type_flags(operand)?;
            canonical_operands.push(operand);
        }

        // number the holes bare, keeping kinds and roles while live bounds stay out
        let holes = self.hole_contents(&mut renaming, 0)?;
        flags |= if renaming.parameters.is_empty() {
            dir::TypeFlags::EMPTY
        } else {
            dir::TypeFlags::HAS_PARAMETER
        };

        // decide closed operands freely
        let premise = if !flags.has_parameter() && !flags.has_this() && holes.is_empty() {
            Premise::Free
        }
        // refuse assumed operands while declaring, whose templates are still forming
        else if self.is_declaring() {
            return Ok((None, false));
        }
        // scope operands that mention this, which carry no hole content
        else if flags.has_this() {
            Premise::Scope(self.assuming_scope(origin)?)
        }
        // intern the assumed parameter bounds with the numbered holes' shapes
        else {
            let parameters = if renaming.parameters.is_empty() {
                Some(Premise::Bounds(BoundSetId(
                    self.bound_sets.insert_full(BoundSet::default()).0 as u32,
                )))
            } else {
                self.intern_premise(origin, &mut renaming)?
            };
            match parameters {
                // extend the interned bound set with the numbered hole shapes
                Some(Premise::Bounds(index)) => {
                    let mut content = self
                        .bound_sets
                        .get_index(index.0 as usize)
                        .cloned()
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("premise bound set {} is not interned", index.0),
                        })?;
                    content.holes = holes;
                    let (combined, _) = self.bound_sets.insert_full(content);

                    Premise::Bounds(BoundSetId(combined as u32))
                }
                // fall back to the assuming template
                Some(Premise::Free | Premise::Scope(_)) | None => {
                    Premise::Scope(self.assuming_scope(origin)?)
                }
            }
        };

        // read the live type behind each numbered parameter
        let mut parameters = SmallVec::new();
        for parameter in renaming.parameters.keys() {
            parameters.push(self.generic_parameter_type(*parameter)?);
        }

        Ok((
            Some(Canonical {
                operands: canonical_operands,
                holes: renaming.holes.keys().copied().collect(),
                renaming: renaming.parameters,
                parameters,
                premise,
            }),
            true,
        ))
    }

    /// Canonicalize one operand, numbering its open roots and parameters.
    pub(super) fn canonical_operand(
        &mut self,
        ty: dir::GlobalTypeId,
        renaming: &mut Renaming,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.shallow_resolve(ty)?;
        let flags = self.type_flags(ty)?;
        if !flags.has_variable() && !flags.has_parameter() {
            return Ok(Some(ty));
        }

        // number every free open root and named parameter in one scan
        self.number_operand_content(ty, renaming)?;

        // rename every numbered root and parameter in one fold
        let canonical =
            self.canonicalize_type(self.module_id, ty, &renaming.holes, &renaming.parameters)?;

        Ok(Some(canonical))
    }

    /// Number one operand's open roots and named parameters by first appearance.
    fn number_operand_content(
        &self,
        id: dir::GlobalTypeId,
        renaming: &mut Renaming,
    ) -> CompilerResult<()> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(&[id]);
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            // descend only where numbered content can appear
            let flags = self.type_flags(id)?;
            let numbers_parameters = flags.has_parameter() && !renaming.keeps_raw_parameters;
            if !flags.has_variable() && !numbers_parameters {
                continue;
            }

            match self.ty_raw(id)? {
                // number each free open root, following solved roots inward
                dir::Type::Variable(variable) => match self.infer.solution(variable)? {
                    Some(solution) => pending.push(solution),
                    None => {
                        let root = self.infer.alias_root(variable)?;
                        if renaming.holes.contains_key(&root) {
                            continue;
                        }

                        let next = dir::HoleIndex(renaming.holes.len() as u16);
                        renaming.holes.insert(root, next);
                    }
                },
                // number each named generic parameter
                dir::Type::Parameter(parameter) if !renaming.keeps_raw_parameters => {
                    if renaming.parameters.contains_key(&parameter) {
                        continue;
                    }

                    let next = dir::RigidIndex(renaming.parameters.len() as u16);
                    renaming.parameters.insert(parameter, next);
                }
                // keep answer-owned placeholders and raw parameters verbatim
                dir::Type::Parameter(_) | dir::Type::Erased(_) => {}
                // descend into every other type's children
                ty => self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?,
            }
        }

        Ok(())
    }

    /// Return whether one type mentions any numbered parameter.
    fn has_numbered_parameter(
        &self,
        id: dir::GlobalTypeId,
        numbered: &FxIndexMap<GenericParameterId, dir::RigidIndex>,
    ) -> CompilerResult<bool> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(&[id]);
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) || !self.type_flags(id)?.has_parameter() {
                continue;
            }

            match self.ty(id)? {
                dir::Type::Parameter(parameter) if numbered.contains_key(&parameter) => {
                    return Ok(true);
                }
                // keep answer-owned placeholders and foreign parameters verbatim
                dir::Type::Parameter(_) | dir::Type::Erased(_) => {}
                // descend into every other type's children
                ty => self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?,
            }
        }

        Ok(false)
    }

    /// Intern the bound content one goal's renamed parameters assume.
    fn intern_premise(
        &mut self,
        origin: Origin,
        renaming: &mut Renaming,
    ) -> CompilerResult<Option<Premise>> {
        // serve the memo over the scope and numbered parameters
        let scope = self.assuming_scope(origin)?;
        let initial: SmallVec<[GenericParameterId; 4]> =
            renaming.parameters.keys().copied().collect();
        if let Some((closed, premise)) = self.premises.get(&(scope, initial.clone())) {
            for parameter in closed.clone() {
                let next = dir::RigidIndex(renaming.parameters.len() as u16);
                renaming.parameters.entry(parameter).or_insert(next);
            }

            return Ok(*premise);
        }

        let predicates = self.assumed_predicates(origin)?;
        let mut content = BoundSet::default();
        let mut included: SmallVec<[bool; 8]> = SmallVec::from_elem(false, predicates.len());
        let mut next = 0;
        while next < renaming.parameters.len() {
            // pull each numbered parameter's declared content once
            let Some((parameter, _)) = renaming.parameters.get_index(next) else {
                return Err(CompilerError::Internal {
                    message: format!("renamed parameter {next} left the numbering"),
                });
            };
            let parameter = *parameter;
            next += 1;
            let Some(binding) = self.generic_parameter(parameter).cloned() else {
                return Err(CompilerError::Internal {
                    message: format!("renamed parameter {parameter:?} has no declared binding"),
                });
            };

            // canonicalize the parameter's declared constraint and default
            let constraint = match binding.constraint {
                Some(constraint) => match self.assumed_operand(constraint, renaming)? {
                    Some(constraint) => Some(constraint),
                    None => return Ok(None),
                },
                None => None,
            };
            let default = match binding.default {
                Some(default) => match self.assumed_operand(default, renaming)? {
                    Some(default) => Some(default),
                    None => return Ok(None),
                },
                None => None,
            };
            content.parameters.push(PremiseParameter {
                kind: binding.kind,
                is_variadic: binding.is_variadic,
                constraint,
                default,
            });

            // pull each assumed predicate mentioning a numbered parameter
            for (index, predicate) in predicates.iter().enumerate() {
                if included[index] {
                    continue;
                }
                let mentions = self.has_numbered_parameter(predicate.left, &renaming.parameters)?
                    || self.has_numbered_parameter(predicate.right, &renaming.parameters)?;
                if !mentions {
                    continue;
                }
                included[index] = true;
                let Some(left) = self.assumed_operand(predicate.left, renaming)? else {
                    return Ok(None);
                };
                let Some(right) = self.assumed_operand(predicate.right, renaming)? else {
                    return Ok(None);
                };
                content.predicates.push((predicate.relation, left, right));
            }
        }

        // intern the content with the parameters the closure added
        let (index, _) = self.bound_sets.insert_full(content);
        let premise = Premise::Bounds(BoundSetId(index as u32));
        let closed: SmallVec<[GenericParameterId; 4]> = renaming
            .parameters
            .keys()
            .copied()
            .filter(|parameter| !initial.contains(parameter))
            .collect();
        self.premises
            .insert((scope, initial), (closed, Some(premise)));

        Ok(Some(premise))
    }

    /// Canonicalize one closed assumed bound type.
    fn assumed_operand(
        &mut self,
        ty: dir::GlobalTypeId,
        renaming: &mut Renaming,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let flags = self.type_flags(ty)?;
        if flags.has_variable() || flags.has_this() {
            return Ok(None);
        }

        self.canonical_operand(ty, renaming)
    }

    /// Return the template one decision assumes, or refuse the memo.
    pub(in crate::sema) fn decision_scope(
        &mut self,
        origin: Origin,
        flags: dir::TypeFlags,
    ) -> CompilerResult<Option<Option<dir::GlobalGenericTemplateId>>> {
        if !flags.has_parameter() && !flags.has_this() {
            return Ok(Some(None));
        }
        if !self.is_checking() {
            return Ok(None);
        }

        Ok(Some(self.assuming_scope(origin)?))
    }

    /// Fold each numbered hole's carried content, from one starting number.
    pub(super) fn hole_contents(
        &mut self,
        renaming: &mut Renaming,
        from: usize,
    ) -> CompilerResult<SmallVec<[Hole; 2]>> {
        let mut holes = SmallVec::new();
        let mut next = from;
        while next < renaming.holes.len() {
            // pull each numbered hole's carried shape once
            let Some((root, _)) = renaming.holes.get_index(next) else {
                return Err(CompilerError::Internal {
                    message: format!("renamed hole {next} left the numbering"),
                });
            };
            let root = *root;
            next += 1;

            holes.push(Hole {
                kind: self.infer.variable(root)?.kind,
                role: self.infer.variable_role(root)?,
            });
        }

        Ok(holes)
    }
}
