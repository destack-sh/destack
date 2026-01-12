use destack_dir::{
    self as dir, Declaration, Member, StaticKey, StringId, SymbolSpaceOrder, SymbolType, TypeKind,
    WellKnownSymbol,
};
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, LowerError, LowerResult, TaskDependencyError};

use super::{FieldInput, LayoutPolicy, TypeLowerer, compute_struct_layout, size_and_align_of_type};

/// Helpers for lowering builtin type layouts.
pub(crate) struct BuiltinTypeLayouts<'a> {
    /// Access to compiler helpers and shared state.
    compiler: &'a Compiler,
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
        profile: ProfileId,
        builder: &'a mut mir::ModuleBuilder,
        type_lowerer: &'a mut TypeLowerer,
    ) -> Self {
        Self {
            compiler,
            profile,
            builder,
            type_lowerer,
        }
    }

    /// Ensure the String layout is cached for lowering.
    pub(crate) fn ensure_string_layout(
        &mut self,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<()> {
        // skip when already cached
        if self.type_lowerer.string_type().is_some() {
            return Ok(());
        }

        // resolve the builtin string symbol
        let Some(string_symbol) = self.resolve_well_known_symbol(WellKnownSymbol::String) else {
            return Ok(());
        };

        // install the struct layout for String
        let Some(ty_struct) = self.ensure_struct_layout_for_symbol(string_symbol, anchor)? else {
            return Ok(());
        };

        // cache the managed reference type
        let ty_string = self.builder.type_managed_reference(ty_struct);
        self.type_lowerer.set_string_type(ty_string);

        Ok(())
    }

    /// Resolve a well-known symbol for this profile.
    pub(crate) fn resolve_well_known_symbol(
        &self,
        symbol: WellKnownSymbol,
    ) -> Option<dir::GlobalSymbolId> {
        // resolve the canonical well-known symbol
        let resolved = self
            .compiler
            .get_well_known_type_symbol(self.profile, symbol)
            .or_else(|| self.compiler.get_well_known_symbol(self.profile, symbol));

        // fall back to declared lib symbols when not registered
        let Some(resolved) = resolved else {
            let name = self.compiler.program.strings.intern(symbol.export_name());
            return self.resolve_declared_lib_symbol(name, SymbolSpaceOrder::ValueThenType);
        };

        Some(resolved)
    }

    /// Resolve a declared lib symbol for this profile.
    pub(crate) fn resolve_declared_lib_symbol(
        &self,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<dir::GlobalSymbolId> {
        // resolve the declared lib symbol for the profile
        self.compiler
            .get_declared_lib_symbol_for_space_order(self.profile, name, order)
    }

    /// Ensure the module has been analyzed for this profile.
    fn require_analyzed_module(&self, module_id: ModuleId) -> LowerResult<()> {
        // request the analyzed module
        let result = self
            .compiler
            .require_analyze_module(module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        // forward dependency failures as lower errors
        match error {
            TaskDependencyError::NotReady { dependency } => Err(LowerError::Yield { dependency }),
            TaskDependencyError::Failed { dependency } => {
                Err(LowerError::UnsatisfiedDependency { dependency })
            }
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
        let Declaration::Struct { members, .. } = tree.get(declaration_id) else {
            return None;
        };

        Some(members.clone())
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
        let pointer_bytes = self.type_lowerer.pointer_bytes();
        let mut field_inputs = Vec::new();
        let mut field_lowerer = TypeLowerer::new(self.builder, pointer_bytes);

        for (source_index, member_id) in members.iter().enumerate() {
            let Member::Field { key, value, .. } = tree.get(*member_id) else {
                continue;
            };

            // resolve a static key for the field
            let Some(key) = key.and_then(|key| {
                self.compiler
                    .static_key_from_dynamic_key(self.profile, key, tree, symbols, types)
            }) else {
                continue;
            };

            // resolve the field type
            let Some(value) = value else {
                return Err(LowerError::MissingType {
                    node: member_id
                        .into_global_any(module_id)
                        .into_anchored(Some(self.profile)),
                });
            };
            let type_id = types
                .get_declared_or_inferred_type_id(value.into_global_any(module_id))
                .ok_or_else(|| LowerError::MissingType {
                    node: value
                        .into_global_any(module_id)
                        .into_anchored(Some(self.profile)),
                })?;
            let type_id = self.resolve_layout_type_id(tree, symbols, types, module_id, type_id);

            // lower the field type to MIR
            let field_mir_type =
                field_lowerer.lower_type(types, type_id, module_id, anchor, self.builder)?;
            let field_type = self.builder.tree().get(field_mir_type);
            let (size, alignment) =
                size_and_align_of_type(field_type, self.builder.tree(), pointer_bytes);

            // collect layout inputs
            field_inputs.push(FieldInput {
                name: self.field_name_for_key(key),
                ty: field_mir_type,
                size,
                alignment,
                source_index: source_index as u32,
            });
        }

        Ok(field_inputs)
    }

    /// Ensure a struct layout is installed for the provided symbol.
    pub(crate) fn ensure_struct_layout_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // require analysis for the module
        self.require_analyzed_module(symbol.module_id)?;

        // load module state
        let module = self.compiler.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // resolve struct members for the symbol
        let Some(members) = self.struct_members_for_symbol(symbol, &symbols, &tree) else {
            return Ok(None);
        };

        // compute field inputs for the struct layout
        let field_inputs =
            self.struct_field_inputs(&tree, &symbols, &types, &members, module.id, anchor)?;

        // install the computed layout
        if field_inputs.is_empty() {
            return Ok(None);
        }

        let layout = compute_struct_layout(field_inputs, LayoutPolicy::default());
        let ty_struct = self.type_lowerer.create_struct_type(&layout, self.builder);
        self.type_lowerer.set_layout(ty_struct, layout);

        Ok(Some(ty_struct))
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
        if symbol_entry.ty != SymbolType::TypeAlias {
            return type_id;
        }

        // resolve the alias declaration
        let Some(primary) = symbol_entry.primary_declaration else {
            return type_id;
        };
        let Ok(declaration_id) = primary.local_id.try_into_typed::<dir::Declaration>() else {
            return type_id;
        };
        let Declaration::Type { kind, value, .. } = tree.get(declaration_id) else {
            return type_id;
        };
        if *kind != TypeKind::Nominal {
            return type_id;
        }

        // return the resolved layout type when available
        types
            .get_declared_or_inferred_type_id(value.into_global_any(module_id))
            .unwrap_or(type_id)
    }

    /// Convert a static key to a field name.
    fn field_name_for_key(&mut self, key: StaticKey) -> StringId {
        // map keys to stable field names
        match key {
            StaticKey::Name(name) | StaticKey::Number(name) => name,
            StaticKey::Symbol(symbol_key) => {
                let synthetic = match symbol_key {
                    dir::SymbolKey::WellKnown(well_known) => {
                        format!("@{}", well_known.global_symbol_name())
                    }
                    dir::SymbolKey::Registry(name) => {
                        let key_str = self.builder.strings().get(name);
                        format!("@Symbol.for:{}", &*key_str)
                    }
                    dir::SymbolKey::Unique(global_id) => format!("@Symbol#{global_id:?}"),
                };
                self.builder.intern(&synthetic)
            }
        }
    }
}
