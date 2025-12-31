use crate::{AnalyzeError, AnalyzeResult, Compiler, InferContext, TaskDependencyError};
use destack_dir::{
    Declaration, InferTable, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId, Member,
    NodeTree, Property, SymbolTable, SymbolType, Type, TypeField, TypeIndexSignature, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

use super::super::common::NormalizationMode;
use super::expression::ObjectLiteralField;

/// Accumulate object shape members for inference.
/// NOTE #Architecture: this should probably be a shared Type::Object builder.
#[derive(Debug, Default, Clone)]
pub(super) struct InferredShape {
    /// Fields collected for the shape.
    fields: Vec<TypeField>,
    /// Call signatures collected for the shape.
    call_signatures: Vec<LocalTypeId>,
    /// Construct signatures collected for the shape.
    construct_signatures: Vec<LocalTypeId>,
    /// Index signatures collected for the shape.
    index_signatures: Vec<TypeIndexSignature>,
}

impl InferredShape {
    /// Extend this set with another shape.
    fn extend_from_shape(&mut self, other: &InferredShape) {
        self.fields.extend_from_slice(&other.fields);
        self.call_signatures
            .extend_from_slice(&other.call_signatures);
        self.construct_signatures
            .extend_from_slice(&other.construct_signatures);
        self.index_signatures
            .extend_from_slice(&other.index_signatures);
    }

    /// Add a call signature to this shape.
    pub(super) fn push_call_signature(&mut self, signature: LocalTypeId) {
        self.call_signatures.push(signature);
    }

    /// Extend this set from an object type.
    pub(super) fn extend_from_object(&mut self, ty: &Type) -> bool {
        let Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        } = ty
        else {
            return false;
        };

        self.fields.extend_from_slice(fields);
        self.call_signatures.extend_from_slice(call_signatures);
        self.construct_signatures
            .extend_from_slice(construct_signatures);
        self.index_signatures.extend_from_slice(index_signatures);
        true
    }

    /// Convert into an object type.
    pub(super) fn into_object_type(self) -> Type {
        Type::Object {
            fields: self.fields,
            call_signatures: self.call_signatures,
            construct_signatures: self.construct_signatures,
            index_signatures: self.index_signatures,
        }
    }

    /// Apply a direct field override to the shape.
    pub(super) fn apply_field(&mut self, field: TypeField) {
        // remove existing field with the same key
        self.fields
            .retain(|existing| !existing.key.matches(&field.key));
        self.fields.push(field);
    }

    /// Apply another shape to this one with override semantics.
    pub(super) fn apply_spread(&mut self, spread: &InferredShape) {
        // override fields on matching keys
        for field in &spread.fields {
            self.fields
                .retain(|existing| !existing.key.matches(&field.key));
            self.fields.push(field.clone());
        }

        self.call_signatures
            .extend_from_slice(&spread.call_signatures);
        self.construct_signatures
            .extend_from_slice(&spread.construct_signatures);
        self.index_signatures
            .extend_from_slice(&spread.index_signatures);
    }

    /// Apply another shape to this one for intersection semantics.
    pub(super) fn apply_intersection(&mut self, other: &InferredShape, types: &mut TypeTable) {
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
                    types.insert_type(Type::Intersection {
                        elements: vec![existing.ty, field.ty],
                    })
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
    /// Infer literal fields and spread shapes for an object literal.
    pub(super) fn infer_object_literal_shapes(
        &self,
        module: &Module,
        properties: &[LocalNodeId<Property>],
        expected_object_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<(
        Vec<ObjectLiteralField>,
        Vec<InferredShape>,
        Option<LocalTypeId>,
    )> {
        let mut literal_fields = Vec::new();
        let mut shapes = vec![InferredShape::default()];
        let mut spread_override = None;
        let mut has_any_spread = false;

        // collect literal fields and spread shapes
        for property_id in properties {
            let property = tree.get(*property_id);
            match property {
                Property::Spread { value, .. } => {
                    let mut spread_ctx = ctx.fork().with_expected_type(None);
                    let spread_type = self.infer_expression(
                        module,
                        *value,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut spread_ctx,
                    )?;
                    // normalize spreads before extracting shapes
                    let spread_type = self.normalize_type(
                        module,
                        ctx.profile,
                        spread_type,
                        symbols,
                        types,
                        NormalizationMode::Assignability,
                    );

                    // short circuit on any or unknown spreads
                    match types.get_type(spread_type) {
                        Type::TypeLiteral {
                            value: TypeLiteral::Any,
                        } => {
                            spread_override = Some(spread_type);
                            has_any_spread = true;
                            continue;
                        }
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        } => {
                            if !has_any_spread {
                                spread_override = Some(spread_type);
                            }
                            continue;
                        }
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        } => continue,
                        _ => {}
                    }

                    // expand spread types into object shapes
                    let spread_shapes = self.collect_object_spread_shapes(
                        module,
                        ctx.profile,
                        (*property_id).into_any(),
                        spread_type,
                        symbols,
                        types,
                    )?;
                    shapes = self.merge_object_spread_shape_sets(shapes, spread_shapes);
                }
                _ => {
                    if let Some(field) = self.infer_property(
                        module,
                        *property_id,
                        expected_object_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )? {
                        literal_fields.push(field.clone());
                        for shape in shapes.iter_mut() {
                            shape.apply_field(field.field().clone());
                        }
                    }
                }
            }
        }

        Ok((literal_fields, shapes, spread_override))
    }

    /// Merge shape sets with spread override semantics.
    fn merge_object_spread_shape_sets(
        &self,
        base_shapes: Vec<InferredShape>,
        spread_shapes: Vec<InferredShape>,
    ) -> Vec<InferredShape> {
        let mut merged = Vec::new();

        // apply each spread shape onto each base shape
        for base_shape in base_shapes {
            for spread_shape in &spread_shapes {
                let mut combined = base_shape.clone();
                combined.apply_spread(spread_shape);
                merged.push(combined);
            }
        }

        merged
    }

    /// Merge shape sets with intersection semantics.
    fn merge_object_spread_shape_sets_for_intersection(
        &self,
        base_shapes: Vec<InferredShape>,
        intersection_shapes: Vec<InferredShape>,
        types: &mut TypeTable,
    ) -> Vec<InferredShape> {
        let mut merged = Vec::new();

        // intersect each incoming shape with each base shape
        for base_shape in base_shapes {
            for intersection_shape in &intersection_shapes {
                let mut combined = base_shape.clone();
                combined.apply_intersection(intersection_shape, types);
                merged.push(combined);
            }
        }

        merged
    }

    /// Collect object shapes from a spread type.
    fn collect_object_spread_shapes(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Vec<InferredShape>> {
        let mut visited = Vec::new();
        self.collect_object_spread_shapes_inner(
            module,
            profile,
            node_id,
            type_id,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Collect object shapes with recursion protection.
    fn collect_object_spread_shapes_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut Vec<LocalTypeId>,
    ) -> AnalyzeResult<Vec<InferredShape>> {
        // guard against recursive types
        if visited.contains(&type_id) {
            return Ok(vec![InferredShape::default()]);
        }
        visited.push(type_id);

        // normalize before inspecting the spread shape
        let normalized_type = self.normalize_type(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assignability,
        );

        // derive shapes based on the normalized type
        let normalized_type_value = types.get_type(normalized_type).clone();
        let shapes = match normalized_type_value {
            Type::Object { .. } => {
                // direct object shapes map to a single inferred shape
                let mut shape = InferredShape::default();
                shape.extend_from_object(types.get_type(normalized_type));
                vec![shape]
            }
            Type::Reference { symbol, .. } => {
                // follow instance types when available
                let instance_id = if let Some(instance_id) = types.get_instance_type_id(symbol) {
                    Some(instance_id)
                } else {
                    self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?
                };

                if let Some(instance_id) = instance_id {
                    self.collect_object_spread_shapes_inner(
                        module,
                        profile,
                        node_id,
                        instance_id,
                        symbols,
                        types,
                        visited,
                    )?
                } else {
                    // default to an empty shape when no instance type is available
                    vec![InferredShape::default()]
                }
            }
            Type::Union { elements } => {
                // union spreads fan out into distinct shapes
                let mut merged = Vec::new();
                for element_id in elements {
                    let mut element_shapes = self.collect_object_spread_shapes_inner(
                        module, profile, node_id, element_id, symbols, types, visited,
                    )?;
                    merged.append(&mut element_shapes);
                }
                merged
            }
            Type::Intersection { elements } => {
                // intersection spreads merge shapes together
                let mut merged = vec![InferredShape::default()];
                for element_id in elements {
                    let element_shapes = self.collect_object_spread_shapes_inner(
                        module, profile, node_id, element_id, symbols, types, visited,
                    )?;
                    merged = self.merge_object_spread_shape_sets_for_intersection(
                        merged,
                        element_shapes,
                        types,
                    );
                }
                merged
            }
            _ => {
                // non object spreads contribute no fields
                vec![InferredShape::default()]
            }
        };

        visited.pop();
        Ok(shapes)
    }

    /// Check whether a symbol should receive merged instance members.
    fn symbol_supports_instance_merge(&self, symbol: &destack_dir::Symbol) -> bool {
        matches!(
            symbol.ty,
            SymbolType::Interface | SymbolType::Class | SymbolType::Struct
        )
    }

    /// Collect member contributions for an object style declaration.
    pub(super) fn collect_member_shape(
        &self,
        module: &Module,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
        this_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<InferredShape> {
        let mut shape = InferredShape::default();

        // walk members and collect their contributions
        for member_id in members {
            let inferred = self.infer_member(
                module, *member_id, tree, symbols, types, infer, ctx, this_ty_id,
            )?;

            // record field contributions when present
            if let Some(field) = inferred.field {
                shape.fields.push(field);
            }

            // append callable and index contributions
            shape.call_signatures.extend(inferred.call_signatures);
            shape
                .construct_signatures
                .extend(inferred.construct_signatures);
            shape.index_signatures.extend(inferred.index_signatures);
        }

        Ok(shape)
    }

    /// Merge instance shape into a symbol instance type.
    fn merge_instance_shape_into_symbol(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        shape: &InferredShape,
        types: &mut TypeTable,
        allow_merge: bool,
    ) -> LocalTypeId {
        let symbol = symbol_id.into_global(module.id);
        let mut merged_shape = InferredShape::default();

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

    /// Merge a function declaration into the symbol value type.
    pub(super) fn merge_function_value_type(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        fn_ty_id: LocalTypeId,
        types: &mut TypeTable,
        allow_merge: bool,
    ) {
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
    pub(super) fn merge_instance_shape_into_merge_group(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        shape: &InferredShape,
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
    pub(super) fn merge_global_instance_shape_for_symbol(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let symbol_entry = symbols.get_symbol(symbol_id);
        let Some(key) = symbol_entry.key else {
            return Ok(());
        };

        let Some(global_symbols) =
            self.get_global_symbol_group(module.id, profile, key, symbol_entry.space)
        else {
            return Ok(());
        };

        // import and merge each global symbol instance type
        for global_symbol in global_symbols {
            // skip the symbol that owns this declaration
            if global_symbol.module_id == module.id && global_symbol.local_id == symbol_id {
                continue;
            }

            // ensure remote module analysis is ready
            self.require_analyze_module(global_symbol.module_id, profile)
                .map_err(|error| match error {
                    TaskDependencyError::NotReady { dependency } => {
                        AnalyzeError::Yield { dependency }
                    }
                    TaskDependencyError::Failed { dependency } => {
                        AnalyzeError::UnsatisfiedDependency { dependency }
                    }
                })?;

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
            let mut shape = InferredShape::default();
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
}
