use destack_artifact::{
    DiagnosticAnchor, DirBound, DirChecked, DirExpanded, DirParsed, GlobalEnvironment,
};
use destack_core::StringPool;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};
use std::sync::Arc;
use {destack_dir as dir, destack_mir as mir};

use crate::{Compiler, CompilerError, CompilerResult, LowerError};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::lower::static_key_to_field_name;

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
    fn require_checked_dir_data(&self, module_id: ModuleId) -> CompilerResult<Arc<DirChecked>> {
        let snapshot = self
            .compiler
            .artifact_reader(self.context)
            .dir_checked(module_id, self.profile);

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(error.into()),
        }
    }

    /// Read one committed bound DIR snapshot for a module.
    fn require_bound_dir(&self, module_id: ModuleId) -> CompilerResult<Arc<DirBound>> {
        let snapshot = self
            .compiler
            .artifact_reader(self.context)
            .dir_bound(module_id, self.profile);

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(error.into()),
        }
    }

    /// Read one committed expanded DIR snapshot for a module.
    fn require_expanded_dir(&self, module_id: ModuleId) -> CompilerResult<Arc<DirExpanded>> {
        let snapshot = self
            .compiler
            .artifact_reader(self.context)
            .dir_expanded(module_id, self.profile);

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(error.into()),
        }
    }

    /// Read one committed parsed DIR snapshot for a module.
    fn require_parsed_dir(&self, module_id: ModuleId) -> CompilerResult<Arc<DirParsed>> {
        let snapshot = self
            .compiler
            .artifact_reader(self.context)
            .dir_parsed(module_id);

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
        let Some(string_symbol) = self.resolve_language_item(dir::LanguageItem::String)? else {
            return Ok(None);
        };

        // install the struct layout for String in source order
        let Some(ty_struct) =
            self.struct_layout_for_symbol(string_symbol, anchor, LayoutPolicy::Source)?
        else {
            return Ok(None);
        };

        // name the language string type metadata
        self.assign_metadata_name_for_symbol(ty_struct, string_symbol);

        // cache the managed reference type
        let ty_string = self.builder.type_managed_reference(ty_struct);
        self.type_lowerer.set_string_type(ty_string);

        Ok(Some(ty_string))
    }

    /// Resolve a language item for this profile.
    pub(crate) fn resolve_language_item(
        &self,
        symbol: dir::LanguageItem,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let environment = self.global_environment()?;
        let resolved = environment.language.item(symbol);

        // fall back to selected global symbols when not registered
        let Some(resolved) = resolved else {
            return Ok(environment.language.symbol(symbol.export_name()));
        };

        Ok(Some(resolved))
    }

    /// Read the global environment used by builtin layout lowering.
    fn global_environment(&self) -> CompilerResult<Arc<GlobalEnvironment>> {
        self.compiler
            .artifact_reader(self.context)
            .global_environment(self.profile)
            .map_err(CompilerError::from)
    }

    /// Ensure the module has been checked for this profile.
    fn require_checked_module(&self, module_id: ModuleId) -> CompilerResult<()> {
        // request checked DIR for the target module
        let result = self
            .compiler
            .artifact_reader(self.context)
            .dir_checked(module_id, self.profile);
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
        symbols: &dir::BindingTable<'_>,
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
        symbols: &dir::BindingTable<'_>,
        types: &dir::TypeTable<'_>,
        strings: &StringPool,
        members: &[dir::LocalNodeId<dir::Member>],
        module_id: ModuleId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<Vec<FieldInput>> {
        // compute layout from declared struct fields only
        let mut field_inputs = Vec::new();
        let pointer_bytes = self.type_lowerer.pointer_bytes();

        let vector_symbol = self.resolve_language_item(dir::LanguageItem::Vector)?;
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
            let Some(key) = key.static_key(tree) else {
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

        // load the bound structure and checked type store
        let parsed = self.require_parsed_dir(symbol.module_id)?;
        let bound = self.require_bound_dir(symbol.module_id)?;
        let expanded = self.require_expanded_dir(symbol.module_id)?;
        let checked = self.require_checked_dir_data(symbol.module_id)?;
        let tree = &parsed.tree;
        let symbols = expanded.binding_table(&bound);
        let types = checked.type_table(&bound, &expanded);

        // resolve struct members for the symbol
        let Some(members) = self.struct_members_for_symbol(symbol, &symbols, tree) else {
            return Ok(None);
        };

        // compute field inputs for the struct layout
        let field_inputs = self.struct_field_inputs(
            tree,
            &symbols,
            &types,
            self.compiler.repository.string_pool().as_ref(),
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
            .artifact_reader(self.context)
            .dir_bound(symbol.module_id, self.profile)
        else {
            return;
        };
        let Ok(expanded) = self
            .compiler
            .artifact_reader(self.context)
            .dir_expanded(symbol.module_id, self.profile)
        else {
            return;
        };
        let bindings = expanded.binding_table(&dir);
        let symbol_entry = bindings.get_symbol(symbol.local_id);

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
        symbols: &dir::BindingTable<'_>,
        types: &dir::TypeTable<'_>,
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
