use destack_core::StringId;
use {destack_dir as dir, destack_mir as mir};

use crate::lower::lower_mutability;
use crate::lower::r#type::{FieldInput, FieldLayoutKind, LayoutPolicy, StructLayout};
use crate::{LowerError, LowerResult, ModuleLowerer};

// suffix for function environment metadata names
const FUNCTION_ENVIRONMENT_METADATA_SUFFIX: &str = "#env";
const EMPTY_FUNCTION_ENVIRONMENT_METADATA_NAME: &str = "EmptyFunctionEnvironment";

/// A lowered function environment layout.
#[derive(Debug, Clone)]
pub(crate) struct FunctionEnvironmentLayout {
    /// The MIR struct type for the environment.
    pub(crate) env_type: mir::LocalNodeId<mir::Type>,
    /// The MIR reference type for the environment pointer.
    pub(crate) env_pointer_type: mir::LocalNodeId<mir::Type>,
    /// The ordered fields stored in the environment.
    pub(crate) fields: Vec<FunctionEnvironmentField>,
}

impl FunctionEnvironmentLayout {
    /// Find the field metadata for a captured symbol.
    pub(crate) fn field_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&FunctionEnvironmentField> {
        self.fields.iter().find(|field| field.symbol == symbol)
    }
}

/// A single field inside one function environment.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionEnvironmentField {
    /// The captured symbol stored in this field.
    pub(crate) symbol: dir::GlobalSymbolId,
    /// The capture mode for this field.
    pub(crate) kind: dir::CaptureKind,
    /// The field index in the environment layout.
    pub(crate) index: u32,
    /// The MIR type of the field.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
}

impl ModuleLowerer<'_> {
    /// Resolve a function environment layout for a function symbol.
    pub(crate) fn function_environment_layout_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<Option<FunctionEnvironmentLayout>> {
        // return cached layouts when available
        if let Some(layout) = self.function_environment_layouts.get(&symbol) {
            return Ok(Some(layout.clone()));
        }

        // skip functions without capture metadata
        let Some(capture_set) = self.captures.capture_set(symbol) else {
            return Ok(None);
        };
        if capture_set.captures.is_empty() {
            return Ok(None);
        }

        // collect captured bindings and their field inputs
        let mut captures = Vec::new();
        let mut field_inputs = Vec::new();
        for capture in &capture_set.captures {
            let (field_type, field_input) = self.capture_field_for_binding(*capture)?;
            captures.push((*capture, field_type));
            field_inputs.push(field_input);
        }

        // compute the actual layout order
        let layout = self
            .type_lowerer
            .compute_struct_layout(field_inputs, LayoutPolicy::Optimized);

        // record fields using their concrete layout indices
        let mut fields = Vec::with_capacity(captures.len());
        for (capture, field_type) in captures {
            let Some(index) = layout.field_index_by_source(capture.symbol.local_id.id) else {
                return Err(LowerError::Internal {
                    module: self.module_id,
                    message: "missing function environment field in computed layout".to_string(),
                });
            };

            fields.push(FunctionEnvironmentField {
                symbol: capture.symbol,
                kind: capture.kind,
                index,
                ty: field_type,
            });
        }

        let env_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);

        // assign a metadata name for the function environment type
        let metadata_name = self
            .symbol_path_name(symbol)
            .map(|name| format!("{name}{FUNCTION_ENVIRONMENT_METADATA_SUFFIX}"))
            .unwrap_or_else(|| format!("env.{}", symbol.local_id.id));
        let metadata_name = self.builder.intern(&metadata_name);
        self.builder
            .tree_mut()
            .metadata
            .layout
            .ensure_display_name(env_type, metadata_name);

        // record layout metadata for the env type
        self.insert_layout_entry(env_type, mir::LayoutKind::CallableEnvironment, &layout);
        self.type_lowerer.set_layout(env_type, layout);

        // build the managed env pointer type
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Local,
            false,
        );

        // cache the env layout for the function
        let env_layout = FunctionEnvironmentLayout {
            env_type,
            env_pointer_type,
            fields,
        };
        self.function_environment_layouts
            .insert(symbol, env_layout.clone());

        Ok(Some(env_layout))
    }

    /// Lower a captured binding into a function environment field.
    fn capture_field_for_binding(
        &mut self,
        capture: dir::CapturedBinding,
    ) -> LowerResult<(mir::LocalNodeId<mir::Type>, FieldInput)> {
        // resolve the capture type
        let anchor = self.anchor_for_symbol(capture.symbol);
        let type_id = self.type_id_for_symbol_or_error(capture.symbol, anchor)?;
        let value_type = self.lower_type(type_id, anchor)?;

        // pick the field type based on capture kind
        let field_type = match capture.kind {
            dir::CaptureKind::ByReference => {
                let mutability = self.mutability_for_symbol(capture.symbol);
                let mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Immutable);
                self.builder.type_reference(
                    mir::ReferenceKind::Managed,
                    value_type,
                    mutability,
                    mir::AddressSpace::Local,
                    false,
                )
            }
            dir::CaptureKind::ByValue | dir::CaptureKind::ByMove => value_type,
        };

        // compute size and alignment for layout
        let field_ty = self.builder.tree().get(field_type);
        let (size, alignment) = self
            .type_lowerer
            .size_and_align_of_type(field_ty, self.builder.tree())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: anchor,
                message: "closure layout requires concrete nested types".to_string(),
            })?;

        // assign a stable field name
        let field_name = self.capture_field_name(capture.symbol);

        let input = FieldInput {
            name: field_name,
            ty: field_type,
            size,
            alignment,
            source_index: Some(capture.symbol.local_id.id),
            kind: FieldLayoutKind::Synthetic,
        };

        Ok((field_type, input))
    }

    /// Resolve a stable field name for a captured symbol.
    fn capture_field_name(&self, symbol: dir::GlobalSymbolId) -> StringId {
        let symbol_data = self.symbols.get_symbol(symbol.local_id);
        symbol_data
            .name()
            .unwrap_or_else(|| self.compiler.repository.strings.intern("capture"))
    }

    /// Resolve a canonical empty function environment type.
    pub(crate) fn empty_function_environment_type(&mut self) -> mir::LocalNodeId<mir::Type> {
        if let Some(env_type) = self.empty_function_environment_type {
            return env_type;
        }

        let layout = StructLayout::empty();
        let env_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);

        // assign a stable metadata name for the canonical empty environment
        let metadata_name = self
            .builder
            .intern(EMPTY_FUNCTION_ENVIRONMENT_METADATA_NAME);
        self.builder
            .tree_mut()
            .metadata
            .layout
            .ensure_display_name(env_type, metadata_name);

        self.type_lowerer.set_layout(env_type, layout);
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Local,
            true,
        );
        self.empty_function_environment_type = Some(env_type);
        self.empty_function_environment_pointer_type = Some(env_pointer_type);
        env_type
    }

    /// Resolve a canonical empty function environment pointer type.
    pub(crate) fn empty_function_environment_pointer_type(
        &mut self,
    ) -> mir::LocalNodeId<mir::Type> {
        if let Some(env_type) = self.empty_function_environment_pointer_type {
            return env_type;
        }

        let env_type = self.empty_function_environment_type();
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Local,
            true,
        );
        self.empty_function_environment_pointer_type = Some(env_pointer_type);
        env_pointer_type
    }

    /// Resolve an anchored node id for a symbol.
    fn anchor_for_symbol(&self, symbol: dir::GlobalSymbolId) -> dir::AnchoredGlobalNodeId {
        let symbol_data = self.symbols.get_symbol(symbol.local_id);
        if let Some(primary) = symbol_data.primary_declaration {
            return primary.into_anchored(Some(self.profile));
        }

        let anchor = self.anchor_node;
        anchor
            .into_global(self.module_id)
            .into_anchored(Some(self.profile))
    }

    /// Resolve a mutability hint for a captured symbol.
    fn mutability_for_symbol(&self, symbol: dir::GlobalSymbolId) -> Option<dir::Mutability> {
        // use the primary declaration when available
        let symbol_data = self.symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_data.primary_declaration?;

        // skip declarations from other modules
        if primary_declaration.module_id != self.module_id {
            return None;
        }

        // parameters are treated as mutable by default
        if primary_declaration.local_id.ty == dir::NodeType::Parameter {
            return None;
        }

        // walk up to find the owning let binding
        let mut current = primary_declaration.local_id;
        loop {
            let parent_id = self.dir_tree.get_parent(current.id)?;
            if parent_id.ty == dir::NodeType::Expression {
                let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_id.id);
                let parent_expression = self.dir_tree.get(parent_expression_id);
                if let dir::Expression::Let { mutability, .. } = parent_expression {
                    return Some(*mutability);
                }
            }
            current = parent_id;
        }
    }
}
