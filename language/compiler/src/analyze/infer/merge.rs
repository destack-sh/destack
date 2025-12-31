use crate::{AnalyzeError, AnalyzeResult, Compiler, InferContext, TaskDependencyError};
use destack_dir::{
    Declaration, InferTable, LocalNodeId, LocalSymbolId, LocalTypeId, Member, NodeTree,
    SymbolTable, SymbolType, Type, TypeField, TypeIndexSignature, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Accumulate instance type members while inferring a declaration.
/// NOTE #Architecture: this should probably be a Type::Object builder
#[derive(Debug, Default, Clone)]
pub(super) struct InferredShape {
    /// Fields collected for the instance shape.
    fields: Vec<TypeField>,
    /// Call signatures collected for the instance shape.
    call_signatures: Vec<LocalTypeId>,
    /// Construct signatures collected for the instance shape.
    construct_signatures: Vec<LocalTypeId>,
    /// Index signatures collected for the instance shape.
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
    fn extend_from_object(&mut self, ty: &Type) -> bool {
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
    fn into_object_type(self) -> Type {
        Type::Object {
            fields: self.fields,
            call_signatures: self.call_signatures,
            construct_signatures: self.construct_signatures,
            index_signatures: self.index_signatures,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
