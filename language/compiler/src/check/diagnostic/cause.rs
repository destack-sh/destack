use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::check::{Answer, Cause, CauseId, CauseKind, CheckState, Origin, Relation};
use crate::{CompilerResult, DiagnosticAnchor};

/// The blamed origin of one failed closed judgment.
pub(in crate::check) enum Blame {
    /// One side reduces to a plainer type before mismatching.
    Reduced {
        /// The reduction notes, one per changed side.
        notes: Vec<String>,
    },
    /// One structural slot beneath the judgment pair mismatches.
    Slot {
        /// Slot descriptions from the leaf outward.
        path: Vec<String>,
        /// The mismatched leaf source type.
        source: dir::GlobalTypeId,
        /// The mismatched leaf target type.
        target: dir::GlobalTypeId,
    },
}

/// One structural descent state while blaming a failed pair.
struct BlameLeaf {
    /// Slot descriptions from the leaf outward.
    path: Vec<String>,
    /// The mismatched leaf source type.
    source: dir::GlobalTypeId,
    /// The mismatched leaf target type.
    target: dir::GlobalTypeId,
    /// Whether the walk entered any slot.
    descended: bool,
}

impl CheckState<'_> {
    /// Decorate one failed judgment with its cause chain.
    pub(in crate::check) fn explain_cause<T>(
        &self,
        mut diagnostic: DiagnosticBuilder<T>,
        cause: CauseId,
        primary: &DiagnosticAnchor,
        blame: Option<&Blame>,
    ) -> CompilerResult<DiagnosticBuilder<T>> {
        let chain = self.cause_chain(cause);
        let module = self.cause_origin(cause).module();

        // describe the slot path into the mismatch, blamed leaf first
        let mut slots: Vec<String> = match blame {
            Some(Blame::Slot { path, .. }) => path.clone(),
            _ => Vec::new(),
        };
        slots.extend(
            chain
                .iter()
                .take_while(|cause| cause.kind.is_slot())
                .filter_map(|cause| self.describe_slot(cause.kind)),
        );
        let note = match (blame, slots.is_empty()) {
            // a blamed leaf names the exact mismatched types
            (Some(Blame::Slot { source, target, .. }), false) => Some(format!(
                "the mismatch is in {}: expected '{}', found '{}'",
                join_path(&slots),
                self.format_type_at(module, *target),
                self.format_type_at(module, *source),
            )),
            (Some(Blame::Slot { source, target, .. }), true) => Some(format!(
                "expected '{}', found '{}'",
                self.format_type_at(module, *target),
                self.format_type_at(module, *source),
            )),
            (_, false) => Some(format!("the mismatch is in {}", join_path(&slots))),
            (_, true) => None,
        };
        if let Some(note) = note {
            diagnostic = diagnostic.note(note);
        }
        if let Some(Blame::Reduced { notes }) = blame {
            for reduction in notes {
                diagnostic = diagnostic.note(reduction.clone());
            }
        }

        // point at the syntax that demanded the judgment
        if let Some(root) = chain.last()
            && let Some((anchor, message)) = self.root_label(root.kind)?
            && anchor != *primary
        {
            diagnostic = diagnostic.label(anchor, message);
        }

        Ok(diagnostic)
    }

    /// Blame the structural leaf beneath one failed closed relation.
    pub(in crate::check) fn blame_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Blame>> {
        // open pairs blame through their judgment causes instead
        if !self.type_variables(source)?.is_empty() || !self.type_variables(target)?.is_empty() {
            return Ok(None);
        }

        let leaf = self.blame_leaf(origin, relation, source, target, 0)?;

        // a descent names the mismatched slot beneath the pair
        if leaf.descended {
            return Ok(Some(Blame::Slot {
                path: leaf.path,
                source: leaf.source,
                target: leaf.target,
            }));
        }

        // an in-place reduction reveals what the written types mean;
        //  compare the module-relative displays the primary message uses
        let module = origin.module();
        let mut notes = Vec::new();
        for (written, reduced) in [(source, leaf.source), (target, leaf.target)] {
            let written = self.format_type_at(module, written);
            let reduced = self.format_type_at(module, reduced);
            if written != reduced {
                notes.push(format!("'{written}' reduces to '{reduced}'"));
            }
        }
        if notes.is_empty() {
            return Ok(None);
        }

        Ok(Some(Blame::Reduced { notes }))
    }

    /// Descend one failed closed pair to its mismatched leaf.
    fn blame_leaf(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        depth: u32,
    ) -> CompilerResult<BlameLeaf> {
        let leaf = BlameLeaf {
            path: Vec::new(),
            source,
            target,
            descended: false,
        };
        if depth >= 16 {
            return Ok(leaf);
        }
        let Answer::Ready(source) = self.reduce_type(origin, source)? else {
            return Ok(leaf);
        };
        let Answer::Ready(target) = self.reduce_type(origin, target)? else {
            return Ok(leaf);
        };

        // descend into the first slot whose relation fails
        for (slot, child_relation, child_source, child_target) in
            self.blame_pairs(relation, source, target)?
        {
            match self.decide_relation(origin, child_relation, child_source, child_target)? {
                Answer::Ready(false) => {}
                _ => continue,
            }
            let mut blame = self.blame_leaf(
                origin,
                child_relation,
                child_source,
                child_target,
                depth + 1,
            )?;
            blame.path.extend(slot);
            blame.descended = true;

            return Ok(blame);
        }

        Ok(BlameLeaf {
            path: Vec::new(),
            source,
            target,
            descended: false,
        })
    }

    /// Decompose one closed pair into described slot relations.
    fn blame_pairs(
        &mut self,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<
        Vec<(
            Option<String>,
            Relation,
            dir::GlobalTypeId,
            dir::GlobalTypeId,
        )>,
    > {
        let mut pairs = Vec::new();
        match (self.ty(source)?, self.ty(target)?) {
            // shapes blame matching fields under their storage relations
            (dir::Type::Shape(source_shape), dir::Type::Shape(target_shape)) => {
                let source_fields = self
                    .shape_fields(source.module_id, source_shape.fields)?
                    .to_vec();
                let target_fields = self
                    .shape_fields(target.module_id, target_shape.fields)?
                    .to_vec();
                for target_field in target_fields {
                    let source_field = source_fields
                        .iter()
                        .find(|field| field.key == target_field.key);
                    let Some(source_field) = source_field else {
                        continue;
                    };
                    let field_relation =
                        if matches!(relation, Relation::Assignable | Relation::Widens) {
                            match self.shape_field_relation(source_field, &target_field) {
                                Some(field_relation) => field_relation,
                                None => continue,
                            }
                        } else {
                            relation
                        };
                    pairs.push((
                        self.describe_slot(CauseKind::Field {
                            key: target_field.key,
                        }),
                        field_relation,
                        source_field.ty,
                        target_field.ty,
                    ));
                }
            }

            // tuples blame elements in position
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple))
                if source_tuple.form == target_tuple.form
                    && source_tuple.elements.len() == target_tuple.elements.len() =>
            {
                let source_elements = self
                    .tuple_elements(source.module_id, source_tuple.elements)?
                    .to_vec();
                let target_elements = self
                    .tuple_elements(target.module_id, target_tuple.elements)?
                    .to_vec();
                for (index, (source_element, target_element)) in source_elements
                    .iter()
                    .zip(target_elements.iter())
                    .enumerate()
                {
                    pairs.push((
                        self.describe_slot(CauseKind::Element {
                            index: index as u32,
                        }),
                        relation.interior(),
                        source_element.ty,
                        target_element.ty,
                    ));
                }
            }

            // signatures blame parameters contravariantly, returns covariantly
            (
                dir::Type::FunctionSignature(source_function),
                dir::Type::FunctionSignature(target_function),
            ) => {
                let source_function = self.type_signature(source.module_id, source_function)?;
                let target_function = self.type_signature(target.module_id, target_function)?;
                let source_parameters = self
                    .signature_parameters(source.module_id, source_function.parameters)?
                    .to_vec();
                let target_parameters = self
                    .signature_parameters(target.module_id, target_function.parameters)?
                    .to_vec();
                let shared = source_parameters.len().min(target_parameters.len());
                for (index, (source_parameter, target_parameter)) in source_parameters[..shared]
                    .iter()
                    .zip(target_parameters[..shared].iter())
                    .enumerate()
                {
                    pairs.push((
                        self.describe_slot(CauseKind::Parameter {
                            index: index as u32,
                        }),
                        relation.interior(),
                        target_parameter.ty,
                        source_parameter.ty,
                    ));
                }
                if let (Some(source_return), Some(target_return)) =
                    (source_function.return_type, target_function.return_type)
                {
                    pairs.push((
                        self.describe_slot(CauseKind::ReturnSlot),
                        relation.interior(),
                        source_return,
                        target_return,
                    ));
                }
            }

            // mutable collections alias their elements and stay invariant
            (dir::Type::Array(source_array), dir::Type::Array(target_array)) => {
                pairs.push((
                    Some("the element type".to_string()),
                    Relation::Equal,
                    source_array.element,
                    target_array.element,
                ));
            }
            (dir::Type::Slice(source_slice), dir::Type::Slice(target_slice)) => {
                pairs.push((
                    Some("the element type".to_string()),
                    Relation::Equal,
                    source_slice.element,
                    target_slice.element,
                ));
            }
            (dir::Type::FixedArray(source_array), dir::Type::FixedArray(target_array)) => {
                pairs.push((
                    Some("the element type".to_string()),
                    relation.interior(),
                    source_array.element,
                    target_array.element,
                ));
                pairs.push((
                    Some("the length".to_string()),
                    Relation::Equal,
                    source_array.count,
                    target_array.count,
                ));
            }

            // memory forms blame their payloads through transparent handles
            (dir::Type::Form(source_form), dir::Type::Form(target_form))
                if source_form.form.same_constructor(&target_form.form) =>
            {
                pairs.push((None, relation, source_form.value, target_form.value));
            }

            // union sources blame the first element that misses the target
            (dir::Type::Union(elements), _) if relation == Relation::Assignable => {
                let elements = self.type_ids(source.module_id, elements.elements)?.to_vec();
                for element in elements {
                    pairs.push((None, relation, element, target));
                }
            }

            // same-symbol applications blame arguments by variance
            (dir::Type::Instance(source_instance), dir::Type::Instance(target_instance))
                if source_instance.symbol == target_instance.symbol
                    && source_instance.arguments.len() == target_instance.arguments.len() =>
            {
                let symbol = source_instance.symbol;
                let source_arguments = self
                    .type_ids(source.module_id, source_instance.arguments)?
                    .to_vec();
                let target_arguments = self
                    .type_ids(target.module_id, target_instance.arguments)?
                    .to_vec();
                let context = self.default_symbol_context(symbol);
                let edge = self.instance_argument_edge(symbol, Relation::Widens);
                for (index, (source_argument, target_argument)) in source_arguments
                    .iter()
                    .zip(target_arguments.iter())
                    .enumerate()
                {
                    let variance = self.argument_variance(symbol, index, context)?;
                    let Some((argument_relation, order)) = variance.argument_relation(edge) else {
                        continue;
                    };
                    let (argument_source, argument_target) =
                        order.orient(*source_argument, *target_argument);
                    pairs.push((
                        self.describe_slot(CauseKind::TypeArgument {
                            symbol,
                            index: index as u32,
                            variance,
                        }),
                        argument_relation,
                        argument_source,
                        argument_target,
                    ));
                }
            }

            _ => {}
        }

        Ok(pairs)
    }

    /// Collect one cause chain from the judgment to its root.
    fn cause_chain(&self, cause: CauseId) -> Vec<Cause> {
        let mut chain = Vec::new();
        let mut current = Some(cause);
        while let Some(id) = current {
            let cause = self.solver.cause(id);
            chain.push(cause);
            current = cause.parent;
        }

        chain
    }

    /// Describe one slot the judgment descended into.
    fn describe_slot(&self, kind: CauseKind) -> Option<String> {
        let description = match kind {
            CauseKind::Field { key } => format!("field '{}'", self.format_static_key(&key)),
            CauseKind::Element { index } => format!("element {index}"),
            CauseKind::Parameter { index } => format!("parameter {index}"),
            CauseKind::ReturnSlot => "the return type".to_string(),
            CauseKind::TypeArgument { symbol, index, .. } => {
                format!("type argument {index} of '{}'", self.format_symbol(symbol))
            }
            CauseKind::Payload => "the payload".to_string(),
            _ => return None,
        };

        Some(description)
    }

    /// Return the label pointing at one root cause's written syntax.
    fn root_label(&self, kind: CauseKind) -> CompilerResult<Option<(DiagnosticAnchor, String)>> {
        let label = match kind {
            CauseKind::Initializer {
                annotation: Some(annotation),
            } => {
                let (_, anchor) = self.source_anchor(annotation);

                (anchor, "expected due to this annotation".to_string())
            }
            CauseKind::Return {
                annotation: Some(annotation),
            } => {
                let (_, anchor) = self.source_anchor(annotation);

                (anchor, "expected due to this result type".to_string())
            }
            CauseKind::Argument { call, .. } => {
                let (_, anchor) = self.source_anchor(call);

                (anchor, "in this call".to_string())
            }
            CauseKind::Bound { parameter } => {
                let Some(binding) = self.generic_parameter(parameter) else {
                    return Ok(None);
                };
                let dir::GenericParameterKey::Symbol(symbol) = binding.key else {
                    return Ok(None);
                };
                let declaration = self.symbol_source(symbol)?;
                let (_, anchor) = self.source_anchor(declaration);
                let name = self.format_symbol(symbol);

                (anchor, format!("required by this bound on '{name}'"))
            }
            CauseKind::Heritage { clause } => {
                let (_, anchor) = self.source_anchor(clause);

                (anchor, "required by this heritage clause".to_string())
            }
            CauseKind::Pattern { pattern } => {
                let (_, anchor) = self.source_anchor(pattern);

                (anchor, "required by this pattern".to_string())
            }
            CauseKind::Write { place } => {
                let (_, anchor) = self.source_anchor(place);

                (
                    anchor,
                    "expected due to the type of this target".to_string(),
                )
            }
            _ => return Ok(None),
        };

        Ok(Some(label))
    }
}

/// Join slot descriptions from the mismatch outward, eliding deep paths.
fn join_path(slots: &[String]) -> String {
    match slots {
        [.., _, _, _, _, last] => {
            let head = slots[..3].join(" of ");
            let elided = slots.len() - 4;

            format!("{head} of … ({elided} elided) … {last}")
        }
        _ => slots.join(" of "),
    }
}
