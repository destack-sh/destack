use super::*;
use crate::analyze::common::TypeContext;
use crate::analyze::module::GlobalMergeCategory;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn unwrap_type_alias_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> AnalyzeResult<LocalTypeId> {
        let symbol = match ctx.types.get_type(type_id).symbol() {
            Some(symbol) => symbol,
            None => return Ok(type_id),
        };

        // only unwrap structural type aliases
        if symbol.ty() != SymbolType::TypeAlias {
            return Ok(type_id);
        }

        // skip remote aliases during local flow computation
        if symbol.module_id != ctx.module.id {
            return Ok(type_id);
        }

        // load the type alias declaration
        let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(type_id);
        };
        let declaration_id = match primary_declaration.local_id.try_into_typed::<Declaration>() {
            Ok(declaration_id) => declaration_id,
            Err(_) => return Ok(type_id),
        };
        let Declaration::Type {
            static_parameters,
            value,
            ..
        } = ctx.tree.get(declaration_id)
        else {
            return Ok(type_id);
        };

        // avoid eager evaluation for generic aliases
        if static_parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_empty())
        {
            return Ok(type_id);
        }

        // reuse any apparent instance type before evaluating the alias body
        let source_id = ctx.types.get_type_source(type_id);
        if let Some(instance_type_id) =
            self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
        {
            return Ok(instance_type_id);
        }

        // evaluate the alias value into an instance type
        let instance_type_id =
            self.resolve_declared_type_expression(&mut ctx.reborrow(), *value, true, true)?;
        ctx.types.set_instance_type(symbol, instance_type_id);

        Ok(instance_type_id)
    }

    /// Ensure instance types for any reference types inside a type.
    pub(crate) fn ensure_reference_instance_types_for_type(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
    ) -> AnalyzeResult<()> {
        let mut visited = HashSet::new();
        self.ensure_reference_instance_types_for_type_inner(
            &mut ctx.reborrow(),
            node_id,
            ty_id,
            &mut visited,
        )
    }

    /// Ensure instance types for any reference types inside a type.
    fn ensure_reference_instance_types_for_type_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // skip types we have already visited
        if !visited.insert(ty_id) {
            return Ok(());
        }

        // clone to avoid holding a borrow across recursion
        let ty = ctx.types.get_type(ty_id).clone();

        // ensure reference symbols have instance types
        if let Type::Reference { symbol, .. } = ty {
            let instance_id =
                self.resolve_instance_type_for_symbol(&mut ctx.reborrow(), node_id, symbol)?;
            if let Some(instance_id) = instance_id {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    instance_id,
                    visited,
                )?;
            }
            return Ok(());
        }

        // walk nested types based on structure
        match ty {
            Type::Value { value } => self.ensure_reference_instance_types_for_type_inner(
                &mut ctx.reborrow(),
                node_id,
                value,
                visited,
            ),
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    left,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    right,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    then_type,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    else_type,
                    visited,
                )?;
                Ok(())
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    parameter.constraint,
                    visited,
                )?;

                if let Some(key_remap) = parameter.key_remap {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        key_remap,
                        visited,
                    )?;
                }

                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    value,
                    visited,
                )
            }
            Type::Index { left, index } => {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    left,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    index,
                    visited,
                )
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        span,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        constraint,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        target,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => self.ensure_reference_instance_types_for_type_inner(
                &mut ctx.reborrow(),
                node_id,
                right,
                visited,
            ),
            Type::Binary { left, right, .. } => {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    left,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    right,
                    visited,
                )
            }
            Type::ArraySized { element, .. } => self
                .ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    element,
                    visited,
                ),
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        element,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        element.ty,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        field.ty,
                        visited,
                    )?;
                }

                for signature in call_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature,
                        visited,
                    )?;
                }

                for signature in construct_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature,
                        visited,
                    )?;
                }

                for signature in index_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature.key_type,
                        visited,
                    )?;
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature.value_type,
                        visited,
                    )?;
                }

                Ok(())
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                for parameter in static_parameters {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        parameter,
                        visited,
                    )?;
                }

                if let Some(this_parameter) = this_parameter {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        this_parameter,
                        visited,
                    )?;
                }

                for parameter in dynamic_parameters {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        parameter,
                        visited,
                    )?;
                }

                if let Some(return_type) = return_type {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        return_type,
                        visited,
                    )?;
                }

                Ok(())
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        element,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::This
            | Type::Reference { .. }
            | Type::Unevaluated(_)
            | Type::Import { .. }
            | Type::Error => Ok(()),
        }
    }

    /// Resolve the instance type for a referenced symbol into the local type table.
    pub(crate) fn resolve_instance_type_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // load symbol metadata for merge group selection
        let (
            symbol_type,
            symbol_key,
            symbol_space,
            symbol_is_global_augmentation,
            owner_is_ambient_lib,
        ) = {
            let symbol_module = self.program.modules.get(symbol.module_id);
            let symbol_module = symbol_module.read();
            let symbol_table = symbol_module.dir_base().symbols.read();
            let symbol_entry = symbol_table.get_symbol(symbol.local_id);
            (
                symbol_entry.ty,
                symbol_entry.key,
                symbol_entry.space,
                symbol_entry.origin.is_global_augmentation(),
                self.module_is_ambient_lib(&symbol_module),
            )
        };

        // normalize the symbol id to the stored symbol type
        let symbol = GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_type));

        // skip symbols that cannot have instance types
        if !self.query_symbol_is_instantiable(symbol) {
            return Ok(None);
        }

        // ensure the defining module is declared before reading its types
        if symbol.module_id != ctx.module.id {
            self.require_dir_declared(symbol.module_id, ctx.profile)
                .map_err(AnalyzeError::from)?;
        }

        // ensure the global symbol table is available for this module
        self.require_dir_resolved(ctx.module.id, ctx.profile)
            .map_err(AnalyzeError::from)?;

        // select the global merge group when the symbol participates
        let mut group_symbols = if symbol_is_global_augmentation || owner_is_ambient_lib {
            if let Some(key) = symbol_key {
                self.collect_global_merge_sources_for_key(
                    ctx.module,
                    ctx.profile,
                    key,
                    symbol_space,
                    GlobalMergeCategory::Instance,
                )
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        group_symbols.push(symbol);
        let mut seen = HashSet::new();
        group_symbols.retain(|symbol| seen.insert(*symbol));

        // normalize group symbols to the stored symbol types
        let mut normalized_group_symbols = Vec::with_capacity(group_symbols.len());
        for group_symbol in group_symbols {
            let group_module = self.program.modules.get(group_symbol.module_id);
            let group_module = group_module.read();
            let group_symbol_table = group_module.dir_base().symbols.read();
            let group_entry = group_symbol_table.get_symbol(group_symbol.local_id);
            let normalized = GlobalSymbolId::new(
                group_symbol.module_id,
                group_symbol.local_id.with_type(group_entry.ty),
            );
            normalized_group_symbols.push(normalized);
        }
        let group_symbols = normalized_group_symbols;

        // reuse cached instance types when no merge is needed
        if group_symbols.len() == 1
            && let Some(existing) = ctx.types.get_instance_type_id(symbol)
        {
            return Ok(Some(existing));
        }

        // reuse an already merged instance type when available
        if group_symbols.len() > 1 {
            let mut merged_id = None;
            let mut all_match = true;
            for group_symbol in &group_symbols {
                let Some(group_instance_id) = ctx.types.get_instance_type_id(*group_symbol) else {
                    all_match = false;
                    break;
                };
                if let Some(existing) = merged_id {
                    if existing != group_instance_id {
                        all_match = false;
                        break;
                    }
                } else {
                    merged_id = Some(group_instance_id);
                }
            }

            if all_match && let Some(merged_id) = merged_id {
                return Ok(Some(merged_id));
            }
        }

        // import instance types for each group symbol
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();
        let mut own_instance_id = None;

        for group_symbol in group_symbols.iter().copied() {
            // load the instance type for the group symbol
            let local_instance_id =
                if let Some(existing) = ctx.types.get_instance_type_id(group_symbol) {
                    existing
                } else if group_symbol.module_id == ctx.module.id {
                    continue;
                } else {
                    let Some(imported) = self.import_instance_type_for_symbol(
                        ctx.profile,
                        node_id,
                        group_symbol,
                        ctx.types,
                    )?
                    else {
                        continue;
                    };
                    imported
                };

            if group_symbol == symbol {
                own_instance_id = Some(local_instance_id);
            }

            // extract instance type members
            let local_instance_ty = ctx.types.get_type(local_instance_id);
            if let Type::Object {
                fields: instance_fields,
                call_signatures: instance_calls,
                construct_signatures: instance_constructs,
                index_signatures: instance_indexes,
            } = local_instance_ty
            {
                fields.extend_from_slice(instance_fields);
                call_signatures.extend_from_slice(instance_calls);
                construct_signatures.extend_from_slice(instance_constructs);
                index_signatures.extend_from_slice(instance_indexes);
            }
        }

        // handle non mergeable instances and empty merges
        if group_symbols.len() == 1
            || (fields.is_empty()
                && call_signatures.is_empty()
                && construct_signatures.is_empty()
                && index_signatures.is_empty())
        {
            if let Some(own_instance_id) = own_instance_id {
                ctx.types.set_instance_type(symbol, own_instance_id);
            }
            return Ok(own_instance_id);
        }

        // create a merged instance type for all group symbols
        let merged_ty = Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        };
        let merged_id = ctx.types.insert_type_from_any(merged_ty, node_id);
        for group_symbol in group_symbols {
            ctx.types.set_instance_type(group_symbol, merged_id);
        }

        Ok(Some(merged_id))
    }

    /// Import a remote instance type into the local type table.
    pub(crate) fn import_instance_type_for_symbol(
        &self,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let remote_instance = self
            .with_module_tree_symbols_by_id_at_boundary(
                profile,
                symbol.module_id,
                DirReadBoundary::Declared,
                |remote_module, _remote_tree, _remote_symbols| {
                    let remote_dir = remote_module.dir(profile);
                    let remote_types = remote_dir.types.read();
                    let remote_instance_id = remote_types.get_instance_type_id(symbol)?;
                    let remote_instance_ty = remote_types.get_type(remote_instance_id).clone();
                    let remote_snapshot = remote_types.clone();
                    Some((remote_instance_ty, remote_snapshot))
                },
            )
            .map_err(AnalyzeError::from)?;

        Ok(
            remote_instance.map(|(remote_instance_ty, remote_snapshot)| {
                self.import_remote_type_for_node(
                    node_id,
                    &remote_instance_ty,
                    &remote_snapshot,
                    types,
                )
            }),
        )
    }
}
