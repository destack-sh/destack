use indexmap::IndexMap;

use destack_dir::{
    CaptureDirective, CaptureKind, CapturePolicy, CaptureSet, CaptureTable, CapturedBinding,
    Declaration, Expression, FunctionKind, GlobalSymbolId, LocalNodeId, LocalScopeId, Member,
    Mutability, NodeTree, NodeType, Parameter, StaticKey, SymbolSpace, SymbolTable,
};
use destack_source::ModuleId;
use destack_workspace::Module;

use crate::{AnalyzeResult, Compiler};

use super::walk::CaptureCollector;

impl Compiler {
    /// Resolve the nearest `this` symbol visible to an expression.
    pub(crate) fn resolve_this_symbol(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let this_name = self.program.strings.intern("this");

        // resolve an explicit this parameter in a signature
        let resolve_signature_this = |signature: &destack_dir::FunctionSignature| {
            if let Some(this_parameter_id) = signature.this_parameter {
                let symbol = tree.get(this_parameter_id).symbol();
                return Some(symbol.into_global(module.id));
            }

            // treat a leading this parameter as explicit when signature metadata is missing
            if let Some(first_parameter_id) = signature.dynamic_parameters.first().copied() {
                let parameter = tree.get(first_parameter_id);
                let is_explicit_this = match parameter {
                    Parameter::Named { name, .. } => *name == this_name,
                    _ => false,
                };
                if is_explicit_this {
                    let symbol = tree.get(first_parameter_id).symbol();
                    return Some(symbol.into_global(module.id));
                }
            }

            None
        };

        // determine whether a signature declares an explicit this parameter
        let signature_has_explicit_this = |signature: &destack_dir::FunctionSignature| {
            if signature.this_parameter.is_some() {
                return true;
            }
            if let Some(first_parameter_id) = signature.dynamic_parameters.first().copied() {
                let parameter = tree.get(first_parameter_id);
                match parameter {
                    Parameter::Named { name, .. } => *name == this_name,
                    _ => false,
                }
            } else {
                false
            }
        };

        // resolve an explicit this parameter bound into the scope
        let resolve_scope_this = |scope: &destack_dir::Scope| {
            let symbol_id = symbols.find_active_symbol(scope, StaticKey::Name(this_name))?;
            Some(symbol_id.into_global(module.id))
        };

        // walk outward through scope owners
        let mut scope = symbols.get_scope(expression_id, tree);

        loop {
            if let Some(owner_symbol) = scope.1.owner_id {
                let owner_symbol = owner_symbol.into_global(module.id);
                let owner_data = symbols.get_symbol(owner_symbol.into_local());
                if let Some(primary_declaration) = owner_data.primary_declaration {
                    let owner_id = primary_declaration.local_id;
                    let signature = match owner_id.ty {
                        NodeType::Declaration => {
                            let declaration_id = LocalNodeId::<Declaration>::new(owner_id.id);
                            match tree.get(declaration_id) {
                                Declaration::Function { signature, .. } => Some(signature),
                                _ => None,
                            }
                        }
                        NodeType::Member => {
                            let member_id = LocalNodeId::<Member>::new(owner_id.id);
                            match tree.get(member_id) {
                                Member::Method { signature, .. } => Some(signature),
                                _ => None,
                            }
                        }
                        NodeType::Expression => {
                            let expression_id = LocalNodeId::<Expression>::new(owner_id.id);
                            match tree.get(expression_id) {
                                Expression::Declaration { declaration } => {
                                    match tree.get(*declaration) {
                                        Declaration::Function { signature, .. } => Some(signature),
                                        _ => None,
                                    }
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    };

                    if let Some(signature) = signature {
                        let has_explicit_this = signature_has_explicit_this(signature);
                        let is_member = owner_id.ty == NodeType::Member;

                        // prefer implicit this for member methods
                        if is_member && let Some(symbol_id) = resolve_scope_this(scope.1) {
                            return Some(symbol_id);
                        }

                        // prefer explicit this parameters when present
                        if has_explicit_this && let Some(symbol_id) = resolve_scope_this(scope.1) {
                            return Some(symbol_id);
                        }

                        if has_explicit_this
                            && let Some(symbol_id) = resolve_signature_this(signature)
                        {
                            return Some(symbol_id);
                        }

                        // stop at non lambda functions to avoid capturing dynamic this
                        if signature.kind != FunctionKind::Lambda {
                            return None;
                        }
                    }
                }
            }

            // move to the parent scope
            let (parent_scope_id, parent_mark) = scope.1.parent?;
            scope = (
                parent_scope_id,
                symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }
    }

    /// Compute capture sets for nested functions and lambdas in a module.
    pub(crate) fn compute_module_captures(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        captures: &mut CaptureTable,
    ) -> AnalyzeResult<()> {
        // scan for function declarations
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            // skip inactive nodes
            if !self.is_node_active(tree, symbols, declaration_id.into_any()) {
                continue;
            }

            // unwrap function declarations
            let declaration = tree.get(declaration_id);
            let Declaration::Function {
                descriptor,
                signature: _,
                scope,
                body,
            } = declaration
            else {
                continue;
            };

            // skip inactive symbols
            let symbol = symbols.get_symbol(descriptor.symbol);
            if !symbol.is_active() {
                continue;
            }

            // skip functions without bodies
            let Some(body_id) = body else {
                continue;
            };

            // resolve capture directive for this closure
            let function_symbol = descriptor.symbol.into_global(module.id);
            let directive = captures
                .capture_directive(function_symbol)
                .cloned()
                .unwrap_or_default();

            // collect captured symbols
            let mut collector =
                CaptureCollector::new(module.id, *scope, tree, symbols, self, module);
            collector.collect(*body_id);

            // build capture bindings and address taken info
            let mut captured_bindings = Vec::new();
            let mut address_taken = IndexMap::<GlobalSymbolId, IndexMap<GlobalSymbolId, ()>>::new();

            for symbol in collector.captured_symbols.into_keys() {
                // resolve capture kind for each symbol
                let kind = self.capture_kind_for_symbol(tree, symbols, symbol, &directive);

                // record the capture in order
                captured_bindings.push(CapturedBinding { symbol, kind });

                // track address taken locals for by reference captures
                if kind == CaptureKind::ByReference {
                    let Some(owner_symbol) =
                        self.owner_symbol_for_symbol(module.id, tree, symbols, symbol)
                    else {
                        continue;
                    };

                    address_taken
                        .entry(owner_symbol)
                        .or_default()
                        .entry(symbol)
                        .or_insert(());
                }
            }

            // store capture set for this closure
            captures.set_capture_set(
                function_symbol,
                CaptureSet {
                    captures: captured_bindings,
                    directive,
                },
            );

            // record locals that need address taken
            for (owner_symbol, locals) in address_taken {
                let locals = locals.into_keys().collect::<Vec<_>>();
                captures.set_reference_locals(owner_symbol, locals);
            }
        }

        Ok(())
    }

    /// Decide whether a symbol should be captured for a closure.
    pub(crate) fn should_capture_symbol(
        &self,
        module_id: ModuleId,
        symbols: &SymbolTable,
        closure_scope: LocalScopeId,
        symbol: GlobalSymbolId,
    ) -> bool {
        // skip symbols outside this module
        if symbol.module_id != module_id {
            return false;
        }

        // skip type only symbols
        let symbol_data = symbols.get_symbol(symbol.into_local());
        if symbol_data.space == SymbolSpace::Type {
            return false;
        }

        // skip root scope bindings
        let scope_id = symbol_data.scope.0;
        let scope = symbols.get_scope_by_id(scope_id);
        if scope.parent.is_none() {
            return false;
        }

        // skip locals declared inside the closure
        if self.is_scope_descendant(symbols, scope_id, closure_scope) {
            return false;
        }

        true
    }

    /// Check whether a scope is a descendant of another scope.
    fn is_scope_descendant(
        &self,
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
        ancestor: LocalScopeId,
    ) -> bool {
        // walk upward until the root or the ancestor is found
        let mut current = scope_id;
        loop {
            if current == ancestor {
                return true;
            }

            let scope = symbols.get_scope_by_id(current);
            let Some((parent_id, _)) = scope.parent else {
                return false;
            };
            current = parent_id;
        }
    }

    /// Resolve the capture kind for a symbol given a directive.
    fn capture_kind_for_symbol(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
        directive: &CaptureDirective,
    ) -> CaptureKind {
        // honor explicit rules when a name is present
        let symbol_data = symbols.get_symbol(symbol.into_local());
        if let Some(name) = symbol_data.name()
            && let Some(kind) = directive.override_for_name(name)
        {
            return kind;
        }

        // apply explicit policy defaults
        if directive.policy != CapturePolicy::Default {
            return directive.policy.default_kind();
        }

        // default policy: const by value, mutable by reference
        let mutability = self.mutability_for_symbol(tree, symbols, symbol);
        if mutability == Some(Mutability::Immutable) {
            return CaptureKind::ByValue;
        }

        CaptureKind::ByReference
    }

    /// Resolve mutability for a symbol when available.
    fn mutability_for_symbol(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
    ) -> Option<Mutability> {
        // use the primary declaration when available
        let symbol_data = symbols.get_symbol(symbol.into_local());
        let primary_declaration = symbol_data.primary_declaration?;

        // skip declarations from other modules
        if primary_declaration.module_id != tree.module_id {
            return None;
        }

        // parameters are treated as mutable by default
        if primary_declaration.local_id.ty == NodeType::Parameter {
            return None;
        }

        // walk up to find the owning let binding
        let mut current = primary_declaration.local_id;
        loop {
            let parent_id = tree.get_parent(current.id)?;

            if parent_id.ty == NodeType::Expression {
                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id.id);
                let parent_expression = tree.get(parent_expression_id);
                if let Expression::Let { mutability, .. } = parent_expression {
                    return Some(*mutability);
                }
            }

            current = parent_id;
        }
    }

    /// Resolve the owning function symbol for a local symbol.
    fn owner_symbol_for_symbol(
        &self,
        module_id: ModuleId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        // start from the primary declaration
        let symbol_data = symbols.get_symbol(symbol.into_local());
        let primary_declaration = symbol_data.primary_declaration?;
        if primary_declaration.module_id != module_id {
            return None;
        }

        // walk up the syntax tree to find the owning function declaration
        let mut current = primary_declaration.local_id;
        loop {
            if current.ty == NodeType::Declaration {
                let declaration_id = LocalNodeId::<Declaration>::new(current.id);
                if let Declaration::Function { descriptor, .. } = tree.get(declaration_id) {
                    return Some(descriptor.symbol.into_global(module_id));
                }
            }

            let parent_id = tree.get_parent(current.id)?;
            current = parent_id;
        }
    }
}
