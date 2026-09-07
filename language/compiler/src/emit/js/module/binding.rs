use destack_dir as dir;
use destack_js as js;
use destack_source::{NodeSpanType, ProvenanceId};

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Return or allocate the JavaScript scope for one DIR scope.
    pub(crate) fn intern_scope(
        &mut self,
        source: dir::LocalScopeId,
    ) -> Result<js::ScopeId, EmitError> {
        // return an existing scope
        if let Some(scope) = self.scopes[source.0 as usize] {
            return Ok(scope);
        }

        // require an enclosing scope
        let source_scope = self.bindings.get_scope_by_id(source);
        let Some(parent) = source_scope.parent else {
            return Err(self.internal_error(format!("DIR scope {source} has no JavaScript root")));
        };

        // allocate the scope after its parent
        let parent = self.intern_scope(parent.id)?;
        let scope = self.output.insert_scope(parent);

        self.scopes[source.0 as usize] = Some(scope);

        Ok(scope)
    }

    /// Return or allocate the JavaScript symbol for one local DIR symbol.
    pub(crate) fn intern_symbol(
        &mut self,
        source: dir::LocalSymbolId,
    ) -> Result<js::SymbolId, EmitError> {
        // return an existing symbol
        if let Some(symbol) = self.symbols[source.id as usize] {
            return Ok(symbol);
        }

        // require a named declaration
        let source_symbol = self.bindings.get_symbol(source);
        let Some(name) = source_symbol.name() else {
            return Err(
                self.internal_error(format!("DIR symbol {source:?} has no JavaScript name"))
            );
        };
        let Some(declaration) = source_symbol.declaration else {
            return Err(
                self.internal_error(format!("DIR symbol {source:?} has no source declaration"))
            );
        };

        // require a local declaration
        if declaration.module_id != self.module {
            return Err(self.internal_error(format!(
                "DIR symbol {source:?} is declared by another module"
            )));
        }

        // map the symbol namespace
        let namespace = match source_symbol.kind {
            dir::SymbolKind::Label => js::SymbolNamespace::Label,
            _ => js::SymbolNamespace::Value,
        };

        // allocate the symbol in its source scope
        let source_scope = source_symbol.scope.id;
        let provenance = self.derive_at(declaration.local_id, NodeSpanType::Main);
        let scope = self.intern_scope(source_scope)?;
        let symbol = self
            .output
            .insert_symbol(name, namespace, scope, provenance);
        self.record_source_symbol(symbol, source.into_global(self.module));

        self.symbols[source.id as usize] = Some(symbol);

        Ok(symbol)
    }

    /// Emit one bound JavaScript identifier.
    pub(crate) fn emit_binding_identifier<T: dir::Node>(
        &mut self,
        name: dir::StringId,
        source: dir::LocalNodeId<T>,
    ) -> Result<js::Identifier, EmitError> {
        let symbol = self.intern_declaration_symbol(source)?;

        Ok(js::Identifier {
            original_name: name,
            symbol,
            provenance: self.derive_at(source, NodeSpanType::Main),
        })
    }

    /// Create one JavaScript binding for a discarded DIR value.
    pub(crate) fn emit_discard_identifier(
        &mut self,
        source: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<js::Identifier, EmitError> {
        let source_scope = self.bindings.scope_at(&self.tree, source.into_any()).id;
        let scope = self.intern_scope(source_scope)?;

        // allocate an unused name in the source scope
        let name = loop {
            let text = format!("__discard{}", self.discard_count);
            self.discard_count += 1;
            let name = self.output.strings.intern(&text);
            let is_available = self
                .output
                .symbols
                .iter()
                .all(|(_, symbol)| symbol.scope != scope || symbol.name != name);

            // select the first available name
            if is_available {
                break name;
            }
        };

        // allocate the generated symbol
        let provenance = self.derive(source);
        let symbol = self
            .output
            .insert_symbol(name, js::SymbolNamespace::Value, scope, provenance);

        Ok(js::Identifier {
            original_name: name,
            symbol,
            provenance,
        })
    }

    /// Emit one optional JavaScript control label.
    pub(crate) fn emit_label(
        &mut self,
        name: Option<dir::StringId>,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<js::Identifier>, EmitError> {
        // preserve the absent label
        let Some(name) = name else {
            return Ok(None);
        };
        let identifier = self.emit_label_identifier(name, source)?;

        Ok(Some(identifier))
    }

    /// Emit one reference to an enclosing JavaScript control label.
    pub(crate) fn emit_label_reference(
        &mut self,
        name: dir::StringId,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::Identifier, EmitError> {
        let global = source.into_global_any(self.module);

        // require the selected control target
        let Some(target) = self.decisions.transfer_decision(global) else {
            return Err(self.internal_error(format!(
                "JavaScript control transfer {global:?} has no selected target"
            )));
        };

        // require the selected label
        if self.tree.get(target.local_id).control_label() != Some(name) {
            return Err(self.internal_error(format!(
                "JavaScript control transfer {global:?} selected an incompatible label target"
            )));
        }

        // require the label symbol
        let Some(symbol) = self.bindings.declaration_symbol(target.into_any()) else {
            return Err(self.internal_error(format!(
                "JavaScript control target {target:?} has no label symbol"
            )));
        };

        let symbol = self.intern_symbol(symbol)?;

        Ok(js::Identifier {
            original_name: name,
            symbol,
            provenance: self.derive_at(source, NodeSpanType::Main),
        })
    }

    /// Emit one JavaScript identifier reference.
    pub(crate) fn emit_identifier_reference(
        &mut self,
        name: dir::StringId,
        source: dir::LocalNodeIdAny,
        span_type: NodeSpanType,
    ) -> Result<js::Identifier, EmitError> {
        let global = source.into_global(self.module);
        let provenance = self.derive_at(source, span_type);
        let symbol = self.intern_reference_symbol(name, global, provenance)?;

        Ok(js::Identifier {
            original_name: name,
            symbol,
            provenance,
        })
    }

    /// Return or allocate the JavaScript symbol selected for one DIR reference.
    fn intern_reference_symbol(
        &mut self,
        name: dir::StringId,
        source: dir::GlobalNodeIdAny,
        provenance: ProvenanceId,
    ) -> Result<js::SymbolId, EmitError> {
        let reference = self.references.declaration(source);

        let symbol = match reference {
            // resolve a bound reference
            Some(dir::Reference::Bound(symbols)) => match symbols.as_slice() {
                [symbol] if symbol.module_id == self.module => {
                    self.intern_symbol(symbol.local_id)?
                }
                // preserve an external binding
                [symbol] => self.intern_global_symbol(name, Some(*symbol), provenance),
                // use the selected symbol
                _ => {
                    let Some(symbol) = self
                        .resolutions
                        .name_resolution(source)
                        .and_then(dir::NameResolution::single_symbol)
                    else {
                        return Err(self.internal_error(format!(
                            "JavaScript reference {source:?} has no single selected symbol"
                        )));
                    };

                    // preserve external selected bindings
                    if symbol.module_id == self.module {
                        self.intern_symbol(symbol.local_id)?
                    } else {
                        self.intern_global_symbol(name, Some(symbol), provenance)
                    }
                }
            },
            // resolve a local namespace declaration
            Some(dir::Reference::Namespace {
                declaration: Some(declaration),
                ..
            }) if declaration.module_id == self.module => {
                let Some(symbol) = self.bindings.declaration_symbol(*declaration) else {
                    return Err(self.internal_error(format!(
                        "JavaScript namespace reference {source:?} has no local symbol"
                    )));
                };

                self.intern_symbol(symbol)?
            }
            // preserve an external namespace reference
            Some(dir::Reference::Namespace { .. }) => {
                self.intern_global_symbol(name, None, provenance)
            }
            // resolve the base of a projected reference
            Some(dir::Reference::Projected { base, .. }) => match base {
                dir::ReferenceTarget::Symbol(symbol) if symbol.module_id == self.module => {
                    self.intern_symbol(symbol.local_id)?
                }
                dir::ReferenceTarget::Symbol(symbol) => {
                    self.intern_global_symbol(name, Some(*symbol), provenance)
                }
                dir::ReferenceTarget::Namespace(_) => {
                    self.intern_global_symbol(name, None, provenance)
                }
            },
            // reject ambiguous references
            Some(dir::Reference::Ambiguous(_)) => {
                return Err(self.internal_error(format!(
                    "ambiguous reference {source:?} reached JavaScript emission"
                )));
            }
            // reject type references
            Some(dir::Reference::TypeLiteral(_)) => {
                return Err(self.internal_error(format!(
                    "type literal reference {source:?} reached JavaScript emission"
                )));
            }
            // reject unresolved references
            Some(dir::Reference::Missing) | None => {
                return Err(self.internal_error(format!(
                    "unresolved reference {source:?} reached JavaScript emission"
                )));
            }
        };

        Ok(symbol)
    }

    /// Return or allocate one root JavaScript symbol.
    fn intern_global_symbol(
        &mut self,
        name: dir::StringId,
        source: Option<dir::GlobalSymbolId>,
        provenance: ProvenanceId,
    ) -> js::SymbolId {
        // return an existing global symbol
        let key = (name, source);
        if let Some(symbol) = self.globals.get(&key).copied() {
            return symbol;
        }

        // allocate a value symbol in the root scope
        let symbol = self.output.insert_symbol(
            name,
            js::SymbolNamespace::Value,
            js::ScopeId::ROOT,
            provenance,
        );
        if let Some(source) = source {
            self.record_source_symbol(symbol, source);
        }
        self.globals.insert(key, symbol);

        symbol
    }

    /// Record the DIR symbol represented by one JavaScript symbol.
    fn record_source_symbol(&mut self, symbol: js::SymbolId, source: dir::GlobalSymbolId) {
        let required_len = symbol.0 as usize + 1;
        if self.source_symbols.len() < required_len {
            self.source_symbols.resize(required_len, None);
        }

        self.source_symbols[symbol.0 as usize] = Some(source);
    }

    /// Emit one JavaScript label identifier.
    fn emit_label_identifier<T>(
        &mut self,
        name: dir::StringId,
        source: dir::LocalNodeId<T>,
    ) -> Result<js::Identifier, EmitError>
    where
        T: dir::Node,
    {
        let symbol = self.intern_declaration_symbol(source)?;

        Ok(js::Identifier {
            original_name: name,
            symbol,
            provenance: self.derive_at(source, NodeSpanType::Main),
        })
    }

    /// Return or allocate the JavaScript symbol declared by one DIR node.
    fn intern_declaration_symbol<T>(
        &mut self,
        source: dir::LocalNodeId<T>,
    ) -> Result<js::SymbolId, EmitError>
    where
        T: dir::Node,
    {
        let source = source.into_global_any(self.module);

        // require the DIR declaration symbol
        let Some(symbol) = self.bindings.declaration_symbol(source) else {
            return Err(self.internal_error(format!("DIR declaration {source:?} has no symbol")));
        };

        self.intern_symbol(symbol)
    }
}
