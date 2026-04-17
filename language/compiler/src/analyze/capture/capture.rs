use indexmap::IndexMap;

use destack_dir::{
    CaptureDirective, CaptureKind, CapturePolicy, CaptureSet, CaptureTable, CapturedBinding,
    Declaration, Expression, FunctionKind, FunctionSignature, GlobalSymbolId, LocalNodeId,
    LocalScopeId, Member, Mutability, NodeTree, NodeType, Scope, StaticKey, Symbol, SymbolSpace,
    SymbolTable,
};
use destack_source::ModuleId;

use crate::{AnalyzeResult, Compiler};

use super::walk::CaptureCollector;
use crate::analyze::common::TreeSymbolView;

impl Compiler {
    /// Resolve the nearest `this` symbol visible to an expression.
    pub(crate) fn resolve_this_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        let this_name = self.repository.strings.intern("this");

        // resolve an explicit this parameter in a signature
        let resolve_signature_this = |signature: &FunctionSignature| {
            if let Some(this_parameter_id) = signature.this_parameter {
                let symbol = ctx.tree.get(this_parameter_id).symbol();
                return Some(symbol.into_global(ctx.module.id));
            }
            None
        };

        // determine whether a signature declares an explicit this parameter
        let signature_has_explicit_this =
            |signature: &FunctionSignature| signature.this_parameter.is_some();

        // resolve an explicit this parameter bound into the scope
        let resolve_scope_this = |scope: &Scope| {
            let symbol_id = ctx
                .symbols
                .find_active_symbol(scope, StaticKey::Name(this_name))?;
            Some(symbol_id.into_global(ctx.module.id))
        };

        // walk outward through scope owners
        let mut scope = ctx.symbols.get_scope(expression_id, ctx.tree);

        loop {
            if let Some(owner_symbol) = scope.1.owner_id {
                let owner_symbol = owner_symbol.into_global(ctx.module.id);
                let owner_data = ctx.symbols.get_symbol(owner_symbol.into_local());
                if let Some(primary_declaration) = owner_data.primary_declaration {
                    let owner_id = primary_declaration.local_id;
                    let signature = match owner_id.ty {
                        NodeType::Declaration => {
                            let declaration_id = LocalNodeId::<Declaration>::new(owner_id.id);
                            match ctx.tree.get(declaration_id) {
                                Declaration::Function(declaration) => Some(&declaration.signature),
                                _ => None,
                            }
                        }
                        NodeType::Member => {
                            let member_id = LocalNodeId::<Member>::new(owner_id.id);
                            match ctx.tree.get(member_id) {
                                Member::Method { signature, .. } => Some(signature),
                                _ => None,
                            }
                        }
                        NodeType::Expression => {
                            let expression_id = LocalNodeId::<Expression>::new(owner_id.id);
                            match ctx.tree.get(expression_id) {
                                Expression::Declaration(declaration) => {
                                    match ctx.tree.get(*declaration) {
                                        Declaration::Function(declaration) => {
                                            Some(&declaration.signature)
                                        }
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

                        // prefer explicit this parameters when present
                        if has_explicit_this && let Some(symbol_id) = resolve_scope_this(scope.1) {
                            return Some(symbol_id);
                        }
                        if has_explicit_this
                            && let Some(symbol_id) = resolve_signature_this(signature)
                        {
                            return Some(symbol_id);
                        }

                        // prefer implicit this for member methods
                        if is_member && let Some(symbol_id) = resolve_scope_this(scope.1) {
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
                ctx.symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }
    }

    /// Compute capture sets for nested functions and lambdas in a module.
    pub(crate) fn compute_module_captures(
        &self,
        ctx: TreeSymbolView<'_>,
        captures: &mut CaptureTable,
    ) -> AnalyzeResult<()> {
        // scan for function declarations
        for declaration_id in ctx.tree.iter_node_ids_of_type::<Declaration>() {
            // skip inactive nodes
            if !self.is_node_active(ctx.tree, ctx.symbols, declaration_id.into_any()) {
                continue;
            }

            // unwrap function declarations
            let declaration = ctx.tree.get(declaration_id);
            let Declaration::Function(declaration) = declaration else {
                continue;
            };

            // skip inactive symbols
            let symbol = ctx.symbols.get_symbol(declaration.symbol);
            if !symbol.is_active() {
                continue;
            }

            // skip functions without bodies
            let Some(body_id) = declaration.body else {
                continue;
            };

            // resolve capture directive for this closure
            let function_symbol = declaration.symbol.into_global(ctx.module.id);
            let directive = captures
                .capture_directive(function_symbol)
                .cloned()
                .unwrap_or_default();

            // collect captured symbols
            let mut collector = CaptureCollector::new(ctx, declaration.scope, self);
            collector.collect(body_id);

            // build capture bindings and address taken info
            let mut captured_bindings = Vec::new();
            let mut address_taken = IndexMap::<GlobalSymbolId, IndexMap<GlobalSymbolId, ()>>::new();

            for symbol in collector.captured_symbols.into_keys() {
                // force `this` to capture by value
                let kind = if collector
                    .this_symbol
                    .is_some_and(|this_symbol| this_symbol == symbol)
                {
                    CaptureKind::ByValue
                } else {
                    self.capture_kind_for_symbol(ctx, symbol, &directive)
                };

                // record the capture in order
                captured_bindings.push(CapturedBinding { symbol, kind });

                // track address taken locals for by reference captures
                if kind == CaptureKind::ByReference {
                    let Some(owner_symbol) = self.owner_symbol_for_symbol(ctx, symbol) else {
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
                    this_symbol: collector.this_symbol,
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
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
        directive: &CaptureDirective,
    ) -> CaptureKind {
        // honor explicit rules when a name is present
        let symbol_data = ctx.symbols.get_symbol(symbol.into_local());
        if let Some(name) = symbol_data.name()
            && let Some(kind) = directive.override_for_name(name)
        {
            return kind;
        }

        // apply explicit policy defaults
        if directive.policy != CapturePolicy::Default {
            return directive.policy.default_kind();
        }

        // default policy: const by valueable by reference
        let mutability = self.mutability_for_symbol(ctx, symbol);
        if mutability == Some(Mutability::Immutable) {
            return CaptureKind::ByValue;
        }

        CaptureKind::ByReference
    }

    /// Resolve mutability for a symbol when available.
    fn mutability_for_symbol(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<Mutability> {
        // use the primary declaration when available
        let symbol_data = ctx.symbols.get_symbol(symbol.into_local());
        let primary_declaration = symbol_data.primary_declaration?;

        // skip declarations from other modules
        if primary_declaration.module_id != ctx.tree.module_id {
            return None;
        }

        // parameters are treated as mutable by default
        if primary_declaration.local_id.ty == NodeType::Parameter {
            return None;
        }

        // walk up to find the owning let binding
        let mut current = primary_declaration.local_id;
        loop {
            let parent_id = ctx.tree.get_parent(current.id)?;

            if parent_id.ty == NodeType::Expression {
                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id.id);
                let parent_expression = ctx.tree.get(parent_expression_id);
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
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        // resolve the owning scope for local symbols when possible
        let symbol_data = ctx.symbols.get_symbol(symbol.into_local());
        if symbol_data.module_id != ctx.module.id {
            return None;
        }
        let mut scope_id = symbol_data.scope.0;
        loop {
            let scope = ctx.symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner_id {
                let owner_symbol = ctx.symbols.get_symbol(owner_id);
                if self.symbol_is_function(ctx.tree, owner_symbol) {
                    return Some(owner_id.into_global(ctx.module.id));
                }
            }
            let Some((parent_id, _)) = scope.parent else {
                break;
            };
            scope_id = parent_id;
        }

        // fall back to walking up from the primary declaration
        let primary_declaration = symbol_data.primary_declaration?;
        if primary_declaration.module_id != ctx.module.id {
            return None;
        }

        // walk up the syntax tree to find the owning function declaration
        let mut current = primary_declaration.local_id;
        loop {
            match current.ty {
                NodeType::Declaration => {
                    let declaration_id = LocalNodeId::<Declaration>::new(current.id);
                    if let Declaration::Function(declaration) = ctx.tree.get(declaration_id) {
                        return Some(declaration.symbol.into_global(ctx.module.id));
                    }
                }
                NodeType::Member => {
                    let member_id = LocalNodeId::<Member>::new(current.id);
                    if let Member::Method { symbol, .. } = ctx.tree.get(member_id) {
                        return Some(symbol.into_global(ctx.module.id));
                    }
                }
                NodeType::Expression => {
                    let expression_id = LocalNodeId::<Expression>::new(current.id);
                    if let Expression::Declaration(declaration) = ctx.tree.get(expression_id)
                        && let Declaration::Function(declaration) = ctx.tree.get(*declaration)
                    {
                        return Some(declaration.symbol.into_global(ctx.module.id));
                    }
                }
                _ => {}
            }

            let parent_id = ctx.tree.get_parent(current.id)?;
            current = parent_id;
        }
    }

    /// Check whether a symbol represents a function declaration or method.
    fn symbol_is_function(&self, tree: &NodeTree, symbol: &Symbol) -> bool {
        let Some(primary) = symbol.primary_declaration else {
            return false;
        };
        let owner_id = primary.local_id;
        match owner_id.ty {
            NodeType::Declaration => {
                let declaration_id = LocalNodeId::<Declaration>::new(owner_id.id);
                matches!(tree.get(declaration_id), Declaration::Function(_))
            }
            NodeType::Member => {
                let member_id = LocalNodeId::<Member>::new(owner_id.id);
                matches!(tree.get(member_id), Member::Method { .. })
            }
            NodeType::Expression => {
                let expression_id = LocalNodeId::<Expression>::new(owner_id.id);
                if let Expression::Declaration(declaration) = tree.get(expression_id) {
                    matches!(tree.get(*declaration), Declaration::Function(_))
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
