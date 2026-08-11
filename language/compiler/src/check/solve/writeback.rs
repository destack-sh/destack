use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Write every commit since the last write into the module tail.
    pub(in crate::check) fn write_back(&mut self) -> CompilerResult<()> {
        let failed = self.failed_generic_applications()?;

        // resolve committed node types, storing their reduced heads
        for node in self.node_types.nodes() {
            let ty = self.node_types.get(&node).expect("collected node type");
            let resolved = self.resolve_committed_type(ty, &failed)?;
            let resolved = match self.is_checking() {
                true => match self.node_origin_maybe(node) {
                    Some(origin) => self.deeply_normalize(origin, resolved)?,
                    None => resolved,
                },
                false => resolved,
            };
            self.node_types.insert(node, resolved);
            if self.module.types.get_node_type_id(node) != Some(resolved) {
                self.module.types_tail.set_node_type(node, resolved);
            }
        }

        // resolve contextual expectations that differ from their node types
        for node in self.expected_types.nodes() {
            let ty = self
                .expected_types
                .get(&node)
                .expect("collected expected type");
            let resolved = self.resolve_committed_type(ty, &failed)?;
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
            let resolved = self.resolve_committed_type(ty, &failed)?;
            self.declaration_types[index] = resolved;
            if self.module.types.get_symbol_type_id(symbol) != Some(resolved) {
                self.module.types_tail.set_symbol_type(symbol, resolved);
            }
        }

        // resolve binding types
        for index in 0..self.binding_types.len() {
            let (symbol, ty) = self
                .binding_types
                .get_index(index)
                .map(|(k, v)| (*k, *v))
                .expect("indexed entry");
            let resolved = self.resolve_committed_type(ty, &failed)?;
            self.binding_types[index] = resolved;
            if self.module.types.get_symbol_type_id(symbol) != Some(resolved) {
                self.module.types_tail.set_symbol_type(symbol, resolved);
            }
        }

        // resolve every interned variable row so the artifact closes
        for index in 0..self.infer.variable_count() {
            let variable = dir::TypeVariableId(index as u32);
            let Some(row) = self
                .module
                .types_tail
                .find_type(&dir::Type::Variable(variable))
            else {
                continue;
            };
            let solution = self.infer.solution(variable)?;
            let content = match solution {
                Some(solution) if !self.type_flags(solution)?.has_variable() => {
                    self.ty(solution)?
                }
                _ => dir::Type::Error,
            };
            let flags = match &content {
                dir::Type::Error => dir::TypeFlags::HAS_ERROR,
                _ => self.type_flags(solution.expect("resolved variable solution"))?,
            };
            self.module.types_tail.resolve_row(row, content, flags);
        }

        Ok(())
    }

    /// Resolve one committed type, poisoning the given failed applications.
    pub(in crate::check) fn resolve_committed_type(
        &mut self,
        ty: dir::GlobalTypeId,
        failed: &FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.resolve_head(ty)?;
        if failed.is_empty() && !self.type_flags(ty)?.has_variable() {
            return Ok(ty);
        }

        // track the roots on the active path to break cycles
        let mut active = FxIndexSet::default();

        self.resolve_open_type(ty, failed, &mut active)
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
                    let solution = self.resolve_head(solution)?;

                    self.resolve_open_type(solution, failed, active)?
                }
                // leave unsolved holes open for later passes
                None if self.is_declaration() => id,
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
