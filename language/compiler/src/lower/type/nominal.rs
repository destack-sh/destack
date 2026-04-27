use std::collections::HashSet;

use destack_core::StringId;
use destack_dir::{self as dir};
use destack_mir as mir;

use crate::lower::{
    FieldInput, FieldLayoutKind, LayoutPolicy, TypeCacheEntry, static_key_to_field_name,
};
use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower and cache the instance layout for a struct or class symbol.
    pub(crate) fn lower_nominal_layout(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // check for cached layout
        if let Some(mir_type) = self.nominal_layouts_by_symbol.get(&symbol).copied() {
            return Ok(mir_type);
        }

        // check for cycles
        if self.nominal_layouts_in_progress.contains(&symbol) {
            let anchor = self
                .declaration_ids_for_symbol(symbol)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    module: self.module_id,
                    message: "nominal layout cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "cycle detected while lowering nominal layout".to_string(),
            });
        }
        self.nominal_layouts_in_progress.insert(symbol);

        // resolve the instance type id
        let Some(instance_type_id) = self.types.get_instance_type_id(symbol) else {
            let anchor = self
                .declaration_ids_for_symbol(symbol)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    module: self.module_id,
                    message: "nominal type missing declaration for instance type".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "nominal type missing instance type".to_string(),
            });
        };

        // resolve a stable declaration for diagnostics
        let instance_declaration = self
            .types
            .get_type_source(instance_type_id)
            .into_global(self.module_id)
            .into_anchored(Some(self.profile));

        // resolve the class base symbol
        let base_symbol = self
            .types
            .get_lineage_for_symbol(symbol)
            .and_then(|lineage| lineage.extends)
            .filter(|_| symbol.ty() == dir::SymbolType::Class);

        // predeclare the base layout for derived classes
        if let Some(base_symbol) = base_symbol {
            self.lower_nominal_layout(base_symbol)?;
        }

        // prepare derived field tracking
        let mut seen_fields = HashSet::new();
        let mut source_index_offset = 0u32;

        // load the base layout prefix when present
        let base_layout = if let Some(base_symbol) = base_symbol {
            let base_instance_type_id =
                self.types
                    .get_instance_type_id(base_symbol)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: instance_declaration,
                        message: "class base instance type missing".to_string(),
                    })?;
            let base_mir_type = self
                .type_lowerer
                .cached_type(base_instance_type_id)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: instance_declaration,
                    message: "class base layout missing".to_string(),
                })?;
            let base_layout = self
                .type_lowerer
                .layout_for_type_or_error(base_mir_type, instance_declaration)?
                .clone();

            for field in &base_layout.fields {
                seen_fields.insert(field.name);
                if field.source_index.is_some() {
                    source_index_offset += 1;
                }
            }

            Some(base_layout)
        } else {
            None
        };

        // collect field inputs from declaration members
        let field_inputs =
            self.collect_nominal_field_inputs(symbol, &mut seen_fields, source_index_offset)?;

        // compute vtable layout symbols when needed
        if self.vtable_layout_symbols.is_none() {
            let symbols = self.collect_vtable_layout_symbols()?;
            self.vtable_layout_symbols = Some(symbols);
        }

        // compute and cache the layout for this instance type
        let has_vtable_header = self
            .vtable_layout_symbols
            .as_ref()
            .map(|symbols| symbols.contains(&symbol))
            .unwrap_or(false);
        let layout = if let Some(base_layout) = base_layout {
            self.type_lowerer.compute_struct_layout_with_base(
                base_layout,
                field_inputs,
                LayoutPolicy::default(),
            )
        } else if symbol.ty() == dir::SymbolType::Class && has_vtable_header {
            let vtable_name = self.vtable_field_name;
            let vtable_type = self.builder.type_reference(
                mir::ReferenceKind::Raw,
                self.type_lowerer.ty_void,
                mir::Mutability::Immutable,
                mir::AddressSpace::Static,
                false,
            );
            let (size, alignment) = self
                .type_lowerer
                .size_and_align_of_type(self.builder.tree().get(vtable_type), self.builder.tree())
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: instance_declaration,
                    message: "nominal layout requires concrete nested types".to_string(),
                })?;

            let vtable_field = FieldInput {
                name: vtable_name,
                ty: vtable_type,
                size,
                alignment,
                source_index: None,
                kind: FieldLayoutKind::VtableHeader,
            };

            self.type_lowerer.compute_struct_layout_with_prefix(
                vtable_field,
                field_inputs,
                LayoutPolicy::default(),
            )
        } else {
            self.type_lowerer
                .compute_struct_layout(field_inputs, LayoutPolicy::default())
        };

        let mir_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);
        self.type_lowerer.set_layout(mir_type, layout);
        self.type_lowerer
            .type_cache
            .insert(instance_type_id, TypeCacheEntry::Ready(mir_type));

        self.nominal_layouts_by_symbol.insert(symbol, mir_type);
        self.nominal_layouts_in_progress.shift_remove(&symbol);

        Ok(mir_type)
    }

    /// Collect field inputs for a nominal struct or class instance layout.
    fn collect_nominal_field_inputs(
        &mut self,
        symbol: dir::GlobalSymbolId,
        seen_fields: &mut HashSet<StringId>,
        source_index_offset: u32,
    ) -> LowerResult<Vec<FieldInput>> {
        // gather all declarations that contribute to this symbol
        let declaration_ids = self.declaration_ids_for_symbol(symbol);
        if declaration_ids.is_empty() {
            return Ok(Vec::new());
        }

        // set up layout collection state
        let mut field_inputs = Vec::new();
        let mut source_index = source_index_offset;

        // collect field members from declarations
        for declaration_id in declaration_ids {
            let declaration = self.dir_tree.get(declaration_id);
            let members = match declaration {
                dir::Declaration::Struct(declaration) => &declaration.members,
                dir::Declaration::Class(declaration) => &declaration.members,
                _ => continue,
            };

            for member_id in members {
                let member = self.dir_tree.get(*member_id);
                let dir::Member::Field {
                    key,
                    declared_type,
                    is_static,
                    ..
                } = member
                else {
                    continue;
                };

                // skip static fields for instance layout
                if *is_static {
                    continue;
                }

                // resolve a static key for layout naming
                let Some(key) = self.compiler.static_key_from_key(self.dir_tree, *key) else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported dynamic field key in nominal layout".to_string(),
                    });
                };

                // require a declared field type
                let declared_type = declared_type.ok_or_else(|| {
                    self.missing_type_error(member_id.into_global_any(self.module_id))
                })?;
                let type_id = self.declared_or_inferred_type_id_for_node_or_error(
                    declared_type.into_global_any(self.module_id),
                )?;

                // lower the field type and compute layout metrics
                let anchor = member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile));
                let mir_type = self.lower_type(type_id, anchor)?;
                // skip void fields that lower to no storage
                if mir_type == self.type_lowerer.ty_void {
                    continue;
                }
                let field_type = self.builder.tree().get(mir_type);
                let (size, alignment) = self
                    .type_lowerer
                    .size_and_align_of_type(field_type, self.builder.tree())
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: anchor,
                        message: "nominal layout requires concrete nested types".to_string(),
                    })?;

                // compute the layout field name
                let field_name = static_key_to_field_name(&key, &mut self.builder);
                if !seen_fields.insert(field_name) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "duplicate field in nominal layout".to_string(),
                    });
                }

                // record the layout input
                field_inputs.push(FieldInput {
                    name: field_name,
                    ty: mir_type,
                    size,
                    alignment,
                    source_index: Some(source_index),
                    kind: FieldLayoutKind::Source,
                });
                source_index += 1;
            }
        }

        Ok(field_inputs)
    }

    /// Collect class symbols that require vtable headers.
    pub(crate) fn collect_vtable_layout_symbols(
        &self,
    ) -> LowerResult<HashSet<dir::GlobalSymbolId>> {
        // collect class symbols in declaration order
        let mut class_symbols = Vec::new();
        for (_declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let dir::Declaration::Class(declaration) = declaration else {
                continue;
            };
            class_symbols.push(declaration.symbol.into_global(self.module_id));
        }

        // deduplicate and sort for determinism
        class_symbols.sort_by_key(|symbol| symbol.local_id.id);
        class_symbols.dedup();

        // expand the lineage for classes that need virtual headers
        let mut vtable_layout_symbols = HashSet::new();
        for symbol in class_symbols {
            if !self.class_has_virtual_methods(symbol)? {
                continue;
            }

            for base_symbol in self.collect_class_lineage(symbol) {
                vtable_layout_symbols.insert(base_symbol);
            }
        }

        Ok(vtable_layout_symbols)
    }

    /// Collect declaration ids that belong to a symbol in this module.
    pub(crate) fn declaration_ids_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::LocalNodeId<dir::Declaration>> {
        // read the symbol table entry for declaration lists
        let symbol_entry = self.symbols.get_symbol(symbol.local_id);
        let mut declaration_ids = Vec::new();

        // add primary declaration first
        if let Some(primary) = symbol_entry.primary_declaration
            && primary.module_id == self.module_id
            && let Ok(local_id) = primary.local_id.try_into_typed::<dir::Declaration>()
        {
            declaration_ids.push(local_id);
        }

        // add secondary declarations in order
        if let Some(secondary) = symbol_entry.secondary_declarations.as_deref() {
            for declaration_id in secondary {
                if declaration_id.module_id != self.module_id {
                    continue;
                }
                if let Ok(local_id) = declaration_id.local_id.try_into_typed::<dir::Declaration>() {
                    declaration_ids.push(local_id);
                }
            }
        }

        declaration_ids
    }
}
