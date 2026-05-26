use destack_dir as dir;
use dir::NodeVisitor as _;

use super::property::MemberReceiverContext;

use crate::check::{CheckModuleState, ReceiverCapture, TypeTerm, VariableId};

impl CheckModuleState {
    /// Walk one declaration.
    pub(in crate::check) fn walk_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Declaration, id.id);

        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                for expression in &declaration.expressions {
                    self.walk_expression(tree, *expression, tree.get(*expression));
                }
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.walk_expression(tree, *expression, tree.get(*expression));
                }
            }
            // type X = T
            dir::Declaration::Type(declaration) => {
                // define the declaration symbol output
                if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                    if !declaration.is_nominal {
                        let variable = self.intern_symbol_type_variable(symbol);
                        let value = self.intern_local_type_variable(declaration.value);

                        self.define_type(variable, TypeTerm::Variable(value));
                    }
                }

                // walk generic header
                for parameter in &declaration.generic_parameters {
                    self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
                }
                for where_clause in &declaration.where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk aliased type expression
                self.walk_type_expression(tree, declaration.value, tree.get(declaration.value));

                if declaration.is_nominal
                    && let Some(symbol) = self.declaration_symbol(id.into_any())
                {
                    self.intern_symbol_type_variable(symbol);
                }
            }
            // struct S { ... }
            dir::Declaration::Struct(declaration) => {
                let symbol = self.declaration_symbol(id.into_any());

                // walk generic header
                for parameter in &declaration.generic_parameters {
                    self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
                }
                for where_clause in &declaration.where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk implemented types
                for implemented_type in &declaration.implements_types {
                    self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
                }

                // walk members with a nominal receiver
                let receiver = symbol.map(|symbol| MemberReceiverContext {
                    owner: Some(symbol),
                    ty: self.intern_symbol_type_variable(symbol),
                });

                for member in &declaration.members {
                    self.walk_member(tree, *member, tree.get(*member), receiver);
                }

                if let Some(symbol) = symbol {
                    self.intern_symbol_type_variable(symbol);
                }
            }
            // class C { ... }
            dir::Declaration::Class(declaration) => {
                let symbol = self.declaration_symbol(id.into_any());

                // walk generic header
                for parameter in &declaration.generic_parameters {
                    self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
                }
                for where_clause in &declaration.where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk superclass and implemented types
                if let Some(extends_expression) = declaration.extends_expression {
                    // check superclass expression in declaration context
                    let before_extends = self.checkpoint_flow();

                    self.walk_expression(tree, extends_expression, tree.get(extends_expression));
                    self.restore_flow(before_extends);
                }
                for implemented_type in &declaration.implements_types {
                    self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
                }

                // walk members with a nominal receiver
                let receiver = symbol.map(|symbol| MemberReceiverContext {
                    owner: Some(symbol),
                    ty: self.intern_symbol_type_variable(symbol),
                });

                for member in &declaration.members {
                    self.walk_member(tree, *member, tree.get(*member), receiver);
                }

                if let Some(symbol) = symbol {
                    self.intern_symbol_type_variable(symbol);
                }
            }
            // enum E { ... }
            dir::Declaration::Enum(declaration) => {
                let symbol = self.declaration_symbol(id.into_any());

                // walk generic header
                for parameter in &declaration.generic_parameters {
                    self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
                }
                for where_clause in &declaration.where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk implemented types and variants
                for implemented_type in &declaration.implements_types {
                    self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
                }
                for field in &declaration.fields {
                    self.walk_enum_field(tree, *field, tree.get(*field));
                }

                // walk members with a nominal receiver
                let receiver = symbol.map(|symbol| MemberReceiverContext {
                    owner: Some(symbol),
                    ty: self.intern_symbol_type_variable(symbol),
                });

                for member in &declaration.members {
                    self.walk_member(tree, *member, tree.get(*member), receiver);
                }

                if let Some(symbol) = symbol {
                    self.intern_symbol_type_variable(symbol);
                }
            }
            // interface I { ... }
            dir::Declaration::Interface(declaration) => {
                // define the interface symbol output
                if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                    if !declaration.is_nominal {
                        self.intern_symbol_type_variable(symbol);
                    }
                }

                // walk generic header
                for parameter in &declaration.generic_parameters {
                    self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
                }
                for where_clause in &declaration.where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk inherited contracts and members
                for heritage in &declaration.extends {
                    // check inherited contract expression in declaration context
                    let before_heritage = self.checkpoint_flow();

                    self.walk_expression(tree, heritage.expression, tree.get(heritage.expression));
                    self.restore_flow(before_heritage);

                    for argument in &heritage.generic_arguments {
                        self.walk_generic_argument(tree, *argument, tree.get(*argument));
                    }
                }
                for member in &declaration.members {
                    self.walk_type_member(tree, *member, tree.get(*member));
                }

                if declaration.is_nominal
                    && let Some(symbol) = self.declaration_symbol(id.into_any())
                {
                    self.intern_symbol_type_variable(symbol);
                }
            }
            // extension T { ... }
            dir::Declaration::Extension(declaration) => {
                if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                    self.intern_symbol_type_variable(symbol);
                }

                // walk generic header
                for parameter in &declaration.generic_parameters {
                    self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
                }
                for where_clause in &declaration.where_clauses {
                    self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
                }

                // walk extended target and implemented contracts
                self.walk_type_expression(
                    tree,
                    declaration.target_type,
                    tree.get(declaration.target_type),
                );
                for implemented_type in &declaration.implements_types {
                    self.walk_type_expression(tree, *implemented_type, tree.get(*implemented_type));
                }

                // extension members receive the target type as `this`
                let receiver = Some(MemberReceiverContext {
                    owner: None,
                    ty: self.intern_local_type_variable(declaration.target_type),
                });

                for member in &declaration.members {
                    self.walk_member(tree, *member, tree.get(*member), receiver);
                }
            }
            // function f() {}
            dir::Declaration::Function(declaration) => {
                let return_type = self.intern_function_return_type_variable(id, declaration, tree);

                // define the callable symbol output
                if let Some(symbol) = self.declaration_symbol(id.into_any()) {
                    let variable = self.intern_symbol_type_variable(symbol);
                    let term = self.build_function_signature_term(
                        &declaration.signature,
                        return_type,
                        tree,
                    );

                    self.define_type(variable, TypeTerm::Function(term));

                    // walk signature and body in the function flow frame
                    self.walk_function_declaration(
                        tree,
                        symbol,
                        &declaration.signature,
                        declaration.body,
                        return_type,
                    );
                } else {
                    self.walk_function_signature(tree, &declaration.signature);
                }
            }
        };
    }

    /// Walk one enum field.
    pub(in crate::check) fn walk_enum_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::EnumField, id.id);

        if let Some(value) = enum_field.value {
            // check enum value in declaration context
            let before_value = self.checkpoint_flow();

            self.walk_expression(tree, value, tree.get(value));
            self.restore_flow(before_value);
        }
    }

    /// Walk one function signature and body with a flow frame.
    fn walk_function_declaration(
        &mut self,
        tree: &dir::Tree,
        symbol: dir::GlobalSymbolId,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        return_type: Option<VariableId>,
    ) {
        self.walk_function_signature(tree, signature);

        let Some(body) = body else {
            return;
        };
        let Some(return_type) = return_type else {
            return;
        };
        let receiver = self.function_receiver(signature.this_parameter, None, tree);

        self.walk_function_body(tree, symbol, signature, body, return_type, receiver);
    }

    /// Constrain the value produced by a function body that reaches its end.
    pub(in crate::check) fn constrain_function_fallthrough_return(
        &mut self,
        tree: &dir::Tree,
        body: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(body) {
            // { ... }
            dir::Expression::Block(block) => {
                let block = tree.get(*block);
                if block.context == dir::BlockContext::Expression {
                    let value = self.intern_local_type_variable(body);

                    self.constrain_return_value(body.into_any(), value);
                } else {
                    self.constrain_void_return(body.into_any());
                }
            }
            // expression
            _ => {
                let value = self.intern_local_type_variable(body);

                self.constrain_return_value(body.into_any(), value);
            }
        };
    }

    /// Walk one function signature without entering the function body.
    pub(in crate::check) fn walk_function_signature(
        &mut self,
        tree: &dir::Tree,
        signature: &dir::FunctionSignature,
    ) {
        // generic parameters
        for parameter in &signature.generic_parameters {
            self.walk_generic_parameter(tree, *parameter, tree.get(*parameter));
        }

        // parameters
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

        // return type
        if let Some(return_type) = signature.return_type {
            self.walk_type_expression(tree, return_type, tree.get(return_type));
        }

        // where clauses
        for where_clause in &signature.where_clauses {
            self.walk_where_clause(tree, *where_clause, tree.get(*where_clause));
        }
    }

    /// Return the declared or inferred return type variable for one function.
    fn intern_function_return_type_variable(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
        tree: &dir::Tree,
    ) -> Option<VariableId> {
        // use explicit return annotations
        if let Some(return_type) = declaration.signature.return_type {
            return Some(self.intern_local_type_variable(return_type));
        }

        // use concise expression bodies directly
        if declaration.signature.asynchrony == dir::Asynchrony::Sync
            && !declaration.signature.is_generator
            && let Some(body) = declaration.body
            && !matches!(tree.get(body), dir::Expression::Block(_))
        {
            return Some(self.intern_local_type_variable(body));
        }

        // declarations without bodies do not infer returns
        declaration.body?;

        // otherwise use the declaration node as the inferred output
        Some(self.intern_local_type_variable(id))
    }

    /// Return the lexical receiver introduced by one function signature.
    pub(in crate::check) fn function_receiver(
        &mut self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        owner: Option<dir::GlobalSymbolId>,
        tree: &dir::Tree,
    ) -> Option<ReceiverCapture> {
        // no `this` parameter means no lexical receiver
        let parameter = this_parameter?;
        let Some(symbol) = self.declaration_symbol(parameter.into_any()) else {
            return None;
        };

        // bind the receiver to its parameter type
        let ty = self.intern_parameter_type_variable(parameter, tree)?;

        Some(ReceiverCapture { symbol, owner, ty })
    }

    /// Return the declared or inferred return type for one function signature.
    pub(in crate::check) fn intern_signature_return_type_variable(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Option<VariableId> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            return Some(self.intern_local_type_variable(return_type));
        }

        // signatures without bodies do not infer returns
        body?;

        // otherwise use the declaration node as the inferred output
        Some(self.intern_node_type_variable(source.into_global(self.input.module_id)))
    }
}
