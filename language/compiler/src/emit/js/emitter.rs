use destack_artifact::Script;
use destack_core::{FxIndexMap, StringPool};
use destack_dir as dir;
use destack_js as js;
use destack_source::{ModuleId, NodeSpanType, ProvenanceId, ProvenanceJournal, ProvenanceTable};

use crate::{DiagnosticAnchor, EmitError};

/// A JavaScript emitter for one DIR module.
#[derive(Debug)]
pub(crate) struct ScriptEmitter<'a> {
    /// The source module identity.
    pub(super) module: ModuleId,

    /// The DIR roots.
    pub(super) roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The DIR tree.
    pub(super) tree: dir::View<'a>,
    /// The source binding table.
    pub(super) bindings: dir::BindingTable<'static>,
    /// The resolved source module edges.
    pub(super) modules: dir::ModuleTable<'static>,
    /// The resolved source references.
    pub(super) references: &'a dir::ReferenceTable,
    /// The checked source name resolutions.
    pub(super) resolutions: dir::ResolutionTable<'static>,
    /// The materialized source decisions.
    pub(super) decisions: dir::DecisionTable<'static>,
    /// The materialized source types.
    pub(super) types: dir::TypeTable<'static>,
    /// The JavaScript module under construction.
    pub(super) output: js::Module,
    /// The JavaScript emission journal.
    pub(super) provenance: ProvenanceJournal<'a>,
    /// JavaScript symbols keyed by source symbol id.
    pub(super) symbols: Vec<Option<js::SymbolId>>,
    /// Root symbols keyed by emitted global name.
    pub(super) globals: FxIndexMap<(dir::StringId, Option<dir::GlobalSymbolId>), js::SymbolId>,
    /// DIR symbols keyed by JavaScript symbol id.
    pub(super) source_symbols: Vec<Option<dir::GlobalSymbolId>>,
    /// Resolved dependency modules keyed by JavaScript node id.
    pub(super) dependency_modules: Vec<Option<ModuleId>>,
    /// JavaScript scopes keyed by source scope id.
    pub(super) scopes: Vec<Option<js::ScopeId>>,
    /// The next generated discard name.
    pub(super) discard_count: u32,
}

impl<'a> ScriptEmitter<'a> {
    /// Return the JavaScript form of one DIR asynchrony.
    pub(super) fn asynchrony(&self, asynchrony: dir::Asynchrony) -> js::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => js::Asynchrony::Sync,
            dir::Asynchrony::Async => js::Asynchrony::Async,
        }
    }

    /// Derive JavaScript provenance from one DIR node.
    pub(super) fn derive(&mut self, source: impl Into<dir::LocalNodeIdAny>) -> ProvenanceId {
        let source = source.into();
        let source = self.tree.provenance_any(source);

        self.provenance.derive(source)
    }

    /// Derive JavaScript provenance from the finest source occurrence of one DIR node.
    pub(super) fn derive_at(
        &mut self,
        source: impl Into<dir::LocalNodeIdAny>,
        span_type: NodeSpanType,
    ) -> ProvenanceId {
        let source = source.into();
        let source = self
            .tree
            .provenance_at(source, span_type)
            .unwrap_or_else(|| self.tree.provenance_any(source));

        self.provenance.derive(source)
    }

    /// Insert one JavaScript node derived from a DIR node.
    pub(super) fn insert_from_source<T>(
        &mut self,
        node: T,
        source: impl Into<dir::LocalNodeIdAny>,
    ) -> js::LocalNodeId<T>
    where
        T: js::Node,
        js::Tree: js::TreeStore<T>,
    {
        let provenance = self.derive(source);

        self.output.tree.insert(node, provenance)
    }

    /// Build one fixed identifier name.
    pub(super) fn identifier_name<T: dir::Node>(
        &mut self,
        text: dir::StringId,
        source: dir::LocalNodeId<T>,
        span_type: NodeSpanType,
    ) -> js::IdentifierName {
        js::IdentifierName {
            text,
            provenance: self.derive_at(source, span_type),
        }
    }

    /// Build one string literal.
    pub(super) fn string_literal<T: dir::Node>(
        &mut self,
        value: dir::StringId,
        source: dir::LocalNodeId<T>,
        span_type: NodeSpanType,
    ) -> js::StringLiteral {
        js::StringLiteral {
            value,
            provenance: self.derive_at(source, span_type),
        }
    }

    /// Build one ECMAScript property name.
    pub(super) fn property_name<T: dir::Node>(
        &mut self,
        name: dir::Name,
        source: dir::LocalNodeId<T>,
    ) -> js::PropertyName {
        match name {
            dir::Name::Identifier(text) => {
                js::PropertyName::Identifier(self.identifier_name(text, source, NodeSpanType::Main))
            }
            dir::Name::String(value) => {
                js::PropertyName::String(self.string_literal(value, source, NodeSpanType::Main))
            }
            dir::Name::Index(index) => {
                let value = self.output.strings.intern(&index.to_string());

                js::PropertyName::String(self.string_literal(value, source, NodeSpanType::Main))
            }
        }
    }

    /// Build one ECMAScript export name.
    pub(super) fn export_name<T: dir::Node>(
        &mut self,
        name: dir::Name,
        source: dir::LocalNodeId<T>,
        span_type: NodeSpanType,
    ) -> js::ModuleExportName {
        let provenance = self.derive_at(source, span_type);
        match name {
            dir::Name::Identifier(text) => {
                js::ModuleExportName::Identifier(js::IdentifierName { text, provenance })
            }
            dir::Name::String(value) => {
                js::ModuleExportName::String(js::StringLiteral { value, provenance })
            }
            dir::Name::Index(index) => {
                let value = self.output.strings.intern(&index.to_string());

                js::ModuleExportName::String(js::StringLiteral { value, provenance })
            }
        }
    }

    /// Create a JavaScript emitter.
    pub(crate) fn new(
        tree: dir::View<'a>,
        roots: &'a [dir::LocalNodeId<dir::Expression>],
        source_strings: &'a StringPool,
        bindings: dir::BindingTable<'static>,
        modules: dir::ModuleTable<'static>,
        references: &'a dir::ReferenceTable,
        resolutions: dir::ResolutionTable<'static>,
        decisions: dir::DecisionTable<'static>,
        types: dir::TypeTable<'static>,
        source_provenance: &ProvenanceTable,
        provenance: ProvenanceJournal<'a>,
    ) -> Self {
        let module = tree.tree().module_id;
        let strings = StringPool::new();

        // copy source strings into the output pool
        strings.ensure_all_from(source_strings);

        // reserve source keyed symbol and scope slots
        let symbols = vec![None; bindings.symbol_count() as usize];
        let mut scopes = vec![None; bindings.scope_count() as usize];
        let root = bindings.module_scope().id;

        scopes[root.0 as usize] = Some(js::ScopeId::ROOT);

        // initialize the output module
        let mut output = js::Module::new(module, source_provenance.clone());
        output.strings = strings;

        Self {
            module,
            tree,
            roots,
            bindings,
            modules,
            references,
            resolutions,
            decisions,
            types,
            output,
            provenance,
            symbols,
            globals: FxIndexMap::default(),
            source_symbols: Vec::new(),
            dependency_modules: Vec::new(),
            scopes,
            discard_count: 0,
        }
    }

    /// Emit the source module as JavaScript.
    pub(crate) fn emit(mut self) -> Result<Script, EmitError> {
        // emit the module roots
        for expression in self.roots.iter().copied() {
            if let Some(root) = self.emit_statement(expression)? {
                self.output.roots.push(root);

                // separate the public export name from its local binding
                if let Some(export) = self.split_export(root, expression)? {
                    self.output.roots.push(export);
                }
            }
        }

        let external_symbols = self
            .globals
            .into_iter()
            .filter_map(|((_, source), symbol)| source.is_none().then_some(symbol));
        let mut script = Script::new(self.output, self.source_symbols, self.dependency_modules);
        for symbol in external_symbols {
            script.set_external_symbol(symbol);
        }

        Ok(script)
    }

    /// Build one source span anchor from a DIR node.
    fn anchor(&self, node: dir::GlobalNodeIdAny) -> Result<DiagnosticAnchor, EmitError> {
        if self.module != node.module_id {
            return Err(self.internal_error(
                "JavaScript diagnostic node belongs to a different module".to_string(),
            ));
        }

        // require a source span
        let Some(span) = self.tree.get_span_by_id(node.local_id.id) else {
            return Err(self.internal_error(
                "JavaScript diagnostic node is missing a source span".to_string(),
            ));
        };

        Ok(DiagnosticAnchor::Span(span))
    }

    /// Build one internal error for a valid construct that emission did not handle.
    pub(super) fn unhandled(
        &self,
        node: dir::GlobalNodeIdAny,
        message: Option<String>,
    ) -> EmitError {
        let anchor = match self.anchor(node) {
            Ok(anchor) => anchor,
            Err(error) => return error,
        };

        EmitError::Internal {
            anchor,
            module: self.module,
            message: message.unwrap_or_else(|| format!("unhandled {}", node.local_id.ty.name())),
        }
    }

    /// Build one internal JavaScript emission error.
    pub(super) fn internal_error(&self, message: String) -> EmitError {
        EmitError::Internal {
            anchor: self.module.into(),
            module: self.module,
            message,
        }
    }

    /// Return whether one DIR expression has no JavaScript runtime form.
    pub(super) fn expression_is_erased(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match self.tree.get(expression_id) {
            // inspect declaration erasure rules
            dir::Expression::Declaration(declaration_id) => match self.tree.get(*declaration_id) {
                dir::Declaration::Global(declaration) => declaration.is_ambient,
                dir::Declaration::Type(_) => true,
                dir::Declaration::Struct(declaration) => declaration.is_ambient,
                dir::Declaration::Class(declaration) => declaration.is_ambient,
                dir::Declaration::Interface(_) => true,
                dir::Declaration::Enum(declaration) => declaration.is_ambient,
                dir::Declaration::Function(declaration) => {
                    declaration.body.is_none()
                        || declaration.signature.phase == dir::FunctionPhase::Const
                }
                _ => false,
            },
            // erase ambient bindings
            dir::Expression::Let { is_ambient, .. } | dir::Expression::Using { is_ambient, .. } => {
                *is_ambient
            }
            // retain executable expressions
            _ => false,
        }
    }

    /// Return whether one DIR expression produces no runtime value.
    pub(super) fn expression_is_valueless(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, EmitError> {
        let node = expression.into_global_any(self.module);
        let Some(ty) = self.types.get_node_type_id(node) else {
            return Err(self.internal_error(format!(
                "JavaScript expression {node:?} has no materialized type"
            )));
        };

        // require a local materialized type
        if ty.module_id != self.module {
            return Err(self.internal_error(format!(
                "JavaScript expression {node:?} has an external materialized type"
            )));
        }

        // load the materialized type
        let Some(ty) = self.types.get_type_maybe(ty.local_id) else {
            return Err(self.internal_error(format!(
                "JavaScript expression {node:?} has no visible materialized type"
            )));
        };

        Ok(matches!(ty, dir::Type::Never | dir::Type::Void))
    }
}
