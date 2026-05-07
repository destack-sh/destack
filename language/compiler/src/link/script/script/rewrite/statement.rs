use std::str::FromStr;

use destack_ast::is_identifier;
use destack_codegen_js as js;
use destack_core::StringId;

use super::linker::Rewriter;

impl Rewriter<'_, '_> {
    /// Return whether output syntax minification may use object shorthand.
    pub(super) fn can_use_object_shorthand(&self) -> bool {
        self.target
            .bundle_output
            .generated_code
            .as_ref()
            .and_then(|generated_code| generated_code.object_shorthand)
            .unwrap_or(true)
    }

    /// Return whether output syntax minification may leave reserved names unquoted as properties.
    fn can_use_reserved_property_names(&self) -> bool {
        self.target
            .bundle_output
            .generated_code
            .as_ref()
            .and_then(|generated_code| generated_code.reserved_names_as_props)
            .unwrap_or(true)
    }

    /// Return one rewritten property key spelling when output syntax changes it.
    pub(super) fn rewritten_property_name(&self, key: &js::Key) -> Option<js::Name> {
        let js::Key::Name(name) = key else {
            return None;
        };

        let (string_id, content) = match name {
            js::Name::Identifier(name) | js::Name::String(name) => {
                (*name, self.module.strings.get(*name))
            }
        };

        let minified_name = if self.can_use_identifier_property_name(&content) {
            js::Name::Identifier(string_id)
        } else {
            js::Name::String(string_id)
        };

        if *name == minified_name {
            return None;
        }

        Some(minified_name)
    }

    /// Rewrite one property key to the minified output spelling.
    pub(super) fn set_property_key_name(key: &mut js::Key, name: js::Name) {
        let js::Key::Name(_) = key else {
            return;
        };

        *key = js::Key::Name(name);
    }

    /// Return whether one property name may use identifier syntax in output.
    pub(super) fn can_use_identifier_property_name(&self, name: &str) -> bool {
        if !is_identifier(name) {
            return false;
        }

        if self.can_use_reserved_property_names() {
            return true;
        }

        js::Keyword::from_str(name).is_err()
    }

    /// Return whether one expression is a direct identifier reference with the given name.
    fn is_matching_identifier_reference(
        module: &js::Module,
        expression_id: js::LocalNodeId<js::Expression>,
        name: StringId,
    ) -> bool {
        let js::Expression::Path {
            path,
            generic_arguments,
        } = module.tree.get(expression_id)
        else {
            return false;
        };

        path.segments.len() == 1 && path.segments[0] == name && generic_arguments.is_empty()
    }

    /// Rewrite `let x = undefined` into `let x` for simple mutable bindings.
    pub(super) fn elide_undefined_let_initializers(module: &mut js::Module) {
        let statement_ids = module.tree.get_nodes::<js::Statement>();

        // let statements
        for statement_id in statement_ids {
            let statement = module.tree.get(statement_id).clone();

            let js::Statement::Let {
                mutability: js::Mutability::Mutable,
                declarators,
                ..
            } = statement
            else {
                continue;
            };

            // declarators
            for declarator_id in declarators {
                let declarator = module.tree.get(declarator_id).clone();
                let Some(value) = declarator.value else {
                    continue;
                };

                if !matches!(
                    module.tree.get(declarator.pattern),
                    js::Pattern::Binding { .. }
                ) {
                    continue;
                }

                if !Self::is_undefined_expression(module, value) {
                    continue;
                }

                let declarator = module.tree.get_mut(declarator_id);
                declarator.value = None;
            }
        }
    }

    /// Merge adjacent binding statements that share declaration fields and mutability.
    pub(super) fn merge_adjacent_binding_statements(module: &mut js::Module) {
        let roots = std::mem::take(&mut module.roots);
        module.roots = Self::merge_adjacent_root_binding_statements(module, roots);

        for block_id in module.tree.get_nodes::<js::Block>() {
            let statements = {
                let block = module.tree.get(block_id);
                block.statements.clone()
            };

            let statements = Self::merge_adjacent_statement_bindings(module, statements);
            let block = module.tree.get_mut(block_id);
            block.statements = statements;
        }
    }

    /// Merge adjacent binding statements across one root list.
    fn merge_adjacent_root_binding_statements(
        module: &mut js::Module,
        roots: Vec<js::LocalNodeIdAny>,
    ) -> Vec<js::LocalNodeIdAny> {
        let mut merged_roots = Vec::with_capacity(roots.len());

        // roots
        for root_id in roots {
            let Some(previous_root_id) = merged_roots.last().copied() else {
                merged_roots.push(root_id);
                continue;
            };

            if !Self::try_merge_root_binding_statement(module, previous_root_id, root_id) {
                merged_roots.push(root_id);
            }
        }

        merged_roots
    }

    /// Merge adjacent binding statements across one statement list.
    fn merge_adjacent_statement_bindings(
        module: &mut js::Module,
        statements: Vec<js::LocalNodeId<js::Statement>>,
    ) -> Vec<js::LocalNodeId<js::Statement>> {
        let mut merged_statements = Vec::with_capacity(statements.len());

        // statements
        for statement_id in statements {
            let Some(previous_statement_id) = merged_statements.last().copied() else {
                merged_statements.push(statement_id);
                continue;
            };

            if !Self::try_merge_binding_statement(module, previous_statement_id, statement_id) {
                merged_statements.push(statement_id);
            }
        }

        merged_statements
    }

    /// Try to merge one root statement into the previous root statement.
    fn try_merge_root_binding_statement(
        module: &mut js::Module,
        left_root_id: js::LocalNodeIdAny,
        right_root_id: js::LocalNodeIdAny,
    ) -> bool {
        if left_root_id.ty != js::NodeType::Statement || right_root_id.ty != js::NodeType::Statement
        {
            return false;
        }

        Self::try_merge_binding_statement(
            module,
            js::LocalNodeId::new(left_root_id.id),
            js::LocalNodeId::new(right_root_id.id),
        )
    }

    /// Try to merge one binding statement into the previous binding statement.
    fn try_merge_binding_statement(
        module: &mut js::Module,
        left_statement_id: js::LocalNodeId<js::Statement>,
        right_statement_id: js::LocalNodeId<js::Statement>,
    ) -> bool {
        let js::Statement::Let {
            export: left_export,
            is_ambient: left_is_ambient,
            mutability: left_mutability,
            declarators: _,
        } = module.tree.get(left_statement_id).clone()
        else {
            return false;
        };

        let js::Statement::Let {
            export: right_export,
            is_ambient: right_is_ambient,
            mutability: right_mutability,
            declarators: right_declarators,
        } = module.tree.get(right_statement_id).clone()
        else {
            return false;
        };

        if left_mutability != right_mutability
            || left_export != right_export
            || left_is_ambient != right_is_ambient
        {
            return false;
        }

        let statement = module.tree.get_mut(left_statement_id);
        let js::Statement::Let { declarators, .. } = statement else {
            unreachable!("binding merge only mutates let statements");
        };

        declarators.extend(right_declarators);

        true
    }

    /// Rewrite `return undefined` inside function bodies where that is implicit.
    pub(super) fn elide_undefined_returns(module: &mut js::Module) {
        let declaration_ids = module.tree.get_nodes::<js::Declaration>();

        // function declarations
        for declaration_id in declaration_ids {
            let declaration = module.tree.get(declaration_id).clone();

            let js::Declaration::Function(js::FunctionDeclaration {
                signature,
                body: Some(body),
                ..
            }) = declaration
            else {
                continue;
            };

            if signature.asynchrony == js::Asynchrony::Async && signature.is_generator {
                continue;
            }

            Self::elide_undefined_returns_in_block(module, body);
        }
    }

    /// Rewrite `return undefined` recursively within one function block.
    fn elide_undefined_returns_in_block(
        module: &mut js::Module,
        block_id: js::LocalNodeId<js::Block>,
    ) {
        let block = module.tree.get(block_id).clone();

        // statements
        for statement_id in block.statements {
            Self::elide_undefined_returns_in_statement(module, statement_id);
        }
    }

    /// Rewrite `return undefined` recursively within one statement tree.
    fn elide_undefined_returns_in_statement(
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
    ) {
        let statement = module.tree.get(statement_id).clone();

        match statement {
            js::Statement::Return { value: Some(value) } => {
                if !Self::is_undefined_expression(module, value) {
                    return;
                }

                let statement = module.tree.get_mut(statement_id);
                let js::Statement::Return { value } = statement else {
                    unreachable!("return rewrite only mutates return statements");
                };
                *value = None;
            }

            js::Statement::Block { block } => {
                Self::elide_undefined_returns_in_block(module, block);
            }

            js::Statement::Labelled { body, .. } => {
                Self::elide_undefined_returns_in_statement(module, body);
            }

            js::Statement::If {
                then_block,
                else_block,
                ..
            } => {
                Self::elide_undefined_returns_in_block(module, then_block);

                if let Some(else_block) = else_block {
                    Self::elide_undefined_returns_in_block(module, else_block);
                }
            }

            js::Statement::While { body, .. }
            | js::Statement::DoWhile { body, .. }
            | js::Statement::For { body, .. }
            | js::Statement::ForIn { body, .. }
            | js::Statement::ForOf { body, .. } => {
                Self::elide_undefined_returns_in_block(module, body);
            }

            js::Statement::Switch { cases, .. } => {
                for switch_case_id in cases {
                    let switch_case = module.tree.get(switch_case_id);
                    Self::elide_undefined_returns_in_block(module, switch_case.body);
                }
            }

            js::Statement::Try {
                try_block,
                catch_clause,
                finally_block,
                ..
            } => {
                Self::elide_undefined_returns_in_block(module, try_block);

                if let Some(catch_clause) = catch_clause {
                    let catch_clause = module.tree.get(catch_clause);
                    Self::elide_undefined_returns_in_block(module, catch_clause.body);
                }

                if let Some(finally_block) = finally_block {
                    Self::elide_undefined_returns_in_block(module, finally_block);
                }
            }

            js::Statement::Import { .. }
            | js::Statement::Export { .. }
            | js::Statement::ExportValue { .. }
            | js::Statement::Declaration { .. }
            | js::Statement::Let { .. }
            | js::Statement::Var { .. }
            | js::Statement::Using { .. }
            | js::Statement::Assign { .. }
            | js::Statement::Expression { .. }
            | js::Statement::Throw { .. }
            | js::Statement::Continue { .. }
            | js::Statement::Break { .. }
            | js::Statement::Return { value: None }
            | js::Statement::Debugger => {}
        }
    }

    /// Rewrite one object field into shorthand form when the final binding name matches.
    pub(super) fn use_object_shorthand_fields(&mut self) {
        if !self.can_use_object_shorthand() {
            return;
        }

        for property_id in self.module.tree.get_nodes::<js::Property>() {
            let property = self.module.tree.get(property_id).clone();

            let js::Property::Field {
                modifiers,
                key: js::Key::Name(js::Name::Identifier(key)),
                value,
                is_shorthand,
            } = property
            else {
                continue;
            };

            if modifiers.is_some() || is_shorthand {
                continue;
            }

            if !Self::is_matching_identifier_reference(self.module, value, key) {
                continue;
            }

            let property = self.module.tree.get_mut(property_id);
            let js::Property::Field { is_shorthand, .. } = property else {
                continue;
            };
            *is_shorthand = true;
        }
    }
}

/// Access declaration binding fields.
pub(super) trait DeclarationBindingAccess {
    /// Return the declaration name.
    fn name(&self) -> Option<js::Name>;

    /// Return the mutable declaration name.
    fn name_mut(&mut self) -> Option<&mut Option<js::Name>>;

    /// Return the declaration export kind.
    fn export(&self) -> Option<js::DependencyBinding>;
}

impl DeclarationBindingAccess for js::Declaration {
    fn name(&self) -> Option<js::Name> {
        match self {
            js::Declaration::Global(_) => None,
            js::Declaration::Namespace(declaration) => declaration.name,
            js::Declaration::Type(declaration) => declaration.name,
            js::Declaration::Class(declaration) => declaration.name,
            js::Declaration::Interface(declaration) => declaration.name,
            js::Declaration::Enum(declaration) => declaration.name,
            js::Declaration::Function(declaration) => declaration.name,
        }
    }

    fn name_mut(&mut self) -> Option<&mut Option<js::Name>> {
        match self {
            js::Declaration::Global(_) => None,
            js::Declaration::Namespace(declaration) => Some(&mut declaration.name),
            js::Declaration::Type(declaration) => Some(&mut declaration.name),
            js::Declaration::Class(declaration) => Some(&mut declaration.name),
            js::Declaration::Interface(declaration) => Some(&mut declaration.name),
            js::Declaration::Enum(declaration) => Some(&mut declaration.name),
            js::Declaration::Function(declaration) => Some(&mut declaration.name),
        }
    }

    fn export(&self) -> Option<js::DependencyBinding> {
        match self {
            js::Declaration::Global(_) => None,
            js::Declaration::Namespace(declaration) => declaration.export,
            js::Declaration::Type(declaration) => declaration.export,
            js::Declaration::Class(declaration) => declaration.export,
            js::Declaration::Interface(declaration) => declaration.export,
            js::Declaration::Enum(declaration) => declaration.export,
            js::Declaration::Function(declaration) => declaration.export,
        }
    }
}
