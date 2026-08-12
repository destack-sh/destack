use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_dir::TypeFold;

use crate::check::{CheckModuleState, CheckState};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Write every commit since the last write into the module tail.
    pub(in crate::check) fn write_back(&mut self) -> CompilerResult<()> {
        let failed = self.failed_generic_applications()?;

        // resolve committed node types, storing their reduced heads
        for node in self.node_types.nodes() {
            let ty = self.node_types.get(&node).expect("collected node type");
            let resolved = self.fully_resolve(ty, &failed)?;
            let resolved = match self.is_checking() {
                true => match self.node_origin_maybe(node) {
                    Some(origin) => self.deeply_resolve(origin, resolved)?,
                    None => resolved,
                },
                false => resolved,
            };
            self.node_types.insert(node, resolved);
            let written = self
                .module
                .types_tail
                .get_node_type_id(node)
                .or_else(|| self.module.types.get_node_type_id(node));
            if written != Some(resolved) {
                self.module.types_tail.set_node_type(node, resolved);
            }
        }

        // resolve contextual expectations that differ from their node types
        for node in self.expected_types.nodes() {
            let ty = self
                .expected_types
                .get(&node)
                .expect("collected expected type");
            let resolved = self.fully_resolve(ty, &failed)?;
            self.expected_types.insert(node, resolved);
            if self.node_types.get(&node) != Some(resolved) {
                self.module.types_tail.set_expected_type(node, resolved);
            }
        }

        // resolve declaration types and normalize their declared rows
        for index in 0..self.declaration_types.len() {
            let (symbol, ty) = self
                .declaration_types
                .get_index(index)
                .map(|(k, v)| (*k, *v))
                .expect("indexed entry");
            let resolved = self.fully_resolve(ty, &failed)?;
            self.declaration_types[index] = resolved;
            self.write_symbol_type(symbol, resolved);
        }

        // resolve binding types
        for index in 0..self.binding_types.len() {
            let (symbol, ty) = self
                .binding_types
                .get_index(index)
                .map(|(k, v)| (*k, *v))
                .expect("indexed entry");
            let resolved = self.fully_resolve(ty, &failed)?;
            self.binding_types[index] = resolved;
            self.write_symbol_type(symbol, resolved);
        }

        // resolve the types every segment this pass wrote carries
        let module = self.module_id;
        self.resolve_segment_types(dir::DecisionSegment::new(module), |state| {
            &mut state.decisions
        })?;
        self.resolve_segment_types(dir::DefinitionSegment::new(module), |state| {
            &mut state.definitions_tail
        })?;
        self.resolve_segment_types(dir::DecoratorSegment::new(module), |state| {
            &mut state.decorators_tail
        })?;
        self.resolve_segment_types(dir::StaticSegment::new(module), |state| {
            &mut state.statics_tail
        })?;
        self.resolve_segment_types(dir::CoercionSegment::new(module), |state| {
            &mut state.coercions
        })?;
        self.resolve_segment_types(dir::CaptureSegment::new(module), |state| {
            &mut state.captures
        })?;
        self.resolve_segment_types(dir::AutoSegment::new(module), |state| &mut state.auto)?;

        Ok(())
    }

    /// Resolve every type one written module segment carries.
    fn resolve_segment_types<S: TypeFold>(
        &mut self,
        replacement: S,
        select: impl Fn(&mut CheckModuleState) -> &mut S,
    ) -> CompilerResult<()> {
        // fold the segment outside the module, since resolving reads the rest of the state
        let mut segment = std::mem::replace(select(&mut self.module), replacement);

        // selections keep the types they chose, so only their variables resolve
        let intact = FxIndexSet::default();
        segment.map_types(&mut |ty| self.fully_resolve(ty, &intact))?;
        *select(&mut self.module) = segment;

        Ok(())
    }

    /// Write one settled symbol type over whatever row the artifact already carries.
    fn write_symbol_type(&mut self, symbol: dir::GlobalSymbolId, resolved: dir::GlobalTypeId) {
        let written = self
            .module
            .types_tail
            .get_symbol_type_id(symbol)
            .or_else(|| self.module.types.get_symbol_type_id(symbol));
        if written != Some(resolved) {
            self.module.types_tail.set_symbol_type(symbol, resolved);
        }
    }

    /// Resolve one committed type, erroring unsolved holes and poisoning failed applications.
    pub(in crate::check) fn fully_resolve(
        &mut self,
        ty: dir::GlobalTypeId,
        failed: &FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.shallow_resolve(ty)?;
        if failed.is_empty() && !self.type_flags(ty)?.has_variable() {
            return Ok(ty);
        }

        // track the roots on the active path to break cycles
        let mut active = FxIndexSet::default();
        let resolved = self.resolve_open_type(ty, failed, &mut active)?;

        // require the write to close, since a pass exports solutions and holes only
        if self.type_flags(resolved)?.has_variable() {
            return Err(CompilerError::Internal {
                message: format!("check variable survived the write of {resolved:?}"),
            });
        }

        Ok(resolved)
    }

    /// Intern the declared hole one unsolved variable stands for.
    fn hole_type(&mut self, variable: dir::TypeVariableId) -> CompilerResult<dir::GlobalTypeId> {
        let origin = self.infer.variables.get(variable)?.origin;
        let origin = self.infer.origin(origin);
        let node = self
            .origin_source_node(origin)?
            .into_global(origin.module());

        self.intern_type(dir::Type::Hole(node))
    }

    /// Resolve one open type graph, cycling through solved variable roots.
    fn resolve_open_type(
        &mut self,
        id: dir::GlobalTypeId,
        failed: &FxIndexSet<dir::GlobalTypeId>,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // poison a generic application whose declared argument bound failed
        if failed.contains(&id) {
            return self.intern_type(dir::Type::Error);
        }

        // return resolved subgraphs free of failures unchanged
        if failed.is_empty() && !self.type_flags(id)?.has_variable() {
            return Ok(id);
        }

        // stop at a root already on the active path
        if !active.insert(id) {
            return Ok(id);
        }

        // resolve a variable through its root, erroring unsolved ones
        let ty = self.ty(id)?;
        let resolved = if let dir::Type::Variable(variable) = ty {
            match self.infer.solution(variable)? {
                Some(solution) => {
                    let solution = self.shallow_resolve(solution)?;

                    self.resolve_open_type(solution, failed, active)?
                }
                // publish an unsolved declaration variable as the hole it stands for
                None if self.is_declaration() => self.hole_type(variable)?,
                None => self.intern_type(dir::Type::Error)?,
            }
        }
        // keep foreign types; their own module writes them back
        else if !self.is_own_module(id.module_id) {
            id
        }
        // rebuild a composite around its resolved children
        else {
            let rebuilt =
                self.map_type_children(id.module_id, id.module_id, ty, &mut |state, child| {
                    state.resolve_open_type(child, failed, active)
                })?;

            // renormalize solved unions like any other construction
            match rebuilt {
                dir::Type::Union(union) => {
                    let elements = self.type_ids(id.module_id, union.elements)?.to_vec();

                    self.normalized_union_type(elements)?
                }
                dir::Type::Intersection(intersection) => {
                    let elements = self.type_ids(id.module_id, intersection.elements)?.to_vec();

                    self.normalized_intersection_type(elements)?
                }
                rebuilt => self.intern_type(rebuilt)?,
            }
        };
        active.swap_remove(&id);

        Ok(resolved)
    }
}
