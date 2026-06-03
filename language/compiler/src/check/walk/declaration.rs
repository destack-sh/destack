use destack_dir as dir;
use destack_source::ModuleId;

use super::property::MemberReceiverContext;

use crate::check::{Condition, Origin, ReceiverCapture, TypeOperand, TypeTerm, WalkState};

impl WalkState<'_, '_> {
    /// Walk one declaration.
    ///
    /// Example:
    /// ```ds
    /// struct User { name: string }
    /// ```
    pub(in crate::check) fn walk_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                self.walk_global_declaration(tree, declaration)
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                self.walk_module_declaration(tree, declaration)
            }
            // type X = T
            dir::Declaration::Type(declaration) => {
                self.walk_type_declaration(tree, id, declaration)
            }
            // struct S { ... }
            dir::Declaration::Struct(declaration) => {
                self.walk_struct_declaration(tree, id, declaration);
            }
            // class C { ... }
            dir::Declaration::Class(declaration) => {
                self.walk_class_declaration(tree, id, declaration)
            }
            // enum E { ... }
            dir::Declaration::Enum(declaration) => {
                self.walk_enum_declaration(tree, id, declaration)
            }
            // interface I { ... }
            dir::Declaration::Interface(declaration) => {
                self.walk_interface_declaration(tree, id, declaration);
            }
            // extension T { ... }
            dir::Declaration::Extension(declaration) => {
                self.walk_extension_declaration(tree, id, declaration);
            }
            // function f() {}
            dir::Declaration::Function(declaration) => {
                self.walk_function_item_declaration(tree, id, declaration);
            }
        }

        self.pop_static_guard();
    }

    /// Walk one global declaration.
    ///
    /// Example:
    /// ```ds
    /// global { console.log("ready") }
    /// ```
    fn walk_global_declaration(&mut self, tree: &dir::Tree, declaration: &dir::GlobalDeclaration) {
        for expression in &declaration.expressions {
            self.walk_expression(tree, *expression, tree.get(*expression));
        }
    }

    /// Walk one module declaration.
    ///
    /// Example:
    /// ```ds
    /// module app { export const value = 1 }
    /// ```
    fn walk_module_declaration(&mut self, tree: &dir::Tree, declaration: &dir::ModuleDeclaration) {
        for expression in &declaration.expressions {
            self.walk_expression(tree, *expression, tree.get(*expression));
        }
    }

    /// Walk one type declaration.
    ///
    /// Example:
    /// ```ds
    /// type Id<T> = T
    /// ```
    fn walk_type_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::TypeDeclaration,
    ) {
        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }

        // get declaration symbol
        let Some(symbol) = self
            .check
            .module(tree.module_id)
            .declaration_symbol(id.into_any())
        else {
            return;
        };
        let value_expression = tree.get(declaration.value);
        let is_intrinsic = matches!(value_expression, dir::TypeExpression::Intrinsic);
        let is_language_item =
            is_intrinsic && self.check.environment.language.item(symbol).is_some();

        // bind (legal) intrinsic declarations
        if is_intrinsic && (declaration.is_nominal || is_language_item) {
            let symbol_type = TypeTerm::Reference {
                origin: Origin::Symbol(symbol),
                symbol,
                arguments: Vec::new(),
            };

            self.bind_node_type(declaration.value, TypeTerm::Intrinsic);
            self.bind_symbol_type(symbol, symbol_type, Condition::Always);

            return;
        }

        // walk type expression itself
        self.walk_type_expression(tree, declaration.value, value_expression);

        // bind transparent aliases to their right-hand side
        if !declaration.is_nominal {
            let value = self.allocate_node_type_operand(declaration.value);
            let condition = self.active_static_guard();

            self.check.push_generic_induction_root(symbol, value);
            self.bind_symbol_type_operand(symbol, value, condition);
        }
        // bind nominal declarations to a fresh symbol type
        else {
            let backing = self.allocate_node_type_operand(declaration.value);

            self.check.push_generic_induction_root(symbol, backing);
            self.allocate_symbol_type_variable(symbol);
        }
    }

    /// Walk one struct declaration.
    ///
    /// Example:
    /// ```ds
    /// struct Point { x: number, y: number }
    /// ```
    fn walk_struct_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::StructDeclaration,
    ) {
        let symbol = self
            .check
            .module(tree.module_id)
            .declaration_symbol(id.into_any());
        let receiver = self.declaration_member_receiver(tree.module_id, symbol);

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }

        // walk implemented contracts
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(tree, *member, tree.get(*member), receiver);
        }
    }

    /// Walk one class declaration.
    ///
    /// Example:
    /// ```ds
    /// class User extends Entity { name: string }
    /// ```
    fn walk_class_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ClassDeclaration,
    ) {
        let symbol = self
            .check
            .module(tree.module_id)
            .declaration_symbol(id.into_any());
        let receiver = self.declaration_member_receiver(tree.module_id, symbol);

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }

        // walk superclass type
        if let Some(extends_type) = declaration.extends_type {
            self.walk_type_expression(tree, extends_type, tree.get(extends_type));
        }

        // walk implemented contracts
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(tree, *member, tree.get(*member), receiver);
        }
    }

    /// Walk one enum declaration.
    ///
    /// Example:
    /// ```ds
    /// enum Option<T> { Some(T), None }
    /// ```
    fn walk_enum_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::EnumDeclaration,
    ) {
        let symbol = self
            .check
            .module(tree.module_id)
            .declaration_symbol(id.into_any());
        let receiver = self.declaration_member_receiver(tree.module_id, symbol);

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }

        // walk implemented contracts and variants
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
        }

        // walk fields
        for field in &declaration.fields {
            self.walk_enum_field(tree, *field, tree.get(*field));
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(tree, *member, tree.get(*member), receiver);
        }
    }

    /// Walk one interface declaration.
    ///
    /// Example:
    /// ```ds
    /// interface Reader { read(): string }
    /// ```
    fn walk_interface_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::InterfaceDeclaration,
    ) {
        if let Some(symbol) = self
            .check
            .module(tree.module_id)
            .declaration_symbol(id.into_any())
        {
            self.allocate_symbol_type_variable(symbol);
        }

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }

        // walk inherited contracts
        for extends_type in &declaration.extends_types {
            self.walk_type_expression(tree, *extends_type, tree.get(*extends_type));
        }

        // walk members
        for member in &declaration.members {
            self.walk_type_member(tree, *member, tree.get(*member));
        }
    }

    /// Walk one extension declaration.
    ///
    /// Example:
    /// ```ds
    /// extension string { len(): number }
    /// ```
    fn walk_extension_declaration(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ExtensionDeclaration,
    ) {
        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }

        // walk target
        self.walk_type_expression(
            tree,
            declaration.target_type,
            tree.get(declaration.target_type),
        );

        // walk implements
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
        }

        // walk members
        let receiver = Some(MemberReceiverContext {
            module: tree.module_id,
            owner: None,
            ty: self.allocate_node_type_operand(declaration.target_type),
        });
        for member in &declaration.members {
            self.walk_member(tree, *member, tree.get(*member), receiver);
        }
    }

    /// Walk one function declaration.
    ///
    /// Example:
    /// ```ds
    /// function id<T>(value: T): T { value }
    /// ```
    fn walk_function_item_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
    ) {
        if let Some(symbol) = self
            .check
            .module(tree.module_id)
            .declaration_symbol(id.into_any())
        {
            // walk signature before reading its term inputs
            self.walk_function_signature(tree, &declaration.signature);

            let result = self.function_result_operand(
                tree.module_id,
                id.into_any(),
                &declaration.signature,
                declaration.body,
            );

            // commit function symbol type
            let term = self.lower_function_signature_term(
                &declaration.signature,
                None,
                result.map(Into::into),
                tree,
            );
            let condition = self.active_static_guard();

            let operand = self
                .check
                .inference
                .push_term(TypeTerm::Function(term))
                .into();

            self.check.push_generic_induction_root(symbol, operand);
            self.bind_symbol_type_operand(symbol, operand, condition);

            // walk body after its result operand exists
            if let (Some(body), Some(result)) = (declaration.body, result) {
                let receiver =
                    self.bind_explicit_receiver(declaration.signature.this_parameter, None, tree);

                self.walk_function_body(
                    tree,
                    symbol,
                    &declaration.signature,
                    body,
                    result,
                    receiver,
                );
            }
        } else {
            // still validate local signature syntax
            self.walk_function_signature(tree, &declaration.signature);
        }
    }

    /// Walk one enum field.
    ///
    /// Example:
    /// ```ds
    /// Some(value)
    /// ```
    pub(in crate::check) fn walk_enum_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }
        if let Some(value) = enum_field.value {
            // check enum value in declaration context
            let before_value = self.fork_flow();

            self.walk_expression(tree, value, tree.get(value));
            self.restore_flow(before_value);
        }

        self.pop_static_guard();
    }

    /// Constrain the value produced by a function body that reaches its end.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    pub(in crate::check) fn constrain_function_completion_return(
        &mut self,
        tree: &dir::Tree,
        body: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(body) {
            // { ... }
            dir::Expression::Block(block) => {
                let block = tree.get(*block);
                if block.context == dir::BlockContext::Expression
                    && let Some(tail) = block.tail_expression
                {
                    let value = self.allocate_node_type_operand(tail);

                    self.constrain_return_value(tail.into_any(), value);
                } else {
                    self.constrain_void_return(body.into_any());
                }
            }
            // expression
            _ => {
                let value = self.allocate_node_type_operand(body);

                self.constrain_return_value(body.into_any(), value);
            }
        };
    }

    /// Walk one function signature without entering the function body.
    ///
    /// Example:
    /// ```ds
    /// <T>(value: T): T where T: Copy
    /// ```
    pub(in crate::check) fn walk_function_signature(
        &mut self,
        tree: &dir::Tree,
        signature: &dir::FunctionSignature,
    ) {
        // walk generic parameters
        for parameter in &signature.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }

        // walk receiver and runtime parameters
        let is_annotation_required = signature.form != dir::FunctionForm::Lambda;
        if let Some(parameter) = signature.this_parameter {
            self.walk_parameter(tree, parameter, tree.get(parameter), is_annotation_required);
        }
        for parameter in &signature.parameters {
            self.walk_parameter(
                tree,
                *parameter,
                tree.get(*parameter),
                is_annotation_required,
            );
        }

        // walk return type
        if let Some(return_type) = signature.return_type {
            self.walk_type_expression(tree, return_type, tree.get(return_type));
        }

        // walk where clauses
        for where_clause in &signature.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }
    }

    /// Bind the explicit receiver introduced by one function signature.
    ///
    /// Example:
    /// ```ds
    /// function method(this: Box): number { 1 }
    /// ```
    pub(in crate::check) fn bind_explicit_receiver(
        &mut self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        owner: Option<dir::GlobalSymbolId>,
        tree: &dir::Tree,
    ) -> Option<ReceiverCapture> {
        // no `this` parameter means no lexical receiver
        let parameter = this_parameter?;
        let Some(symbol) = self
            .check
            .module(tree.module_id)
            .declaration_symbol(parameter.into_any())
        else {
            return None;
        };

        // bind the receiver to its parameter type
        let ty = self.parameter_type(parameter, tree)?;
        if let Some(owner) = owner {
            self.check.push_generic_induction_root(owner, ty);
        }

        Some(ReceiverCapture { symbol, owner, ty })
    }

    /// Return one public function result operand.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    pub(in crate::check) fn function_result_operand(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Option<TypeOperand> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            return Some(self.allocate_node_type_operand(return_type));
        }

        // skip ambient signatures
        let Some(_) = body else {
            return None;
        };

        // allocate the inferred result operand
        let origin = Origin::Node(source.into_global(module));
        let variable = self.check.create_type_variable(module, origin);

        Some(TypeOperand::Variable(variable))
    }

    /// Return one nominal declaration member receiver context.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    fn declaration_member_receiver(
        &mut self,
        module: ModuleId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> Option<MemberReceiverContext> {
        let symbol = symbol?;
        let ty = self.allocate_symbol_type_variable(symbol).into();

        Some(MemberReceiverContext {
            module,
            owner: Some(symbol),
            ty,
        })
    }
}
