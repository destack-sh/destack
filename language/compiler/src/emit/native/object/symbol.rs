use tspp_core::FxIndexMap;

use cranelift_module::{DataDescription, DataId, Linkage, Module, ModuleResult};
use cranelift_object::ObjectModule;
use tspp_native as native;

/// Object-local symbols referenced by generated native code.
#[derive(Debug, Default)]
pub(in crate::emit::native) struct SymbolTable {
    /// Symbols in object-local identity order.
    values: Vec<native::Symbol>,
    /// Object-local identities keyed by symbol.
    indices: FxIndexMap<native::Symbol, native::SymbolId>,
    /// Cranelift data identities keyed by object symbol.
    data: FxIndexMap<native::Symbol, DataId>,
    /// Object-local symbols keyed by Cranelift data identity.
    data_symbols: FxIndexMap<DataId, native::SymbolId>,
}

impl SymbolTable {
    /// Return or insert one object-local symbol.
    pub(in crate::emit::native) fn insert(&mut self, symbol: native::Symbol) -> native::SymbolId {
        if let Some(id) = self.indices.get(&symbol) {
            return *id;
        }

        let id = native::SymbolId(self.values.len() as u32);
        self.values.push(symbol);
        self.indices.insert(symbol, id);

        id
    }

    /// Return or declare one addressable Program index.
    pub(in crate::emit::native) fn declare(
        &mut self,
        index: native::Index,
        byte_len: usize,
        alignment: u64,
        output: &mut ObjectModule,
    ) -> ModuleResult<DataId> {
        self.declare_symbol(native::Symbol::Index(index), byte_len, alignment, output)
    }

    /// Return or declare one addressable object symbol.
    fn declare_symbol(
        &mut self,
        symbol: native::Symbol,
        byte_len: usize,
        alignment: u64,
        output: &mut ObjectModule,
    ) -> ModuleResult<DataId> {
        if let Some(data) = self.data.get(&symbol) {
            return Ok(*data);
        }

        // define one local cell so Cranelift emits a position-independent data reference
        let symbol_id = self.insert(symbol);
        let name = format!("__destack_symbol_{}", symbol_id.0);
        let data = output.declare_data(&name, Linkage::Local, false, false)?;
        let mut description = DataDescription::new();
        description.define_zeroinit(byte_len);
        description.set_align(alignment);
        output.define_data(data, &description)?;

        self.data.insert(symbol, data);
        self.data_symbols.insert(data, symbol_id);

        Ok(data)
    }

    /// Return the object symbol represented by one Cranelift data identity.
    pub(in crate::emit::native) fn data(&self, data: DataId) -> Option<native::SymbolId> {
        self.data_symbols.get(&data).copied()
    }

    /// Consume this table into symbols in object-local identity order.
    pub(in crate::emit::native) fn into_values(self) -> Vec<native::Symbol> {
        self.values
    }
}
