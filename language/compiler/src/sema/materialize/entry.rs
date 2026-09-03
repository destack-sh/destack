use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckState, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

use super::instance::InstanceWorklist;

/// One materialization: the substitution committed types close under and the instance recording them.
pub(super) struct Materialization<'a> {
    /// The substitution closing parameters and the receiver, absent over the module's own entries.
    pub(super) substitution: Option<&'a TypeSubstitution>,
    /// The interface owner's self application the receiver stands in for.
    pub(super) owner_self: Option<dir::GlobalTypeId>,
    /// The instance recording the moved types, the module's own tails when absent.
    pub(super) instance: Option<dir::LocalInstanceId>,
    /// The node every entry anchors at, the instance source under an instance.
    pub(super) anchor: Option<dir::GlobalNodeIdAny>,
    /// The instantiation chain depth the reached applications intern at.
    pub(super) depth: u32,
}

/// One committed entry carrying types, with the node it anchors at.
pub(super) enum Entry {
    /// One symbol's type.
    Symbol(
        dir::GlobalSymbolId,
        dir::GlobalTypeId,
        Option<dir::GlobalNodeIdAny>,
    ),
    /// One node's type.
    Node(dir::GlobalNodeIdAny, dir::GlobalTypeId),
    /// One definition with its source.
    Definition(dir::GlobalSymbolId, dir::GlobalNodeIdAny, dir::Definition),
    /// One node's decision.
    Decision(dir::GlobalNodeIdAny, dir::Decision),
    /// One node's place resolution.
    Place(dir::GlobalNodeIdAny, dir::PlaceResolution),
    /// One node's coercion.
    Coercion(dir::GlobalNodeIdAny, dir::Coercion),
}

impl CheckState<'_> {
    /// Materialize every committed entry of the checked module into its own tails.
    pub(in crate::sema) fn materialize_module_entries(
        &mut self,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let materialization = Materialization {
            substitution: None,
            owner_self: None,
            instance: None,
            anchor: None,
            depth: 0,
        };
        let entries = self.module_entries()?;

        self.materialize_entries(&materialization, entries, worklist)
    }

    /// Close every committed row one template reaches under one materialization.
    pub(super) fn materialize_template_entries(
        &mut self,
        materialization: &Materialization<'_>,
        template: dir::GlobalSymbolId,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let entries = self.template_entries(template)?;

        self.materialize_entries(materialization, entries, worklist)
    }

    /// Return every committed entry of the checked module.
    fn module_entries(&self) -> CompilerResult<Vec<Entry>> {
        let module = &self.module;
        let bindings = module.binding_table();
        let mut entries = Vec::new();

        // the definitions with their sources
        for (symbol, definition) in module.iter_definitions() {
            let Some(source) = module.definition_source_maybe(symbol) else {
                continue;
            };
            entries.push(Entry::Definition(symbol, source, definition.clone()));
        }

        // the symbol types at their declarations
        for (symbol, ty) in module.types.symbol_types() {
            let declaration = bindings.get_symbol(symbol.local_id).declaration;
            entries.push(Entry::Symbol(symbol, ty, declaration));
        }

        // the node types and the payloads checking committed
        entries.extend(
            module
                .types
                .node_types()
                .map(|(node, ty)| Entry::Node(node, ty)),
        );
        entries.extend(
            module
                .decisions
                .decision_entries()
                .map(|(node, decision)| Entry::Decision(node, decision.clone())),
        );
        entries.extend(
            module
                .decisions
                .place_entries()
                .map(|(node, place)| Entry::Place(node, *place)),
        );
        entries.extend(
            module
                .coercions
                .coercions()
                .map(|(node, coercion)| Entry::Coercion(node, coercion.clone())),
        );

        Ok(entries)
    }

    /// Return the committed entries one template reaches: its definition or its body.
    fn template_entries(&mut self, template: dir::GlobalSymbolId) -> CompilerResult<Vec<Entry>> {
        let mut entries = Vec::new();

        // a type template reaches its definition and the field and method types lowering lays out
        if let Some(definition) = self.definition(template)?.cloned() {
            for member in definition.members() {
                let symbol = match member {
                    dir::DefinitionMember::Field(field) => field.symbol,
                    dir::DefinitionMember::Method(method) => method.symbol,
                    _ => continue,
                };
                if let Some(entry) = self.symbol_entry(symbol) {
                    entries.push(entry);
                }
            }

            // add the entries of the member initializer nodes
            let nodes = self.template_initializer_body(template, &definition)?;
            entries.extend(self.entries_at(template.module_id, nodes)?);

            let source = self.committed_definition_source(template)?;
            entries.push(Entry::Definition(template, source, definition));

            return Ok(entries);
        }

        // a callable template reaches its own type and the entries of its body nodes
        if let Some(entry) = self.symbol_entry(template) {
            entries.push(entry);
        }
        let Some(nodes) = self.template_body(template)? else {
            return Ok(entries);
        };
        entries.extend(self.entries_at(template.module_id, nodes)?);

        Ok(entries)
    }

    /// Return the committed entries standing at one module's nodes.
    fn entries_at(
        &self,
        module: ModuleId,
        nodes: Vec<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<Vec<Entry>> {
        let Some(committed) = self.committed(module) else {
            return Ok(Vec::new());
        };

        // read every entry recorded at each node
        let mut entries = Vec::new();
        for node in nodes {
            if let Some(ty) = committed.types.get_node_type_id(node) {
                entries.push(Entry::Node(node, ty));
            }
            if let Some(decision) = committed.decisions.decision(node) {
                entries.push(Entry::Decision(node, decision.clone()));
            }
            if let Some(place) = committed.decisions.place_resolution(node) {
                entries.push(Entry::Place(node, *place));
            }
            if let Some(coercion) = committed.coercions.coercion(node) {
                entries.push(Entry::Coercion(node, coercion.clone()));
            }
        }

        Ok(entries)
    }

    /// Return one symbol's committed type entry at its declaration.
    fn symbol_entry(&self, symbol: dir::GlobalSymbolId) -> Option<Entry> {
        let committed = self.committed(symbol.module_id)?;
        let ty = committed.types.get_symbol_type_id(symbol)?;
        let declaration = committed.bindings.get_symbol(symbol.local_id).declaration;

        Some(Entry::Symbol(symbol, ty, declaration))
    }

    /// Materialize every type of each entry, writing the entries the materialization moves.
    fn materialize_entries(
        &mut self,
        materialization: &Materialization<'_>,
        entries: Vec<Entry>,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        for entry in entries {
            match entry {
                Entry::Symbol(symbol, ty, declaration) => {
                    let Some(anchor) = materialization.anchor.or(declaration) else {
                        continue;
                    };
                    let resolved = self.materialize_type(materialization, anchor, ty, worklist)?;
                    match materialization.instance {
                        Some(instance) => self
                            .module
                            .generics_tail
                            .bind_instance_symbol(instance, symbol, resolved),
                        None if resolved != ty => {
                            self.module.types_tail.set_symbol_type(symbol, resolved);
                        }
                        None => {}
                    }
                }
                Entry::Node(node, ty) => {
                    let anchor = materialization.anchor.unwrap_or(node);
                    let resolved = self.materialize_type(materialization, anchor, ty, worklist)?;
                    if materialization.instance.is_none() && resolved != ty {
                        self.module.types_tail.set_node_type(node, resolved);
                    }
                }
                Entry::Definition(symbol, source, definition) => {
                    let anchor = materialization.anchor.unwrap_or(source);
                    if let Some(resolved) =
                        self.materialize_payload(materialization, anchor, definition, worklist)?
                    {
                        self.module
                            .definitions_tail
                            .insert_definition(symbol, source, resolved);
                    }
                }
                Entry::Decision(node, decision) => {
                    let anchor = materialization.anchor.unwrap_or(node);
                    if let Some(resolved) =
                        self.materialize_payload(materialization, anchor, decision, worklist)?
                    {
                        self.module.decisions_tail.set_decision(node, resolved);
                    }
                }
                Entry::Place(node, place) => {
                    let anchor = materialization.anchor.unwrap_or(node);
                    if let Some(resolved) =
                        self.materialize_payload(materialization, anchor, place, worklist)?
                    {
                        self.module
                            .decisions_tail
                            .set_place_resolution(node, resolved);
                    }
                }
                Entry::Coercion(node, coercion) => {
                    let anchor = materialization.anchor.unwrap_or(node);
                    if let Some(resolved) =
                        self.materialize_payload(materialization, anchor, coercion, worklist)?
                    {
                        self.module.coercions_tail.bind_coercion(node, resolved);
                    }
                }
            }
        }

        Ok(())
    }

    /// Materialize the types one payload carries, returning the payload when the module takes it.
    fn materialize_payload<T: dir::TypeFold>(
        &mut self,
        materialization: &Materialization<'_>,
        anchor: dir::GlobalNodeIdAny,
        mut payload: T,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<Option<T>> {
        // rewrite each type the payload carries, noting whether one moves
        let mut is_moved = false;
        dir::TypeFold::map_types(&mut payload, &mut |ty| -> CompilerResult<_> {
            let resolved = self.materialize_type(materialization, anchor, ty, worklist)?;
            is_moved |= resolved != ty;

            Ok(resolved)
        })?;

        // an instance records the moved types alone
        let is_written = materialization.instance.is_none() && is_moved;

        Ok(is_written.then_some(payload))
    }

    /// Materialize one committed type, interning the applications it reaches.
    fn materialize_type(
        &mut self,
        materialization: &Materialization<'_>,
        anchor: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(anchor, None);
        let mut ty = ty;
        let mut flags = self.type_flags(ty)?;

        // resolve the module's own solved variables into the committed spelling
        if materialization.substitution.is_none() && flags.has_variable() {
            ty = self.deeply_resolve(origin, ty)?;
            flags = self.type_flags(ty)?;
        }

        // leave open inference types and unbound receivers to the materialization binding them
        let has_receiver = materialization
            .substitution
            .is_some_and(|substitution| substitution.receiver.is_some());
        if flags.has_variable() || (flags.has_this() && !has_receiver) {
            return Ok(ty);
        }

        let resolved = match materialization.substitution {
            // keep a written template type written, interning its closable applications
            None if flags.has_parameter() => {
                self.intern_applications(ty, anchor, materialization.depth, worklist)?;

                return Ok(ty);
            }
            // evaluate the module's own computation results, keeping written aliases
            None => match self.has_reachable_computation(ty)? {
                true => self.evaluate_type(origin, ty)?,
                false => ty,
            },
            // closed types are identical across instances and stay as written
            Some(_)
                if !flags.has_parameter()
                    && !flags.has_this()
                    && materialization.owner_self.is_none() =>
            {
                return Ok(ty);
            }
            // substitute, closing the owner's self application at the receiver
            Some(substitution) => {
                let mut substituted = self.substitute_type(ty, substitution)?;
                if let (Some(receiver), Some(base)) =
                    (substitution.receiver, materialization.owner_self)
                {
                    substituted = self.replace_type(self.module_id, substituted, base, receiver)?;
                }
                let resolved = self.evaluate_closed_type(origin, substituted)?;

                // record the type wherever the instance moves the written one
                if let Some(instance) = materialization.instance
                    && resolved != ty
                {
                    let is_evaluated = resolved != substituted;
                    self.module.generics_tail.bind_instance_type(
                        instance,
                        ty,
                        resolved,
                        is_evaluated,
                    );
                }

                resolved
            }
        };

        // admit the concrete applications the closed type reaches
        self.intern_applications(resolved, anchor, materialization.depth, worklist)?;

        Ok(resolved)
    }

    /// Return one definition's committed source node in its module.
    fn committed_definition_source(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        self.committed(symbol.module_id)
            .and_then(|committed| committed.definitions.definition_source(symbol))
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a definition without its committed source {symbol:?}"),
            })
    }
}
