use std::collections::HashSet;

use destack_dir as dir;

use super::builder::CompletionBuilder;
use super::{Completion, CompletionValueShape};
use crate::format::format_global_type;

impl CompletionBuilder<'_, '_> {
    /// Return the canonical dependency target for one symbol.
    pub(super) fn canonical_symbol(&self, symbol_id: dir::GlobalSymbolId) -> dir::GlobalSymbolId {
        let module = self.module.module_context(symbol_id.module_id);

        module.canonical_symbol(symbol_id)
    }

    /// Resolve one callable and constructable value shape from one symbol.
    pub(super) fn symbol_value_shape(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<CompletionValueShape> {
        let module = self.module.module_context(symbol_id.module_id);
        let types = module.types();
        let type_id = types.get_symbol_type_id(symbol_id)?;

        Some(self.type_value_shape(type_id))
    }

    /// Resolve one callable and constructable value shape from one type id.
    pub(super) fn type_value_shape(&self, type_id: dir::GlobalTypeId) -> CompletionValueShape {
        self.module.read_global_type(type_id, |ty, _| match ty {
            dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => CompletionValueShape {
                is_callable: true,
                is_constructable: false,
            },
            dir::Type::Shape(object) => CompletionValueShape {
                is_callable: !object.call_signatures.is_empty(),
                is_constructable: !object.construct_signatures.is_empty(),
            },
            _ => CompletionValueShape::default(),
        })
    }

    /// Return one canonical nominal symbol from one symbol.
    pub(super) fn symbol_nominal(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let module = self.module.module_context(symbol_id.module_id);
        let types = module.types();
        let type_id = types.get_symbol_type_id(symbol_id)?;

        self.type_nominal(type_id)
    }

    /// Return one canonical nominal symbol from one type id.
    pub(super) fn type_nominal(&self, type_id: dir::GlobalTypeId) -> Option<dir::GlobalSymbolId> {
        let symbol_id = self.module.read_global_type(type_id, |ty, _| ty.symbol())?;

        Some(self.canonical_symbol(symbol_id))
    }

    /// Return related nominal symbols from one symbol.
    pub(super) fn symbol_related_nominals(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        let module = self.module.module_context(symbol_id.module_id);
        let types = module.types();
        let Some(type_id) = types.get_symbol_type_id(symbol_id) else {
            return Vec::new();
        };

        self.related_nominals(type_id)
    }

    /// Return related nominal symbols from one type id.
    pub(super) fn related_nominals(&self, type_id: dir::GlobalTypeId) -> Vec<dir::GlobalSymbolId> {
        let mut symbols = Vec::new();
        let mut seen_types = HashSet::new();
        let mut seen_symbols = HashSet::new();

        self.collect_related_nominals(type_id, &mut seen_types, &mut seen_symbols, &mut symbols);

        symbols
    }

    /// Format a type detail string for a symbol's declared or inferred type.
    pub(super) fn format_symbol_type_detail(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<String> {
        let module = self.module.module_context(symbol_id.module_id);

        let symbols = module.symbols();
        let types = module.types();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let declaration = symbol.declaration?;
        let type_id = types.get_node_type_id(declaration)?;

        format_global_type(type_id, &module)
    }

    /// Attach documentation to a completion when the backing symbol has docs.
    pub(super) fn attach_completion_documentation(
        &self,
        completion: Completion,
        symbol_id: dir::GlobalSymbolId,
    ) -> Completion {
        let symbol_id = self.canonical_symbol(symbol_id);
        let documentation = self.module.symbol_doc_text(symbol_id);
        let Some(documentation) = documentation else {
            return completion;
        };

        completion.with_documentation(documentation)
    }

    /// Collect related nominal symbols from one type.
    fn collect_related_nominals(
        &self,
        type_id: dir::GlobalTypeId,
        seen_types: &mut HashSet<dir::GlobalTypeId>,
        seen_symbols: &mut HashSet<dir::GlobalSymbolId>,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) {
        let type_id = self.module.unwrap_global_form_payload_type_id(type_id);
        if !seen_types.insert(type_id) {
            return;
        }

        self.module
            .read_global_type(type_id, |ty, type_module| match ty {
                dir::Type::Instance(reference) => {
                    let symbol = reference.symbol;
                    let canonical_symbol = self.canonical_symbol(symbol);
                    if seen_symbols.insert(canonical_symbol) {
                        symbols.push(canonical_symbol);
                    }

                    let types = type_module.types();
                    if let Some(target_type_id) = types.get_symbol_type_id(symbol) {
                        self.collect_related_nominals(
                            target_type_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Union(union) => {
                    for &element_id in type_module.types().type_ids(union.elements) {
                        self.collect_related_nominals(
                            element_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Intersection(intersection) => {
                    for &element_id in type_module.types().type_ids(intersection.elements) {
                        self.collect_related_nominals(
                            element_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Form(value) => {
                    self.collect_related_nominals(value.value, seen_types, seen_symbols, symbols);
                }
                _ => {}
            });
    }
}
