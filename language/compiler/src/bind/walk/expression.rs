use destack_dir as dir;
use dir::NodeVisitor as _;

use super::super::state::{BindState, BindingContext};

use crate::Compiler;

impl Compiler {
    /// Bind one expression and visit its children with expression-specific scope rules.
    pub(in crate::bind) fn bind_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Label { label, body } => {
                // bind label body scope
                state.bind_node(id.into_any());
                let key = dir::StaticKey::Name(*label);
                let (symbol_id, scope_id) = state.insert_symbol_with_scope(
                    dir::SymbolRole::Local,
                    dir::SymbolForm::Label,
                    Some(key),
                    None,
                    dir::ScopeKind::Label,
                );
                state.declare_symbol(symbol_id, id);
                state.bind_node_to_scope(id.into_any(), scope_id);

                state.push_scope(scope_id);
                self.visit_expression_by_id(state, tree, *body);
                state.pop_scope();
            }
            dir::Expression::Import { items, .. } => {
                // bind import edge
                state.bind_node(id.into_any());
                self.bind_import_items(state, tree, items.as_deref());
            }
            dir::Expression::Export { items, .. } => {
                // bind export edge
                state.bind_node(id.into_any());
                self.bind_export_items(state, tree, items);
            }
            dir::Expression::Let {
                export,
                mutability,
                declarators,
                is_ambient: _,
                ..
            } => {
                // bind let declarators
                state.bind_node(id.into_any());
                let binding = BindingContext {
                    export: *export,
                    mutability: Some(*mutability),
                    declared_type: None,
                };
                self.bind_declarators(state, tree, declarators, binding);
            }
            dir::Expression::LetElse {
                mutability,
                declarator,
                else_branch,
                ..
            } => {
                // bind guard declarator
                state.bind_node(id.into_any());
                let binding = BindingContext {
                    export: None,
                    mutability: Some(*mutability),
                    declared_type: None,
                };
                self.bind_declarators(state, tree, &[*declarator], binding);
                self.visit_expression_by_id(state, tree, *else_branch);
            }
            dir::Expression::Using {
                export,
                declarators,
                is_ambient: _,
                ..
            } => {
                // bind resource declarators
                state.bind_node(id.into_any());
                let binding = BindingContext {
                    export: *export,
                    mutability: None,
                    declared_type: None,
                };
                self.bind_declarators(state, tree, declarators, binding);
            }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.bind_if_expression(
                state,
                tree,
                id,
                condition,
                *then_expression,
                *else_expression,
            ),
            dir::Expression::ForEach {
                binding,
                iterator,
                body,
                ..
            } => self.bind_for_each_expression(state, tree, id, binding, *iterator, *body),
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => self.bind_for_expression(
                state,
                tree,
                id,
                *initialization,
                *condition,
                *increment,
                *body,
            ),
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => self.bind_try_expression(state, tree, id, *body, *catch, *finally),
            _ => dir::walk_expression(state, tree, id, expression),
        }
    }

    /// Visit one expression id.
    fn visit_expression_by_id(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        let expression = tree.get(id);
        state.visit_expression(tree, id, expression);
    }

    /// Visit one block id.
    fn visit_block_by_id(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Block>,
    ) {
        let block = tree.get(id);
        state.visit_block(tree, id, block);
    }

    /// Visit one dependency item id.
    fn visit_dependency_item_by_id(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
    ) {
        let item = tree.get(id);
        state.visit_dependency_item(tree, id, item);
    }

    /// Bind declarators under one binding context.
    fn bind_declarators(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
        binding: BindingContext,
    ) {
        // enter binding context
        state.push_binding(binding);

        // visit declarators
        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            state.visit_declarator(tree, *declarator_id, declarator);
        }

        state.pop_binding();
    }

    /// Bind one if expression.
    fn bind_if_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        condition: &dir::IfCondition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        state.bind_node(id.into_any());

        match condition {
            dir::IfCondition::Expression { condition } => {
                // visit boolean condition
                self.visit_expression_by_id(state, tree, *condition);

                // visit then branch
                self.visit_expression_by_id(state, tree, then_expression);

                // visit optional else branch
                if let Some(else_expression) = else_expression {
                    self.visit_expression_by_id(state, tree, else_expression);
                }
            }
            dir::IfCondition::Let { declarator, .. } => {
                // create condition scope
                let scope_id = state.insert_child_scope(dir::ScopeKind::Block);
                state.bind_node_to_scope(id.into_any(), scope_id);

                // bind condition declaration and then branch
                state.push_scope(scope_id);
                let binding = BindingContext {
                    export: None,
                    mutability: None,
                    declared_type: None,
                };
                self.bind_declarators(state, tree, &[*declarator], binding);
                self.visit_expression_by_id(state, tree, then_expression);
                state.pop_scope();

                // visit optional else branch
                if let Some(else_expression) = else_expression {
                    self.visit_expression_by_id(state, tree, else_expression);
                }
            }
        }
    }

    /// Bind one foreach expression.
    fn bind_for_each_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // bind loop expression and iterator first
        state.bind_node(id.into_any());
        self.visit_expression_by_id(state, tree, iterator);

        // create loop scope
        let scope_id = state.insert_child_scope(dir::ScopeKind::Block);
        state.bind_node_to_scope(id.into_any(), scope_id);

        // bind loop pattern
        state.push_scope(scope_id);
        let binding_context = BindingContext {
            export: None,
            mutability: None,
            declared_type: None,
        };
        state.push_binding(binding_context);
        match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => {
                let pattern_node = tree.get(*pattern);
                state.visit_pattern(tree, *pattern, pattern_node);
            }
        }
        state.pop_binding();

        // visit loop body
        self.visit_block_by_id(state, tree, body);
        state.pop_scope();
    }

    /// Bind one traditional for expression.
    fn bind_for_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // create loop scope
        state.bind_node(id.into_any());
        let scope_id = state.insert_child_scope(dir::ScopeKind::Block);
        state.bind_node_to_scope(id.into_any(), scope_id);

        // visit loop clauses
        state.push_scope(scope_id);
        if let Some(initialization) = initialization {
            self.visit_expression_by_id(state, tree, initialization);
        }
        if let Some(condition) = condition {
            self.visit_expression_by_id(state, tree, condition);
        }
        if let Some(increment) = increment {
            self.visit_expression_by_id(state, tree, increment);
        }

        // visit loop body
        self.visit_block_by_id(state, tree, body);
        state.pop_scope();
    }

    /// Bind one try expression.
    fn bind_try_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        // visit try body
        state.bind_node(id.into_any());
        self.visit_expression_by_id(state, tree, body);

        // visit catch body in catch scope
        if let Some(catch) = catch {
            let scope_id = state.insert_child_scope(dir::ScopeKind::Block);
            state.bind_node_to_scope(catch.into_any(), scope_id);
            state.push_scope(scope_id);
            let catch_node = tree.get(catch);
            state.visit_catch(tree, catch, catch_node);
            state.pop_scope();
        }

        // visit finally body
        if let Some(finally) = finally {
            self.visit_expression_by_id(state, tree, finally);
        }
    }

    /// Bind import dependency items.
    fn bind_import_items(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
    ) {
        // skip bare side effect imports
        let Some(items) = items else {
            return;
        };

        // bind imported items
        for item_id in items {
            let item = tree.get(*item_id);
            self.bind_import_item(state, *item_id, item);
            self.visit_dependency_item_by_id(state, tree, *item_id);
        }
    }

    /// Bind export dependency items.
    fn bind_export_items(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) {
        // visit exported items
        for item_id in items {
            let item = tree.get(*item_id);
            self.bind_export_item(state, *item_id, item);
            self.visit_dependency_item_by_id(state, tree, *item_id);
        }
    }

    /// Bind one catch branch.
    pub(in crate::bind) fn bind_catch(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Catch>,
        catch: &dir::Catch,
    ) {
        state.bind_node(id.into_any());

        // bind catch type before introducing the catch pattern
        if let Some(ty) = catch.ty {
            let ty_node = tree.get(ty);
            state.visit_type_expression(tree, ty, ty_node);
        }

        // bind catch pattern before the catch body
        if let Some(pattern) = catch.pattern {
            let pattern_node = tree.get(pattern);
            state.visit_pattern(tree, pattern, pattern_node);
        }

        // visit catch body
        self.visit_expression_by_id(state, tree, catch.body);
    }
}
