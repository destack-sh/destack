use destack_artifact::{DiagnosticAnchor, DirDeclared, GlobalEnvironment};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};
use std::sync::Arc;

use crate::{Compiler, CompilerError, CompilerResult, LowerError};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::lower::{static_key_from_key, static_key_to_field_name};

/// Helpers for lowering builtin type layouts.
pub(crate) struct BuiltinTypeLayouts<'a, 'b> {
    /// Access to compiler helpers and shared state.
    compiler: &'a Compiler,
    /// Pinned context for artifact reads.
    context: &'a dyn ProviderContext,
    /// Profile used for lookup and analysis.
    profile: ProfileId,
    /// MIR module builder to install layouts.
    builder: &'b mut mir::ModuleBuilder,
    /// Type lowerer used to cache builtin layouts.
    type_lowerer: &'b mut TypeLowerer<'a>,
}

impl<'a, 'b> BuiltinTypeLayouts<'a, 'b> {
    /// Create a builtin layout helper.
    pub(crate) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        profile: ProfileId,
        builder: &'b mut mir::ModuleBuilder,
        type_lowerer: &'b mut TypeLowerer<'a>,
    ) -> Self {
        Self {
            compiler,
            context,
            profile,
            builder,
            type_lowerer,
        }
    }

    /// Return a diagnostic anchor for one DIR node.
    fn diagnostic_anchor(&self, node: dir::AnchoredGlobalNodeId) -> DiagnosticAnchor {
        self.type_lowerer.diagnostic_anchor(node)
    }

    /// Read one committed checked DIR snapshot for a module.
    fn require_checked_dir_data(&self, module_id: ModuleId) -> CompilerResult<dir::TypeTable> {
        let snapshot = self
            .compiler
            .dir_checked(self.context, module_id, self.profile);

        match snapshot {
            Ok(snapshot) => Ok(snapshot.types.clone()),
            Err(error) => Err(error.into()),
        }
    }

    /// Read one committed declared DIR snapshot for a module.
    fn require_declared_dir_data(&self, module_id: ModuleId) -> CompilerResult<Arc<DirDeclared>> {
        let snapshot = self
            .compiler
            .dir_declared(self.context, module_id, self.profile);

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(error.into()),
        }
    }

    /// Return the builtin String type for lowering.
    pub(crate) fn string_type_for_builtin(
        &mut self,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // skip when already cached
        if let Some(string_type) = self.type_lowerer.string_type() {
            return Ok(Some(string_type));
        }

        // resolve the builtin string symbol
        let Some(string_symbol) =
            self.resolve_well_known_symbol(dir::WellKnownSymbol::String, dir::SymbolSpace::Type)?
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
        order: dir::SymbolSpace,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let environment = self.global_environment()?;
        let resolved = self.well_known_symbol(symbol, order)?;

        // fall back to selected global symbols when not registered
        let Some(resolved) = resolved else {
            return Ok(environment.symbol_from(symbol.export_name(), order));
        };

        Ok(Some(resolved))
    }

    /// Read the global environment used by builtin layout lowering.
    fn global_environment(&self) -> CompilerResult<Arc<GlobalEnvironment>> {
        self.compiler
            .global_environment(self.context, self.profile)
            .map_err(CompilerError::from)
    }

    /// Resolve one well-known symbol from the global environment.
    fn well_known_symbol(
        &self,
        symbol: dir::WellKnownSymbol,
        space: dir::SymbolSpace,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let environment = self.global_environment()?;
        let pair = environment.well_known_symbols().get_pair(symbol);

        Ok(pair.and_then(|pair| match space {
            dir::SymbolSpace::Type => pair.ty.or(pair.value),
            dir::SymbolSpace::Value | dir::SymbolSpace::Label => pair.value.or(pair.ty),
        }))
    }

    /// Ensure the module has been checked for this profile.
    fn require_checked_module(&self, module_id: ModuleId) -> CompilerResult<()> {
        // request checked DIR for the target module
        let result = self
            .compiler
            .require_dir_checked(self.context, module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        // forward dependency failures as lower errors
        Err(error.into())
    }

    /// Create a MissingType error for a node.
    fn missing_type_error(&self, node_id: dir::GlobalNodeIdAny) -> LowerError {
        LowerError::MissingType {
            anchor: self.diagnostic_anchor(node_id.into_anchored(Some(self.profile))),
        }
    }

    /// Resolve the struct members in the builtin module.
    fn struct_members_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
        symbols: &dir::SymbolTable,
        tree: &dir::Tree,
    ) -> Option<Vec<dir::LocalNodeId<dir::Member>>> {
        // resolve the declaration for the symbol
        let declaration = symbols.get_symbol(symbol.local_id).declaration?;
        let declaration_id = declaration
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
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        strings: &StringPool,
        members: &[dir::LocalNodeId<dir::Member>],
        module_id: ModuleId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<Vec<FieldInput>> {
        // compute layout from declared struct fields only
        let mut field_inputs = Vec::new();
        let pointer_bytes = self.type_lowerer.pointer_bytes();

        let vector_symbol =
            self.well_known_symbol(dir::WellKnownSymbol::Vector, dir::SymbolSpace::Type)?;
        let mut field_lowerer = TypeLowerer::new(
            self.builder,
            pointer_bytes,
            self.compiler,
            self.context,
            strings,
            self.profile,
            tree,
            symbols,
            vector_symbol,
        );

        for (source_index, member_id) in members.iter().enumerate() {
            let dir::Member::Field {
                key, declared_type, ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            // resolve a static key for the field
            let Some(key) = static_key_from_key(tree, strings, *key) else {
                continue;
            };

            // require an explicit field type for builtin layouts
            let Some(declared_type) = declared_type else {
                return Err(self
                    .missing_type_error(member_id.into_global_any(module_id))
                    .into());
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
                    anchor: self.diagnostic_anchor(anchor),
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
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // require checked DIR for the module
        self.require_checked_module(symbol.module_id)?;

        // load the declared structure and checked type store
        let declared = self.require_declared_dir_data(symbol.module_id)?;
        let types = self.require_checked_dir_data(symbol.module_id)?;
        let tree = &declared.tree;
        let symbols = &declared.symbols;

        // resolve struct members for the symbol
        let Some(members) = self.struct_members_for_symbol(symbol, symbols, tree) else {
            return Ok(None);
        };

        // compute field inputs for the struct layout
        let field_inputs = self.struct_field_inputs(
            tree,
            symbols,
            &types,
            &declared.strings,
            &members,
            symbol.module_id,
            anchor,
        )?;

        // install the computed layout
        if field_inputs.is_empty() {
            return Ok(None);
        }

        let layout = TypeLowerer::compute_struct_layout(field_inputs, policy);
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
        let Ok(dir) = self
            .compiler
            .dir_declared(self.context, symbol.module_id, self.profile)
        else {
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
            .types
            .ensure_display_name(mir_type, name_id);
    }

    /// Resolve nominal aliases to their layout type.
    fn resolve_layout_type_id(
        &self,
        tree: &dir::Tree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        module_id: ModuleId,
        type_id: dir::LocalTypeId,
    ) -> dir::LocalTypeId {
        // stop when the type is not a reference
        let dir::Type::Reference(reference) = types.get_type(type_id) else {
            return type_id;
        };

        // ignore non alias symbols
        let symbol_entry = symbols.get_symbol(reference.symbol.local_id);
        if !matches!(
            symbol_entry.form,
            dir::SymbolForm::TypeAlias | dir::SymbolForm::Newtype
        ) {
            return type_id;
        }

        // resolve the alias declaration
        let Some(primary) = symbol_entry.declaration else {
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
