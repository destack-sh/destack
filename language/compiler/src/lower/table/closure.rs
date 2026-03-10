use destack_core::StringId;
use destack_dir::{CaptureKind, CapturedBinding, GlobalSymbolId, Mutability};
use {destack_dir as dir, destack_mir as mir};

use crate::lower::item::lower_mutability;
use crate::lower::r#type::{FieldInput, FieldLayoutKind, LayoutPolicy, StructLayout};
use crate::{LowerResult, ModuleLowerer};

// suffix for closure environment metadata names
const CLOSURE_ENV_METADATA_SUFFIX: &str = "#env";

/// A lowered closure environment layout.
#[derive(Debug, Clone)]
pub(crate) struct ClosureEnvLayout {
    /// The MIR struct type for the environment.
    pub(crate) env_type: mir::LocalNodeId<mir::Type>,
    /// The MIR reference type for the environment pointer.
    pub(crate) env_pointer_type: mir::LocalNodeId<mir::Type>,
    /// The ordered fields stored in the environment.
    pub(crate) fields: Vec<ClosureEnvField>,
}

impl ClosureEnvLayout {
    /// Find the field metadata for a captured symbol.
    pub(crate) fn field_for_symbol(&self, symbol: GlobalSymbolId) -> Option<&ClosureEnvField> {
        self.fields.iter().find(|field| field.symbol == symbol)
    }
}

/// A single field inside a closure environment.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClosureEnvField {
    /// The captured symbol stored in this field.
    pub(crate) symbol: GlobalSymbolId,
    /// The capture mode for this field.
    pub(crate) kind: CaptureKind,
    /// The field index in the environment layout.
    pub(crate) index: u32,
    /// The MIR type of the field.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
}

impl ModuleLowerer<'_> {
    /// Resolve a closure environment layout for a function symbol.
    pub(crate) fn closure_env_layout_for_symbol(
        &mut self,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Option<ClosureEnvLayout>> {
        // return cached layouts when available
        if let Some(layout) = self.closure_env_layouts.get(&symbol) {
            return Ok(Some(layout.clone()));
        }

        // skip functions without capture metadata
        let Some(capture_set) = self.captures.capture_set(symbol) else {
            return Ok(None);
        };
        if capture_set.captures.is_empty() {
            return Ok(None);
        }

        // build fields from captured bindings
        let mut fields = Vec::new();
        let mut field_inputs = Vec::new();
        for (index, capture) in capture_set.captures.iter().enumerate() {
            let (field_type, field_input) = self.capture_field_for_binding(*capture)?;
            fields.push(ClosureEnvField {
                symbol: capture.symbol,
                kind: capture.kind,
                index: index as u32,
                ty: field_type,
            });
            field_inputs.push(field_input);
        }

        // compute and cache the layout
        let layout = self
            .type_lowerer
            .compute_struct_layout(field_inputs, LayoutPolicy::Optimized);
        let env_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);

        // assign a metadata name for the closure environment type (manually, synthetic type)
        let metadata_name = self
            .symbol_path_name(symbol)
            .map(|name| format!("{name}{CLOSURE_ENV_METADATA_SUFFIX}"))
            .unwrap_or_else(|| format!("closure_env#{}", symbol.local_id.id));
        let metadata_name = self.builder.intern(&metadata_name);
        let metadata = self
            .builder
            .tree_mut()
            .type_table
            .type_metadata_by_id
            .entry(env_type)
            .or_default();
        if metadata.name.is_none() {
            metadata.name = Some(metadata_name);
        }

        // record layout metadata for the env type
        self.insert_layout_entry(env_type, mir::LayoutType::ClosureEnv, &layout);
        self.type_lowerer.set_layout(env_type, layout);

        // build the managed env pointer type
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );

        // cache the env layout for the function
        let env_layout = ClosureEnvLayout {
            env_type,
            env_pointer_type,
            fields,
        };
        self.closure_env_layouts.insert(symbol, env_layout.clone());

        Ok(Some(env_layout))
    }

    /// Lower a captured binding into a closure environment field.
    fn capture_field_for_binding(
        &mut self,
        capture: CapturedBinding,
    ) -> LowerResult<(mir::LocalNodeId<mir::Type>, FieldInput)> {
        // resolve the capture type
        let anchor = self.anchor_for_symbol(capture.symbol);
        let type_id = self.type_id_for_symbol_or_error(capture.symbol, anchor)?;
        let value_type = self.lower_type(type_id, anchor)?;

        // pick the field type based on capture kind
        let field_type = match capture.kind {
            CaptureKind::ByReference => {
                let mutability = self.mutability_for_symbol(capture.symbol);
                let mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Immutable);
                self.builder.type_reference(
                    mir::ReferenceKind::Managed,
                    value_type,
                    mutability,
                    mir::AddressSpace::Generic,
                    false,
                )
            }
            CaptureKind::ByValue | CaptureKind::ByMove => value_type,
        };

        // compute size and alignment for layout
        let field_ty = self.builder.tree().get(field_type);
        let (size, alignment) = self
            .type_lowerer
            .size_and_align_of_type(field_ty, self.builder.tree());

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
    fn capture_field_name(&self, symbol: GlobalSymbolId) -> StringId {
        let symbol_data = self.symbols.get_symbol(symbol.local_id);
        symbol_data
            .name()
            .unwrap_or_else(|| self.compiler.program.strings.intern("capture"))
    }

    /// Resolve a canonical empty environment type for closures.
    pub(crate) fn empty_closure_env_type(&mut self) -> mir::LocalNodeId<mir::Type> {
        if let Some(env_type) = self.empty_closure_env_type {
            return env_type;
        }

        let layout = StructLayout::empty();
        let env_type = self
            .type_lowerer
            .create_struct_type(&layout, &mut self.builder);
        self.type_lowerer.set_layout(env_type, layout);
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            true,
        );
        self.empty_closure_env_type = Some(env_type);
        self.empty_closure_env_pointer_type = Some(env_pointer_type);
        env_type
    }

    /// Resolve a canonical empty environment pointer type for closures.
    pub(crate) fn empty_closure_env_pointer_type(&mut self) -> mir::LocalNodeId<mir::Type> {
        if let Some(env_type) = self.empty_closure_env_pointer_type {
            return env_type;
        }

        let env_type = self.empty_closure_env_type();
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            true,
        );
        self.empty_closure_env_pointer_type = Some(env_pointer_type);
        env_pointer_type
    }

    /// Resolve an anchored node id for a symbol.
    fn anchor_for_symbol(&self, symbol: GlobalSymbolId) -> dir::AnchoredGlobalNodeId {
        let symbol_data = self.symbols.get_symbol(symbol.local_id);
        if let Some(primary) = symbol_data.primary_declaration {
            return primary.into_anchored(Some(self.profile));
        }

        let anchor = self.module.dir(self.profile).anchor_node;
        anchor
            .into_global(self.module_id)
            .into_anchored(Some(self.profile))
    }

    /// Resolve a mutability hint for a captured symbol.
    fn mutability_for_symbol(&self, symbol: GlobalSymbolId) -> Option<Mutability> {
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
