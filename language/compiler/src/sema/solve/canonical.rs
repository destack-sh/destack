use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Ask, BoundSet, BoundSetId, BoundSide, Callee, CheckState, GenericParameterId, Hole, Origin,
    Premise, PremiseParameter, Question, TypeBound, VariableRole,
};
use crate::{CompilerError, CompilerResult};

/// One question's operands in canonical form, shared across equal asks.
#[derive(Debug, Clone)]
pub(in crate::sema) struct Canonical {
    /// The canonical operands, in ask order.
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

/// The renaming one canonicalization threads through its folds.
#[derive(Debug)]
pub(super) struct Renaming {
    /// The canonical hole per open root.
    pub(super) holes: FxIndexMap<dir::TypeVariableId, dir::HoleIndex>,
    /// The canonical rigid number per generic parameter.
    pub(super) parameters: FxIndexMap<GenericParameterId, dir::RigidIndex>,
    /// Whether new roots and parameters may still join the numbering.
    pub(super) may_grow: bool,
    /// Whether answer-owned generics stay raw.
    pub(super) keeps_raw_parameters: bool,
}

impl Renaming {
    /// Create a renaming that numbers content as the folds reach it.
    pub(super) fn growing() -> Self {
        Self {
            holes: FxIndexMap::default(),
            parameters: FxIndexMap::default(),
            may_grow: true,
            keeps_raw_parameters: false,
        }
    }

    /// Create a renaming seeded at one ask's numbering, growing answer holes.
    pub(super) fn seeded(canonical: &Canonical) -> Self {
        // number each of the ask's holes in its decided order
        let mut holes = FxIndexMap::default();
        for (index, root) in canonical.holes.iter().enumerate() {
            holes.insert(*root, dir::HoleIndex(index as u16));
        }

        Self {
            holes,
            parameters: canonical.renaming.clone(),
            may_grow: true,
            keeps_raw_parameters: true,
        }
    }
}

impl CheckState<'_> {
    /// Canonicalize one subject's operands into a memoizable question.
    pub(in crate::sema) fn ask(
        &mut self,
        origin: Origin,
        ask: Ask,
        operands: &[dir::GlobalTypeId],
        open: bool,
    ) -> CompilerResult<Option<(Question, Arc<Canonical>)>> {
        // canonicalize pairs through the settled memo, lists past it
        let canonical = match operands {
            [source, target] => self.canonicalize(origin, [*source, *target], open)?,
            operands => self.canonicalize_list(origin, operands, open)?,
        };
        let Some(canonical) = canonical else {
            return Ok(None);
        };
        let question = self.question(ask, &canonical)?;

        Ok(Some((question, canonical)))
    }

    /// Mint one further question over an already canonicalized form.
    pub(in crate::sema) fn question(
        &mut self,
        ask: Ask,
        canonical: &Canonical,
    ) -> CompilerResult<Question> {
        let operands = self.intern_type_ids(&canonical.operands)?;

        Ok(Question {
            ask,
            operands,
            premise: canonical.premise,
        })
    }

    /// Canonicalize one selection ask over its callee and operands.
    pub(in crate::sema) fn selection_question(
        &mut self,
        origin: Origin,
        callee: Callee,
        expected: Option<dir::GlobalTypeId>,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(Question, Arc<Canonical>)>> {
        // lead the operand list with the expectation when one exists
        let mut asked = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        asked.extend(expected);
        asked.extend_from_slice(operands);
        let subject = Ask::Selection {
            callee,
            expected: expected.is_some(),
        };

        self.ask(origin, subject, &asked, true)
    }

    /// Canonicalize one question's operands so equal questions look equal.
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
        let is_settled = !flags.has_variable();

        // refuse an open pair the ask cannot memoize
        if !is_settled && !open {
            return Ok(None);
        }

        // serve the memo for settled pairs
        let scope = if is_settled && (flags.has_parameter() || flags.has_this()) {
            self.assuming_scope(origin)?
        } else {
            None
        };
        let key = (resolved, scope);
        if is_settled && let Some(cached) = self.canonicals.get(&key) {
            return Ok(cached.clone());
        }

        let (canonical, cacheable) = self.canonicalize_operands(origin, &resolved)?;
        let canonical = canonical.map(Arc::new);
        if is_settled && cacheable {
            self.canonicals.insert(key, canonical.clone());
        }

        Ok(canonical)
    }

    /// Canonicalize one operand list so equal questions look equal.
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

        // refuse an open list the ask cannot memoize
        if !open && flags.has_variable() {
            return Ok(None);
        }
        let (canonical, _) = self.canonicalize_operands(origin, &resolved)?;

        Ok(canonical.map(Arc::new))
    }

    /// Canonicalize one question's operands past the memo.
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

        // fold the numbered holes' carried bound content into the premise
        let Some(holes) = self.hole_contents(&mut renaming, 0, true)? else {
            return Ok((None, true));
        };
        let is_bounded = holes.iter().any(Hole::is_bound);
        flags |= if renaming.parameters.is_empty() {
            dir::TypeFlags::EMPTY
        } else {
            dir::TypeFlags::HAS_PARAMETER
        };

        // decide settled operands freely
        let premise = if !flags.has_parameter() && !flags.has_this() && !is_bounded {
            Premise::Free
        }
        // refuse assumed operands while declaring, whose templates are still forming
        else if self.is_declaration() {
            return Ok((None, false));
        }
        // scope operands that mention this, which carry no hole content
        else if flags.has_this() {
            if is_bounded {
                return Ok((None, true));
            }

            Premise::Scope(self.assuming_scope(origin)?)
        }
        // intern the bounds the renamed parameters assume
        else {
            let parameters = if renaming.parameters.is_empty() {
                Some(Premise::Bounds(BoundSetId(
                    self.bound_sets.insert_full(BoundSet::default()).0 as u32,
                )))
            } else {
                self.intern_premise(origin, &mut renaming)?
            };
            match parameters {
                // extend the interned bound set with the numbered hole content
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
                // refuse a bounded ask no interned premise can carry
                Some(Premise::Free | Premise::Scope(_)) | None if is_bounded => {
                    return Ok((None, true));
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

        // number each free open root
        if flags.has_variable() {
            for variable in self.type_variables(ty)? {
                let root = self.infer.alias_root(variable)?;
                if renaming.holes.contains_key(&root) {
                    continue;
                }
                if !renaming.may_grow {
                    return Ok(None);
                }

                let next = dir::HoleIndex(renaming.holes.len() as u16);
                renaming.holes.insert(root, next);
            }
        }

        // number each named generic parameter
        if flags.has_parameter() && !renaming.keeps_raw_parameters {
            let Some(parameters) = self.mentioned_parameters(ty)? else {
                return Ok(None);
            };
            for parameter in parameters {
                if renaming.parameters.contains_key(&parameter) {
                    continue;
                }
                if !renaming.may_grow {
                    return Ok(None);
                }

                let next = dir::RigidIndex(renaming.parameters.len() as u16);
                renaming.parameters.insert(parameter, next);
            }
        }

        // rename every numbered root and parameter in one fold
        let canonical =
            self.canonicalize_type(self.module_id, ty, &renaming.holes, &renaming.parameters)?;

        Ok(Some(canonical))
    }

    /// Collect the named parameters one type mentions.
    fn mentioned_parameters(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[GenericParameterId; 4]>>> {
        let mut parameters = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(&[id]);
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) || !self.type_flags(id)?.has_parameter() {
                continue;
            }

            let ty = self.ty(id)?;
            match ty {
                // collect each named parameter once
                dir::Type::Parameter(parameter) => {
                    if !parameters.contains(&parameter) {
                        parameters.push(parameter);
                    }
                }
                // refuse a type carrying an answer-owned generic
                dir::Type::Erased(_) => return Ok(None),
                // descend into every other type's children
                _ => self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?,
            }
        }

        Ok(Some(parameters))
    }

    /// Intern the bound content one question's renamed parameters assume.
    fn intern_premise(
        &mut self,
        origin: Origin,
        renaming: &mut Renaming,
    ) -> CompilerResult<Option<Premise>> {
        // serve the memo over the scope and numbered parameters
        let scope = self.assuming_scope(origin)?;
        let initial: SmallVec<[GenericParameterId; 4]> =
            renaming.parameters.keys().copied().collect();
        if let Some((closed, premise)) = self.premise_contents.get(&(scope, initial.clone())) {
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
            let Some(binding) = self.generic_parameter(parameter).copied() else {
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
                let Some(left) = self.mentioned_parameters(predicate.left)? else {
                    return Ok(None);
                };
                let Some(right) = self.mentioned_parameters(predicate.right)? else {
                    return Ok(None);
                };
                let mentions = left
                    .iter()
                    .chain(right.iter())
                    .any(|parameter| renaming.parameters.contains_key(parameter));
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
        self.premise_contents
            .insert((scope, initial), (closed, Some(premise)));

        Ok(Some(premise))
    }

    /// Canonicalize one settled assumed bound type.
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
        bounds: bool,
    ) -> CompilerResult<Option<SmallVec<[Hole; 2]>>> {
        let mut holes = SmallVec::new();
        let mut next = from;
        while next < renaming.holes.len() {
            // pull each numbered hole's carried content once
            let Some((root, _)) = renaming.holes.get_index(next) else {
                return Err(CompilerError::Internal {
                    message: format!("renamed hole {next} left the numbering"),
                });
            };
            let root = *root;
            next += 1;

            // keep memory roots bare, refusing unsettled constraint ids
            let role = self.infer.variable_role(root)?;
            let is_bare = !bounds
                || match role {
                    // memory content re-derives per site under verify
                    VariableRole::Memory { constraint, .. } => {
                        if let Some(constraint) = constraint {
                            let flags = self.type_flags(constraint)?;
                            if flags.has_variable() || flags.has_parameter() || flags.has_this() {
                                return Ok(None);
                            }
                        }

                        true
                    }
                    // memory parameters carry their bounds at their site
                    VariableRole::Instantiation { parameter } => self
                        .generic_parameter(parameter)
                        .is_some_and(|binding| binding.memory_parameter().is_some()),
                    // every other root carries its live bounds
                    _ => false,
                };
            if is_bare {
                holes.push(Hole {
                    lower: SmallVec::new(),
                    upper: SmallVec::new(),
                    default: None,
                    kind: self.infer.variable(root)?.kind,
                    role,
                });

                continue;
            }

            // fold each side's live bounds before renaming reaches more holes
            let declared = self.infer.variables.variable_default(root);
            let mut sides = [SmallVec::new(), SmallVec::new()];
            for (side, folded) in [BoundSide::Lower, BoundSide::Upper]
                .into_iter()
                .zip(&mut sides)
            {
                let bounds: SmallVec<[TypeBound; 2]> =
                    self.infer.variables.side_bounds(root, side)?.collect();
                for bound in bounds {
                    let Some(ty) = self.canonical_operand(bound.ty, renaming)? else {
                        return Ok(None);
                    };
                    folded.push((bound.relation, ty));
                }
            }
            let [lower, upper] = sides;

            // fold the declared default the root solves toward
            let default = match declared {
                Some(default) => match self.canonical_operand(default, renaming)? {
                    Some(default) => Some(default),
                    None => return Ok(None),
                },
                None => None,
            };

            holes.push(Hole {
                lower,
                upper,
                default,
                kind: self.infer.variable(root)?.kind,
                role,
            });
        }

        Ok(Some(holes))
    }
}
