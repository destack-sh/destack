use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

use super::instance::InstanceWorklist;

/// The substitution committed types close under, with the node their entries anchor at.
pub(super) struct Materialization<'a> {
    /// The substitution closing parameters and the receiver, absent over the module's own entries.
    pub(super) substitution: Option<&'a TypeSubstitution>,
    /// The node every entry anchors at.
    pub(super) anchor: Option<dir::GlobalNodeIdAny>,
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
    Definition(dir::GlobalNodeIdAny, Box<dir::Definition>),
    /// One node's decision.
    Decision(dir::GlobalNodeIdAny, Box<dir::Decision>),
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
            anchor: None,
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
    fn module_entries(&mut self) -> CompilerResult<Vec<Entry>> {
        let mut entries = Vec::new();

        // the definitions with their sources
        let definitions: Vec<_> = self
            .module
            .iter_definitions()
            .filter_map(|(symbol, definition)| {
                let source = self.module.definition_source_maybe(symbol)?;

                Some((source, definition.clone()))
            })
            .collect();
        for (source, definition) in definitions {
            entries.push(Entry::Definition(source, Box::new(definition)));
        }
        // the symbol types at their declarations
        let module = &self.module;
        let bindings = module.binding_table();
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
                .map(|(node, decision)| Entry::Decision(node, Box::new(decision.clone()))),
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
    pub(super) fn template_entries(
        &mut self,
        template: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<Entry>> {
        let mut entries = Vec::new();

        // a type template reaches its definition and the field and method types lowering lays out
        if let Some(definition) = self.definition(template)? {
            for member in definition.members() {
                let symbol = match member {
                    dir::DefinitionMember::Field(field) => field.symbol,
                    dir::DefinitionMember::Method(method) => method.symbol,
                    _ => continue,
                };
                if let Some(entry) = self.symbol_entry(symbol)? {
                    entries.push(entry);
                }
            }

            // add the entries of the member initializer nodes
            let nodes = self.template_initializer_body(template, &definition)?;
            entries.extend(self.entries_at(template.module_id, nodes)?);

            let source = self.committed_definition_source(template)?;
            entries.push(Entry::Definition(
                source,
                Box::new(dir::Definition::clone(&definition)),
            ));

            return Ok(entries);
        }

        // a callable template reaches its own type and the entries of its body nodes
        if let Some(entry) = self.symbol_entry(template)? {
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
        let Some(committed) = self.committed(module)? else {
            return Ok(Vec::new());
        };

        // read every entry recorded at each node
        let mut entries = Vec::new();
        for node in nodes {
            if let Some(ty) = committed.types.get_node_type_id(node) {
                entries.push(Entry::Node(node, ty));
            }
            if let Some(decision) = committed
                .decisions
                .and_then(|decisions| decisions.decision(node))
            {
                entries.push(Entry::Decision(node, Box::new(decision.clone())));
            }
            if let Some(place) = committed
                .decisions
                .and_then(|decisions| decisions.place_resolution(node))
            {
                entries.push(Entry::Place(node, *place));
            }
            if let Some(coercion) = committed
                .coercions
                .and_then(|coercions| coercions.coercion(node))
            {
                entries.push(Entry::Coercion(node, coercion.clone()));
            }
        }

        Ok(entries)
    }

    /// Return one symbol's committed type entry at its declaration.
    fn symbol_entry(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<Option<Entry>> {
        let Some(committed) = self.committed(symbol.module_id)? else {
            return Ok(None);
        };
        let Some(ty) = committed.types.get_symbol_type_id(symbol) else {
            return Ok(None);
        };
        let declaration = committed.bindings.get_symbol(symbol.local_id).declaration;

        Ok(Some(Entry::Symbol(symbol, ty, declaration)))
    }

    /// Materialize every type of each entry, writing the entries the materialization moves.
    fn materialize_entries(
        &mut self,
        materialization: &Materialization<'_>,
        entries: Vec<Entry>,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        for entry in entries {
            // record the dependents of each own symbol lowering lays out
            if let Entry::Symbol(symbol, ..) = &entry
                && symbol.module_id == self.module_id
            {
                self.symbol_dependents(*symbol)?;
            }

            // name the entry in an internal failure, a symbol by its name
            let (kind, symbol) = match &entry {
                Entry::Symbol(symbol, ..) => ("symbol", Some(*symbol)),
                Entry::Node(..) => ("node", None),
                Entry::Definition(..) => ("definition", None),
                Entry::Decision(..) => ("decision", None),
                Entry::Place(..) => ("place", None),
                Entry::Coercion(..) => ("coercion", None),
            };
            self.materialize_entry(materialization, entry, worklist)
                .map_err(|error| match error {
                    CompilerError::Internal { message } => {
                        let name = symbol
                            .map(|symbol| format!(" '{}'", self.format_symbol(symbol)))
                            .unwrap_or_default();

                        CompilerError::Internal {
                            message: format!("materializing the {kind}{name} entry: {message}"),
                        }
                    }
                    error => error,
                })?;
        }

        Ok(())
    }

    /// Materialize the types of one entry.
    fn materialize_entry(
        &mut self,
        materialization: &Materialization<'_>,
        entry: Entry,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        match entry {
            Entry::Symbol(symbol, ty, declaration) => {
                let Some(anchor) = materialization.anchor.or(declaration) else {
                    return Ok(());
                };
                let resolved = self.materialize_type(materialization, anchor, ty, worklist)?;
                if resolved != ty {
                    self.module.types_tail.set_symbol_type(symbol, resolved);
                }

                Ok(())
            }
            Entry::Node(node, ty) => {
                let anchor = materialization.anchor.unwrap_or(node);
                let resolved = self.materialize_type(materialization, anchor, ty, worklist)?;
                if resolved != ty {
                    self.module.types_tail.set_node_type(node, resolved);
                }

                Ok(())
            }
            Entry::Definition(source, definition) => {
                let anchor = materialization.anchor.unwrap_or(source);

                // lowering reads the module's own definitions unreduced, keyed at their heads
                if materialization.substitution.is_none() {
                    dir::TypeVisit::visit_types(&*definition, &mut |ty| {
                        self.walk_type_graph(ty, anchor, worklist)
                    })?;
                }

                self.materialize_payload(materialization, anchor, *definition, worklist)?;

                Ok(())
            }
            Entry::Decision(node, decision) => {
                let anchor = materialization.anchor.unwrap_or(node);
                let moved = self.materialize_payload(
                    materialization,
                    anchor,
                    (*decision).clone(),
                    worklist,
                )?;
                self.intern_selections(moved.as_ref().unwrap_or(&decision), anchor, worklist)?;
                if let Some(moved) = moved {
                    self.module.decisions_tail.set_decision(node, moved);
                }

                Ok(())
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

                Ok(())
            }
            Entry::Coercion(node, coercion) => {
                let anchor = materialization.anchor.unwrap_or(node);
                if let Some(resolved) =
                    self.materialize_payload(materialization, anchor, coercion, worklist)?
                {
                    self.module.coercions_tail.bind_coercion(node, resolved);
                }

                Ok(())
            }
        }
    }

    /// Materialize the types one payload carries, returning the payload when the module takes it.
    pub(super) fn materialize_payload<T: dir::TypeFold>(
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

        Ok(is_moved.then_some(payload))
    }

    /// Materialize one committed type, interning the applications it reaches.
    pub(super) fn materialize_type(
        &mut self,
        materialization: &Materialization<'_>,
        anchor: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.anchored_origin(anchor)?;
        let mut ty = ty;
        let mut flags = self.type_flags(ty)?;

        // resolve the module's own solved variables into the committed type
        if materialization.substitution.is_none() && flags.has_variable() {
            ty = self.deeply_resolve(origin, ty)?;
            flags = self.type_flags(ty)?;
        }

        // leave open inference types to their later passes
        if flags.has_variable() {
            return Ok(ty);
        }

        let resolved = match materialization.substitution {
            // reduce the module's own types as far as their inputs allow
            None => self.evaluate_type(origin, ty)?,
            // closed types are identical under every receiver and stay as written
            Some(_) if !flags.has_parameter() && !flags.has_this() => return Ok(ty),
            Some(substitution) => {
                let substituted = self.substitute_type(ty, substitution)?;

                self.evaluate_type(origin, substituted)?
            }
        };

        // record the types the resolved and the unreduced graphs name
        self.walk_type_graph(resolved, anchor, worklist)?;
        if resolved != ty {
            self.walk_reduction_graph(ty, anchor)?;
        }

        Ok(resolved)
    }

    /// Intern the instance behind every selection one decision carries, recorded beside it.
    fn intern_selections(
        &mut self,
        decision: &dir::Decision,
        anchor: dir::GlobalNodeIdAny,
        worklist: &mut InstanceWorklist,
    ) -> CompilerResult<()> {
        let mut written = Vec::new();
        dir::InstanceKeyVisit::visit_instance_keys(decision, &mut |key| {
            if !key.arguments.is_empty() || key.receiver.is_some() {
                written.push(key.clone());
            }
        });
        for key in written {
            let instance = self.intern_instance(
                key.symbol,
                key.receiver,
                key.arguments.clone(),
                anchor,
                dir::InstanceOrigin::Instantiation,
                worklist,
            )?;
            if let Some(instance) = instance {
                self.module
                    .generics_tail
                    .bind_selection_instance(key, instance);
            }
        }

        Ok(())
    }

    /// Return one definition's committed source node in its module.
    pub(super) fn committed_definition_source(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        self.committed(symbol.module_id)?
            .and_then(|committed| committed.definitions.definition_source(symbol))
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a definition without its committed source {symbol:?}"),
            })
    }
}
