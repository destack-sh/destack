use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    GenericInductionDeclaration, GenericTemplateId, Origin, Receiver, ReceiverBinding, TypeOperand,
    TypeTerm, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Declare generic headers introduced by one expression.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn declare_expression_header(
        &mut self,
        _id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        if let dir::Expression::Declaration(declaration) = expression {
            self.declare_declaration_header(*declaration, self.tree.get(*declaration))?;
        }

        Ok(())
    }

    /// Declare generic headers introduced by one declaration.
    ///
    /// Example:
    /// ```ds
    /// function value<T>(input: T): T { input }
    /// ```
    fn declare_declaration_header(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                for expression in &declaration.expressions {
                    self.declare_expression_header(*expression, self.tree.get(*expression))?;
                }
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.declare_expression_header(*expression, self.tree.get(*expression))?;
                }
            }
            // type X<T> = T
            dir::Declaration::Type(declaration) => {
                self.declare_declaration_generic_header(id, &declaration.generic_parameters)?;
            }
            // struct S<T> {}
            dir::Declaration::Struct(declaration) => {
                self.declare_declaration_generic_header(id, &declaration.generic_parameters)?;
            }
            // class C<T> {}
            dir::Declaration::Class(declaration) => {
                self.declare_declaration_generic_header(id, &declaration.generic_parameters)?;
            }
            // enum E<T> {}
            dir::Declaration::Enum(declaration) => {
                self.declare_declaration_generic_header(id, &declaration.generic_parameters)?;
            }
            // interface I<T> {}
            dir::Declaration::Interface(declaration) => {
                self.declare_declaration_generic_header(id, &declaration.generic_parameters)?;
            }
            // extension T<U> {}
            dir::Declaration::Extension(declaration) => {
                self.declare_declaration_generic_header(id, &declaration.generic_parameters)?;
            }
            // function f<T>() {}
            dir::Declaration::Function(declaration) => {
                self.declare_function_generic_header(id, &declaration.signature)?;
            }
        }

        Ok(())
    }

    /// Declare the generic header owned by one symbol declaration.
    fn declare_declaration_generic_header(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        let source = id.into_global_any(self.module);
        self.declare_generic_template(source, None, Some(symbol), parameters)?;

        Ok(())
    }

    /// Declare the generic header owned by one function declaration.
    fn declare_function_generic_header(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        let source = id.into_global_any(self.module);
        let Some(template) = self.signature_template(source, None, Some(symbol), signature)? else {
            return Ok(());
        };

        // declare explicit signature parameters before walking bodies
        for parameter in &signature.generic_parameters {
            self.declare_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(())
    }

    /// Walk one declaration.
    ///
    /// Example:
    /// ```ds
    /// struct User { name: string }
    /// ```
    pub(in crate::check) fn walk_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                self.walk_global_declaration(declaration)?;
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                self.walk_module_declaration(declaration)?;
            }
            // type X = T
            dir::Declaration::Type(declaration) => {
                self.walk_type_declaration(id, declaration)?;
            }
            // struct S { ... }
            dir::Declaration::Struct(declaration) => {
                self.walk_struct_declaration(id, declaration)?;
            }
            // class C { ... }
            dir::Declaration::Class(declaration) => {
                self.walk_class_declaration(id, declaration)?;
            }
            // enum E { ... }
            dir::Declaration::Enum(declaration) => {
                self.walk_enum_declaration(id, declaration)?;
            }
            // interface I { ... }
            dir::Declaration::Interface(declaration) => {
                self.walk_interface_declaration(id, declaration)?;
            }
            // extension T { ... }
            dir::Declaration::Extension(declaration) => {
                self.walk_extension_declaration(id, declaration)?;
            }
            // function f() {}
            dir::Declaration::Function(declaration) => {
                self.walk_function_item_declaration(id, declaration)?;
            }
        }

        Ok(())
    }

    /// Walk one global declaration.
    ///
    /// Example:
    /// ```ds
    /// global { console.log("ready") }
    /// ```
    fn walk_global_declaration(
        &mut self,
        declaration: &dir::GlobalDeclaration,
    ) -> CompilerResult<()> {
        for expression in &declaration.expressions {
            self.walk_expression(*expression, self.tree.get(*expression))?;
        }

        Ok(())
    }

    /// Walk one module declaration.
    ///
    /// Example:
    /// ```ds
    /// module app { export const value = 1 }
    /// ```
    fn walk_module_declaration(
        &mut self,
        declaration: &dir::ModuleDeclaration,
    ) -> CompilerResult<()> {
        for expression in &declaration.expressions {
            self.walk_expression(*expression, self.tree.get(*expression))?;
        }

        Ok(())
    }

    /// Walk one type declaration.
    ///
    /// Example:
    /// ```ds
    /// type Id<T> = T
    /// ```
    fn walk_type_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::TypeDeclaration,
    ) -> CompilerResult<()> {
        // get declaration symbol
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        // walk generic header
        let source = id.into_global_any(self.module);
        let induction_declaration = GenericInductionDeclaration::new(source, None, Some(symbol));
        self.walk_generic_template(source, None, Some(symbol), &declaration.generic_parameters)?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        let value_expression = self.tree.get(declaration.value);
        let is_intrinsic = matches!(value_expression, dir::TypeExpression::Intrinsic);

        // constrain intrinsic nominal declarations
        if is_intrinsic {
            self.constrain_node_type_term(declaration.value, TypeTerm::Intrinsic)?;

            if !declaration.is_nominal {
                self.check
                    .report_invalid_intrinsic_type(self.module, declaration.value.into_any());
            }

            return Ok(());
        }

        // walk type expression itself
        self.walk_type_expression(declaration.value, value_expression)?;

        // constrain transparent aliases to their right hand side
        if !declaration.is_nominal {
            let value = self.node_type_operand(declaration.value)?;
            let condition = self.active_static_guard();
            self.record_generic_induction_site(induction_declaration, value);
            self.constrain_symbol_type(symbol, value, condition)?;
        }
        // constrain nominal declarations to a fresh symbol type
        else {
            let backing = self.node_type_operand(declaration.value)?;
            self.record_generic_induction_site(induction_declaration, backing);
        }

        Ok(())
    }

    /// Walk one struct declaration.
    ///
    /// Example:
    /// ```ds
    /// struct Point { x: number, y: number }
    /// ```
    fn walk_struct_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::StructDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        // walk generic header
        let source = id.into_global_any(self.module);
        let induction_declaration = GenericInductionDeclaration::new(source, None, Some(symbol));
        self.walk_generic_template(source, None, Some(symbol), &declaration.generic_parameters)?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }
        let receiver = self.nominal_receiver(symbol);

        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk implemented contracts
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
            self.record_type_induction_site(induction_declaration, *implemented_type)?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction_declaration),
            )?;
        }

        Ok(())
    }

    /// Walk one class declaration.
    ///
    /// Example:
    /// ```ds
    /// class User extends Entity { name: string }
    /// ```
    fn walk_class_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ClassDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        // walk generic header
        let source = id.into_global_any(self.module);
        let induction_declaration = GenericInductionDeclaration::new(source, None, Some(symbol));
        self.walk_generic_template(source, None, Some(symbol), &declaration.generic_parameters)?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }
        let receiver = self.nominal_receiver(symbol);

        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk superclass type
        if let Some(extends_type) = declaration.extends_type {
            self.walk_type_expression(extends_type, self.tree.get(extends_type))?;
            self.record_type_induction_site(induction_declaration, extends_type)?;
        }

        // walk implemented contracts
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
            self.record_type_induction_site(induction_declaration, *implemented_type)?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction_declaration),
            )?;
        }

        Ok(())
    }

    /// Walk one enum declaration.
    ///
    /// Example:
    /// ```ds
    /// enum Option<T> { Some(T), None }
    /// ```
    fn walk_enum_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::EnumDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        // walk generic header
        let source = id.into_global_any(self.module);
        let induction_declaration = GenericInductionDeclaration::new(source, None, Some(symbol));
        self.walk_generic_template(source, None, Some(symbol), &declaration.generic_parameters)?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }
        let receiver = self.nominal_receiver(symbol);

        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk implemented contracts and variants
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
            self.record_type_induction_site(induction_declaration, *implemented_type)?;
        }

        // walk fields
        for field in &declaration.fields {
            self.walk_enum_field(*field, self.tree.get(*field))?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction_declaration),
            )?;
        }

        Ok(())
    }

    /// Walk one interface declaration.
    ///
    /// Example:
    /// ```ds
    /// interface Reader { read(): string }
    /// ```
    fn walk_interface_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::InterfaceDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        // walk generic header
        let source = id.into_global_any(self.module);
        let induction_declaration = GenericInductionDeclaration::new(source, None, Some(symbol));
        self.walk_generic_template(source, None, Some(symbol), &declaration.generic_parameters)?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }
        let receiver = self.nominal_receiver(symbol);

        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk inherited contracts
        for extends_type in &declaration.extends_types {
            self.walk_type_expression(*extends_type, self.tree.get(*extends_type))?;
            self.record_type_induction_site(induction_declaration, *extends_type)?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_type_member(*member, self.tree.get(*member), Some(induction_declaration))?;
        }

        Ok(())
    }

    /// Walk one extension declaration.
    ///
    /// Example:
    /// ```ds
    /// extension string { len(): number }
    /// ```
    fn walk_extension_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ExtensionDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction_declaration = GenericInductionDeclaration::new(source, None, Some(symbol));
        self.walk_generic_template(source, None, Some(symbol), &declaration.generic_parameters)?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // walk target before exposing receiver
        self.walk_type_expression(
            declaration.target_type,
            self.tree.get(declaration.target_type),
        )?;
        self.record_type_induction_site(induction_declaration, declaration.target_type)?;

        let receiver = Some(Receiver {
            owner: Some(symbol),
            ty: self.node_type_operand(declaration.target_type)?,
        });

        let _receiver = self.enter_receiver_maybe(receiver);

        // walk implements
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
            self.record_type_induction_site(induction_declaration, *implemented_type)?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(
                *member,
                self.tree.get(*member),
                receiver,
                Some(induction_declaration),
            )?;
        }

        Ok(())
    }

    /// Walk one function declaration.
    ///
    /// Example:
    /// ```ds
    /// function id<T>(value: T): T { value }
    /// ```
    fn walk_function_item_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
    ) -> CompilerResult<()> {
        if let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        {
            // walk signature before reading its term inputs
            let source = id.into_global_any(self.module);
            let induction_declaration =
                GenericInductionDeclaration::new(source, None, Some(symbol));
            let template =
                self.signature_template(source, None, Some(symbol), &declaration.signature)?;
            self.walk_function_signature(template, &declaration.signature)?;

            let result = self.function_result_operand(
                self.module,
                id.into_any(),
                &declaration.signature,
                declaration.body,
            )?;

            // constrain function symbol type
            let term = self.function_signature_term(
                &declaration.signature,
                template,
                None,
                result.map(Into::into),
            )?;
            let condition = self.active_static_guard();

            let operand = self
                .check
                .inference
                .push_term(TypeTerm::Function(term))
                .into();

            self.record_generic_induction_site(induction_declaration, operand);
            self.constrain_symbol_type(symbol, operand, condition)?;

            // walk body after its result operand exists
            if let (Some(body), Some(result)) = (declaration.body, result) {
                let receiver = declaration
                    .signature
                    .this_parameter
                    .map(|parameter| self.this_parameter_receiver_binding(parameter, None))
                    .transpose()?;
                self.walk_function_body(symbol, &declaration.signature, body, result, receiver)?;
            }
        } else {
            // validate local signature
            self.walk_function_signature(None, &declaration.signature)?;
        }

        Ok(())
    }

    /// Walk one enum field.
    ///
    /// Example:
    /// ```ds
    /// Some(value)
    /// ```
    pub(in crate::check) fn walk_enum_field(
        &mut self,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        if let Some(value) = enum_field.value {
            // check enum value in declaration context
            let before_value = self.fork_flow();
            self.walk_expression(value, self.tree.get(value))?;
            self.restore_flow(before_value);
        }

        Ok(())
    }

    /// Constrain the value produced by a function body that reaches its end.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    pub(in crate::check) fn constrain_function_completion_return(
        &mut self,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        match self.tree.get(body) {
            // { ... }
            dir::Expression::Block(block) => {
                let block = self.tree.get(*block);
                if block.context == dir::BlockContext::Expression
                    && let Some(tail) = block.tail_expression
                {
                    let value = self.node_type_operand(tail)?;
                    self.constrain_return_value(tail.into_any(), value);
                } else {
                    self.constrain_void_return(body.into_any());
                }
            }
            // expression
            _ => {
                let value = self.node_type_operand(body)?;
                self.constrain_return_value(body.into_any(), value);
            }
        };

        Ok(())
    }

    /// Walk one function signature without entering the function body.
    ///
    /// Example:
    /// ```ds
    /// <T>(value: T): T where T: Copy
    /// ```
    pub(in crate::check) fn walk_function_signature(
        &mut self,
        template: Option<GenericTemplateId>,
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<()> {
        // walk generic parameters
        if let Some(template) = template {
            for parameter in &signature.generic_parameters {
                self.declare_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
            }
            for parameter in &signature.generic_parameters {
                self.walk_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
            }
        } else if !signature.generic_parameters.is_empty() {
            return Err(CompilerError::Internal {
                message: "generic function signature has no template".to_owned(),
            });
        }

        // walk receiver and runtime parameters
        let is_annotation_required = signature.form != dir::FunctionForm::Lambda;
        if let Some(parameter) = signature.this_parameter {
            self.walk_parameter(
                template,
                parameter,
                self.tree.get(parameter),
                is_annotation_required,
            )?;
        }
        for parameter in &signature.parameters {
            self.walk_parameter(
                template,
                *parameter,
                self.tree.get(*parameter),
                is_annotation_required,
            )?;
        }

        // walk return type
        if let Some(return_type) = signature.return_type {
            self.walk_type_expression(return_type, self.tree.get(return_type))?;
        }

        // walk where clauses
        for where_clause in &signature.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        Ok(())
    }

    /// Return the generic template declared by one function signature.
    pub(in crate::check) fn signature_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if !signature.declares_generic_template(&self.tree) {
            return Ok(None);
        }

        self.check
            .declare_generic_template(source, parent, symbol)
            .map(Some)
    }

    /// Return the receiver introduced by one `this` parameter.
    ///
    /// Example:
    /// ```ds
    /// function method(this: Box): number { 1 }
    /// ```
    pub(in crate::check) fn this_parameter_receiver_binding(
        &mut self,
        parameter: dir::LocalNodeId<dir::Parameter>,
        owner: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<ReceiverBinding> {
        // read the receiver binding
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(parameter.into_any())
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "this parameter {:?} has no declaration symbol",
                    parameter.into_global(self.module)
                ),
            });
        };

        // constrain the receiver to its parameter type
        let Some(ty) = self.parameter_type(parameter)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "this parameter {:?} has no type operand",
                    parameter.into_global(self.module)
                ),
            });
        };
        Ok(ReceiverBinding {
            symbol,
            receiver: Receiver { owner, ty },
        })
    }

    /// Return one function result operand.
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
    ) -> CompilerResult<Option<TypeOperand>> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            return Ok(Some(self.node_type_operand(return_type)?));
        }

        // skip ambient signatures
        let Some(_) = body else {
            return Ok(None);
        };

        // allocate the inferred result operand
        let origin = Origin::Node(source.into_global(module));
        let variable = self.check.push_type_variable(module, origin);

        Ok(Some(TypeOperand::Variable(variable)))
    }

    /// Return one nominal declaration receiver scope.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    fn nominal_receiver(&mut self, symbol: dir::GlobalSymbolId) -> Receiver {
        let ty = self.check.declaration_type_term(symbol);
        let ty = self.check.inference.push_term(ty).into();

        Receiver {
            owner: Some(symbol),
            ty,
        }
    }
}
