use std::collections::HashMap;

use crate::analyze::common::TypeContext;
use crate::analyze::module::GlobalMergeCategory;
use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler};
use destack_dir::{
    Asynchrony, Declaration, FunctionCardinality, GlobalSymbolId, LocalNodeId, LocalNodeIdAny,
    LocalSymbolId, LocalTypeId, Symbol, SymbolType, Type, TypeExpression, TypeField,
    TypeIndexSignature, TypeTable, are_types_equal,
};

// value shape source used to keep type ids anchored consistently
enum ValueShapeSource {
    /// Declaration node.
    Declaration(LocalNodeId<Declaration>),
    /// Any node.
    Any(LocalNodeIdAny),
}

/// Remote shape import mode for merge symbols.
#[derive(Clone, Copy)]
enum RemoteMergeShapeKind {
    /// Import the remote instance type shape.
    Instance,
    /// Import the remote value type shape.
    Value,
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
    generic_parameters: Vec<LocalTypeId>,
    /// The optional `this` parameter type id.
    this_parameter: Option<LocalTypeId>,
    /// The dynamic parameter type ids.
    parameters: Vec<LocalTypeId>,
    /// The optional return type id.
    return_type: Option<LocalTypeId>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Remap merged owner parameters from one carrier symbol into another.
    pub(crate) fn remap_merged_owner_parameters_in_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // skip when the type already lives in the target owner space
        if source_symbol == target_symbol {
            return type_id;
        }

        // collect the declaration ordered parameter lists for both carriers
        let Some(source_parameters) =
            self.collect_static_parameter_symbols(ctx.type_view(), source_symbol)
        else {
            return type_id;
        };
        if source_parameters.is_empty() {
            return type_id;
        }

        let Some(target_parameters) =
            self.collect_static_parameter_symbols(ctx.type_view(), target_symbol)
        else {
            return type_id;
        };
        if target_parameters.len() != source_parameters.len() {
            return type_id;
        }

        // build a positional owner parameter remap
        let mut substitutions = HashMap::new();
        for (source_parameter, target_parameter) in
            source_parameters.iter().zip(target_parameters.iter())
        {
            let target_parameter_type = ctx.types.insert_type_from_any(
                Type::Reference {
                    symbol: *target_parameter,
                    generic_arguments: None,
                },
                source_id,
            );
            substitutions.insert(*source_parameter, target_parameter_type);
        }
        if substitutions.is_empty() {
            return type_id;
        }

        // rewrite the imported type into the target carrier parameter space
        let mut cache = HashMap::new();
        self.substitute_static_parameters(type_id, &substitutions, ctx.types, &mut cache)
    }

    /// Remap one merged object shape into the target carrier parameter space.
    fn remap_merged_owner_parameters_in_shape(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        shape: &ObjectShape,
    ) -> ObjectShape {
        // reuse the incoming shape when both carriers already match
        if source_symbol == target_symbol {
            return shape.clone();
        }

        // rewrite every typed member edge in the imported shape
        let mut remapped_shape = shape.clone();

        for field in &mut remapped_shape.fields {
            field.ty = self.remap_merged_owner_parameters_in_type(
                &mut ctx.reborrow(),
                source_id,
                source_symbol,
                target_symbol,
                field.ty,
            );
        }

        for signature in &mut remapped_shape.call_signatures {
            *signature = self.remap_merged_owner_parameters_in_type(
                &mut ctx.reborrow(),
                source_id,
                source_symbol,
                target_symbol,
                *signature,
            );
        }

        for signature in &mut remapped_shape.construct_signatures {
            *signature = self.remap_merged_owner_parameters_in_type(
                &mut ctx.reborrow(),
                source_id,
                source_symbol,
                target_symbol,
                *signature,
            );
        }

        for signature in &mut remapped_shape.index_signatures {
            signature.key_type = self.remap_merged_owner_parameters_in_type(
                &mut ctx.reborrow(),
                source_id,
                source_symbol,
                target_symbol,
                signature.key_type,
            );
            signature.value_type = self.remap_merged_owner_parameters_in_type(
                &mut ctx.reborrow(),
                source_id,
                source_symbol,
                target_symbol,
                signature.value_type,
            );
        }

        remapped_shape
    }

    /// Import one remote merge shape for a symbol into the local type table.
    fn import_remote_merge_shape_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        global_symbol: GlobalSymbolId,
        kind: RemoteMergeShapeKind,
    ) -> AnalyzeResult<Option<ObjectShape>> {
        self.require_remote_artifact_dir(
            ctx.compiler_context,
            ctx.module.id,
            global_symbol.module_id,
            ctx.profile,
            destack_artifact::ArtifactKey::dir_declared,
        )
        .map_err(AnalyzeError::from)?;
        let snapshot = self
            .require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                global_symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;

        // pick the remote shape source type
        let remote_type_id = match kind {
            RemoteMergeShapeKind::Instance => snapshot.types.get_instance_type_id(global_symbol),
            RemoteMergeShapeKind::Value => snapshot.types.get_value_type_id(global_symbol),
        };
        let remote_shape_type = remote_type_id.map(|remote_type_id| {
            let remote_type = snapshot.types.get_type(remote_type_id).clone();
            let remote_snapshot = snapshot.types.clone();
            (remote_type, remote_snapshot)
        });
        let Some((remote_type, remote_snapshot)) = remote_shape_type else {
            return Ok(None);
        };

        // import the remote type after dropping remote locks
        let local_type_id = self.import_remote_type_for_node(
            declaration_id.into_any(),
            &remote_type,
            &remote_snapshot,
            ctx.types,
        );

        // extract the imported shape by mode
        let mut shape = ObjectShape::default();
        match kind {
            RemoteMergeShapeKind::Instance => {
                let local_type = ctx.types.get_type(local_type_id);
                if !shape.extend_from_object(local_type) {
                    return Ok(None);
                }
            }
            RemoteMergeShapeKind::Value => {
                let mut extras = Vec::new();
                let mut visited = Vec::new();
                self.collect_value_shape_from_type(
                    local_type_id,
                    ctx.types,
                    &mut shape,
                    &mut extras,
                    &mut visited,
                );
                if shape.is_empty() {
                    return Ok(None);
                }
            }
        }

        Ok(Some(shape))
    }

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
            generic_arguments: None,
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
    fn symbol_supports_instance_merge(&self, symbol: &Symbol) -> bool {
        // only structured nominal symbols support instance merging
        matches!(
            symbol.ty,
            SymbolType::Interface | SymbolType::Class | SymbolType::Struct | SymbolType::Enum
        )
    }

    /// Merge instance shape into a symbol instance type.
    fn merge_instance_shape_into_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        source_symbol: GlobalSymbolId,
        shape: &ObjectShape,
        allow_merge: bool,
    ) -> LocalTypeId {
        // seed the merge with any existing instance members
        let symbol = symbol_id.into_global(ctx.module.id);
        let mut merged_shape = ObjectShape::default();

        // rewrite the incoming shape into the target carrier space
        let shape = self.remap_merged_owner_parameters_in_shape(
            &mut ctx.reborrow(),
            declaration_id.into_any(),
            source_symbol,
            symbol,
            shape,
        );

        // reuse existing instance members when merges are allowed
        if allow_merge && let Some(existing_id) = ctx.types.get_instance_type_id(symbol) {
            let existing_ty = ctx.types.get_type(existing_id);
            merged_shape.extend_from_object(existing_ty);
        }

        // merge the new shape into the instance type
        merged_shape.extend_from_shape(&shape);
        self.canonicalize_merged_object_shape(&mut merged_shape, ctx.types);
        let instance_ty = merged_shape.into_object_type();
        let instance_ty_id = ctx.types.insert_type_from(instance_ty, declaration_id);
        ctx.types.set_instance_type(symbol, instance_ty_id);
        instance_ty_id
    }

    /// Canonicalize merged object members by dropping equivalent duplicates.
    fn canonicalize_merged_object_shape(&self, shape: &mut ObjectShape, types: &TypeTable) {
        // dedupe equivalent fields by key and type
        let mut deduped_fields = Vec::new();
        for field in &shape.fields {
            let duplicate = deduped_fields.iter().any(|existing: &TypeField| {
                existing.key.matches(&field.key)
                    && existing.is_optional == field.is_optional
                    && existing.is_readonly == field.is_readonly
                    && are_types_equal(existing.ty, field.ty, types)
            });
            if !duplicate {
                deduped_fields.push(field.clone());
            }
        }
        shape.fields = deduped_fields;

        // dedupe equivalent call signatures
        let mut deduped_call_signatures = Vec::new();
        for signature in &shape.call_signatures {
            let duplicate = deduped_call_signatures
                .iter()
                .any(|existing| are_types_equal(*existing, *signature, types));
            if !duplicate {
                deduped_call_signatures.push(*signature);
            }
        }
        shape.call_signatures = deduped_call_signatures;

        // dedupe equivalent construct signatures
        let mut deduped_construct_signatures = Vec::new();
        for signature in &shape.construct_signatures {
            let duplicate = deduped_construct_signatures
                .iter()
                .any(|existing| are_types_equal(*existing, *signature, types));
            if !duplicate {
                deduped_construct_signatures.push(*signature);
            }
        }
        shape.construct_signatures = deduped_construct_signatures;

        // dedupe equivalent index signatures
        let mut deduped_index_signatures = Vec::new();
        for signature in &shape.index_signatures {
            let duplicate = deduped_index_signatures
                .iter()
                .any(|existing: &TypeIndexSignature| {
                    existing.name == signature.name
                        && existing.is_readonly == signature.is_readonly
                        && are_types_equal(existing.key_type, signature.key_type, types)
                        && are_types_equal(existing.value_type, signature.value_type, types)
                });
            if !duplicate {
                deduped_index_signatures.push(signature.clone());
            }
        }
        shape.index_signatures = deduped_index_signatures;
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
        ctx: &mut TypeContext<'_>,
        value: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<ObjectShape> {
        // resolve the embed target type
        let embed_ty_id =
            self.resolve_declared_type_expression(&mut ctx.reborrow(), value, true, true)?;

        // unwrap value types and resolve nominal references when possible
        let mut embed_ty_id = embed_ty_id;
        let mut embed_symbol = None;
        match ctx.types.get_type(embed_ty_id) {
            Type::Reference { symbol, .. } => {
                embed_symbol = Some(*symbol);
            }
            Type::Value { value } => {
                embed_ty_id = *value;
                if let Type::Reference { symbol, .. } = ctx.types.get_type(*value) {
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
                &mut ctx.reborrow(),
                value.into_any(),
                symbol,
                &mut embed_shape,
                &mut extras,
                &mut visited,
                &mut visited_symbols,
            )?;
        } else {
            self.collect_value_shape_from_type(
                embed_ty_id,
                ctx.types,
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
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
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
            self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
        {
            self.collect_value_shape_from_type(
                instance_id,
                ctx.types,
                shape,
                extras,
                visited_types,
            );
        }

        // traverse lineage to collect inherited or embedded fields
        if let Some(lineage) = self
            .lineage_for_symbol_or_local_for_artifact(
                ctx.compiler_context,
                ctx.module,
                ctx.profile,
                symbol,
                ctx.types,
                destack_artifact::ArtifactKey::dir_declared,
            )
            .map_err(AnalyzeError::from)?
        {
            if let Some(extends) = lineage.extends {
                self.collect_embed_shape_for_symbol(
                    &mut ctx.reborrow(),
                    source_id,
                    extends,
                    shape,
                    extras,
                    visited_types,
                    visited_symbols,
                )?;
            }

            for implements in lineage.implements {
                self.collect_embed_shape_for_symbol(
                    &mut ctx.reborrow(),
                    source_id,
                    implements,
                    shape,
                    extras,
                    visited_types,
                    visited_symbols,
                )?;
            }

            for embedded in lineage.embedded {
                self.collect_embed_shape_for_symbol(
                    &mut ctx.reborrow(),
                    source_id,
                    embedded,
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
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        source_symbol: GlobalSymbolId,
        shape: &ObjectShape,
        allow_merge: bool,
    ) -> LocalTypeId {
        // seed the merge with any existing value members
        let symbol = symbol_id.into_global(ctx.module.id);
        let mut merged_shape = ObjectShape::default();
        let mut extras = Vec::new();

        // rewrite the incoming shape into the target carrier space
        let shape = self.remap_merged_owner_parameters_in_shape(
            &mut ctx.reborrow(),
            declaration_id.into_any(),
            source_symbol,
            symbol,
            shape,
        );

        // reuse existing value members when merges are allowed
        if allow_merge && let Some(existing_id) = ctx.types.get_value_type_id(symbol) {
            let mut visited = Vec::new();
            self.collect_value_shape_from_type(
                existing_id,
                ctx.types,
                &mut merged_shape,
                &mut extras,
                &mut visited,
            );
        }

        // merge the new shape into the value shape
        merged_shape.extend_from_shape(&shape);

        let source = ValueShapeSource::Declaration(declaration_id);
        self.build_value_shape_type(&source, symbol, merged_shape, extras, ctx.types)
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
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        fn_ty_id: LocalTypeId,
        previous_signature_id: Option<LocalTypeId>,
        allow_merge: bool,
    ) {
        // resolve the symbol and merge mode
        let symbol = symbol_id.into_global(ctx.module.id);

        // assign directly when merging is disallowed
        if !allow_merge {
            ctx.types.set_value_type(symbol, fn_ty_id);
            return;
        }

        // reuse the existing value type when available
        let Some(existing_id) = ctx.types.get_value_type_id(symbol) else {
            ctx.types.set_value_type(symbol, fn_ty_id);
            return;
        };

        // unpack the existing value type into a callable shape
        let existing_ty = ctx.types.get_type(existing_id).clone();
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();
        match existing_ty {
            Type::Function { .. } => {
                if previous_signature_id == Some(existing_id) {
                    // replace the cached signature for this declaration
                    ctx.types.set_value_type(symbol, fn_ty_id);
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
            &mut ctx.reborrow(),
            declaration_id.into_any(),
            &call_signatures,
            fn_ty_id,
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
        let value_ty_id = ctx.types.insert_type_from(value_ty, declaration_id);
        ctx.types.set_value_type(symbol, value_ty_id);
    }

    /// Report duplicate overload signatures in non-declaration modules.
    pub(crate) fn report_duplicate_overload_signature(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        existing_signatures: &[LocalTypeId],
        candidate_signature: LocalTypeId,
    ) {
        // allow duplicate overloads in declaration modules
        if ctx.module.language_type.is_declaration() {
            return;
        }

        // skip checks when no prior overloads exist
        if existing_signatures.is_empty() {
            return;
        }

        // detect equivalent overloads by shape
        let has_duplicate = existing_signatures.iter().any(|signature_id| {
            self.signature_types_equivalent(&mut ctx.reborrow(), *signature_id, candidate_signature)
        });

        if has_duplicate {
            self.error(AnalyzeError::DuplicateOverloadSignature {
                node: source_id.into_anchored(ctx.module.id, Some(ctx.profile)),
            });
        }
    }

    /// Check whether two signature types are equivalent by shape.
    fn signature_types_equivalent(
        &self,
        ctx: &mut TypeContext<'_>,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
    ) -> bool {
        // unpack function signatures
        let Some(left) = self.signature_shape_from_type(left_id, ctx.types) else {
            return false;
        };
        let Some(right) = self.signature_shape_from_type(right_id, ctx.types) else {
            return false;
        };

        // compare async and cardinality modifiers
        if left.asynchrony != right.asynchrony || left.cardinality != right.cardinality {
            return false;
        }

        // compare static parameter arity
        if left.generic_parameters.len() != right.generic_parameters.len() {
            return false;
        }

        // compare dynamic parameter arity
        if left.parameters.len() != right.parameters.len() {
            return false;
        }

        // compare this parameter presence
        match (left.this_parameter, right.this_parameter) {
            (None, None) => {}
            (Some(left_this), Some(right_this)) => {
                if !self.signature_type_ids_equivalent(&mut ctx.reborrow(), left_this, right_this) {
                    return false;
                }
            }
            _ => return false,
        }

        // compare static parameter shapes
        for (left_param, right_param) in left
            .generic_parameters
            .iter()
            .zip(right.generic_parameters.iter())
        {
            if !self.signature_type_ids_equivalent(&mut ctx.reborrow(), *left_param, *right_param) {
                return false;
            }
        }

        // compare dynamic parameter shapes
        for (left_param, right_param) in left.parameters.iter().zip(right.parameters.iter()) {
            if !self.signature_type_ids_equivalent(&mut ctx.reborrow(), *left_param, *right_param) {
                return false;
            }
        }

        // compare return type shapes
        match (left.return_type, right.return_type) {
            (None, None) => true,
            (Some(left_return), Some(right_return)) => {
                self.signature_type_ids_equivalent(&mut ctx.reborrow(), left_return, right_return)
            }
            _ => false,
        }
    }

    /// Check whether two type ids are mutually assignable for overload equivalence.
    fn signature_type_ids_equivalent(
        &self,
        ctx: &mut TypeContext<'_>,
        left_id: LocalTypeId,
        right_id: LocalTypeId,
    ) -> bool {
        // compare assignability in both directions
        let left_assignable = self.is_type_assignable(&mut ctx.reborrow(), left_id, right_id);
        let right_assignable = self.is_type_assignable(&mut ctx.reborrow(), right_id, left_id);
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
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        } = types.get_type(ty_id)
        else {
            return None;
        };

        Some(SignatureShape {
            asynchrony: *asynchrony,
            cardinality: *cardinality,
            generic_parameters: generic_parameters.clone(),
            this_parameter: *this_parameter,
            parameters: parameters.clone(),
            return_type: *return_type,
        })
    }

    /// Merge instance shape into the symbol and any merge group peers.
    pub(crate) fn merge_instance_shape_into_merge_group(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
        shape: &ObjectShape,
        allow_merge: bool,
    ) -> Option<LocalTypeId> {
        let symbol_entry = ctx.symbols.get_symbol(symbol_id);
        let mut instance_ty_id = None;
        let source_symbol = symbol_id.into_global(ctx.module.id);

        // build an instance type for mergeable symbols
        if self.symbol_supports_instance_merge(symbol_entry) {
            instance_ty_id = Some(self.merge_instance_shape_into_symbol(
                &mut ctx.reborrow(),
                declaration_id,
                symbol_id,
                source_symbol,
                shape,
                allow_merge,
            ));
        }

        // propagate merged shapes across the merge group when allowed
        if allow_merge && let Some(group_id) = symbol_entry.merge_group {
            let group_symbols = ctx.symbols.merge_group_symbols(group_id).to_vec();

            // merge into each merge group symbol
            for group_symbol_id in group_symbols {
                // skip the originating symbol
                if group_symbol_id == symbol_id {
                    continue;
                }

                // skip non mergeable symbols
                let group_symbol = ctx.symbols.get_symbol(group_symbol_id);
                if !self.symbol_supports_instance_merge(group_symbol) {
                    continue;
                }

                // merge into the peer symbol instance type
                self.merge_instance_shape_into_symbol(
                    &mut ctx.reborrow(),
                    declaration_id,
                    group_symbol_id,
                    source_symbol,
                    shape,
                    true,
                );
            }
        }

        instance_ty_id
    }

    /// Merge global augmentation types into a symbol instance type.
    pub(crate) fn merge_global_instance_shape_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
    ) -> AnalyzeResult<()> {
        // skip ambient lib modules
        if self.module_is_ambient_lib(ctx.module) {
            return Ok(());
        }

        // resolve the merge key for the symbol
        let symbol_entry = ctx.symbols.get_symbol(symbol_id);
        if !symbol_entry.origin.is_global_augmentation() {
            return Ok(());
        }
        let Some(key) = symbol_entry.key else {
            return Ok(());
        };

        // collect merge symbols for this key and space
        let merge_symbols = self.collect_global_merge_sources_for_key(
            ctx.compiler_context.revision(),
            ctx.module,
            &ctx.index,
            ctx.symbols,
            ctx.profile,
            key,
            symbol_entry.space,
            GlobalMergeCategory::Instance,
        )?;
        if merge_symbols.is_empty() {
            return Ok(());
        }

        // import and merge each global symbol instance type
        for global_symbol in merge_symbols {
            // skip the symbol that owns this declaration
            if global_symbol.module_id == ctx.module.id && global_symbol.local_id == symbol_id {
                continue;
            }

            let shape = self.import_remote_merge_shape_for_symbol(
                &mut ctx.reborrow(),
                declaration_id,
                global_symbol,
                RemoteMergeShapeKind::Instance,
            )?;

            let Some(shape) = shape else {
                continue;
            };

            // merge the imported shape into this symbol
            self.merge_instance_shape_into_symbol(
                &mut ctx.reborrow(),
                declaration_id,
                symbol_id,
                global_symbol,
                &shape,
                true,
            );
        }

        Ok(())
    }

    /// Merge global augmentation types into a symbol value type.
    pub(crate) fn merge_global_value_shape_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        symbol_id: LocalSymbolId,
    ) -> AnalyzeResult<()> {
        // skip ambient lib modules
        if self.module_is_ambient_lib(ctx.module) {
            return Ok(());
        }

        // resolve the merge key for the symbol
        let symbol_entry = ctx.symbols.get_symbol(symbol_id);
        if !symbol_entry.origin.is_global_augmentation() {
            return Ok(());
        }
        let Some(key) = symbol_entry.key else {
            return Ok(());
        };

        // collect merge symbols for this key and space
        let merge_symbols = self.collect_global_merge_sources_for_key(
            ctx.compiler_context.revision(),
            ctx.module,
            &ctx.index,
            ctx.symbols,
            ctx.profile,
            key,
            symbol_entry.space,
            GlobalMergeCategory::Value,
        )?;
        if merge_symbols.is_empty() {
            return Ok(());
        }

        // import and merge each global symbol value type
        for global_symbol in merge_symbols {
            // skip the symbol that owns this declaration
            if global_symbol.module_id == ctx.module.id && global_symbol.local_id == symbol_id {
                continue;
            }

            let remote_shape = self.import_remote_merge_shape_for_symbol(
                &mut ctx.reborrow(),
                declaration_id,
                global_symbol,
                RemoteMergeShapeKind::Value,
            )?;
            let Some(remote_shape) = remote_shape else {
                continue;
            };

            // merge the imported shape into this symbol
            self.merge_value_shape_into_symbol(
                &mut ctx.reborrow(),
                declaration_id,
                symbol_id,
                global_symbol,
                &remote_shape,
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
