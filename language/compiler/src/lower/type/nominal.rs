use std::collections::HashSet;

use destack_dir::{GlobalSymbolId, LocalNodeId};

use crate::lower::{
    FieldInput, LayoutPolicy, compute_struct_layout, compute_struct_layout_with_prefix,
    size_and_align_of_type, static_key_to_field_name,
};
use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Predeclare nominal layouts for struct and class instance types.
    pub(crate) fn predeclare_nominal_layouts(&mut self) -> LowerResult<()> {
        // track symbols already handled to avoid duplicate work
        let mut seen_symbols = HashSet::new();

        // scan declarations for nominal types
        for (_declaration_id, declaration) in self
            .dir_tree
            .iter_nodes_of_type::<destack_dir::Declaration>()
        {
            let symbol = match declaration {
                destack_dir::Declaration::Struct { descriptor, .. }
                | destack_dir::Declaration::Class { descriptor, .. } => {
                    descriptor.symbol.into_global(self.module_id)
                }
                _ => continue,
            };

            if !seen_symbols.insert(symbol) {
                continue;
            }

            self.predeclare_nominal_layout_for_symbol(symbol)?;
        }

        Ok(())
    }

    /// Predeclare interface reference types for this module.
    pub(crate) fn predeclare_interface_reference_types(&mut self) -> LowerResult<()> {
        // track interface symbols already processed
        let mut seen_symbols = HashSet::new();

        // scan interface declarations
        for (declaration_id, declaration) in self
            .dir_tree
            .iter_nodes_of_type::<destack_dir::Declaration>()
        {
            // skip non interface declarations
            let destack_dir::Declaration::Interface { descriptor, .. } = declaration else {
                continue;
            };

            // deduplicate symbols across declarations
            let symbol = descriptor.symbol.into_global(self.module_id);
            if !seen_symbols.insert(symbol) {
                continue;
            }

            // build a stable anchor for diagnostics
            let anchor = declaration_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));

            // resolve reference type ids for this interface
            let reference_type_ids = self.nominal_reference_type_ids_for_symbol(symbol);
            for reference_type_id in reference_type_ids {
                // skip cached types
                if self
                    .type_lowerer
                    .type_cache
                    .contains_key(&reference_type_id)
                {
                    continue;
                }

                // lower the interface reference type
                let _ = self.type_lowerer.lower_type(
                    self.types,
                    reference_type_id,
                    self.module_id,
                    anchor,
                    &mut self.builder,
                )?;
            }
        }

        Ok(())
    }

    /// Predeclare the instance layout for a struct or class symbol.
    fn predeclare_nominal_layout_for_symbol(&mut self, symbol: GlobalSymbolId) -> LowerResult<()> {
        // skip when no instance type is registered
        let Some(instance_type_id) = self.types.get_instance_type_id(symbol) else {
            return Ok(());
        };

        // skip when already cached
        if self.type_lowerer.type_cache.contains_key(&instance_type_id) {
            return Ok(());
        }

        // collect field inputs from declaration members
        let field_inputs = self.collect_nominal_field_inputs(symbol)?;

        // compute and cache the layout for this instance type
        let layout = if symbol.ty() == destack_dir::SymbolType::Class
            && self.class_has_virtual_methods(symbol)?
        {
            let pointer_bytes = self.type_lowerer.pointer_bytes();
            let vtable_name = self.builder.intern("@vtable");
            let vtable_type = self.builder.type_raw_pointer(self.type_lowerer.ty_void);
            let (size, alignment) = size_and_align_of_type(
                self.builder.tree().get(vtable_type),
                self.builder.tree(),
                pointer_bytes,
            );

            let vtable_field = FieldInput {
                name: vtable_name,
                ty: vtable_type,
                size,
                alignment,
                source_index: None,
            };

            compute_struct_layout_with_prefix(vtable_field, field_inputs, LayoutPolicy::default())
        } else {
            compute_struct_layout(field_inputs, LayoutPolicy::default())
        };

        let mir_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);
        self.type_lowerer.set_layout(mir_type, layout);
        self.type_lowerer
            .type_cache
            .insert(instance_type_id, mir_type);

        // predeclare nominal reference types for class symbols
        if symbol.ty() == destack_dir::SymbolType::Class {
            let source = self.types.get_type_source(instance_type_id);
            let anchor = source
                .into_global(self.module_id)
                .into_anchored(Some(self.profile));
            let reference_type_ids = self.nominal_reference_type_ids_for_symbol(symbol);
            if reference_type_ids.is_empty() {
                return Err(LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "class missing nominal reference type".to_string(),
                });
            }

            for reference_type_id in reference_type_ids {
                if self
                    .type_lowerer
                    .type_cache
                    .contains_key(&reference_type_id)
                {
                    continue;
                }

                let _ = self.type_lowerer.lower_type(
                    self.types,
                    reference_type_id,
                    self.module_id,
                    anchor,
                    &mut self.builder,
                )?;
            }
        }

        Ok(())
    }

    /// Collect field inputs for a nominal struct or class instance layout.
    fn collect_nominal_field_inputs(
        &mut self,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Vec<FieldInput>> {
        // gather all declarations that contribute to this symbol
        let declaration_ids = self.declaration_ids_for_symbol(symbol);
        if declaration_ids.is_empty() {
            return Ok(Vec::new());
        }

        // set up layout collection state
        let pointer_bytes = self.type_lowerer.pointer_bytes();
        let mut field_inputs = Vec::new();
        let mut seen_fields = HashSet::new();
        let mut source_index = 0u32;

        // collect field members from declarations
        for declaration_id in declaration_ids {
            let declaration = self.dir_tree.get(declaration_id);
            let members = match declaration {
                destack_dir::Declaration::Struct { members, .. }
                | destack_dir::Declaration::Class { members, .. } => members,
                _ => continue,
            };

            for member_id in members {
                let member = self.dir_tree.get(*member_id);
                let destack_dir::Member::Field {
                    modifiers,
                    key,
                    value,
                    ..
                } = member
                else {
                    continue;
                };

                // skip static fields for instance layout
                if self.member_is_static(modifiers.as_ref()) {
                    continue;
                }

                // resolve a static key for layout naming
                let Some(key) = key.and_then(|key| {
                    self.compiler.static_key_from_dynamic_key(
                        self.profile,
                        key,
                        self.dir_tree,
                        self.symbols,
                        self.types,
                    )
                }) else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported dynamic field key in nominal layout".to_string(),
                    });
                };

                // require a declared field type
                let value_id = value.ok_or_else(|| LowerError::MissingType {
                    node: member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;
                let type_id = self
                    .types
                    .get_declared_or_inferred_type_id(value_id.into_global_any(self.module_id))
                    .ok_or_else(|| LowerError::MissingType {
                        node: value_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    })?;

                // lower the field type and compute layout metrics
                let anchor = member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile));
                let mir_type = self.type_lowerer.lower_type(
                    self.types,
                    type_id,
                    self.module_id,
                    anchor,
                    &mut self.builder,
                )?;
                let field_type = self.builder.tree().get(mir_type);
                let (size, alignment) =
                    size_and_align_of_type(field_type, self.builder.tree(), pointer_bytes);

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
                });
                source_index += 1;
            }
        }

        Ok(field_inputs)
    }

    /// Collect declaration ids that belong to a symbol in this module.
    pub(crate) fn declaration_ids_for_symbol(
        &self,
        symbol: GlobalSymbolId,
    ) -> Vec<LocalNodeId<destack_dir::Declaration>> {
        // read the symbol table entry for declaration lists
        let symbol_entry = self.symbols.get_symbol(symbol.local_id);
        let mut declaration_ids = Vec::new();

        // add primary declaration first
        if let Some(primary) = symbol_entry.primary_declaration
            && primary.module_id == self.module_id
            && let Ok(local_id) = primary
                .local_id
                .try_into_typed::<destack_dir::Declaration>()
        {
            declaration_ids.push(local_id);
        }

        // add secondary declarations in order
        if let Some(secondary) = symbol_entry.secondary_declarations.as_deref() {
            for declaration_id in secondary {
                if declaration_id.module_id != self.module_id {
                    continue;
                }
                if let Ok(local_id) = declaration_id
                    .local_id
                    .try_into_typed::<destack_dir::Declaration>()
                {
                    declaration_ids.push(local_id);
                }
            }
        }

        declaration_ids
    }

    /// Return true if a member is marked static.
    pub(crate) fn member_is_static(
        &self,
        modifiers: Option<&destack_dir::BindingModifier>,
    ) -> bool {
        modifiers
            .is_some_and(|modifiers| modifiers.anchor == Some(destack_dir::BindingAnchor::Static))
    }
}
