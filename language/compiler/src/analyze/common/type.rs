use std::collections::HashSet;

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NodeTree, NodeType, StaticArgument,
    StaticExpression, StaticParameterKind, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
    TypeVisitor, TypeVisitorOptions, walk_static_expression, walk_type,
};
use destack_workspace::{Module, ProfileId};

use super::CanonicalSymbolMode;
use crate::{AnalyzeResult, Compiler};

/// Walk types to detect error types.
struct ErrorTypeVisitor<'a> {
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether an error type was found.
    found: bool,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> ErrorTypeVisitor<'a> {
    /// Create a visitor for error type detection.
    fn new(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self {
            visited,
            found: false,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for ErrorTypeVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if self.found {
            return;
        }
        if !self.visited.insert(id) {
            return;
        }
        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
        self.visited.remove(&id);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        if self.found {
            return;
        }
        if matches!(ty, Type::Error) {
            self.found = true;
            return;
        }
        walk_type(self, types, id, ty);
    }

    fn visit_static_expression(&mut self, types: &TypeTable, expression: &StaticExpression) {
        if self.found {
            return;
        }
        if matches!(expression, StaticExpression::RangeExpression { .. }) {
            return;
        }
        walk_static_expression(self, types, expression);
    }
}

/// Walk types to detect unevaluated static arguments.
struct UnevaluatedStaticArgumentVisitor<'a> {
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether an unevaluated static argument was found.
    found: bool,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> UnevaluatedStaticArgumentVisitor<'a> {
    /// Create a visitor for unevaluated static argument detection.
    fn new(visited: &'a mut HashSet<LocalTypeId>) -> Self {
        Self {
            visited,
            found: false,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for UnevaluatedStaticArgumentVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if self.found {
            return;
        }
        if !self.visited.insert(id) {
            return;
        }
        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
        self.visited.remove(&id);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        if self.found {
            return;
        }
        walk_type(self, types, id, ty);
    }

    fn visit_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        if self.found {
            return;
        }
        match argument {
            StaticArgument::Unevaluated { .. } => {
                self.found = true;
            }
            StaticArgument::Evaluated { value, .. } => {
                walk_static_expression(self, types, value);
            }
        }
    }

    fn visit_static_expression(&mut self, types: &TypeTable, expression: &StaticExpression) {
        if self.found {
            return;
        }
        if matches!(expression, StaticExpression::Unevaluated { .. }) {
            self.found = true;
            return;
        }
        walk_static_expression(self, types, expression);
    }
}

/// Walk types to detect unevaluated value static arguments.
struct UnevaluatedValueStaticArgumentVisitor<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The node tree for the current module.
    tree: &'a NodeTree,
    /// The symbol table for the current module.
    symbols: &'a SymbolTable,
    /// The type table for the current module.
    types: &'a TypeTable,
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether an unevaluated value argument was found.
    found: bool,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> UnevaluatedValueStaticArgumentVisitor<'a> {
    /// Create a visitor for unevaluated value static arguments.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self {
            compiler,
            module,
            profile,
            tree,
            symbols,
            types,
            visited,
            found: false,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for UnevaluatedValueStaticArgumentVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        if self.found {
            return;
        }
        if !self.visited.insert(id) {
            return;
        }
        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
        self.visited.remove(&id);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        if self.found {
            return;
        }
        if let Type::Reference {
            symbol,
            static_arguments,
        } = ty
        {
            if self
                .compiler
                .reference_contains_unevaluated_value_arguments(
                    self.module,
                    self.profile,
                    *symbol,
                    static_arguments.as_deref(),
                    self.tree,
                    self.symbols,
                    self.types,
                    self.visited,
                )
            {
                self.found = true;
            }
            return;
        }
        walk_type(self, types, id, ty);
    }

    fn visit_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        if self.found {
            return;
        }
        match argument {
            StaticArgument::Unevaluated { .. } => {
                self.found = true;
            }
            StaticArgument::Evaluated { value, .. } => {
                walk_static_expression(self, types, value);
            }
        }
    }

    fn visit_static_expression(&mut self, types: &TypeTable, expression: &StaticExpression) {
        if self.found {
            return;
        }
        if matches!(expression, StaticExpression::Unevaluated { .. }) {
            self.found = true;
            return;
        }
        walk_static_expression(self, types, expression);
    }
}

/// Walk types to detect static parameter usage.
struct StaticParameterVisitor<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The symbol table for the current module.
    symbols: &'a SymbolTable,
    /// The type table for the current module.
    types: &'a TypeTable,
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether a static parameter was found.
    found: bool,
    /// Whether traversal is inside a static argument.
    in_static_argument: bool,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> StaticParameterVisitor<'a> {
    /// Create a visitor for static parameter detection.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self {
            compiler,
            module,
            profile,
            symbols,
            types,
            visited,
            found: false,
            in_static_argument: false,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for StaticParameterVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        // stop after a match was found
        if self.found {
            return;
        }

        // avoid walking cycles
        if !self.visited.insert(id) {
            return;
        }

        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        // stop after a match was found
        if self.found {
            return;
        }

        // check for static parameter references and infer usage inside static arguments
        match ty {
            Type::Reference { symbol, .. } => {
                if self.compiler.symbol_is_static_parameter(
                    self.module,
                    self.profile,
                    *symbol,
                    self.symbols,
                    self.types,
                ) {
                    self.found = true;
                    return;
                }
            }
            Type::Infer { .. } => {
                if self.in_static_argument {
                    self.found = true;
                    return;
                }
            }
            _ => {}
        }

        // continue walking the type graph
        walk_type(self, types, id, ty);
    }

    fn visit_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        // stop after a match was found
        if self.found {
            return;
        }

        // inspect argument expressions for infer usage
        match argument {
            StaticArgument::Unevaluated { .. } => {
                self.found = true;
            }
            StaticArgument::Evaluated { value, .. } => {
                let previous = self.in_static_argument;
                self.in_static_argument = true;
                walk_static_expression(self, types, value);
                self.in_static_argument = previous;
            }
        }
    }
}

/// Walk types to detect free static parameters.
struct FreeStaticParameterVisitor<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The symbol table for the current module.
    symbols: &'a SymbolTable,
    /// The type table for the current module.
    types: &'a TypeTable,
    /// The bound static parameter symbols.
    bound: HashSet<GlobalSymbolId>,
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether a free static parameter was found.
    found: bool,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> FreeStaticParameterVisitor<'a> {
    /// Create a visitor for free static parameter detection.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        bound: &HashSet<GlobalSymbolId>,
        visited: &'a mut HashSet<LocalTypeId>,
    ) -> Self {
        Self {
            compiler,
            module,
            profile,
            symbols,
            types,
            bound: bound.clone(),
            visited,
            found: false,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for FreeStaticParameterVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        // stop after a match was found
        if self.found {
            return;
        }

        // avoid walking cycles
        if !self.visited.insert(id) {
            return;
        }

        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        // stop after a match was found
        if self.found {
            return;
        }

        // handle references and mapped parameters that bind static parameters
        match ty {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if static_arguments.is_none()
                    && self.compiler.symbol_is_static_parameter(
                        self.module,
                        self.profile,
                        *symbol,
                        self.symbols,
                        self.types,
                    )
                    && !self.bound.contains(symbol)
                {
                    self.found = true;
                    return;
                }
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                let inserted = self.bound.insert(parameter.symbol);
                self.visit_type_id(types, parameter.constraint);
                if let Some(key_remap) = parameter.key_remap {
                    self.visit_type_id(types, key_remap);
                }
                self.visit_type_id(types, *value);
                if inserted {
                    self.bound.remove(&parameter.symbol);
                }
                return;
            }
            _ => {}
        }

        // continue walking the type graph
        walk_type(self, types, id, ty);
    }

    fn visit_static_argument(&mut self, types: &TypeTable, argument: &StaticArgument) {
        // stop after a match was found
        if self.found {
            return;
        }

        // inspect argument expressions for unevaluated forms
        match argument {
            StaticArgument::Unevaluated { .. } => {
                self.found = true;
            }
            StaticArgument::Evaluated { value, .. } => {
                walk_static_expression(self, types, value);
            }
        }
    }
}

/// Select which infer containment to detect.
enum InferContainmentMode {
    /// Detect conditional infer bindings.
    Binding,
    /// Detect inference variables.
    Var,
}

/// Walk types to detect infer containment.
struct InferContainmentVisitor<'a> {
    /// The visited type ids.
    visited: &'a mut HashSet<LocalTypeId>,
    /// Whether a matching node was found.
    found: bool,
    /// The containment mode to detect.
    mode: InferContainmentMode,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> InferContainmentVisitor<'a> {
    /// Create a visitor for infer containment detection.
    fn new(visited: &'a mut HashSet<LocalTypeId>, mode: InferContainmentMode) -> Self {
        Self {
            visited,
            found: false,
            mode,
            options: TypeVisitorOptions::default(),
        }
    }
}

impl TypeVisitor for InferContainmentVisitor<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, types: &TypeTable, id: LocalTypeId) {
        // stop after a match was found
        if self.found {
            return;
        }

        // avoid walking cycles
        if !self.visited.insert(id) {
            return;
        }

        let ty = types.get_type(id);
        self.visit_type(types, id, ty);
    }

    fn visit_type(&mut self, types: &TypeTable, id: LocalTypeId, ty: &Type) {
        // stop after a match was found
        if self.found {
            return;
        }

        // detect infer containment
        let is_match = match self.mode {
            InferContainmentMode::Binding => matches!(ty, Type::Infer { .. }),
            InferContainmentMode::Var => matches!(ty, Type::InferVar { .. }),
        };
        if is_match {
            self.found = true;
            return;
        }

        // continue walking the type graph
        walk_type(self, types, id, ty);
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when a type is wrapped in explicit ownership modifiers.
    pub(crate) fn type_is_explicit_ownership_wrapper(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> bool {
        match types.get_type(type_id) {
            Type::ValueOf { .. } | Type::ReferenceOf { .. } | Type::PointerOf { .. } => true,
            Type::Value { value } => self.type_is_explicit_ownership_wrapper(types, *value),
            _ => false,
        }
    }

    /// Check whether a symbol is a static parameter.
    pub(crate) fn symbol_is_static_parameter(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // honor cached constraints for mapped parameters
        if types.get_static_parameter_constraint_type(symbol).is_some() {
            return true;
        }

        // rely on the declared parameter metadata
        if symbol.module_id == module.id {
            let symbol = symbols.get_symbol(symbol.local_id);
            if symbol.is_static_parameter() {
                return true;
            }
            return symbol
                .primary_declaration
                .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter);
        }

        // FUGU #Cleanup: audit all logic splits between local and remote modules
        // (and see if we can't introduce a more general helper somehow..?)
        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        let symbol = remote_symbols.get_symbol(symbol.local_id);
        if symbol.is_static_parameter() {
            return true;
        }
        symbol
            .primary_declaration
            .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter)
    }

    /// Check whether a type contains a static parameter reference.
    pub(crate) fn type_contains_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor =
            StaticParameterVisitor::new(self, module, profile, symbols, types, visited);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type contains free static parameter references.
    pub(crate) fn type_contains_free_static_parameters(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        bound: &HashSet<GlobalSymbolId>,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor =
            FreeStaticParameterVisitor::new(self, module, profile, symbols, types, bound, visited);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type contains conditional infer bindings.
    pub(crate) fn type_contains_infer(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = InferContainmentVisitor::new(visited, InferContainmentMode::Binding);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Check whether a type contains inference variables.
    pub(crate) fn type_contains_infer_vars(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = InferContainmentVisitor::new(visited, InferContainmentMode::Var);
        visitor.visit_type_id(types, type_id);
        visitor.found
    }

    /// Evaluate a type id in place when it is unevaluated.
    pub(crate) fn evaluate_unevaluated_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if matches!(types.get_type(type_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, type_id, tree, symbols, types)?;
        }
        Ok(type_id)
    }

    /// Resolve a type symbol from a type reference or type-as-value.
    pub(crate) fn unwrap_type_value_symbol(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // unwrap direct references
        if let Type::Reference { symbol, .. } = types.get_type(type_id) {
            return Some(*symbol);
        }

        // unwrap references stored in type-as-value wrappers
        if let Type::Value { value } = types.get_type(type_id)
            && let Type::Reference { symbol, .. } = types.get_type(*value)
        {
            return Some(*symbol);
        }

        None
    }

    /// Import the alias target type for a symbol when available.
    pub(crate) fn alias_target_type_id_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let mut visited = HashSet::new();
        let mut current = symbol;

        loop {
            if !visited.insert(current) {
                return None;
            }

            // load the local alias target when the symbol is local
            if current.module_id == module.id {
                let symbol_entry = symbols.get_symbol(current.local_id);
                if matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                    let typed_symbol = GlobalSymbolId::new(
                        current.module_id,
                        current.local_id.with_type(symbol_entry.ty),
                    );
                    if let Some(target) = types.get_alias_target_type_id(typed_symbol) {
                        return Some(target);
                    }
                }

                // follow import targets for local alias references
                let target_symbol = symbol_entry
                    .target_symbol
                    .or(symbol_entry.canonical_symbol)?;
                current = target_symbol;
                continue;
            }

            // import the alias target when the symbol is remote
            if current.module_id != module.id {
                let _ = self.require_analyze_module_declare(current.module_id, profile);
            }
            let remote_module = self.program.modules.get(current.module_id);
            let remote_module = remote_module.read();
            let remote_dir = remote_module.dir(profile);
            let remote_tree = remote_dir.tree.read();
            let remote_symbols = remote_dir.symbols.read();
            let symbol_entry = remote_symbols.get_symbol(current.local_id);
            if !matches!(symbol_entry.ty, SymbolType::TypeAlias | SymbolType::Newtype) {
                // follow remote import targets when present
                let target_symbol = symbol_entry
                    .target_symbol
                    .or(symbol_entry.canonical_symbol)?;
                current = target_symbol;
                continue;
            }

            let typed_symbol = GlobalSymbolId::new(
                current.module_id,
                current.local_id.with_type(symbol_entry.ty),
            );

            // import remote alias targets from the export summary
            let mut remote_types = remote_dir.types.write();
            let remote_target_id = remote_types.get_alias_target_type_id(typed_symbol)?;

            // evaluate remote alias targets before importing
            if matches!(
                remote_types.get_type(remote_target_id),
                Type::Unevaluated(_)
            ) && let Err(error) = self.evaluate_type(
                &remote_module,
                profile,
                remote_target_id,
                &remote_tree,
                &remote_symbols,
                &mut remote_types,
            ) {
                self.error(error);
                return None;
            }

            // skip alias targets that still need value materialization
            let needs_materialization = self.type_contains_unevaluated_value_static_arguments(
                &remote_module,
                profile,
                remote_target_id,
                &remote_tree,
                &remote_symbols,
                &remote_types,
                &mut HashSet::new(),
            );
            if needs_materialization {
                return None;
            }

            let remote_target_ty = remote_types.get_type(remote_target_id);
            let local_alias_target_id = self.import_type_from_remote_for_node(
                source_id,
                remote_target_ty,
                &remote_types,
                typed_symbol,
                types,
            );
            types.set_alias_target_type_id(typed_symbol, local_alias_target_id);
            return Some(local_alias_target_id);
        }
    }

    /// Require an instance type for a symbol into the local type table.
    pub(crate) fn require_instance_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // reuse local instance types when already available
        if let Some(instance_id) = types.get_instance_type_id(symbol) {
            return Some(instance_id);
        }

        // load or import the instance type through the existing resolver
        match self.resolve_instance_type_for_symbol(module, profile, source_id, symbol, types) {
            Ok(instance_id) => instance_id,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Resolve the apparent instance type for shape queries like `keyof`.
    pub(crate) fn apparent_instance_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve through canonical import targets while preserving aliases
        let symbol = if symbol.ty() == SymbolType::Extension {
            symbol
        } else {
            self.canonical_symbol_id(
                module,
                symbols,
                profile,
                symbol,
                CanonicalSymbolMode::PreserveAliases,
            )
        };

        // prefer alias targets as the apparent type when available
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(module, profile, symbol, source_id, symbols, types)
        {
            return Some(alias_target_id);
        }

        // otherwise fall back to the instance type
        self.require_instance_type(module, profile, source_id, symbol, symbols, types)
    }

    /// Unwrap a type-as-value wrapper to the underlying type id.
    pub(crate) fn unwrap_type_value(&self, type_id: LocalTypeId, types: &TypeTable) -> LocalTypeId {
        match types.get_type(type_id) {
            Type::Value { value } => *value,
            _ => type_id,
        }
    }

    /// Resolve an enum symbol from a type when possible.
    pub(crate) fn enum_symbol_for_type(
        &self,
        ty: &Type,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        match ty {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Enum => Some(*symbol),
            Type::Value { value } => {
                let inner = types.get_type(*value);
                self.enum_symbol_for_type(inner, types)
            }
            Type::Intersection { elements } => elements.iter().find_map(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.enum_symbol_for_type(element_ty, types)
            }),
            _ => None,
        }
    }

    /// Build a union type from two type ids.
    pub(crate) fn union_type(
        &self,
        left: LocalTypeId,
        right: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // reuse the left source for the combined union
        self.union_type_from_list(vec![left, right], left, types)
    }

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested unions and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union { elements: union } => {
                    for element_id in union {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and remove never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating any or unknown
        if let Some(any_type) = any_type {
            return any_type;
        }
        if let Some(unknown_type) = unknown_type {
            return unknown_type;
        }

        // fall back to never when the union is empty
        if filtered.is_empty() {
            return never_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the union type
        let union = Type::Union { elements: filtered };
        types.insert_type_from_any(union, types.get_type_source(source_type_id))
    }

    /// Build an intersection type from a list of elements.
    pub(crate) fn intersection_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested intersections and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Intersection { elements } => {
                    for element_id in elements {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and handle never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating never or any
        if let Some(never_type) = never_type {
            return never_type;
        }
        if let Some(any_type) = any_type {
            return any_type;
        }

        // fall back to unknown when the intersection is empty
        if filtered.is_empty() {
            return unknown_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the intersection type
        let intersection = Type::Intersection { elements: filtered };
        types.insert_type_from_any(intersection, types.get_type_source(source_type_id))
    }

    /// Check whether a type contains an error type.
    pub(crate) fn type_contains_error(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = ErrorTypeVisitor::new(visited);
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    pub(crate) fn type_contains_unevaluated_value_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = UnevaluatedValueStaticArgumentVisitor::new(
            self, module, profile, tree, symbols, types, visited,
        );
        visitor.visit_type_id(types, ty_id);
        visitor.found
    }

    fn reference_contains_unevaluated_value_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        arguments: Option<&[StaticArgument]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(arguments) = arguments else {
            return false;
        };

        // no arguments means no value materialization is needed
        if arguments.is_empty() {
            return false;
        }

        // resolve parameter kinds for the referenced declaration
        let Some(parameter_symbols) =
            self.collect_static_parameter_symbols(module, symbol, profile, tree, symbols)
        else {
            return false;
        };

        for (index, argument) in arguments.iter().enumerate() {
            let kind = parameter_symbols
                .get(index)
                .map(|parameter_symbol| {
                    if parameter_symbol.module_id == module.id {
                        self.static_parameter_kind_for_symbol_in_module(
                            *parameter_symbol,
                            tree,
                            symbols,
                        )
                    } else {
                        let remote_module = self.program.modules.get(parameter_symbol.module_id);
                        let remote_module = remote_module.read();
                        let remote_tree = remote_module.dir(profile).tree.read();
                        let remote_symbols = remote_module.dir(profile).symbols.read();
                        self.static_parameter_kind_for_symbol_in_module(
                            *parameter_symbol,
                            &remote_tree,
                            &remote_symbols,
                        )
                    }
                })
                .unwrap_or(StaticParameterKind::Type);

            // only value parameters require unevaluated materialization
            if kind != StaticParameterKind::Value {
                continue;
            }

            let has_unevaluated = match argument {
                StaticArgument::Unevaluated { .. } => true,
                StaticArgument::Evaluated { value, .. } => self
                    .static_expression_contains_unevaluated_static_arguments(value, types, visited),
            };
            if has_unevaluated {
                return true;
            }
        }

        false
    }

    /// Check whether a static expression contains unevaluated static arguments.
    fn static_expression_contains_unevaluated_static_arguments(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let mut visitor = UnevaluatedStaticArgumentVisitor::new(visited);
        visitor.visit_static_expression(types, expression);
        visitor.found
    }
}
