use super::*;
use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::analyze::module::GlobalMergeCategory;

/// The scope for eager instance preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstancePreparationScope {
    /// Prepare local and remote reference instance types.
    All,
    /// Prepare only references owned by the current type context module.
    LocalOnly,
}

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
        let Declaration::Type(declaration) = ctx.tree.get(declaration_id) else {
            return Ok(type_id);
        };

        // avoid eager evaluation for generic aliases
        if !declaration.generic_parameters.is_empty() {
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
        let instance_type_id = self.resolve_declared_type_expression(
            &mut ctx.reborrow(),
            declaration.value,
            true,
            true,
        )?;
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
            InstancePreparationScope::All,
            &mut visited,
        )
    }

    /// Ensure local instance types for any reference types inside a type.
    pub(crate) fn ensure_local_reference_instance_types_for_type(
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
            InstancePreparationScope::LocalOnly,
            &mut visited,
        )
    }

    /// Ensure instance types for any reference types inside a type.
    fn ensure_reference_instance_types_for_type_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
        preparation_scope: InstancePreparationScope,
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
            let instance_id = match preparation_scope {
                InstancePreparationScope::All => {
                    self.resolve_instance_type_for_symbol(&mut ctx.reborrow(), node_id, symbol)?
                }
                InstancePreparationScope::LocalOnly if symbol.module_id == ctx.module.id => {
                    self.resolve_instance_type_for_symbol(&mut ctx.reborrow(), node_id, symbol)?
                }
                InstancePreparationScope::LocalOnly => None,
            };
            if let Some(instance_id) = instance_id {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    instance_id,
                    preparation_scope,
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
                preparation_scope,
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
                    preparation_scope,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    right,
                    preparation_scope,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    then_type,
                    preparation_scope,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    else_type,
                    preparation_scope,
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
                    preparation_scope,
                    visited,
                )?;

                if let Some(key_remap) = parameter.key_remap {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        key_remap,
                        preparation_scope,
                        visited,
                    )?;
                }

                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    value,
                    preparation_scope,
                    visited,
                )
            }
            Type::Index { left, index } => {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    left,
                    preparation_scope,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    index,
                    preparation_scope,
                    visited,
                )
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        span,
                        preparation_scope,
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
                        preparation_scope,
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
                        preparation_scope,
                        visited,
                    )?;
                }
                Ok(())
            }
            Type::Readonly { target_type: right }
            | Type::KeyOf { target_type: right }
            | Type::Must { target_type: right }
            | Type::AsComptime { target_type: right }
            | Type::Not { target_type: right }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => self.ensure_reference_instance_types_for_type_inner(
                &mut ctx.reborrow(),
                node_id,
                right,
                preparation_scope,
                visited,
            ),
            Type::In { left, right }
            | Type::Extends { left, right }
            | Type::Implements { left, right } => {
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    left,
                    preparation_scope,
                    visited,
                )?;
                self.ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    right,
                    preparation_scope,
                    visited,
                )
            }
            Type::ArraySized { element, .. } => self
                .ensure_reference_instance_types_for_type_inner(
                    &mut ctx.reborrow(),
                    node_id,
                    element,
                    preparation_scope,
                    visited,
                ),
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        element,
                        preparation_scope,
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
                        preparation_scope,
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
                        preparation_scope,
                        visited,
                    )?;
                }

                for signature in call_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature,
                        preparation_scope,
                        visited,
                    )?;
                }

                for signature in construct_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature,
                        preparation_scope,
                        visited,
                    )?;
                }

                for signature in index_signatures {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature.key_type,
                        preparation_scope,
                        visited,
                    )?;
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        signature.value_type,
                        preparation_scope,
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
                        preparation_scope,
                        visited,
                    )?;
                }

                if let Some(this_parameter) = this_parameter {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        this_parameter,
                        preparation_scope,
                        visited,
                    )?;
                }

                for parameter in dynamic_parameters {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        parameter,
                        preparation_scope,
                        visited,
                    )?;
                }

                if let Some(return_type) = return_type {
                    self.ensure_reference_instance_types_for_type_inner(
                        &mut ctx.reborrow(),
                        node_id,
                        return_type,
                        preparation_scope,
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
                        preparation_scope,
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
        // load symbol metadata from the declared boundary
        let (
            symbol_type,
            symbol_key,
            symbol_space,
            symbol_is_global_augmentation,
            owner_is_ambient_lib,
        ) = self
            .with_module_symbols_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol.module_id,
                ctx.symbols,
                destack_artifact::ArtifactKey::dir_declared,
                |owner_module, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                    (
                        symbol_entry.ty,
                        symbol_entry.key,
                        symbol_entry.space,
                        symbol_entry.origin.is_global_augmentation(),
                        self.module_is_ambient_lib(owner_module),
                    )
                },
            )
            .map_err(AnalyzeError::from)?;

        // normalize the symbol id to the stored symbol type
        let symbol = GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(symbol_type));

        // skip symbols that cannot have instance types
        if !self.query_symbol_is_instantiable(symbol) {
            return Ok(None);
        }

        // reuse any already imported or merged instance type first
        if let Some(existing) = ctx.types.get_instance_type_id(symbol) {
            return Ok(Some(existing));
        }

        // ensure the defining module is declared before reading its types
        if symbol.module_id != ctx.module.id {
            self.require_dir_declared(
                ctx.compiler_context.revision(),
                symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        }

        // ensure the global symbol table is available for this module
        self.require_dir_resolved(ctx.compiler_context.revision(), ctx.module.id, ctx.profile)
            .map_err(AnalyzeError::from)?;

        // select the global merge group when the symbol participates
        let mut group_symbols = if symbol_is_global_augmentation || owner_is_ambient_lib {
            if let Some(key) = symbol_key {
                self.collect_global_merge_sources_for_key(
                    ctx.compiler_context.revision(),
                    ctx.module,
                    &ctx.index,
                    ctx.symbols,
                    ctx.profile,
                    key,
                    symbol_space,
                    GlobalMergeCategory::Instance,
                )?
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
            let normalized = self
                .with_module_symbols_or_local_for_artifact(
                    ctx.compiler_context,
                    ctx.module,
                    ctx.profile,
                    group_symbol.module_id,
                    ctx.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |owner_module, owner_symbols| {
                        let group_entry = owner_symbols.get_symbol(group_symbol.local_id);
                        GlobalSymbolId::new(
                            owner_module.id,
                            group_symbol.local_id.with_type(group_entry.ty),
                        )
                    },
                )
                .map_err(AnalyzeError::from)?;
            normalized_group_symbols.push(normalized);
        }
        let group_symbols = normalized_group_symbols;

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
                } else {
                    let Some(imported) = self.import_instance_type_for_symbol(
                        &ctx.index,
                        ctx.compiler_context.revision(),
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

            // rewrite peer instance surfaces into the queried carrier space
            let local_instance_id = self.remap_merged_owner_parameters_in_type(
                &mut ctx.reborrow(),
                node_id,
                group_symbol,
                symbol,
                local_instance_id,
            );

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

    /// Query the instance type for a symbol when dependency state is ready.
    pub(crate) fn query_instance_type_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        match self.resolve_instance_type_for_symbol(&mut ctx.reborrow(), node_id, symbol) {
            Ok(type_id) => type_id,
            Err(AnalyzeError::Yield { .. } | AnalyzeError::UnsatisfiedRequirement { .. }) => None,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Import a remote instance type into the local type table.
    pub(crate) fn import_instance_type_for_symbol(
        &self,
        index: &AnalyzeIndex,
        revision: destack_workspace::Revision,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // same-module reads should reuse the current local type table
        if symbol.module_id == types.module_id {
            return Ok(types.get_instance_type_id(symbol));
        }

        // declared instance shapes are the earliest stable boundary for class, struct,
        // interface, and extension member access
        let snapshot = self
            .require_indexed_dir_declared(index, revision, symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;
        let remote_types = &snapshot.types;
        let Some(remote_instance_id) = remote_types.get_instance_type_id(symbol) else {
            return Ok(None);
        };

        let remote_instance_ty = remote_types.get_type(remote_instance_id).clone();
        let remote_snapshot = remote_types.clone();
        let imported =
            self.import_remote_type_for_node(node_id, &remote_instance_ty, &remote_snapshot, types);

        Ok(Some(imported))
    }
}
