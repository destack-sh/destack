use destack_artifact::{DirAnalyzed, DirDeclared};
use destack_dir::{self as dir};
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};
use std::sync::Arc;

use crate::{Compiler, LowerError, LowerResult, RequirementError};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::lower::static_key_to_field_name;

/// Helpers for lowering builtin type layouts.
pub(crate) struct BuiltinTypeLayouts<'a> {
    /// Access to compiler helpers and shared state.
    compiler: &'a Compiler,
    /// Pinned revision for cross-module reads.
    revision: Revision,
    /// Profile used for lookup and analysis.
    profile: ProfileId,
    /// MIR module builder to install layouts.
    builder: &'a mut mir::ModuleBuilder,
    /// Type lowerer used to cache builtin layouts.
    type_lowerer: &'a mut TypeLowerer,
}

impl<'a> BuiltinTypeLayouts<'a> {
    /// Create a builtin layout helper.
    pub(crate) fn new(
        compiler: &'a Compiler,
        revision: Revision,
        profile: ProfileId,
        builder: &'a mut mir::ModuleBuilder,
        type_lowerer: &'a mut TypeLowerer,
    ) -> Self {
        Self {
            compiler,
            revision,
            profile,
            builder,
            type_lowerer,
        }
    }

    /// Read one committed analyzed DIR snapshot for a module.
    fn require_analyzed_dir_data(&self, module_id: ModuleId) -> LowerResult<Arc<DirAnalyzed>> {
        let snapshot =
            self.compiler
                .require_artifact_dir_analyzed(self.revision, module_id, self.profile);

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(RequirementError::NotReady { requirement }) => {
                Err(LowerError::Yield { requirement })
            }
            Err(RequirementError::Failed { requirement }) => {
                Err(LowerError::UnsatisfiedRequirement { requirement })
            }
        }
    }

    /// Read one committed declared DIR snapshot for a module when available.
    fn artifact_dir_data_if_present(&self, module_id: ModuleId) -> Option<Arc<DirDeclared>> {
        self.compiler.dir_declared(module_id, self.profile)
    }

    /// Return the builtin String type for lowering.
    pub(crate) fn string_type_for_builtin(
        &mut self,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // skip when already cached
        if let Some(string_type) = self.type_lowerer.string_type() {
            return Ok(Some(string_type));
        }

        // resolve the builtin string symbol
        let Some(string_symbol) = self.resolve_well_known_symbol(
            dir::WellKnownSymbol::String,
            dir::SymbolSpaceOrder::TypeThenValue,
        )?
        else {
            return Ok(None);
        };

        // install the struct layout for String in source order
        let Some(ty_struct) =
            self.struct_layout_for_symbol(string_symbol, anchor, LayoutPolicy::Source)?
        else {
            return Ok(None);
        };

        // name the well known string type metadata
        self.assign_metadata_name_for_symbol(ty_struct, string_symbol);

        // cache the managed reference type
        let ty_string = self.builder.type_managed_reference(ty_struct);
        self.type_lowerer.set_string_type(ty_string);

        Ok(Some(ty_string))
    }

    /// Resolve a well-known symbol for this profile.
    pub(crate) fn resolve_well_known_symbol(
        &self,
        symbol: dir::WellKnownSymbol,
        order: dir::SymbolSpaceOrder,
    ) -> LowerResult<Option<dir::GlobalSymbolId>> {
        // resolve the canonical well-known symbol
        let resolved =
            self.compiler
                .get_well_known_concrete_symbol_from(self.profile, symbol, order);

        // fall back to declared lib symbols when not registered
        let Some(resolved) = resolved else {
            let name = self
                .compiler
                .repository
                .strings
                .intern(symbol.export_name());
            return Ok(self.compiler.get_declared_concrete_library_symbol_from(
                self.profile,
                name,
                order,
            ));
        };

        Ok(Some(resolved))
    }

    /// Ensure the module has been analyzed for this profile.
    fn require_analyzed_module(&self, module_id: ModuleId) -> LowerResult<()> {
        // request the analyzed module
        let result = self
            .compiler
            .require_dir_analyzed(self.revision, module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        // forward dependency failures as lower errors
        match error {
            RequirementError::NotReady { requirement } => Err(LowerError::Yield { requirement }),
            RequirementError::Failed { requirement } => {
                Err(LowerError::UnsatisfiedRequirement { requirement })
            }
        }
    }

    /// Create a MissingType error for a node.
    fn missing_type_error(&self, node_id: dir::GlobalNodeIdAny) -> LowerError {
        LowerError::MissingType {
            node: node_id.into_anchored(Some(self.profile)),
        }
    }

    /// Resolve the struct members in the builtin module.
    fn struct_members_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
        symbols: &dir::SymbolTable,
        tree: &dir::NodeTree,
    ) -> Option<Vec<dir::LocalNodeId<dir::Member>>> {
        // resolve the primary declaration for the symbol
        let primary_declaration = symbols.get_symbol(symbol.local_id).primary_declaration?;
        let declaration_id = primary_declaration
            .local_id
            .try_into_typed::<dir::Declaration>()
            .ok()?;

        // extract struct members
        let dir::Declaration::Struct(declaration) = tree.get(declaration_id) else {
            return None;
        };

        Some(declaration.members.clone())
    }

    /// Collect struct field inputs for layout computation.
    fn struct_field_inputs(
        &mut self,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        members: &[dir::LocalNodeId<dir::Member>],
        module_id: ModuleId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Vec<FieldInput>> {
        // compute layout from declared struct fields only
        let mut field_inputs = Vec::new();
        let pointer_bytes = self.type_lowerer.pointer_bytes();

        let vector_symbol = self.resolve_well_known_symbol(
            dir::WellKnownSymbol::Vector,
            dir::SymbolSpaceOrder::TypeThenValue,
        )?;
        let mut field_lowerer = TypeLowerer::new(
            self.builder,
            pointer_bytes,
            self.compiler.repository.clone(),
            vector_symbol,
        )
        .with_artifact_context(self.revision, self.profile);

        for (source_index, member_id) in members.iter().enumerate() {
            let dir::Member::Field {
                key, declared_type, ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            // resolve a static key for the field
            let Some(key) = self.compiler.static_key_from_key(
                self.revision,
                self.profile,
                tree,
                symbols,
                types,
                *key,
            ) else {
                continue;
            };

            // require an explicit field type for builtin layouts
            let Some(declared_type) = declared_type else {
                return Err(self.missing_type_error(member_id.into_global_any(module_id)));
            };

            // resolve the field type
            let type_id = types
                .get_declared_or_inferred_type_id(declared_type.into_global_any(module_id))
                .ok_or_else(|| self.missing_type_error(declared_type.into_global_any(module_id)))?;
            let type_id = self.resolve_layout_type_id(tree, symbols, types, module_id, type_id);

            // lower the field type to MIR
            let field_mir_type =
                field_lowerer.lower_type(types, type_id, module_id, anchor, self.builder)?;
            let field_type = self.builder.tree().get(field_mir_type);
            let (size, alignment) = field_lowerer
                .size_and_align_of_type(field_type, self.builder.tree())
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "builtin layout requires concrete nested types".to_string(),
                })?;

            // collect layout inputs
            field_inputs.push(FieldInput {
                name: static_key_to_field_name(&key, self.builder),
                ty: field_mir_type,
                size,
                alignment,
                source_index: Some(source_index as u32),
                kind: FieldLayoutKind::Source,
            });
        }

        Ok(field_inputs)
    }

    /// Return a struct layout installed for the provided symbol.
    pub(crate) fn struct_layout_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
        policy: LayoutPolicy,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // require analysis for the module
        self.require_analyzed_module(symbol.module_id)?;

        // load the analyzed dir artifact
        let dir = self.require_analyzed_dir_data(symbol.module_id)?;
        let tree = &dir.tree;
        let symbols = &dir.symbols;
        let types = &dir.types;

        // resolve struct members for the symbol
        let Some(members) = self.struct_members_for_symbol(symbol, symbols, tree) else {
            return Ok(None);
        };

        // compute field inputs for the struct layout
        let field_inputs =
            self.struct_field_inputs(tree, symbols, types, &members, symbol.module_id, anchor)?;

        // install the computed layout
        if field_inputs.is_empty() {
            return Ok(None);
        }

        let layout = self
            .type_lowerer
            .compute_struct_layout(field_inputs, policy);
        let ty_struct = self.type_lowerer.create_struct_type(&layout, self.builder);
        self.type_lowerer.set_layout(ty_struct, layout);

        Ok(Some(ty_struct))
    }

    /// Assign a metadata name for a builtin type symbol.
    fn assign_metadata_name_for_symbol(
        &mut self,
        mir_type: mir::LocalNodeId<mir::Type>,
        symbol: dir::GlobalSymbolId,
    ) {
        // resolve the module symbol name
        let Some(dir) = self.artifact_dir_data_if_present(symbol.module_id) else {
            return;
        };
        let symbol_entry = dir.symbols.get_symbol(symbol.local_id);

        // resolve the symbol key
        let Some(key) = symbol_entry.key else {
            return;
        };

        // build the name string
        let name_id = static_key_to_field_name(&key, self.builder);

        // attach the name when missing
        self.builder
            .tree_mut()
            .metadata
            .layout
            .ensure_display_name(mir_type, name_id);
    }

    /// Resolve nominal aliases to their layout type.
    fn resolve_layout_type_id(
        &self,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        module_id: ModuleId,
        type_id: dir::LocalTypeId,
    ) -> dir::LocalTypeId {
        // stop when the type is not a reference
        let dir::Type::Reference { symbol, .. } = types.get_type(type_id) else {
            return type_id;
        };

        // ignore non alias symbols
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        if !matches!(
            symbol_entry.ty,
            dir::SymbolType::TypeAlias | dir::SymbolType::Newtype
        ) {
            return type_id;
        }

        // resolve the alias declaration
        let Some(primary) = symbol_entry.primary_declaration else {
            return type_id;
        };
        let Ok(declaration_id) = primary.local_id.try_into_typed::<dir::Declaration>() else {
            return type_id;
        };
        let dir::Declaration::Type(declaration) = tree.get(declaration_id) else {
            return type_id;
        };
        if !declaration.is_nominal {
            return type_id;
        }

        // return the resolved layout type when available
        types
            .get_declared_or_inferred_type_id(declaration.value.into_global_any(module_id))
            .unwrap_or(type_id)
    }
}
