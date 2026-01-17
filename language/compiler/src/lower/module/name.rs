use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::Package;

use crate::lower::ModuleLowerer;

/// Suffix for object metadata names.
const OBJECT_METADATA_SUFFIX: &str = "#object";
/// Suffix for tuple metadata names.
const TUPLE_METADATA_SUFFIX: &str = "#tuple";
/// Suffix for union metadata names.
const UNION_METADATA_SUFFIX: &str = "#union";
/// Suffix for function metadata names.
const FUNCTION_METADATA_SUFFIX: &str = "#function";
/// Suffix for array metadata names.
const ARRAY_METADATA_SUFFIX: &str = "#array";
/// Suffix for pointer metadata names.
const POINTER_METADATA_SUFFIX: &str = "#pointer";
/// Suffix for parameter context in union metadata names.
const UNION_PARAMETER_SUFFIX: &str = "#parameter:";
/// Suffix for field context in union metadata names.
const UNION_FIELD_SUFFIX: &str = "#field:";
/// Suffix for method context in union metadata names.
const UNION_METHOD_SUFFIX: &str = "#method:";
/// Suffix for return types in union metadata names.
const UNION_RETURN_SUFFIX: &str = "#return";
/// Suffix for local bindings in union metadata names.
const UNION_LOCAL_SUFFIX: &str = "#let:";

/// Context derived from a type source node for naming.
#[derive(Default)]
struct TypeNameContext {
    /// The symbol owning the type context.
    declaration_symbol: Option<dir::GlobalSymbolId>,
    /// The parameter name when the type is attached to a parameter.
    parameter_name: Option<String>,
    /// The field name when the type is attached to a field.
    field_name: Option<String>,
    /// The method name when the type is attached to a method signature.
    method_name: Option<String>,
    /// The declarator binding name when the type is attached to a local.
    declarator_name: Option<String>,
    /// Whether this type is a return type.
    is_return_type: bool,
    /// Whether this type is the value of a type alias declaration.
    is_type_alias: bool,
}

impl ModuleLowerer<'_> {
    /// Assign deterministic metadata names to anonymous types.
    pub(crate) fn assign_anonymous_metadata_names(&mut self) -> crate::LowerResult<()> {
        // collect lowered type ids to avoid borrowing conflicts
        let type_ids: Vec<_> = self.type_lowerer.type_cache.keys().copied().collect();

        // collect nominal instance types to skip anonymous naming
        let nominal_instance_types = self.nominal_instance_type_ids();

        // assign metadata names for anonymous type ids
        for type_id in type_ids {
            // skip nominal instance types
            if nominal_instance_types.contains(&type_id) {
                continue;
            }

            // resolve the suffix for anonymous types
            let dir_type = self.types.get_type(type_id);
            let Some(suffix) = self.anonymous_metadata_suffix(dir_type) else {
                continue;
            };

            // resolve the metadata name
            let Some(name) = self.anonymous_metadata_name(type_id, suffix) else {
                continue;
            };

            // resolve the cached mir type
            let Some(mir_type) = self.type_lowerer.type_cache.get(&type_id).copied() else {
                continue;
            };

            // intern the name
            let name_id = self.builder.intern(&name);

            // update metadata when no name is assigned
            let metadata = self
                .builder
                .tree_mut()
                .type_table
                .type_metadata_by_id
                .entry(mir_type)
                .or_default();
            if metadata.name.is_none() {
                metadata.name = Some(name_id);
            }
        }

        // report success
        Ok(())
    }

    /// Resolve the metadata suffix for an anonymous type.
    fn anonymous_metadata_suffix(&self, dir_type: &dir::Type) -> Option<&'static str> {
        match dir_type {
            dir::Type::Object { .. } => Some(OBJECT_METADATA_SUFFIX),
            dir::Type::Tuple { .. } => Some(TUPLE_METADATA_SUFFIX),
            dir::Type::Union { .. } => Some(UNION_METADATA_SUFFIX),
            dir::Type::Function { .. } => Some(FUNCTION_METADATA_SUFFIX),
            dir::Type::ArraySized { .. } | dir::Type::Array { .. } => Some(ARRAY_METADATA_SUFFIX),
            dir::Type::PointerOf { .. } => Some(POINTER_METADATA_SUFFIX),
            _ => None,
        }
    }

    /// Build the metadata name for an anonymous type from its source context.
    fn anonymous_metadata_name(&self, type_id: dir::LocalTypeId, suffix: &str) -> Option<String> {
        // resolve the naming context
        let context = self.type_name_context(type_id)?;
        let base = self.qualified_symbol_name(context.declaration_symbol?)?;

        // start with the base path
        let mut name = base;

        // return the alias name without extra context
        if context.is_type_alias {
            return Some(name);
        }

        // append parameter name when present
        if let Some(parameter_name) = context.parameter_name {
            name.push_str(&format!("{UNION_PARAMETER_SUFFIX}{parameter_name}"));
        } else if let Some(field_name) = context.field_name {
            // append field name when present
            name.push_str(&format!("{UNION_FIELD_SUFFIX}{field_name}"));
        } else if let Some(method_name) = context.method_name {
            // append method name and return marker when present
            name.push_str(&format!("{UNION_METHOD_SUFFIX}{method_name}"));
            if context.is_return_type {
                name.push_str(UNION_RETURN_SUFFIX);
            }
        } else if context.is_return_type {
            // append return marker for declaration return types
            name.push_str(UNION_RETURN_SUFFIX);
        } else if let Some(declarator_name) = context.declarator_name {
            // append local name when present
            name.push_str(&format!("{UNION_LOCAL_SUFFIX}{declarator_name}"));
        }

        // append the type suffix
        name.push_str(suffix);

        Some(name)
    }

    /// Collect instance type ids for nominal declarations.
    fn nominal_instance_type_ids(&self) -> HashSet<dir::LocalTypeId> {
        // seed the set with all nominal instance types
        let mut type_ids = HashSet::new();

        for (_, declaration) in self
            .dir_tree
            .iter_nodes_of_type::<destack_dir::Declaration>()
        {
            // match nominal declaration kinds
            let symbol = match declaration {
                destack_dir::Declaration::Struct { descriptor, .. }
                | destack_dir::Declaration::Class { descriptor, .. }
                | destack_dir::Declaration::Enum { descriptor, .. }
                | destack_dir::Declaration::Interface { descriptor, .. } => {
                    descriptor.symbol.into_global(self.module_id)
                }
                destack_dir::Declaration::Type {
                    descriptor, kind, ..
                } => {
                    if !matches!(kind, destack_dir::TypeKind::Nominal) {
                        continue;
                    }
                    descriptor.symbol.into_global(self.module_id)
                }
                _ => continue,
            };

            // collect the instance type id when available
            if let Some(instance_type_id) = self.types.get_instance_type_id(symbol) {
                type_ids.insert(instance_type_id);
            }
        }

        type_ids
    }

    /// Collect naming context from a type source node.
    fn type_name_context(&self, type_id: dir::LocalTypeId) -> Option<TypeNameContext> {
        // resolve the source node and expression id
        let source = self.types.get_type_source(type_id);
        let expression_id = if source.ty == dir::NodeType::Expression {
            Some(dir::LocalNodeId::new(source.id))
        } else {
            None
        };

        // seed context and traversal cursor
        let mut context = TypeNameContext::default();
        let mut current = Some(source);

        // walk parent nodes for naming context
        while let Some(node_id) = current {
            match node_id.ty {
                dir::NodeType::Parameter => {
                    // fill parameter context when missing
                    if context.parameter_name.is_none() || context.declaration_symbol.is_none() {
                        // load the parameter node
                        let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(node_id.id);
                        let parameter = self.dir_tree.get(parameter_id);

                        // fill parameter name when missing
                        if context.parameter_name.is_none() {
                            context.parameter_name = self.parameter_name_from_node(parameter);
                        }

                        // fill declaration symbol from the parameter owner
                        if context.declaration_symbol.is_none() {
                            context.declaration_symbol =
                                self.owner_symbol_from_symbol(parameter.symbol());
                        }
                    }
                }
                dir::NodeType::Member => {
                    // load the member node
                    let member_id = dir::LocalNodeId::<dir::Member>::new(node_id.id);
                    let member = self.dir_tree.get(member_id);

                    // fill declaration symbol from the member owner
                    if context.declaration_symbol.is_none() {
                        context.declaration_symbol = self.owner_symbol_from_symbol(member.symbol());
                    }

                    // apply member naming context
                    self.apply_member_context(member, expression_id, &mut context);
                }
                dir::NodeType::Property => {
                    // load the property node
                    let property_id = dir::LocalNodeId::<dir::Property>::new(node_id.id);
                    let property = self.dir_tree.get(property_id);

                    // fill declaration symbol from the property owner
                    if context.declaration_symbol.is_none() {
                        context.declaration_symbol =
                            self.owner_symbol_from_symbol(self.property_symbol(property));
                    }

                    // apply property naming context
                    self.apply_property_context(property, expression_id, &mut context);
                }
                dir::NodeType::Declarator => {
                    // load the declarator node
                    let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(node_id.id);
                    let declarator = self.dir_tree.get(declarator_id);

                    // fill declarator name when missing
                    if context.declarator_name.is_none() {
                        context.declarator_name = self.declarator_name_from_node(declarator);
                    }

                    // fill declaration symbol from the declarator binding
                    if context.declaration_symbol.is_none()
                        && let Some(symbol_id) = self.pattern_symbol(declarator.pattern)
                    {
                        context.declaration_symbol = self.owner_symbol_from_symbol(symbol_id);
                    }
                }
                dir::NodeType::Declaration => {
                    // load the declaration node
                    let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(node_id.id);
                    let declaration = self.dir_tree.get(declaration_id);

                    // fill declaration symbol from the declaration
                    if context.declaration_symbol.is_none() {
                        context.declaration_symbol =
                            Some(declaration.symbol().into_global(self.module_id));
                    }

                    // apply declaration naming context
                    self.apply_declaration_context(declaration, expression_id, &mut context);
                }
                _ => {}
            }

            // move to the parent node
            current = self.dir_tree.get_parent(node_id.id);
        }

        // require a declaration symbol
        context.declaration_symbol?;

        // return the collected context
        Some(context)
    }

    /// Apply declaration-specific naming context.
    fn apply_declaration_context(
        &self,
        declaration: &dir::Declaration,
        expression_id: Option<dir::LocalNodeId<dir::Expression>>,
        context: &mut TypeNameContext,
    ) {
        // exit when no expression id is available
        let Some(expression_id) = expression_id else {
            return;
        };

        // check declaration kinds for naming context
        match declaration {
            dir::Declaration::Function { signature, .. } => {
                // mark function return types
                if signature.return_type == Some(expression_id) {
                    context.is_return_type = true;
                }
            }
            dir::Declaration::Type { value, .. } => {
                // mark type alias values
                if *value == expression_id {
                    context.is_type_alias = true;
                }
            }
            _ => {}
        }
    }

    /// Apply member-specific naming context.
    fn apply_member_context(
        &self,
        member: &dir::Member,
        expression_id: Option<dir::LocalNodeId<dir::Expression>>,
        context: &mut TypeNameContext,
    ) {
        // check member kinds for naming context
        match member {
            dir::Member::Field { key, .. } => {
                // capture field name when missing
                if context.field_name.is_none() {
                    context.field_name = self.member_name_from_key(*key);
                }
            }
            dir::Member::Method { key, signature, .. } => {
                // capture method name when missing
                if context.method_name.is_none() {
                    context.method_name = self.member_name_from_key(*key);
                }

                // mark method return types
                if let Some(expression_id) = expression_id
                    && signature.return_type == Some(expression_id)
                {
                    context.is_return_type = true;
                }
            }
            _ => {}
        }
    }

    /// Apply property-specific naming context.
    fn apply_property_context(
        &self,
        property: &dir::Property,
        expression_id: Option<dir::LocalNodeId<dir::Expression>>,
        context: &mut TypeNameContext,
    ) {
        // check property kinds for naming context
        match property {
            dir::Property::Field { key, .. } => {
                // capture field name when missing
                if context.field_name.is_none() {
                    context.field_name = self.member_name_from_key(*key);
                }
            }
            dir::Property::Method { key, signature, .. } => {
                // capture method name when missing
                if context.method_name.is_none() {
                    context.method_name = self.member_name_from_key(*key);
                }

                // mark method return types
                if let Some(expression_id) = expression_id
                    && signature.return_type == Some(expression_id)
                {
                    context.is_return_type = true;
                }
            }
            _ => {}
        }
    }

    /// Resolve a parameter name from a parameter node.
    fn parameter_name_from_node(&self, parameter: &dir::Parameter) -> Option<String> {
        // load string pool for name lookup
        let strings = &self.compiler.program.strings;

        // match parameter kinds to resolve names
        match parameter {
            dir::Parameter::Named { name, .. } | dir::Parameter::Variadic { name, .. } => {
                Some(strings.get(*name).to_string())
            }
            dir::Parameter::Pattern {
                symbol, pattern, ..
            } => {
                // prefer the bound symbol name
                let symbol = self.symbols.get_symbol(*symbol);
                if let Some(name_id) = symbol.name() {
                    return Some(strings.get(name_id).to_string());
                }

                // fall back to the pattern binding name
                self.pattern_binding_name(*pattern)
            }
        }
    }

    /// Resolve a declarator binding name from a declarator node.
    fn declarator_name_from_node(&self, declarator: &dir::Declarator) -> Option<String> {
        // use the declarator pattern for binding name
        self.pattern_binding_name(declarator.pattern)
    }

    /// Resolve a binding name from a simple pattern.
    fn pattern_binding_name(&self, pattern_id: dir::LocalNodeId<dir::Pattern>) -> Option<String> {
        // load string pool and pattern node
        let strings = &self.compiler.program.strings;
        let pattern = self.dir_tree.get(pattern_id);

        // extract binding names when available
        match pattern {
            dir::Pattern::Binding { name, .. } => Some(strings.get(*name).to_string()),
            _ => None,
        }
    }

    /// Resolve a binding symbol from a simple pattern.
    fn pattern_symbol(
        &self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> Option<dir::LocalSymbolId> {
        // load the pattern node
        let pattern = self.dir_tree.get(pattern_id);

        // return the binding symbol when present
        pattern.symbol()
    }

    /// Resolve a property symbol from a property node.
    fn property_symbol(&self, property: &dir::Property) -> dir::LocalSymbolId {
        // extract the symbol from each property kind
        match property {
            dir::Property::Field { symbol, .. } => *symbol,
            dir::Property::Method { symbol, .. } => *symbol,
            dir::Property::Spread { symbol, .. } => *symbol,
        }
    }

    /// Resolve the owning declaration symbol for a local symbol.
    fn owner_symbol_from_symbol(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        // seed with the symbol scope
        let symbol = self.symbols.get_symbol(symbol_id);
        let mut scope_id = symbol.scope.0;

        // walk up scopes to find an owner
        let mut seen_scopes = HashSet::new();
        loop {
            // avoid cycles in scope ownership
            if !seen_scopes.insert(scope_id) {
                break;
            }

            // return the first scope owner
            let scope = self.symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner_id {
                return Some(owner_id.into_global(self.module_id));
            }

            // move to the parent scope
            let Some((parent_id, _)) = scope.parent else {
                break;
            };
            scope_id = parent_id;
        }

        // fall back to the symbol itself when no owner is registered
        Some(symbol_id.into_global(self.module_id))
    }

    /// Resolve a name string from a member key.
    fn member_name_from_key(&self, key: Option<dir::DynamicKey>) -> Option<String> {
        // require a key for name resolution
        let key = key?;

        // resolve the key into a static key
        let key = self.compiler.static_key_from_dynamic_key(
            self.profile,
            key,
            self.dir_tree,
            self.symbols,
            self.types,
        )?;

        // return the static key name
        self.static_key_name(key)
    }

    /// Resolve a name string from a static key.
    fn static_key_name(&self, key: dir::StaticKey) -> Option<String> {
        // load string pool for name lookup
        let strings = &self.compiler.program.strings;

        // map supported key types to names
        match key {
            dir::StaticKey::Name(name_id) | dir::StaticKey::Number(name_id) => {
                Some(strings.get(name_id).to_string())
            }
            dir::StaticKey::Symbol(_) => None,
        }
    }

    /// Resolve the qualified name for a symbol in this module.
    fn qualified_symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        // load the owning package
        let package = self.compiler.program.packages.get(self.module.package_id);
        let package = package.read();

        // build the module prefix
        let package_name = package.name.as_ref()?;
        if package_name.is_empty() {
            return None;
        }
        let module_path = self.module_path_without_extension(&package)?;
        let module_prefix = if module_path.is_empty() {
            package_name.to_string()
        } else {
            format!("{package_name}/{module_path}")
        };

        // build the symbol path
        let symbol_path = self.symbol_path_from_symbols(symbol_id)?;

        Some(format!("{module_prefix}:{symbol_path}"))
    }

    /// Resolve the package relative module path without extension.
    fn module_path_without_extension(&self, package: &Package) -> Option<String> {
        // prefer package relative paths when available
        let module_path = if let Some(path) = &self.module.path {
            let relative = package
                .path
                .as_ref()
                .and_then(|package_path| path.strip_prefix(package_path).ok())
                .unwrap_or(path);
            relative.to_string_lossy().to_string()
        } else {
            self.module.uri.to_string()
        };

        // normalize separators and drop extension
        let module_path = self.normalize_path_separators(&module_path);
        Some(self.strip_extension_from_path(&module_path))
    }

    /// Build a symbol path from the module symbol table.
    fn symbol_path_from_symbols(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        // seed with the symbol name
        let symbol = self.symbols.get_symbol(symbol_id.into_local());
        let symbol_name = self.static_key_name(symbol.key?)?;
        let mut segments = vec![symbol_name];

        // walk owner scopes for namespaces and types
        let mut scope_id = symbol.scope.0;
        let mut seen_scopes = HashSet::new();
        loop {
            // avoid cycles in scope ownership
            if !seen_scopes.insert(scope_id) {
                break;
            }

            // collect named owners into the path
            let scope = self.symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner_id
                && owner_id != symbol_id.into_local()
            {
                let owner = self.symbols.get_symbol(owner_id);
                if let Some(owner_name) = owner.key
                    && let Some(owner_name) = self.static_key_name(owner_name)
                {
                    segments.push(owner_name);
                }
            }

            // climb to the parent scope
            let Some((parent_id, _)) = scope.parent else {
                break;
            };
            scope_id = parent_id;
        }

        // reverse for root to leaf order
        segments.reverse();
        Some(segments.join("."))
    }

    /// Normalize a module path to use forward slashes.
    fn normalize_path_separators(&self, path: &str) -> String {
        // replace separators and trim leading slashes
        let normalized = path.replace('\\', "/");
        normalized.trim_start_matches('/').to_string()
    }

    /// Strip the file extension from a module path.
    fn strip_extension_from_path(&self, path: &str) -> String {
        // split prefix and leaf for extension removal
        let (prefix, leaf) = path.rsplit_once('/').unwrap_or(("", path));
        let stripped = leaf.rsplit_once('.').map(|(base, _)| base).unwrap_or(leaf);

        // rebuild the path without the extension
        if prefix.is_empty() {
            stripped.to_string()
        } else {
            format!("{prefix}/{stripped}")
        }
    }
}
