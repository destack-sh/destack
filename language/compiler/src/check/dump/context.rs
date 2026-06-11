use destack_dir as dir;
use destack_source::{ModuleId, Span};

use crate::check::{CheckState, VariableId};

/// Context used to render check state values.
pub(in crate::check) struct DumpContext<'a, 'b> {
    /// The check state that owns term arenas and strings.
    pub(in crate::check) check: &'a CheckState<'b>,
    /// The source module used for local labels.
    pub(in crate::check) module: Option<ModuleId>,
}

/// Value that can be rendered through a check dump context.
pub(in crate::check) trait Dump {
    /// Render this value as one inline dump string.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String;
}

impl<'a, 'b> DumpContext<'a, 'b> {
    /// Create a dump context for one check state.
    pub(in crate::check) fn new(check: &'a CheckState<'b>) -> Self {
        Self {
            check,
            module: None,
        }
    }

    /// Return this context with a source module.
    pub(in crate::check) fn with_module(mut self, module: ModuleId) -> Self {
        self.module = Some(module);

        self
    }

    /// Render one dumpable value.
    pub(in crate::check) fn render<T: Dump + ?Sized>(&self, value: &T) -> String {
        value.dump(self)
    }

    /// Return a compact module label.
    pub(in crate::check) fn module_label(&self, module: ModuleId) -> String {
        if let Some(module) = self.check.modules.get(&module) {
            return trim_builtin_uri(module.module.uri.as_ref());
        }

        self.check
            .compiler
            .repository
            .module(self.check.context.revision(), module)
            .ok()
            .flatten()
            .map(|module| trim_builtin_uri(module.uri.as_ref()))
            .unwrap_or_else(|| format!("module#{module}"))
    }

    /// Return a compact global node label.
    pub(in crate::check) fn node_label(&self, node: dir::GlobalNodeIdAny) -> String {
        let module = self.module_label(node.module_id);
        let ty = node.local_id.ty.name().replace(' ', "_");
        let index = node.local_id.id;

        format!("{module}:{ty}#{index}")
    }

    /// Return the source location for one global node label.
    pub(in crate::check) fn node_source_label(&self, node: dir::GlobalNodeIdAny) -> String {
        let span = self.node_span(node);

        span.and_then(|span| self.span_label(span))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Return a compact symbol label.
    pub(in crate::check) fn symbol_label(&self, symbol: dir::GlobalSymbolId) -> String {
        let module = self.module_label(symbol.module_id);
        let key = self.symbol_key(symbol);

        if self.symbol_is_generic_parameter(symbol)
            && let Some(owner) = self.symbol_scope_owner(symbol)
        {
            let owner = owner.into_global(symbol.module_id);
            let owner = self.symbol_key(owner);

            return format!("{module}:{owner}:{key}");
        }

        format!("{module}:{key}")
    }

    /// Return the source location for one global symbol label.
    pub(in crate::check) fn symbol_source_label(&self, symbol: dir::GlobalSymbolId) -> String {
        let declaration = self.symbol_declaration(symbol);

        declaration
            .map(|declaration| self.node_source_label(declaration))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Return a compact variable label.
    pub(in crate::check) fn variable_label(&self, variable: VariableId) -> String {
        format!("v{}", variable.index)
    }

    /// Return a compact global type label.
    pub(in crate::check) fn type_label(&self, ty: dir::GlobalTypeId) -> String {
        let module = self.module_label(ty.module_id);
        let index = ty.local_id.0;

        format!("{module}:type#{index}")
    }

    /// Return a compact global static label.
    pub(in crate::check) fn static_label(&self, value: dir::GlobalStaticId) -> String {
        let module = self.module_label(value.module_id);
        let index = value.local_id.0;

        format!("{module}:static#{index}")
    }

    /// Return a compact generic template label.
    pub(in crate::check) fn generic_template_label(
        &self,
        template: dir::GlobalGenericTemplateId,
    ) -> String {
        if let Some(template) = self
            .check
            .inference
            .generic_template(template)
            .map(|template| template.source)
            .or_else(|| self.external_generic_template_source(template))
        {
            return self.node_label(template);
        }

        let module = self.module_label(template.module_id);
        let index = template.local_id.0;

        format!("{module}:template#{index}")
    }

    /// Return a compact static key label.
    pub(in crate::check) fn static_key_label(&self, key: &dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.string_label(*name),
            dir::StaticKey::Index(index) => format!("#{index}"),
            dir::StaticKey::Symbol(symbol) => {
                symbol.debug_string(self.check.compiler.repository.string_pool())
            }
        }
    }

    /// Return a compact generic parameter label.
    pub(in crate::check) fn generic_parameter_label(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> String {
        if let Some(generic) = self.check.inference.generic_parameter(parameter) {
            let parameter = generic.parameter();
            let template = self.generic_template_label(parameter.template);
            let key = self.generic_parameter_key(parameter.key);

            return format!("{template}:{key}");
        }

        if let Some(external) = self.check.external_modules.get(&parameter.module_id) {
            let generic = external.generics.get_parameter(parameter.local_id);
            let template = generic.template().into_global(parameter.module_id);
            let template = self.generic_template_label(template);
            let key = self.generic_parameter_key(generic.key());

            return format!("{template}:{key}");
        }

        let module = self.module_label(parameter.module_id);
        let index = parameter.local_id.0;

        format!("{module}:generic#{index}")
    }

    /// Return a readable interned string.
    pub(in crate::check) fn string(&self, string: dir::StringId) -> String {
        format!(
            "'{}'",
            self.check.compiler.repository.string_pool().get(string)
        )
    }

    /// Return a compact interned string label.
    pub(in crate::check) fn string_label(&self, string: dir::StringId) -> String {
        self.check
            .compiler
            .repository
            .string_pool()
            .get(string)
            .to_string()
    }

    /// Return a readable optional interned string.
    pub(in crate::check) fn optional_string(&self, string: Option<dir::StringId>) -> String {
        match string {
            Some(string) => self.string(string),
            None => "none".to_string(),
        }
    }

    /// Return a compact generic parameter key label.
    pub(super) fn generic_parameter_key(&self, key: dir::GenericParameterKey) -> String {
        match key {
            dir::GenericParameterKey::Symbol(symbol) => self.symbol_key(symbol),
            dir::GenericParameterKey::Generated(name) => self.string_label(name),
        }
    }

    /// Return a compact symbol key.
    fn symbol_key(&self, symbol: dir::GlobalSymbolId) -> String {
        let Some(symbol) = self.binding_symbol(symbol) else {
            return format!("symbol#{}", symbol.local_id.id);
        };

        symbol
            .key
            .map(|key| self.static_key_label(&key))
            .unwrap_or_else(|| symbol_kind_label(symbol.kind).to_string())
    }

    /// Return whether one symbol is a generic parameter.
    fn symbol_is_generic_parameter(&self, symbol: dir::GlobalSymbolId) -> bool {
        let Some(symbol) = self.binding_symbol(symbol) else {
            return false;
        };

        matches!(
            symbol.kind,
            dir::SymbolKind::GenericTypeParameter | dir::SymbolKind::GenericValueParameter
        )
    }

    /// Return the symbol that owns the declaring scope.
    fn symbol_scope_owner(&self, symbol: dir::GlobalSymbolId) -> Option<dir::LocalSymbolId> {
        self.binding_symbol(symbol)
            .and_then(|binding| self.binding_scope_owner(symbol.module_id, binding))
    }

    /// Return the declaration node for one symbol.
    fn symbol_declaration(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalNodeIdAny> {
        self.binding_symbol(symbol)
            .and_then(|symbol| symbol.declaration)
    }

    /// Return one generic template source from an external module.
    fn external_generic_template_source(
        &self,
        template: dir::GlobalGenericTemplateId,
    ) -> Option<dir::GlobalNodeIdAny> {
        let external = self.check.external_modules.get(&template.module_id)?;
        let template = external.generics.get_template(template.local_id);

        Some(template.source)
    }

    /// Return one visible binding symbol.
    fn binding_symbol(&self, symbol: dir::GlobalSymbolId) -> Option<DumpSymbol> {
        if let Some(module) = self.check.modules.get(&symbol.module_id) {
            let binding_table = module.binding_table();
            let symbol = binding_table.get_symbol(symbol.local_id);

            return Some(DumpSymbol::from(symbol));
        }

        let external = self.check.external_modules.get(&symbol.module_id)?;
        let symbol = external.bindings.get_symbol(symbol.local_id);

        Some(DumpSymbol::from(symbol))
    }

    /// Return the owner symbol for one visible binding scope.
    fn binding_scope_owner(
        &self,
        module: ModuleId,
        symbol: DumpSymbol,
    ) -> Option<dir::LocalSymbolId> {
        if let Some(module) = self.check.modules.get(&module) {
            let binding_table = module.binding_table();
            let scope = binding_table.get_scope(symbol.scope);

            return scope.owner;
        }

        let external = self.check.external_modules.get(&module)?;
        let scope = external.bindings.get_scope(symbol.scope);

        scope.owner
    }

    /// Return the source span for one global node.
    fn node_span(&self, node: dir::GlobalNodeIdAny) -> Option<Span> {
        if let Some(module) = self.check.modules.get(&node.module_id) {
            let view = module.view();

            return view.tree().get_span_by_id(node.local_id.id);
        }

        let external = self.check.external_modules.get(&node.module_id)?;
        let view = external.view();

        view.tree().get_span_by_id(node.local_id.id)
    }

    /// Return the source label for one span.
    fn span_label(&self, span: Span) -> Option<String> {
        let file = self
            .check
            .compiler
            .repository
            .file(self.check.context.revision(), span.file)
            .ok()??;
        let (line, column) = file.get_position(span.start)?;
        let line = line + 1;
        let column = column + 1;

        Some(format!(
            "{}:{line}:{column}",
            trim_builtin_uri(file.uri.as_ref())
        ))
    }
}

/// Copied binding fields needed by dump labels.
struct DumpSymbol {
    /// The declaration kind of the symbol.
    kind: dir::SymbolKind,
    /// The key of the symbol.
    key: Option<dir::StaticKey>,
    /// The scope that introduces the symbol.
    scope: dir::LocalScope,
    /// The declaration node that introduced this symbol.
    declaration: Option<dir::GlobalNodeIdAny>,
}

impl From<&dir::Symbol> for DumpSymbol {
    /// Copy the binding fields used by dump labels.
    fn from(symbol: &dir::Symbol) -> Self {
        Self {
            kind: symbol.kind,
            key: symbol.key,
            scope: symbol.scope,
            declaration: symbol.declaration,
        }
    }
}

/// Trim the builtin URI scheme for human check dumps.
fn trim_builtin_uri(uri: &str) -> String {
    uri.strip_prefix("destack://")
        .map(|path| format!("/{path}"))
        .unwrap_or_else(|| uri.to_string())
}

/// Return a compact symbol kind label.
fn symbol_kind_label(kind: dir::SymbolKind) -> &'static str {
    match kind {
        dir::SymbolKind::Variable => "variable",
        dir::SymbolKind::Import => "import",
        dir::SymbolKind::Class => "class",
        dir::SymbolKind::Struct => "struct",
        dir::SymbolKind::Interface => "interface",
        dir::SymbolKind::NewtypeInterface => "newtype_interface",
        dir::SymbolKind::Enum => "enum",
        dir::SymbolKind::EnumField => "enum_field",
        dir::SymbolKind::Function => "function",
        dir::SymbolKind::Label => "label",
        dir::SymbolKind::Extension => "extension",
        dir::SymbolKind::TypeAlias => "type_alias",
        dir::SymbolKind::GenericTypeParameter => "generic_type_parameter",
        dir::SymbolKind::GenericValueParameter => "generic_value_parameter",
        dir::SymbolKind::AssociatedType => "associated_type",
        dir::SymbolKind::AssociatedConst => "associated_const",
        dir::SymbolKind::Newtype => "newtype",
    }
}

impl<T: Dump + ?Sized> Dump for &T {
    /// Render this referenced value.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        (*self).dump(context)
    }
}

impl CheckState<'_> {
    /// Render one check value through this state and module.
    pub(in crate::check) fn dump_in_module<T: Dump + ?Sized>(
        &self,
        module: ModuleId,
        value: &T,
    ) -> String {
        DumpContext::new(self).with_module(module).render(value)
    }
}
