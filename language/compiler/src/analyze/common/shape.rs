use std::collections::HashSet;

use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, LocalTypeId, NodeTree,
    StaticKey, SymbolSpace, SymbolTable, SymbolType, Type, TypeField, TypeIndexSignature,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Store object type members for shape assembly.
#[derive(Debug, Default, Clone)]
pub(crate) struct ObjectShape {
    /// Fields collected for the shape.
    pub(crate) fields: Vec<TypeField>,
    /// Call signatures collected for the shape.
    pub(crate) call_signatures: Vec<LocalTypeId>,
    /// Construct signatures collected for the shape.
    pub(crate) construct_signatures: Vec<LocalTypeId>,
    /// Index signatures collected for the shape.
    pub(crate) index_signatures: Vec<TypeIndexSignature>,
}

impl ObjectShape {
    /// Extend this set with another shape.
    pub(crate) fn extend_from_shape(&mut self, other: &ObjectShape) {
        // append fields and signatures
        self.fields.extend_from_slice(&other.fields);
        self.call_signatures
            .extend_from_slice(&other.call_signatures);
        self.construct_signatures
            .extend_from_slice(&other.construct_signatures);
        self.index_signatures
            .extend_from_slice(&other.index_signatures);
    }

    /// Add a call signature to this shape.
    pub(crate) fn push_call_signature(&mut self, signature: LocalTypeId) {
        // record the call signature
        self.call_signatures.push(signature);
    }

    /// Extend this set from an object type.
    pub(crate) fn extend_from_object(&mut self, ty: &Type) -> bool {
        // unwrap the object members
        let Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        } = ty
        else {
            return false;
        };

        // append the object members into this shape
        self.fields.extend_from_slice(fields);
        self.call_signatures.extend_from_slice(call_signatures);
        self.construct_signatures
            .extend_from_slice(construct_signatures);
        self.index_signatures.extend_from_slice(index_signatures);
        true
    }

    /// Convert into an object type.
    pub(crate) fn into_object_type(self) -> Type {
        // build a type object from the collected members
        Type::Object {
            fields: self.fields,
            call_signatures: self.call_signatures,
            construct_signatures: self.construct_signatures,
            index_signatures: self.index_signatures,
        }
    }

    /// Check whether the shape has any members.
    pub(crate) fn is_empty(&self) -> bool {
        self.fields.is_empty()
            && self.call_signatures.is_empty()
            && self.construct_signatures.is_empty()
            && self.index_signatures.is_empty()
    }

    /// Apply a direct field override to the shape.
    pub(crate) fn apply_field(&mut self, field: TypeField) {
        // remove existing field with the same key
        self.fields
            .retain(|existing| !existing.key.matches(&field.key));
        self.fields.push(field);
    }

    /// Apply another shape to this one with override semantics.
    pub(crate) fn apply_spread(&mut self, spread: &ObjectShape) {
        // override fields on matching keys
        for field in &spread.fields {
            self.fields
                .retain(|existing| !existing.key.matches(&field.key));
            self.fields.push(field.clone());
        }

        // append signatures from the spread
        self.call_signatures
            .extend_from_slice(&spread.call_signatures);
        self.construct_signatures
            .extend_from_slice(&spread.construct_signatures);
        self.index_signatures
            .extend_from_slice(&spread.index_signatures);
    }

    /// Apply another shape to this one for intersection semantics.
    pub(crate) fn apply_intersection(&mut self, other: &ObjectShape, types: &mut TypeTable) {
        // merge fields by intersecting overlapping keys
        for field in &other.fields {
            if let Some(existing) = self
                .fields
                .iter_mut()
                .find(|existing| existing.key.matches(&field.key))
            {
                let merged_type = if existing.ty == field.ty {
                    existing.ty
                } else {
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: vec![existing.ty, field.ty],
                        },
                        existing.ty,
                    )
                };
                existing.ty = merged_type;
                existing.is_optional = existing.is_optional && field.is_optional;
                existing.is_readonly = existing.is_readonly || field.is_readonly;
            } else {
                self.fields.push(field.clone());
            }
        }

        self.call_signatures
            .extend_from_slice(&other.call_signatures);
        self.construct_signatures
            .extend_from_slice(&other.construct_signatures);
        self.index_signatures
            .extend_from_slice(&other.index_signatures);
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check whether a symbol should receive merged instance members.
    fn symbol_supports_instance_merge(&self, symbol: &destack_dir::Symbol) -> bool {
        // only structured nominal symbols support instance merging
        matches!(
            symbol.ty,
            SymbolType::Interface | SymbolType::Class | SymbolType::Struct | SymbolType::Enum
        )
    }

    /// Merge instance shape into a symbol instance type.
    fn merge_instance_shape_into_symbol(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        shape: &ObjectShape,
        types: &mut TypeTable,
        allow_merge: bool,
    ) -> LocalTypeId {
        // seed the merge with any existing instance members
        let symbol = symbol_id.into_global(module.id);
        let mut merged_shape = ObjectShape::default();

        // reuse existing instance members when merges are allowed
        if allow_merge && let Some(existing_id) = types.get_instance_type_id(symbol) {
            let existing_ty = types.get_type(existing_id);
            merged_shape.extend_from_object(existing_ty);
        }

        // merge the new shape into the instance type
        merged_shape.extend_from_shape(shape);
        let instance_ty = merged_shape.into_object_type();
        let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
        types.set_instance_type(symbol, instance_ty_id);
        instance_ty_id
    }

    /// Collect value members and extras from a value type.
    pub(crate) fn collect_value_shape_from_type(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        shape: &mut ObjectShape,
        extras: &mut Vec<LocalTypeId>,
        visited: &mut Vec<LocalTypeId>,
    ) {
        // avoid recursion loops in cyclic type graphs
        if visited.contains(&ty_id) {
            return;
        }
        visited.push(ty_id);

        // collect shape data based on the value type
        let ty = types.get_type(ty_id);
        match ty {
            Type::Object { .. } => {
                shape.extend_from_object(ty);
            }
            Type::Function { .. } => {
                shape.call_signatures.push(ty_id);
            }
            Type::Intersection { elements } => {
                for element_id in elements {
                    self.collect_value_shape_from_type(*element_id, types, shape, extras, visited);
                }
            }
            Type::Value { .. } => {}
            _ => extras.push(ty_id),
        }
    }

    /// Collect embedded fields and index signatures for an embed member.
    pub(crate) fn embed_member_shape(
        &self,
        module: &Module,
        profile: ProfileId,
        value: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<ObjectShape> {
        // resolve the embed target type
        let embed_ty_id = self.try_evaluate_expression_to_type(
            module, profile, value, tree, symbols, types, true, true,
        )?;

        // prefer instance types for nominal references
        let embed_ty_id = match types.get_type(embed_ty_id) {
            Type::Reference { symbol, .. } => {
                types.get_instance_type_id(*symbol).unwrap_or(embed_ty_id)
            }
            Type::Value { value } => *value,
            _ => embed_ty_id,
        };

        // collect field-like members from the embedded type
        let mut embed_shape = ObjectShape::default();
        let mut extras = Vec::new();
        let mut visited = Vec::new();
        self.collect_value_shape_from_type(
            embed_ty_id,
            types,
            &mut embed_shape,
            &mut extras,
            &mut visited,
        );

        // embedding only contributes fields and index signatures
        embed_shape.call_signatures.clear();
        embed_shape.construct_signatures.clear();

        Ok(embed_shape)
    }

    /// Merge value shape into a symbol value type.
    pub(crate) fn merge_value_shape_into_symbol(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        shape: &ObjectShape,
        types: &mut TypeTable,
        allow_merge: bool,
    ) -> LocalTypeId {
        // seed the merge with any existing value members
        let symbol = symbol_id.into_global(module.id);
        let mut merged_shape = ObjectShape::default();
        let mut extras = Vec::new();

        // reuse existing value members when merges are allowed
        if allow_merge && let Some(existing_id) = types.get_value_type_id(symbol) {
            let mut visited = Vec::new();
            self.collect_value_shape_from_type(
                existing_id,
                types,
                &mut merged_shape,
                &mut extras,
                &mut visited,
            );
        }

        // merge the new shape into the value shape
        merged_shape.extend_from_shape(shape);

        // build the merged object type when members exist
        let object_ty_id = if merged_shape.is_empty() {
            None
        } else {
            Some(types.insert_type_from(merged_shape.into_object_type(), declaration_id))
        };

        // always include the nominal type descriptor
        let nominal_reference = Type::Reference {
            symbol,
            static_arguments: None,
        };
        let nominal_reference_id = types.insert_type_from(nominal_reference, declaration_id);
        let descriptor_ty_id = types.insert_type_from(
            Type::Value {
                value: nominal_reference_id,
            },
            declaration_id,
        );

        // keep only non object extras to avoid duplicate shapes
        extras.retain(|extra_id| {
            !matches!(
                types.get_type(*extra_id),
                Type::Object { .. } | Type::Function { .. } | Type::Value { .. }
            )
        });

        // assemble the final value type
        let mut elements = Vec::new();
        if let Some(object_ty_id) = object_ty_id {
            elements.push(object_ty_id);
        }
        elements.push(descriptor_ty_id);
        elements.extend(extras);

        let value_ty_id = if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type_from(Type::Intersection { elements }, declaration_id)
        };
        types.set_value_type(symbol, value_ty_id);
        value_ty_id
    }

    /// Merge a function declaration into the symbol value type.
    pub(crate) fn merge_function_value_type(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        fn_ty_id: LocalTypeId,
        types: &mut TypeTable,
        allow_merge: bool,
    ) {
        // resolve the symbol and merge mode
        let symbol = symbol_id.into_global(module.id);

        // assign directly when merging is disallowed
        if !allow_merge {
            types.set_value_type(symbol, fn_ty_id);
            return;
        }

        // reuse the existing value type when available
        let Some(existing_id) = types.get_value_type_id(symbol) else {
            types.set_value_type(symbol, fn_ty_id);
            return;
        };

        // unpack the existing value type into a callable shape
        let existing_ty = types.get_type(existing_id).clone();
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();
        match existing_ty {
            Type::Function { .. } => {
                call_signatures.push(existing_id);
            }
            Type::Object {
                fields: existing_fields,
                call_signatures: existing_calls,
                construct_signatures: existing_constructs,
                index_signatures: existing_indexes,
            } => {
                fields = existing_fields;
                call_signatures = existing_calls;
                construct_signatures = existing_constructs;
                index_signatures = existing_indexes;
            }
            _ => {}
        }

        // append the new overload signature
        call_signatures.push(fn_ty_id);

        // rebuild the merged callable value type
        let value_ty = Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        };
        let value_ty_id = types.insert_type_from(value_ty, declaration_id);
        types.set_value_type(symbol, value_ty_id);
    }

    /// Merge instance shape into the symbol and any merge group peers.
    pub(crate) fn merge_instance_shape_into_merge_group(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        shape: &ObjectShape,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        allow_merge: bool,
    ) -> Option<LocalTypeId> {
        let symbol_entry = symbols.get_symbol(symbol_id);
        let mut instance_ty_id = None;

        // build an instance type for mergeable symbols
        if self.symbol_supports_instance_merge(symbol_entry) {
            instance_ty_id = Some(self.merge_instance_shape_into_symbol(
                module,
                declaration_id,
                symbol_id,
                shape,
                types,
                allow_merge,
            ));
        }

        // propagate merged shapes across the merge group when allowed
        if allow_merge && let Some(group_id) = symbol_entry.merge_group {
            // merge into each merge group symbol
            for group_symbol_id in symbols.merge_group_symbols(group_id) {
                // skip the originating symbol
                if *group_symbol_id == symbol_id {
                    continue;
                }

                // skip non mergeable symbols
                let group_symbol = symbols.get_symbol(*group_symbol_id);
                if !self.symbol_supports_instance_merge(group_symbol) {
                    continue;
                }

                // merge into the peer symbol instance type
                self.merge_instance_shape_into_symbol(
                    module,
                    declaration_id,
                    *group_symbol_id,
                    shape,
                    types,
                    true,
                );
            }
        }

        instance_ty_id
    }

    /// Merge global augmentation types into a symbol instance type.
    pub(crate) fn merge_global_instance_shape_for_symbol(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // skip ambient lib modules
        if self.module_is_ambient_lib(module) {
            return Ok(());
        }

        // resolve the merge key for the symbol
        let symbol_entry = symbols.get_symbol(symbol_id);
        let Some(key) = symbol_entry.key else {
            return Ok(());
        };

        // collect merge symbols for this key and space
        let merge_symbols =
            self.collect_global_merge_symbols(module, profile, key, symbol_entry.space);
        if merge_symbols.is_empty() {
            return Ok(());
        }

        // import and merge each global symbol instance type
        for global_symbol in merge_symbols {
            // skip the symbol that owns this declaration
            if global_symbol.module_id == module.id && global_symbol.local_id == symbol_id {
                continue;
            }

            // ensure remote module declare is ready
            self.require_analyze_module_declare(global_symbol.module_id, profile)?;

            // load remote instance type data
            let remote_module = self.program.modules.get(global_symbol.module_id);
            let remote_module = remote_module.read();
            let remote_types = remote_module.dir(profile).types.read();

            // skip symbols without instance types
            let Some(remote_instance_id) = remote_types.get_instance_type_id(global_symbol) else {
                continue;
            };

            // import the remote instance type into this module
            let remote_ty = remote_types.get_type(remote_instance_id);
            let local_ty_id = self.import_type_from_remote_for_node(
                declaration_id.into_any(),
                remote_ty,
                &remote_types,
                global_symbol,
                types,
            );
            let local_ty = types.get_type(local_ty_id);

            // skip non object instance types
            let mut shape = ObjectShape::default();
            if !shape.extend_from_object(local_ty) {
                continue;
            }

            // merge the imported shape into this symbol
            self.merge_instance_shape_into_symbol(
                module,
                declaration_id,
                symbol_id,
                &shape,
                types,
                true,
            );
        }

        Ok(())
    }

    /// Collect merge symbols for global augmentations.
    fn collect_global_merge_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Vec<GlobalSymbolId> {
        // start merge symbol collection
        let mut merge_symbols = Vec::new();

        // include symbols from the global group
        if let Some(global_symbols) = self.get_global_symbol_group(module.id, profile, key, space) {
            merge_symbols.extend(global_symbols);
        }

        // include ambient lib symbols when available
        if !self.module_is_ambient_lib(module)
            && let Some(ambient_symbols) =
                self.get_ambient_lib_symbol_sources_for_merge(profile, key, space)
        {
            merge_symbols.extend(ambient_symbols);
        }

        // remove duplicates in a stable order
        let mut seen = HashSet::new();
        merge_symbols.retain(|symbol| seen.insert(*symbol));

        merge_symbols
    }

    /// Merge global augmentation types into a symbol value type.
    pub(crate) fn merge_global_value_shape_for_symbol(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // skip ambient lib modules
        if self.module_is_ambient_lib(module) {
            return Ok(());
        }

        // resolve the merge key for the symbol
        let symbol_entry = symbols.get_symbol(symbol_id);
        let Some(key) = symbol_entry.key else {
            return Ok(());
        };

        // collect merge symbols for this key and space
        let merge_symbols =
            self.collect_global_merge_symbols(module, profile, key, symbol_entry.space);
        if merge_symbols.is_empty() {
            return Ok(());
        }

        // import and merge each global symbol value type
        for global_symbol in merge_symbols {
            // skip the symbol that owns this declaration
            if global_symbol.module_id == module.id && global_symbol.local_id == symbol_id {
                continue;
            }

            // ensure remote module declare is ready
            self.require_analyze_module_declare(global_symbol.module_id, profile)?;

            // load remote value type data
            let remote_module = self.program.modules.get(global_symbol.module_id);
            let remote_module = remote_module.read();
            let remote_types = remote_module.dir(profile).types.read();
            let Some(remote_value_id) = remote_types.get_value_type_id(global_symbol) else {
                continue;
            };

            // import the remote value type into this module
            let remote_value_ty = remote_types.get_type(remote_value_id);
            let local_value_id = self.import_type_from_remote_for_node(
                declaration_id.into_any(),
                remote_value_ty,
                &remote_types,
                global_symbol,
                types,
            );

            // collect the remote value shape
            let mut remote_shape = ObjectShape::default();
            let mut extras = Vec::new();
            let mut visited = Vec::new();
            self.collect_value_shape_from_type(
                local_value_id,
                types,
                &mut remote_shape,
                &mut extras,
                &mut visited,
            );
            if remote_shape.is_empty() {
                continue;
            }

            // merge the imported shape into this symbol
            self.merge_value_shape_into_symbol(
                module,
                declaration_id,
                symbol_id,
                &remote_shape,
                types,
                true,
            );
        }

        Ok(())
    }
}

/// Collected member shapes for instance and value sides.
#[derive(Debug, Default)]
pub(crate) struct ObjectShapeSet {
    /// Instance shape for members.
    pub(crate) instance: ObjectShape,
    /// Value shape for static members.
    pub(crate) value: ObjectShape,
}
