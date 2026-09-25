use dir::NodeVisitor as _;
use tspp_dir as dir;

use super::super::state::{BindState, BindingModifiers};

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
        // declare the control label outside lexical lookup
        if let Some(label) = expression.control_label() {
            state.declare_control_label(label, id);
        }

        // apply expression scope rules
        match expression {
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
                is_shared,
                declarators,
                ..
            } => {
                // bind let declarators
                state.bind_node(id.into_any());
                let modifiers = BindingModifiers {
                    export: *export,
                    mutability: Some(*mutability),
                    is_shared: *is_shared,
                    kind: dir::SymbolKind::Variable,
                };
                self.bind_declarators(state, tree, declarators, modifiers);
            }
            dir::Expression::LetElse {
                mutability,
                declarator,
                else_branch,
                ..
            } => {
                // bind guard declarator
                state.bind_node(id.into_any());
                let modifiers = BindingModifiers {
                    export: None,
                    mutability: Some(*mutability),
                    is_shared: false,
                    kind: dir::SymbolKind::Variable,
                };
                self.bind_declarators(state, tree, &[*declarator], modifiers);
                self.visit_expression_by_id(state, tree, *else_branch);
            }
            dir::Expression::Using {
                export,
                declarators,
                ..
            } => {
                // bind resource declarators
                state.bind_node(id.into_any());
                let modifiers = BindingModifiers {
                    export: *export,
                    mutability: Some(dir::Mutability::Immutable),
                    is_shared: false,
                    kind: dir::SymbolKind::Variable,
                };
                self.bind_declarators(state, tree, declarators, modifiers);
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
            dir::Expression::While {
                condition, body, ..
            } => self.bind_while_expression(state, tree, id, condition, *body),
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
                ..
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

    /// Bind declarators under one set of source modifiers.
    fn bind_declarators(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
        modifiers: BindingModifiers,
    ) {
        // visit declarators with modifiers scoped to their patterns
        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            self.bind_declarator(state, tree, *declarator_id, declarator, modifiers);
        }
    }

    /// Bind one if expression.
    fn bind_if_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        state.bind_node(id.into_any());

        // create condition scope when operands bind names
        if condition.has_binding() {
            let scope_id = state.insert_child_scope(dir::ScopeKind::Block);
            state.bind_node_to_scope(id.into_any(), scope_id);

            state.push_scope(scope_id);
            self.bind_condition_operands(state, tree, &condition.operands);
            self.visit_expression_by_id(state, tree, then_expression);
            state.pop_scope();
        }
        // otherwise visit operands and then branch in the current scope
        else {
            self.bind_condition_operands(state, tree, &condition.operands);
            self.visit_expression_by_id(state, tree, then_expression);
        }

        // visit optional else branch
        if let Some(else_expression) = else_expression {
            self.visit_expression_by_id(state, tree, else_expression);
        }
    }

    /// Bind one while expression.
    fn bind_while_expression(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        condition: &dir::Condition,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // bind the loop expression
        state.bind_node(id.into_any());

        // scope condition bindings through the loop body
        if condition.has_binding() {
            let scope_id = state.insert_child_scope(dir::ScopeKind::Block);
            state.bind_node_to_scope(id.into_any(), scope_id);

            state.push_scope(scope_id);
            self.bind_condition_operands(state, tree, &condition.operands);
            self.visit_block_by_id(state, tree, body);
            state.pop_scope();
        }
        // visit binding-free loops in the current scope
        else {
            self.bind_condition_operands(state, tree, &condition.operands);
            self.visit_block_by_id(state, tree, body);
        }
    }

    /// Bind one condition chain.
    pub(in crate::bind) fn bind_condition_operands(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        operands: &[dir::ConditionOperand],
    ) {
        for operand in operands {
            match operand {
                // boolean condition
                dir::ConditionOperand::Expression { condition } => {
                    self.visit_expression_by_id(state, tree, *condition);
                }
                // pattern binding condition
                dir::ConditionOperand::Binding {
                    mutability,
                    declarator,
                    ..
                } => {
                    let modifiers = BindingModifiers {
                        export: None,
                        mutability: Some(*mutability),
                        is_shared: false,
                        kind: dir::SymbolKind::Variable,
                    };
                    self.bind_declarators(state, tree, &[*declarator], modifiers);
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

        // select declaration mutability from the loop binding form
        let (pattern, mutability) = match binding {
            dir::ForEachBinding::Pattern { pattern, keyword } => {
                let mutability = match keyword {
                    Some(dir::BindingKeyword::Let) => Some(dir::Mutability::Mutable),
                    Some(dir::BindingKeyword::Const) => Some(dir::Mutability::Immutable),
                    None => None,
                };

                (*pattern, mutability)
            }
            dir::ForEachBinding::Using { pattern, .. } => {
                (*pattern, Some(dir::Mutability::Immutable))
            }
        };

        // bind loop pattern
        state.push_scope(scope_id);
        let modifiers = BindingModifiers {
            export: None,
            mutability,
            is_shared: false,
            kind: dir::SymbolKind::Variable,
        };
        state.push_binding_modifiers(modifiers);
        let pattern_node = tree.get(pattern);
        state.visit_pattern(tree, pattern, pattern_node);
        state.pop_binding_modifiers();

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
            state.push_binding_modifiers(BindingModifiers {
                export: None,
                mutability: Some(dir::Mutability::Mutable),
                is_shared: false,
                kind: dir::SymbolKind::Variable,
            });
            state.visit_pattern(tree, pattern, pattern_node);
            state.pop_binding_modifiers();
        }

        // bind catch match failure before the catch body
        if catch.pattern.is_none()
            && catch.ty.is_none()
            && let dir::Expression::Match { value, .. } = tree.get(catch.body)
            && let dir::Expression::Identifier { name } = tree.get(*value)
        {
            let modifiers = BindingModifiers {
                export: None,
                mutability: Some(dir::Mutability::Mutable),
                is_shared: false,
                kind: dir::SymbolKind::Variable,
            };
            let symbol_id = state.insert_binding_symbol(dir::StaticKey::Name(*name), modifiers);
            state.declare_symbol(symbol_id, *value);
        }

        // visit catch body
        self.visit_expression_by_id(state, tree, catch.body);
    }
}
