use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{LowerError, LowerResult};

const ITAB_GLOBAL_SEPARATOR: &str = "#as#";
const ITAB_GLOBAL_SUFFIX: &str = "#itab";
const VTABLE_GLOBAL_SUFFIX: &str = "#vtable";

/// MIR global that stores one dispatch table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DispatchTableGlobal {
    /// The global containing the table data.
    pub(crate) global_id: mir::LocalNodeId<mir::Global>,
    /// The raw pointer type used to reference the table.
    pub(crate) address_type: mir::LocalNodeId<mir::Type>,
}

impl ModuleLowerer<'_> {
    /// Create static storage for one virtual dispatch table.
    pub(crate) fn create_vtable_global(
        &mut self,
        symbol: dir::GlobalSymbolId,
        slot_count: u64,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<DispatchTableGlobal> {
        // table name
        let base_name =
            self.qualified_symbol_name(symbol)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(anchor),
                    message: "missing qualified name for vtable global".to_string(),
                })?;
        let name = format!("{base_name}{VTABLE_GLOBAL_SUFFIX}");

        // nullable pointer table slots
        let slot_type = self.builder.type_reference(
            mir::ReferenceKind::Raw,
            self.type_lowerer.ty_void,
            mir::Mutability::Immutable,
            mir::AddressSpace::Static,
            true,
        );
        let table_type = self
            .builder
            .type_array(slot_type, slot_count, mir::Copy::Yes);
        let global_id =
            self.builder
                .global_constant(&name, table_type, mir::GlobalInitializer::zero());

        // static table pointer
        let address_type = self.builder.type_reference(
            mir::ReferenceKind::Raw,
            table_type,
            mir::Mutability::Immutable,
            mir::AddressSpace::Static,
            false,
        );
        self.builder.tree_mut().get_mut(global_id).space = mir::AddressSpace::Static;

        Ok(DispatchTableGlobal {
            global_id,
            address_type,
        })
    }

    /// Create static storage for one interface dispatch table.
    pub(crate) fn create_itab_global(
        &mut self,
        concrete: dir::GlobalSymbolId,
        interface: dir::GlobalSymbolId,
        slot_count: u64,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<DispatchTableGlobal> {
        // table name
        let concrete_name = self.qualified_symbol_name(concrete).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "missing qualified name for itab concrete type".to_string(),
            }
        })?;
        let interface_name = self.qualified_symbol_name(interface).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "missing qualified name for itab interface type".to_string(),
            }
        })?;
        let name =
            format!("{concrete_name}{ITAB_GLOBAL_SEPARATOR}{interface_name}{ITAB_GLOBAL_SUFFIX}");

        // pointer sized table slots
        let slot_type = self.type_lowerer.ty_usize;
        let table_type = self
            .builder
            .type_array(slot_type, slot_count, mir::Copy::Yes);
        let global_id =
            self.builder
                .global_constant(&name, table_type, mir::GlobalInitializer::zero());

        // static table pointer
        let address_type = self.builder.type_reference(
            mir::ReferenceKind::Raw,
            table_type,
            mir::Mutability::Immutable,
            mir::AddressSpace::Static,
            false,
        );
        self.builder.tree_mut().get_mut(global_id).space = mir::AddressSpace::Static;

        Ok(DispatchTableGlobal {
            global_id,
            address_type,
        })
    }
}
