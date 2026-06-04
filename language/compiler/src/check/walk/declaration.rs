use destack_dir as dir;
use destack_source::ModuleId;

use super::property::MemberReceiverContext;

use crate::check::{Origin, ReceiverCapture, TypeOperand, TypeTerm, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
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
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
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
        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // get declaration symbol
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        let value_expression = self.tree.get(declaration.value);
        let is_intrinsic = matches!(value_expression, dir::TypeExpression::Intrinsic);

        // bind intrinsic nominal declarations
        if is_intrinsic {
            self.bind_node_type(declaration.value, TypeTerm::Intrinsic)?;

            if declaration.is_nominal {
                self.allocate_symbol_type_variable(symbol)?;
            } else {
                self.check
                    .report_invalid_intrinsic_type(self.module, declaration.value.into_any());
            }
            return Ok(());
        }

        // walk type expression itself
        self.walk_type_expression(declaration.value, value_expression)?;

        // bind transparent aliases to their right-hand side
        if !declaration.is_nominal {
            let value = self.allocate_node_type_operand(declaration.value)?;
            let condition = self.active_static_guard();

            self.check.push_generic_induction_root(symbol, value);
            self.bind_symbol_type_operand(symbol, value, condition)?;
        }
        // bind nominal declarations to a fresh symbol type
        else {
            let backing = self.allocate_node_type_operand(declaration.value)?;

            self.check.push_generic_induction_root(symbol, backing);
            self.allocate_symbol_type_variable(symbol)?;
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
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any());
        let receiver = self.allocate_declaration_member_receiver(self.module, symbol)?;

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // walk implemented contracts
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(*member, self.tree.get(*member), receiver)?;
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
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any());
        let receiver = self.allocate_declaration_member_receiver(self.module, symbol)?;

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // walk superclass type
        if let Some(extends_type) = declaration.extends_type {
            self.walk_type_expression(extends_type, self.tree.get(extends_type))?;
        }

        // walk implemented contracts
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(*member, self.tree.get(*member), receiver)?;
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
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any());
        let receiver = self.allocate_declaration_member_receiver(self.module, symbol)?;

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // walk implemented contracts and variants
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
        }

        // walk fields
        for field in &declaration.fields {
            self.walk_enum_field(*field, self.tree.get(*field))?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_member(*member, self.tree.get(*member), receiver)?;
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
        if let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        {
            self.allocate_symbol_type_variable(symbol)?;
        }

        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // walk inherited contracts
        for extends_type in &declaration.extends_types {
            self.walk_type_expression(*extends_type, self.tree.get(*extends_type))?;
        }

        // walk members
        for member in &declaration.members {
            self.walk_type_member(*member, self.tree.get(*member))?;
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
        _id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ExtensionDeclaration,
    ) -> CompilerResult<()> {
        // walk generic header
        for parameter in &declaration.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause, self.tree.get(*where_clause))?;
        }

        // walk target
        self.walk_type_expression(
            declaration.target_type,
            self.tree.get(declaration.target_type),
        )?;

        // walk implements
        for implemented_type in &declaration.implements_types {
            self.walk_type_expression(*implemented_type, self.tree.get(*implemented_type))?;
        }

        // walk members
        let receiver = Some(MemberReceiverContext {
            module: self.module,
            owner: None,
            ty: self.allocate_node_type_operand(declaration.target_type)?,
        });
        for member in &declaration.members {
            self.walk_member(*member, self.tree.get(*member), receiver)?;
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
            self.walk_function_signature(&declaration.signature)?;

            let result = self.allocate_function_result_operand(
                self.module,
                id.into_any(),
                &declaration.signature,
                declaration.body,
            )?;

            // commit function symbol type
            let term = self.lower_function_signature_term(
                &declaration.signature,
                None,
                result.map(Into::into),
            )?;
            let condition = self.active_static_guard();

            let operand = self
                .check
                .inference
                .push_term(TypeTerm::Function(term))
                .into();

            self.check.push_generic_induction_root(symbol, operand);
            self.bind_symbol_type_operand(symbol, operand, condition)?;

            // walk body after its result operand exists
            if let (Some(body), Some(result)) = (declaration.body, result) {
                let receiver = declaration
                    .signature
                    .this_parameter
                    .map(|parameter| self.bind_this_parameter_receiver(parameter, None))
                    .transpose()?;

                self.walk_function_body(symbol, &declaration.signature, body, result, receiver)?;
            }
        } else {
            // still validate local signature syntax
            self.walk_function_signature(&declaration.signature)?;
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
        let Some(_guard) = self.enter_static_guard_for(id.into_any(), None)? else {
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
                    let value = self.allocate_node_type_operand(tail)?;

                    self.constrain_return_value(tail.into_any(), value);
                } else {
                    self.constrain_void_return(body.into_any());
                }
            }
            // expression
            _ => {
                let value = self.allocate_node_type_operand(body)?;

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
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<()> {
        // walk generic parameters
        for parameter in &signature.generic_parameters {
            self.walk_generic_parameter(*parameter, self.tree.get(*parameter))?;
        }

        // walk receiver and runtime parameters
        let is_annotation_required = signature.form != dir::FunctionForm::Lambda;
        if let Some(parameter) = signature.this_parameter {
            self.walk_parameter(parameter, self.tree.get(parameter), is_annotation_required)?;
        }
        for parameter in &signature.parameters {
            self.walk_parameter(
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

    /// Bind the receiver introduced by one `this` parameter.
    ///
    /// Example:
    /// ```ds
    /// function method(this: Box): number { 1 }
    /// ```
    pub(in crate::check) fn bind_this_parameter_receiver(
        &mut self,
        parameter: dir::LocalNodeId<dir::Parameter>,
        owner: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<ReceiverCapture> {
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

        // bind the receiver to its parameter type
        let Some(ty) = self.parameter_type(parameter)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "this parameter {:?} has no type operand",
                    parameter.into_global(self.module)
                ),
            });
        };
        if let Some(owner) = owner {
            self.check.push_generic_induction_root(owner, ty);
        }

        Ok(ReceiverCapture { symbol, owner, ty })
    }

    /// Return one public function result operand.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    pub(in crate::check) fn allocate_function_result_operand(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<TypeOperand>> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            return Ok(Some(self.allocate_node_type_operand(return_type)?));
        }

        // skip ambient signatures
        let Some(_) = body else {
            return Ok(None);
        };

        // allocate the inferred result operand
        let origin = Origin::Node(source.into_global(module));
        let variable = self.check.create_type_variable(module, origin);

        Ok(Some(TypeOperand::Variable(variable)))
    }

    /// Return one nominal declaration member receiver context.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    fn allocate_declaration_member_receiver(
        &mut self,
        module: ModuleId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<MemberReceiverContext>> {
        let Some(symbol) = symbol else {
            return Ok(None);
        };
        let ty = self.allocate_symbol_type_variable(symbol)?.into();

        Ok(Some(MemberReceiverContext {
            module,
            owner: Some(symbol),
            ty,
        }))
    }
}
