use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, Decision, Origin};

use super::CheckModuleOutput;

/// One module's resolved commit rows.
pub(in crate::check) struct ModuleCommit {
    /// The committed module.
    module: ModuleId,
    /// Variable entries patched with their solution types.
    patches: Vec<(dir::LocalTypeId, dir::Type)>,
    /// Resolved node types.
    node_types: Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>,
    /// Resolved symbol types.
    symbol_types: Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>,
    // TODO #Suspicious Cleanup: wht are "definition_values" and "symbol_literals" in commit about..?
    /// Resolved alias definition values.
    definition_values: Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>,
    /// Resolved literal symbol values.
    symbol_literals: Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>,
    /// Resolved implicit coercions.
    coercions: Vec<(dir::GlobalNodeIdAny, dir::Coercion)>,
}

impl ModuleCommit {
    /// Return the committed module.
    pub(in crate::check) fn module(&self) -> ModuleId {
        self.module
    }
}

impl CheckState<'_> {
    /// Read one module's commit rows against live working state.
    pub(in crate::check) fn module_commit(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<ModuleCommit> {
        Ok(ModuleCommit {
            module,
            patches: self.commit_variable_patches(module)?,
            node_types: self.commit_node_types(module)?,
            symbol_types: self.commit_symbol_types(module)?,
            definition_values: self.commit_definition_values(module)?,
            symbol_literals: self.commit_symbol_literals(module)?,
            coercions: self.commit_coercions(module)?,
        })
    }

    /// Commit one module's solved state into output DIR tables.
    pub(in crate::check) fn commit_module(
        &mut self,
        commit: ModuleCommit,
    ) -> CompilerResult<CheckModuleOutput> {
        let module = commit.module;
        let mut output = CheckModuleOutput::new(module, self.module(module));

        // move the working segments wholesale
        let working = self.module_mut(module).take_working();
        output.types = working.types;
        output.generics = working.generics;
        output.definitions = working.definitions;
        if let Some(layouts) = self.layouts.swap_remove(&module) {
            output.layouts = layouts;
        }

        // patch open variable entries with their solution types
        for (local, patched) in commit.patches {
            output.types.update_type(local, patched);
        }

        // alias definition values commit evaluated like their symbol types
        for (symbol, value) in commit.definition_values {
            if let Some(dir::Definition::TypeAlias(definition)) =
                output.definitions.definition_mut(symbol)
            {
                definition.value = value;
            }
        }

        // record inferred node and symbol types
        for (node, ty) in commit.node_types {
            output.types.set_node_type(node, ty);
        }
        for (symbol, ty) in commit.symbol_types {
            output.types.set_symbol_type(symbol, ty);
        }

        // record implicit coercions beside their value nodes
        for (node, coercion) in commit.coercions {
            output.coercions.bind_coercion(node, coercion);
        }

        // symbol values materialize as committed statics
        for (symbol, literal) in commit.symbol_literals {
            let id = output
                .statics
                .push_static(dir::StaticTerm::ScalarLiteral { value: literal });
            output
                .statics
                .set_symbol_static(symbol, id.into_global(module));
        }

        // drain decided node meanings into resolutions
        self.drain_decisions(module, &mut output);

        // record nominal heritage relations for queries
        self.drain_relations(&mut output);

        // TODO #Incomplete: synthesize capture frames from collected captures

        Ok(output)
    }

    /// Resolve every solved variable entry in one live working segment.
    /// Unsolved variables patch to the error type, their diagnostics
    /// come from the unsolved sweep.
    fn commit_variable_patches(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::LocalTypeId, dir::Type)>> {
        // collect the variable entries owned by this segment
        let working = &self.module(module).working.types;
        let mut variables = Vec::new();
        for local in working.iter_type_ids() {
            let Some(dir::Type::Variable(variable)) = working.get_type_maybe(local) else {
                continue;
            };

            variables.push((local, *variable));
        }

        // resolve each entry to its solution's type
        let mut patches = Vec::with_capacity(variables.len());
        let mut reported = IndexSet::new();
        for (local, variable) in variables {
            let representative = self.variables.representative(variable)?;
            let solution = self.variables.solution(representative)?;
            let patched = match solution {
                Some(solution) => {
                    let solved = self.resolve_root(solution)?;

                    self.ty(solved)?.clone()
                }
                // unsolved variables demand an annotation loudly
                None => {
                    let origin = self.variables.get(representative)?.origin;
                    let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                    if self.modules.contains_key(&module)
                        && reported.insert((module, anchor.clone()))
                    {
                        let error = CheckError::MissingTypeAnnotation { anchor, module };
                        self.module_mut(module).diagnostics.push(error.into());
                    }

                    dir::Type::Error
                }
            };
            patches.push((local, patched));
        }

        Ok(patches)
    }

    /// Resolve one module's recorded node types.
    fn commit_node_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::GlobalTypeId)>> {
        let node_types = self.inputs.node_types_in(module).collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(node_types.len());
        for (node, ty) in node_types {
            resolved.push((node, self.resolve_root(ty)?));
        }

        Ok(resolved)
    }

    /// Resolve one committed type to its evaluated canonical form.
    /// Commit stores answers: alias applications and preserved type
    /// operations reduce before the tables seal.
    fn commit_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.resolve_root(ty)?;
        match self.evaluate_root(origin, ty)? {
            Answer::Ready(evaluated) => Ok(evaluated),
            // unevaluable forms keep their resolved spelling
            Answer::Pending(_) => Ok(ty),
        }
    }

    /// Resolve one module's recorded implicit coercions.
    fn commit_coercions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalNodeIdAny, dir::Coercion)>> {
        let coercions = self
            .coercions
            .iter()
            .filter(|(node, _)| node.module_id == module)
            .map(|(node, coercion)| (*node, *coercion))
            .collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(coercions.len());
        for (node, coercion) in coercions {
            resolved.push((
                node,
                dir::Coercion::new(
                    self.resolve_root(coercion.source)?,
                    self.resolve_root(coercion.target)?,
                    coercion.origin,
                ),
            ));
        }

        Ok(resolved)
    }

    /// Resolve one module's recorded symbol types.
    fn commit_symbol_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let symbol_types = self.inputs.symbol_types_in(module).collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(symbol_types.len());
        for (symbol, ty) in symbol_types {
            // alias values commit their evaluated answer; every other
            // symbol keeps its written spelling for lazy use sites
            let ty = if self.symbol_kind(symbol) == dir::SymbolKind::TypeAlias {
                self.commit_type(Origin::Symbol(symbol), ty)?
            } else {
                self.resolve_root(ty)?
            };
            resolved.push((symbol, ty));
        }

        Ok(resolved)
    }

    /// Resolve one module's alias definition values.
    ///
    /// Alias values commit their evaluated answer like alias symbol types.
    fn commit_definition_values(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let aliases = self
            .module(module)
            .working
            .definitions
            .iter_definitions()
            .filter_map(|(symbol, definition)| match definition {
                dir::Definition::TypeAlias(definition) => Some((symbol, definition.value)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut resolved = Vec::with_capacity(aliases.len());
        for (symbol, value) in aliases {
            resolved.push((symbol, self.commit_type(Origin::Symbol(symbol), value)?));
        }

        Ok(resolved)
    }

    /// Resolve one module's literal symbol values.
    fn commit_symbol_literals(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Vec<(dir::GlobalSymbolId, dir::ScalarLiteral)>> {
        let symbol_values = self.inputs.symbol_values_in(module).collect::<Vec<_>>();
        let mut literals = Vec::new();
        for (symbol, value) in symbol_values {
            let value = self.resolve_root(value)?;
            if let dir::Type::Literal(literal) = self.ty(value)? {
                literals.push((symbol, *literal));
            }
        }

        Ok(literals)
    }

    /// Drain decided node meanings into output resolutions.
    fn drain_decisions(&mut self, module: ModuleId, output: &mut CheckModuleOutput) {
        for (node, decision) in self.decisions.iter() {
            if node.module_id != module {
                continue;
            }

            match decision {
                Decision::Name(resolution) => {
                    output
                        .resolutions
                        .set_name_resolution(node, resolution.clone());
                }
                Decision::Receiver(resolution) => {
                    output
                        .resolutions
                        .set_receiver_resolution(node, *resolution);
                }
                Decision::Member(resolution) => {
                    output
                        .resolutions
                        .set_member_resolution(node, resolution.clone());
                }
                Decision::Call(resolution) => {
                    output
                        .resolutions
                        .set_call_resolution(node, resolution.clone());
                }
                Decision::ReadWrite(resolution) => {
                    output
                        .resolutions
                        .set_read_write_resolution(node, resolution.clone());
                }
                Decision::Construct(resolution) => {
                    output
                        .resolutions
                        .set_construct_resolution(node, resolution.clone());
                }
                Decision::Pattern(resolution) => {
                    output
                        .resolutions
                        .set_pattern_resolution(node, resolution.clone());
                }
                // rejections already carry their diagnostics
                Decision::Rejected => {}
            }
        }
    }

    /// Record nominal heritage relations from committed definitions.
    fn drain_relations(&mut self, output: &mut CheckModuleOutput) {
        let mut relations = Vec::new();

        // collect heritage edges from this module's definitions
        for (symbol, definition) in output.definitions.iter_definitions() {
            match definition {
                dir::Definition::Class(definition) => {
                    if let Some(extends) = &definition.extends {
                        relations.push((symbol, extends.clone(), true));
                    }
                    for heritage in &definition.implements {
                        relations.push((symbol, heritage.clone(), false));
                    }
                }
                dir::Definition::Struct(definition) => {
                    for heritage in &definition.implements {
                        relations.push((symbol, heritage.clone(), false));
                    }
                }
                dir::Definition::Enum(definition) => {
                    for heritage in &definition.implements {
                        relations.push((symbol, heritage.clone(), false));
                    }
                }
                dir::Definition::Interface(definition) => {
                    for heritage in &definition.extends {
                        relations.push((symbol, heritage.clone(), true));
                    }
                }
                dir::Definition::Extension(definition) => {
                    for heritage in &definition.implements {
                        relations.push((symbol, heritage.clone(), false));
                    }
                }
                dir::Definition::TypeAlias(_) | dir::Definition::Newtype(_) => {}
            }
        }

        for (symbol, heritage, is_extends) in relations {
            // the walked heritage annotation carries the applied type
            let Some(ty) = self.inputs.node_type(heritage.source) else {
                continue;
            };
            if is_extends {
                output
                    .relations
                    .push_extends(symbol, dir::Relation::extends(ty));
            } else {
                output
                    .relations
                    .push_implements(symbol, dir::Relation::implements(ty));
            }
        }
    }
}
