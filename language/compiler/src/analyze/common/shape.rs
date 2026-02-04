use std::collections::HashSet;

use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler};
use destack_dir::{
    Asynchrony, Declaration, Expression, FunctionCardinality, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalSymbolId, LocalTypeId, NodeTree, StaticKey, SymbolSpace, SymbolTable,
    SymbolType, Type, TypeField, TypeIndexSignature, TypeTable,
};
use destack_workspace::{Module, ProfileId};

// value shape source used to keep type ids anchored consistently
enum ValueShapeSource {
    /// Declaration node.
    Declaration(LocalNodeId<Declaration>),
    /// Any node.
    Any(LocalNodeIdAny),
}

impl ValueShapeSource {
    fn insert_type(&self, types: &mut TypeTable, ty: Type) -> LocalTypeId {
        match self {
            Self::Declaration(node_id) => types.insert_type_from(ty, *node_id),
            Self::Any(node_id) => types.insert_type_from_any(ty, *node_id),
        }
    }
}

/// Object shape builder and identity.
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

/// Function signature shape builder and identity.
#[derive(Debug, Clone)]
struct SignatureShape {
    /// Whether the function is async.
    asynchrony: Asynchrony,
    /// The function cardinality.
    cardinality: FunctionCardinality,
    /// The static parameter type ids.
    static_parameters: Vec<LocalTypeId>,
    /// The optional `this` parameter type id.
    this_parameter: Option<LocalTypeId>,
    /// The dynamic parameter type ids.
    dynamic_parameters: Vec<LocalTypeId>,
    /// The optional return type id.
    return_type: Option<LocalTypeId>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    // build a value type from a merged shape and extras
    fn build_value_shape_type(
        &self,
        source: &ValueShapeSource,
        symbol: GlobalSymbolId,
        merged_shape: ObjectShape,
        mut extras: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // build the merged object type when members exist
        let object_ty_id = if merged_shape.is_empty() {
            None
        } else {
            Some(source.insert_type(types, merged_shape.into_object_type()))
        };

        // always include the nominal type descriptor
        let nominal_reference = Type::Reference {
            symbol,
            static_arguments: None,
        };
        let nominal_reference_id = source.insert_type(types, nominal_reference);
        let descriptor_ty_id = source.insert_type(
            types,
            Type::Value {
                value: nominal_reference_id,
            },
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
            source.insert_type(types, Type::Intersection { elements })
        };
        types.set_value_type(symbol, value_ty_id);

        value_ty_id
    }

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

        // unwrap value types and resolve nominal references when possible
        let mut embed_ty_id = embed_ty_id;
        let mut embed_symbol = None;
        match types.get_type(embed_ty_id) {
            Type::Reference { symbol, .. } => {
                embed_symbol = Some(*symbol);
            }
            Type::Value { value } => {
                embed_ty_id = *value;
                if let Type::Reference { symbol, .. } = types.get_type(*value) {
                    embed_symbol = Some(*symbol);
                }
            }
            _ => {}
        }

        // collect field-like members from the embedded type
        let mut embed_shape = ObjectShape::default();
        let mut extras = Vec::new();
        let mut visited = Vec::new();
        let mut visited_symbols = Vec::new();
        if let Some(symbol) = embed_symbol {
            self.collect_embed_shape_for_symbol(
                module,
                profile,
                value.into_any(),
                symbol,
                symbols,
                types,
                &mut embed_shape,
                &mut extras,
                &mut visited,
                &mut visited_symbols,
            )?;
        } else {
            self.collect_value_shape_from_type(
                embed_ty_id,
                types,
                &mut embed_shape,
                &mut extras,
                &mut visited,
            );
        }

        // embedding only contributes fields and index signatures
        embed_shape.call_signatures.clear();
        embed_shape.construct_signatures.clear();

        Ok(embed_shape)
    }

    /// Collect embedded shape data for a nominal symbol, including lineage.
    fn collect_embed_shape_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        shape: &mut ObjectShape,
        extras: &mut Vec<LocalTypeId>,
        visited_types: &mut Vec<LocalTypeId>,
        visited_symbols: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<()> {
        // avoid cycles in embedding chains
        if visited_symbols.contains(&symbol) {
            return Ok(());
        }
        visited_symbols.push(symbol);

        // collect fields from the apparent instance type
        if let Some(instance_id) =
            self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
        {
            self.collect_value_shape_from_type(instance_id, types, shape, extras, visited_types);
        }

        // traverse lineage to collect inherited or embedded fields
        let lineage = types.get_lineage_for_symbol(symbol).cloned();
        if let Some(lineage) = lineage {
            if let Some(extends) = lineage.extends {
                self.collect_embed_shape_for_symbol(
                    module,
                    profile,
                    source_id,
                    extends,
                    symbols,
                    types,
                    shape,
                    extras,
                    visited_types,
                    visited_symbols,
                )?;
            }

            for implements in lineage.implements {
                self.collect_embed_shape_for_symbol(
                    module,
                    profile,
                    source_id,
                    implements,
                    symbols,
                    types,
                    shape,
                    extras,
                    visited_types,
                    visited_symbols,
                )?;
            }

            for embedded in lineage.embedded {
                self.collect_embed_shape_for_symbol(
                    module,
                    profile,
                    source_id,
                    embedded,
                    symbols,
                    types,
                    shape,
                    extras,
                    visited_types,
                    visited_symbols,
                )?;
            }
        }

        Ok(())
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

        let source = ValueShapeSource::Declaration(declaration_id);
        self.build_value_shape_type(&source, symbol, merged_shape, extras, types)
    }

    /// Update a symbol value shape with an inferred static field.
    pub(crate) fn update_value_shape_with_field(
        &self,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        field: TypeField,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // seed with existing value members
        let mut merged_shape = ObjectShape::default();
        let mut extras = Vec::new();
        if let Some(existing_id) = types.get_value_type_id(symbol) {
            let mut visited = Vec::new();
            self.collect_value_shape_from_type(
                existing_id,
                types,
                &mut merged_shape,
                &mut extras,
                &mut visited,
            );
        }

        // override the field in the current value shape
        merged_shape.apply_field(field);

        let source = ValueShapeSource::Any(source_id);
        self.build_value_shape_type(&source, symbol, merged_shape, extras, types)
    }

    /// Merge a function declaration into the symbol value type.
    pub(crate) fn merge_function_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        fn_ty_id: LocalTypeId,
        previous_signature_id: Option<LocalTypeId>,
        symbols: &SymbolTable,
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
                if previous_signature_id == Some(existing_id) {
                    // replace the cached signature for this declaration
                    types.set_value_type(symbol, fn_ty_id);
                    return;
                }
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

        // drop old cached signatures for this declaration
        if let Some(previous_signature_id) = previous_signature_id {
            call_signatures.retain(|signature_id| *signature_id != previous_signature_id);
        }

        // report duplicate overloads in non-declaration modules
        self.report_duplicate_overload_signature(
            module,
            profile,
            declaration_id.into_any(),
            &call_signatures,
            fn_ty_id,
            symbols,
            types,
        );

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

    /// Report duplicate overload signatures in non-declaration modules.
    pub(crate) fn report_duplicate_overload_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        existing_signatures: &[LocalTypeId],
        candidate_signature: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) {
        // allow duplicate overloads in declaration modules
        if module.language_type.is_declaration() {
            return;
        }

        // skip checks when no prior overloads exist
        if existing_signatures.is_empty() {
            return;
        }

        // resolve options for assignability checks
        let options = self.analyze_context_options_for_module(module.id);

        // detect equivalent overloads by shape
        let has_duplicate = existing_signatures.iter().any(|signature_id| {
            self.signature_types_equivalent(
                module,
                profile,
                *signature_id,
                candidate_signature,
                symbols,
                types,
                &options,
            )
        });

        if has_duplicate {
            self.error(AnalyzeError::DuplicateOverloadSignature {
                node: source_id.into_anchored(module.id, Some(profile)),
            });
        }
    }

    /// Check whether two signature types are equivalent by shape.
    fn signature_types_equivalent(
        &self,
        module: &Module,
        profile: ProfileId,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // unpack function signatures
        let Some(left) = self.signature_shape_from_type(left_id, types) else {
            return false;
        };
        let Some(right) = self.signature_shape_from_type(right_id, types) else {
            return false;
        };

        // compare async and cardinality modifiers
        if left.asynchrony != right.asynchrony || left.cardinality != right.cardinality {
            return false;
        }

        // compare static parameter arity
        if left.static_parameters.len() != right.static_parameters.len() {
            return false;
        }

        // compare dynamic parameter arity
        if left.dynamic_parameters.len() != right.dynamic_parameters.len() {
            return false;
        }

        // compare this parameter presence
        match (left.this_parameter, right.this_parameter) {
            (None, None) => {}
            (Some(left_this), Some(right_this)) => {
                if !self.signature_type_ids_equivalent(
                    module, profile, left_this, right_this, symbols, types, options,
                ) {
                    return false;
                }
            }
            _ => return false,
        }

        // compare static parameter shapes
        for (left_param, right_param) in left
            .static_parameters
            .iter()
            .zip(right.static_parameters.iter())
        {
            if !self.signature_type_ids_equivalent(
                module,
                profile,
                *left_param,
                *right_param,
                symbols,
                types,
                options,
            ) {
                return false;
            }
        }

        // compare dynamic parameter shapes
        for (left_param, right_param) in left
            .dynamic_parameters
            .iter()
            .zip(right.dynamic_parameters.iter())
        {
            if !self.signature_type_ids_equivalent(
                module,
                profile,
                *left_param,
                *right_param,
                symbols,
                types,
                options,
            ) {
                return false;
            }
        }

        // compare return type shapes
        match (left.return_type, right.return_type) {
            (None, None) => true,
            (Some(left_return), Some(right_return)) => self.signature_type_ids_equivalent(
                module,
                profile,
                left_return,
                right_return,
                symbols,
                types,
                options,
            ),
            _ => false,
        }
    }

    /// Check whether two type ids are mutually assignable for overload equivalence.
    fn signature_type_ids_equivalent(
        &self,
        module: &Module,
        profile: ProfileId,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // compare assignability in both directions
        let left_assignable =
            self.is_type_assignable(module, profile, symbols, left_id, right_id, types, options);
        let right_assignable =
            self.is_type_assignable(module, profile, symbols, right_id, left_id, types, options);
        left_assignable != Assignability::NotAssignable
            && right_assignable != Assignability::NotAssignable
    }

    /// Borrow a function signature shape from a type id.
    fn signature_shape_from_type(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<SignatureShape> {
        let Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = types.get_type(ty_id)
        else {
            return None;
        };

        Some(SignatureShape {
            asynchrony: *asynchrony,
            cardinality: *cardinality,
            static_parameters: static_parameters.clone(),
            this_parameter: *this_parameter,
            dynamic_parameters: dynamic_parameters.clone(),
            return_type: *return_type,
        })
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

            // TODO #Architecture: centralize local vs remote merge imports to keep symbol handling consistent
            // ensure remote module declare is ready
            if global_symbol.module_id != module.id {
                self.require_analyze_module_declare(global_symbol.module_id, profile)?;
            }

            let shape = self.with_module_types(
                module,
                profile,
                global_symbol.module_id,
                |_, remote_types| {
                    // skip symbols without instance types
                    let Some(remote_instance_id) = remote_types.get_instance_type_id(global_symbol)
                    else {
                        return None;
                    };

                    // import the remote instance type into this module
                    let remote_ty = remote_types.get_type(remote_instance_id);
                    let local_ty_id = self.import_type_from_remote_for_node(
                        declaration_id.into_any(),
                        remote_ty,
                        remote_types,
                        global_symbol,
                        types,
                    );
                    let local_ty = types.get_type(local_ty_id);

                    // skip non object instance types
                    let mut shape = ObjectShape::default();
                    if !shape.extend_from_object(local_ty) {
                        return None;
                    }

                    Some(shape)
                },
            );

            let Some(shape) = shape else {
                continue;
            };

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
            if global_symbol.module_id != module.id {
                self.require_analyze_module_declare(global_symbol.module_id, profile)?;
            }

            let remote_shape = self.with_module_types(
                module,
                profile,
                global_symbol.module_id,
                |_, remote_types| {
                    let Some(remote_value_id) = remote_types.get_value_type_id(global_symbol)
                    else {
                        return None;
                    };

                    // import the remote value type into this module
                    let remote_value_ty = remote_types.get_type(remote_value_id);
                    let local_value_id = self.import_type_from_remote_for_node(
                        declaration_id.into_any(),
                        remote_value_ty,
                        remote_types,
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
                        return None;
                    }

                    Some(remote_shape)
                },
            );
            let Some(remote_shape) = remote_shape else {
                continue;
            };

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
