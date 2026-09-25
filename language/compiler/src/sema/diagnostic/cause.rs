use tspp_artifact::DiagnosticBuilder;
use tspp_dir as dir;

use crate::sema::{Cause, CauseId, CauseKind, CheckState, Origin, PropertySource, Relation};
use crate::{CompilerResult, DiagnosticAnchor};

/// The deepest structural slot the blame walk descends into.
const BLAME_DEPTH_LIMIT: u32 = 16;

/// The number of leading slot labels a deep path keeps before eliding.
const PATH_HEAD_LABELS: usize = 3;

/// The blamed origin of one failed closed relation.
pub(in crate::sema) enum Blame {
    /// One side reduces to a plainer type before mismatching.
    Reduced {
        /// The reduction notes, one per changed side.
        notes: Vec<String>,
    },
    /// One structural slot beneath the related pair mismatches.
    Slot {
        /// Slot labels from the leaf outward.
        path: Vec<String>,
        /// The mismatched leaf source type.
        source: dir::GlobalTypeId,
        /// The mismatched leaf target type.
        target: dir::GlobalTypeId,
    },
}

/// One structural descent state while blaming a failed pair.
struct BlameLeaf {
    /// Slot labels from the leaf outward.
    path: Vec<String>,
    /// The mismatched leaf source type.
    source: dir::GlobalTypeId,
    /// The mismatched leaf target type.
    target: dir::GlobalTypeId,
    /// Whether the walk entered any slot.
    descended: bool,
}

impl CheckState<'_> {
    /// Format one failure's cause chain into its diagnostic.
    pub(in crate::sema) fn format_cause<T>(
        &self,
        mut diagnostic: DiagnosticBuilder<T>,
        cause: CauseId,
        primary: &DiagnosticAnchor,
        blame: Option<&Blame>,
    ) -> CompilerResult<DiagnosticBuilder<T>> {
        // read the cause chain and the module it reports in
        let chain = self.cause_chain(cause);
        let module = self.cause_origin(cause).module();

        // format the slot path into the mismatch, blamed leaf first
        let mut slots: Vec<String> = match blame {
            Some(Blame::Slot { path, .. }) => path.clone(),
            _ => Vec::new(),
        };
        slots.extend(
            chain
                .iter()
                .take_while(|cause| cause.kind.is_slot())
                .filter_map(|cause| self.format_slot(cause.kind)),
        );
        let note = match (blame, slots.is_empty()) {
            // name the exact mismatched types for a blamed leaf
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

        // note the mismatch the slots name
        if let Some(note) = note {
            diagnostic = diagnostic.note(note);
        }

        // note each reduction the blamed sides went through
        if let Some(Blame::Reduced { notes }) = blame {
            for reduction in notes {
                diagnostic = diagnostic.note(reduction.clone());
            }
        }

        // point at the syntax that required the check
        if let Some(root) = chain.last()
            && let Some((anchor, message)) = self.root_label(root.kind)?
            && anchor != *primary
        {
            diagnostic = diagnostic.label(anchor, message);
        }

        Ok(diagnostic)
    }

    /// Blame the structural leaf beneath one failed closed relation.
    pub(in crate::sema) fn blame_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Blame>> {
        // leave open pairs to their constraint causes
        if !self.type_variables(source)?.is_empty() || !self.type_variables(target)?.is_empty() {
            return Ok(None);
        }

        // descend to the mismatched leaf
        let leaf = self.blame_leaf(origin, relation, source, target, 0)?;

        // name the mismatched slot when the walk descended
        if leaf.descended {
            return Ok(Some(Blame::Slot {
                path: leaf.path,
                source: leaf.source,
                target: leaf.target,
            }));
        }

        // note each written type that reduces to a different display
        let module = origin.module();
        let mut notes = Vec::new();
        for (written, reduced) in [(source, leaf.source), (target, leaf.target)] {
            if self.type_flags(reduced)?.has_error() {
                continue;
            }
            let written = self.format_type_at(module, written);
            let reduced = self.format_type_at(module, reduced);
            if written != reduced {
                notes.push(format!("'{written}' reduces to '{reduced}'"));
            }
        }

        // hint at interfaces when only class exactness refused a storage pair
        let is_storage = relation == Relation::Storable;
        let is_value_class = is_storage
            && match self.ty(leaf.source)? {
                dir::Type::Object(_) => true,
                dir::Type::Application(instance) => matches!(
                    self.definition(instance.symbol)?.as_deref(),
                    Some(
                        dir::Definition::Class(_)
                            | dir::Definition::Struct(_)
                            | dir::Definition::Interface(_)
                    )
                ),
                _ => false,
            };
        let is_exact_target = match self.ty(leaf.target)? {
            dir::Type::Object(shape) => !shape.declares_signatures(),
            _ => false,
        };
        if is_value_class && is_exact_target {
            let target = self.format_type_at(module, leaf.target);
            notes.push(format!(
                "'{target}' stores its exact object type, declare an interface to accept \
                 structurally wider values"
            ));
        }

        // leave a pair with nothing to note unblamed
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
        // stop the descent at the depth limit
        let leaf = BlameLeaf {
            path: Vec::new(),
            source,
            target,
            descended: false,
        };
        if depth >= BLAME_DEPTH_LIMIT {
            return Ok(leaf);
        }

        // resolve both sides and expand their written aliases before pairing their slots
        let source = self.deeply_resolve(origin, source)?;
        let source = self.structurally_normalize(origin, source)?;
        let target = self.deeply_resolve(origin, target)?;
        let target = self.structurally_normalize(origin, target)?;

        // descend into the first slot whose relation fails
        for (slot, child_relation, child_source, child_target) in
            self.blame_pairs(origin, relation, source, target)?
        {
            if self
                .decide_relation(origin, child_relation, child_source, child_target)?
                .holds()
            {
                continue;
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

    /// Decompose one closed pair into its labeled slot relations.
    fn blame_pairs(
        &mut self,
        origin: Origin,
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
        // decompose the pair by the heads on both sides
        let mut pairs = Vec::new();
        match (self.ty(source)?, self.ty(target)?) {
            // blame matching shape fields under their storage relations
            (
                dir::Type::Object(_) | dir::Type::Application(_),
                dir::Type::Object(_) | dir::Type::Application(_),
            ) if let (Some((source_fields, _)), Some((target_fields, _))) = (
                self.apparent_object_members(origin, source)?,
                self.apparent_object_members(origin, target)?,
            ) =>
            {
                for target_field in target_fields {
                    let source_field = source_fields
                        .iter()
                        .find(|field| field.key == target_field.key);
                    let Some(source_field) = source_field else {
                        continue;
                    };
                    let Some(relations) = Self::shape_property_relations(
                        relation,
                        PropertySource::Stored,
                        source_field,
                        &target_field,
                    ) else {
                        continue;
                    };
                    for (field_relation, source_ty, target_ty) in relations {
                        pairs.push((
                            self.format_slot(CauseKind::Field {
                                key: target_field.key,
                            }),
                            field_relation,
                            source_ty,
                            target_ty,
                        ));
                    }
                }
            }

            // blame tuple elements in position
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple))
                if source_tuple.form == target_tuple.form
                    && source_tuple.elements.len() == target_tuple.elements.len() =>
            {
                let source_elements =
                    self.tuple_elements(source.module_id, source_tuple.elements)?;
                let target_elements =
                    self.tuple_elements(target.module_id, target_tuple.elements)?;
                for (index, (source_element, target_element)) in source_elements
                    .iter()
                    .zip(target_elements.iter())
                    .enumerate()
                {
                    pairs.push((
                        self.format_slot(CauseKind::Element {
                            index: index as u32,
                        }),
                        relation,
                        source_element.ty,
                        target_element.ty,
                    ));
                }
            }

            // blame signature parameters contravariantly, returns covariantly
            (
                dir::Type::FunctionSignature(source_function),
                dir::Type::FunctionSignature(target_function),
            ) => {
                let source_function = self.type_signature(source.module_id, source_function)?;
                let target_function = self.type_signature(target.module_id, target_function)?;
                let source_parameters =
                    self.signature_parameters(source.module_id, source_function.parameters)?;
                let target_parameters =
                    self.signature_parameters(target.module_id, target_function.parameters)?;
                let shared = source_parameters.len().min(target_parameters.len());
                for (index, (source_parameter, target_parameter)) in source_parameters[..shared]
                    .iter()
                    .zip(target_parameters[..shared].iter())
                    .enumerate()
                {
                    pairs.push((
                        self.format_slot(CauseKind::Parameter {
                            index: index as u32,
                        }),
                        relation,
                        target_parameter.ty,
                        source_parameter.ty,
                    ));
                }
                if let (Some(source_return), Some(target_return)) =
                    (source_function.return_type, target_function.return_type)
                {
                    pairs.push((
                        self.format_slot(CauseKind::ReturnSlot),
                        relation,
                        source_return,
                        target_return,
                    ));
                }
            }

            // blame collection elements and lengths
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
                    relation,
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

            // blame the payload beneath matching memory forms
            (dir::Type::Form(source_form), dir::Type::Form(target_form))
                if source_form.form.same_constructor(&target_form.form) =>
            {
                pairs.push((None, relation, source_form.value, target_form.value));
            }

            // blame each source union element against the target
            (dir::Type::Union(elements), _) if relation != Relation::Equal => {
                let elements = self.type_ids(source.module_id, elements.elements)?;
                for element in elements {
                    pairs.push((None, relation, *element, target));
                }
            }

            // blame arguments of same-symbol applications by variance
            (dir::Type::Application(source_instance), dir::Type::Application(target_instance))
                if source_instance.symbol == target_instance.symbol
                    && source_instance.arguments.len() == target_instance.arguments.len() =>
            {
                let symbol = source_instance.symbol;
                let source_arguments =
                    self.type_ids(source.module_id, source_instance.arguments)?;
                let target_arguments =
                    self.type_ids(target.module_id, target_instance.arguments)?;
                let relation = Relation::Storable;
                let form = self.default_variance_form(symbol)?;
                for (index, (source_argument, target_argument)) in source_arguments
                    .iter()
                    .zip(target_arguments.iter())
                    .enumerate()
                {
                    let variance = self.argument_variance(symbol, index, form)?;
                    let Some((argument_relation, order)) = variance.argument_relation(relation)
                    else {
                        continue;
                    };
                    let (argument_source, argument_target) =
                        order.orient(*source_argument, *target_argument);
                    pairs.push((
                        self.format_slot(CauseKind::TypeArgument {
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

            // leave every other pair whole
            _ => {}
        }

        Ok(pairs)
    }

    /// Collect one cause chain from the constraint to its root.
    fn cause_chain(&self, cause: CauseId) -> Vec<Cause> {
        // walk from the constraint up to its root
        let mut chain = Vec::new();
        let mut current = Some(cause);
        while let Some(id) = current {
            let cause = self.infer.cause(id);
            chain.push(cause);
            current = cause.parent;
        }

        chain
    }

    /// Return the root of one constraint's cause chain.
    pub(in crate::sema) fn root_cause(&self, cause: CauseId) -> Cause {
        // walk to the top of the chain
        let mut root = self.infer.cause(cause);
        while let Some(parent) = root.parent {
            root = self.infer.cause(parent);
        }

        root
    }

    /// Format one slot the constraint descended into.
    fn format_slot(&self, kind: CauseKind) -> Option<String> {
        // name the slot the constraint descended into
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
        // point at the syntax each root cause names
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
                let Some(binding) = self.generic_parameter(parameter)? else {
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

/// Join slot labels from the mismatch outward, eliding deep paths.
fn join_path(slots: &[String]) -> String {
    // elide the middle of a deep path
    match slots {
        [.., _, _, _, _, last] => {
            let head = slots[..PATH_HEAD_LABELS].join(" of ");
            let elided = slots.len() - PATH_HEAD_LABELS - 1;

            format!("{head} of … ({elided} elided) … {last}")
        }
        _ => slots.join(" of "),
    }
}
