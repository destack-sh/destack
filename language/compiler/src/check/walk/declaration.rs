use destack_dir as dir;

use crate::check::{
    Decision, GenericInductionDeclaration, GenericTemplateId, HeritageObligation, LayoutObligation,
    Obligation, Origin, Receiver, ReceiverBinding, Relation, WalkState,
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
            let declaration = *declaration;
            self.declare_declaration_header(declaration, self.tree.get(declaration))?;
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                for expression in &declaration.expressions {
                    self.walk_expression(*expression, self.tree.get(*expression))?;
                }
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.walk_expression(*expression, self.tree.get(*expression))?;
                }
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
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template = self.walk_generic_template(
            source,
            None,
            Some(symbol),
            &declaration.generic_parameters,
        )?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause)?;
        }

        // intrinsic nominal declarations stay opaque
        let is_intrinsic = matches!(
            self.tree.get(declaration.value),
            dir::TypeExpression::Intrinsic
        );
        if is_intrinsic {
            if !declaration.is_nominal {
                // declared intrinsic aliases expand to builtin operations
                let mapping = self
                    .check
                    .environment
                    .language
                    .item(symbol)
                    .and_then(string_mapping_for_item);
                if let Some(mapping) = mapping
                    && let Some(template_id) = template
                    && let Some(parameter) = self
                        .check
                        .generic_template(template_id)
                        .and_then(|template| template.parameters.first().copied())
                {
                    let source = declaration.value.into_any();
                    let target = self.push_type(
                        dir::Type::Parameter(parameter.into_global(self.module)),
                        source,
                    )?;
                    let value = self.push_type(
                        dir::Type::Operation(dir::TypeOperation::StringMapping { mapping, target }),
                        source,
                    )?;
                    self.declare_symbol_type(symbol, value)?;
                    self.check.insert_definition(
                        symbol,
                        id.into_global_any(self.module),
                        dir::Definition::TypeAlias(dir::TypeAliasDefinition {
                            template: template.map(|template| template.local_id),
                            value,
                        }),
                    )?;

                    return Ok(());
                }

                self.check
                    .report_invalid_intrinsic_type(self.module, declaration.value.into_any());
            }

            return Ok(());
        }

        // walk the written value
        let value = self.walk_type_expression(declaration.value)?;
        self.record_type_induction_site(induction, value);

        // transparent aliases expand to their value, newtypes wrap it
        let definition = if declaration.is_nominal {
            dir::Definition::Newtype(dir::NewtypeDefinition {
                template: template.map(|template| template.local_id),
                value,
            })
        } else {
            self.declare_symbol_type(symbol, value)?;

            dir::Definition::TypeAlias(dir::TypeAliasDefinition {
                template: template.map(|template| template.local_id),
                value,
            })
        };
        self.check.insert_definition(symbol, source, definition)?;

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
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template = self.walk_generic_template(
            source,
            None,
            Some(symbol),
            &declaration.generic_parameters,
        )?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause)?;
        }
        let receiver = self.nominal_receiver(id.into_any(), symbol)?;
        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk implemented contracts
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            implements.extend(self.nominal_heritage(*implemented_type, ty)?);
        }

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
            )?);
        }

        let definition = dir::Definition::Struct(dir::StructDefinition {
            template: template.map(|template| template.local_id),
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // concrete structs need one fixed representation
        self.oblige_declaration_layout(symbol, receiver, template);

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
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template = self.walk_generic_template(
            source,
            None,
            Some(symbol),
            &declaration.generic_parameters,
        )?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause)?;
        }
        let receiver = self.nominal_receiver(id.into_any(), symbol)?;
        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk superclass type
        let mut extends = None;
        let mut super_ty = None;
        if let Some(extends_type) = declaration.extends_type {
            let ty = self.walk_type_expression(extends_type)?;
            self.record_type_induction_site(induction, ty);
            extends = self.nominal_heritage(extends_type, ty)?;
            super_ty = Some(ty);
        }

        // walk implemented contracts
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            implements.extend(self.nominal_heritage(*implemented_type, ty)?);
        }

        // members see the superclass through the receiver
        let receiver = Receiver {
            super_ty,
            ..receiver
        };

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
            )?);
        }

        let definition = dir::Definition::Class(dir::ClassDefinition {
            template: template.map(|template| template.local_id),
            is_abstract: declaration.is_abstract,
            is_final: declaration.is_final,
            extends,
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // heritage rules check once the inherited members close
        self.oblige_class_heritage(source, symbol);

        // concrete classes need one fixed representation
        self.oblige_declaration_layout(symbol, receiver, template);

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
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template = self.walk_generic_template(
            source,
            None,
            Some(symbol),
            &declaration.generic_parameters,
        )?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause)?;
        }
        let receiver = self.nominal_receiver(id.into_any(), symbol)?;
        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk implemented contracts
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            implements.extend(self.nominal_heritage(*implemented_type, ty)?);
        }

        // walk variants and members
        let mut members = Vec::new();
        for field in &declaration.fields {
            members.extend(self.walk_enum_field(*field, self.tree.get(*field))?);
        }
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
            )?);
        }

        let definition = dir::Definition::Enum(dir::EnumDefinition {
            template: template.map(|template| template.local_id),
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // enums need one fixed backing representation
        self.oblige_declaration_layout(symbol, receiver, template);

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
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template = self.walk_generic_template(
            source,
            None,
            Some(symbol),
            &declaration.generic_parameters,
        )?;
        for where_clause in &declaration.where_clauses {
            self.walk_where_clause(*where_clause)?;
        }
        let receiver = self.nominal_receiver(id.into_any(), symbol)?;
        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk inherited contracts
        let mut extends = Vec::new();
        for extends_type in &declaration.extends_types {
            let ty = self.walk_type_expression(*extends_type)?;
            self.record_type_induction_site(induction, ty);
            extends.extend(self.nominal_heritage(*extends_type, ty)?);
        }

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            members.extend(self.walk_type_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
            )?);
        }

        let definition = dir::Definition::Interface(dir::InterfaceDefinition {
            template: template.map(|template| template.local_id),
            is_nominal: declaration.is_nominal,
            extends,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

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
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template = self.walk_generic_template(
            source,
            None,
            Some(symbol),
            &declaration.generic_parameters,
        )?;
        let mut where_clauses = Vec::new();
        for where_clause in &declaration.where_clauses {
            where_clauses.extend(self.walk_extension_where_clause(*where_clause)?);
        }

        // walk target before exposing receiver
        let target_type = self.walk_type_expression(declaration.target_type)?;
        self.record_type_induction_site(induction, target_type);
        let target = self.extension_target(declaration.target_type, target_type)?;
        let receiver = Receiver {
            owner: Some(symbol),
            ty: target_type,
            super_ty: None,
        };
        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk implemented contracts
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            implements.extend(self.nominal_heritage(*implemented_type, ty)?);
        }

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
            )?);
        }

        // named extensions import explicitly, inherent ones travel with their target declaration
        let form = match &target {
            dir::ExtensionTarget::Nominal { root, .. } if root.module_id == self.module => {
                dir::ExtensionForm::Inherent
            }
            _ if declaration.name.is_some() => dir::ExtensionForm::Named,
            _ => dir::ExtensionForm::Local,
        };
        let definition = dir::Definition::Extension(dir::Extension {
            symbol,
            form,
            template: template.map(|template| template.local_id),
            target,
            implements,
            where_clauses,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // coherence rules report at the later declaration
        self.check
            .check_extension_coherence(self.module, source, symbol)?;

        Ok(())
    }

    /// Queue one class heritage obligation under the active guard.
    fn oblige_class_heritage(&mut self, source: dir::GlobalNodeIdAny, symbol: dir::GlobalSymbolId) {
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::Heritage(HeritageObligation {
                source,
                condition,
                symbol,
            }));
    }

    /// Demand one concrete declaration's layout.
    /// Generic declarations lay out per instantiation instead.
    fn oblige_declaration_layout(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Receiver,
        template: Option<GenericTemplateId>,
    ) {
        if template.is_some() {
            return;
        }
        let Some(source) = self
            .check
            .module(self.module)
            .symbol_declaration_node(symbol.local_id)
            .ok()
        else {
            return;
        };
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::Layout(LayoutObligation {
                source: source.into_global(self.module),
                condition,
                ty: receiver.ty,
            }));
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
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any());
        let Some(symbol) = symbol else {
            // validate local signatures without a declaration symbol
            self.walk_function_signature(None, &declaration.signature)?;

            return Ok(());
        };

        // walk signature before reading its inputs
        let source = id.into_global_any(self.module);
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template =
            self.signature_template(source, None, Some(symbol), &declaration.signature)?;
        self.walk_function_signature(template, &declaration.signature)?;
        let result =
            self.function_result_type(id.into_any(), &declaration.signature, declaration.body)?;

        // tie the function symbol to its signature type
        let function = self.function_signature_type(
            id.into_any(),
            &declaration.signature,
            template,
            None,
            result,
        )?;
        self.record_type_induction_site(induction, function);
        self.declare_symbol_type(symbol, function)?;

        // walk body after its result exists
        if let (Some(body), Some(result)) = (declaration.body, result) {
            let receiver = declaration
                .signature
                .this_parameter
                .map(|parameter| self.this_parameter_receiver_binding(parameter, None))
                .transpose()?;
            self.walk_function_body(symbol, &declaration.signature, body, result, receiver)?;
        }

        Ok(())
    }

    /// Walk one enum field and return its variant row.
    ///
    /// Example:
    /// ```ds
    /// Some(value)
    /// ```
    pub(in crate::check) fn walk_enum_field(
        &mut self,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(None);
        };
        let condition = self.member_condition(id.into_any())?;
        let (name, value) = (enum_field.name, enum_field.value);

        if let Some(value) = value {
            // check enum values in declaration context
            let before_value = self.fork_flow();
            self.walk_expression(value, self.tree.get(value))?;
            self.restore_flow(before_value);

            // record the written variant value
            if let Some(symbol) = self
                .check
                .module(self.module)
                .declaration_symbol(id.into_any())
            {
                let written = self.lower_static_predicate(value)?;
                self.declare_symbol_value(symbol, written)?;
            }
        }

        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(None);
        };

        Ok(Some(dir::DefinitionMember::Variant(
            dir::VariantDefinition {
                symbol,
                source: id.into_global_any(self.module),
                key: name.static_key(),
                value: None,
                condition,
            },
        )))
    }

    /// Walk one where clause as a satisfaction constraint.
    ///
    /// Example:
    /// ```ds
    /// where T: Copy
    /// ```
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        id: dir::LocalNodeId<dir::WhereClause>,
    ) -> CompilerResult<()> {
        self.walk_extension_where_clause(id)?;

        Ok(())
    }

    /// Walk one where clause and return its checked sides.
    fn walk_extension_where_clause(
        &mut self,
        id: dir::LocalNodeId<dir::WhereClause>,
    ) -> CompilerResult<Option<dir::ExtensionWhereClause>> {
        let clause = self.tree.get(id);
        let (left, right) = (clause.left, clause.right);
        let left = self.walk_type_expression(left)?;
        let right = self.walk_type_expression(right)?;

        // bounds on own unconstrained parameters attach as constraints,
        // every other clause checks satisfaction at the declaration
        let attached = self.attach_parameter_bound(left, right)?;
        if !attached {
            let origin = Origin::Node(id.into_global_any(self.module));
            self.relate_type(origin, Relation::Satisfies, left, right);
        }

        Ok(Some(dir::ExtensionWhereClause {
            source: id.into_global_any(self.module),
            left,
            right,
        }))
    }

    /// Attach one where bound onto its bare parameter when it has no
    /// constraint yet. Returns whether the bound was attached.
    fn attach_parameter_bound(
        &mut self,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Parameter(parameter) = self.check.ty(left)? else {
            return Ok(false);
        };
        let parameter = *parameter;

        // keep explicit inline bounds, their clause still checks
        let Some(binding) = self.check.generic_parameter(parameter) else {
            return Ok(false);
        };
        if binding.constraint.is_some() {
            return Ok(false);
        }
        let default = binding.default;
        self.check
            .update_generic_parameter_bounds(parameter, Some(right), default)?;

        Ok(true)
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
            self.walk_type_expression(return_type)?;
        }

        // walk where clauses
        for where_clause in &signature.where_clauses {
            self.walk_where_clause(*where_clause)?;
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
                    "this parameter {:?} has no type",
                    parameter.into_global(self.module)
                ),
            });
        };

        Ok(ReceiverBinding {
            symbol,
            receiver: Receiver {
                owner,
                ty,
                super_ty: None,
            },
        })
    }

    /// Return one function result type.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    pub(in crate::check) fn function_result_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            let lowered = self.walk_type_expression(return_type)?;
            self.bind_result_lifetimes(source, lowered, body.is_some())?;

            return Ok(Some(lowered));
        }

        // skip ambient signatures
        if body.is_none() {
            return Ok(None);
        }

        // open the inferred result
        Ok(Some(self.open_type(source)?))
    }

    /// Bind elided result lifetimes by the declaration's body.
    /// NOTE #Suspicious: not entirely sure if bind_result_lifetimes is where we should be inducing?
    fn bind_result_lifetimes(
        &mut self,
        source: dir::LocalNodeIdAny,
        lowered: dir::GlobalTypeId,
        has_body: bool,
    ) -> CompilerResult<()> {
        let mut reported = false;
        for variable in self.check.type_variables(lowered)? {
            let representative = self.check.variables.representative(variable)?;
            let Some(recipe) = self.check.generics.induction(representative) else {
                continue;
            };
            if recipe.induction != dir::GenericParameterInduction::Form {
                continue;
            }

            if has_body {
                self.check.generics.remove_induction(representative);
            } else if !reported {
                self.check
                    .report_ambient_lifetime_elided(self.module, source);
                reported = true;
            }
        }

        Ok(())
    }

    /// Return one nominal declaration receiver scope.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    fn nominal_receiver(
        &mut self,
        source: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Receiver> {
        // apply the declaration's own parameters as arguments
        let parameters = self
            .check
            .generics
            .template_by_symbol(symbol)
            .map(|template| self.check.generic_template_parameters(template))
            .unwrap_or_default();
        let mut arguments = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            arguments.push(self.push_type(dir::Type::Parameter(parameter), source)?);
        }
        let ty = self.push_type(
            dir::Type::Reference(dir::GenericInstance { symbol, arguments }),
            source,
        )?;

        Ok(Receiver {
            owner: Some(symbol),
            ty,
            super_ty: None,
        })
    }

    /// Return one heritage application from a walked annotation.
    /// Non-reference heritages are reported by their own relations.
    fn nominal_heritage(
        &mut self,
        source: dir::LocalNodeId<dir::TypeExpression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::NominalHeritage>> {
        let heritage = match self.check.ty(ty)? {
            dir::Type::Reference(instance) => Some(dir::NominalHeritage {
                source: source.into_global_any(self.module),
                symbol: instance.symbol,
                arguments: instance.arguments.clone(),
            }),
            _ => None,
        };

        Ok(heritage)
    }

    /// Return one extension target from a walked target annotation.
    /// Nominal targets classify by their written name resolution, so
    /// still-open header types cannot demote them to blankets.
    fn extension_target(
        &mut self,
        annotation: dir::LocalNodeId<dir::TypeExpression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ExtensionTarget> {
        // nominal roots anchor member lookup
        if let dir::Type::Reference(instance) = self.check.ty(ty)? {
            return Ok(dir::ExtensionTarget::Nominal {
                root: instance.symbol,
                ty,
            });
        }

        // written references classify by their resolved declaration
        if let dir::TypeExpression::Reference { .. } = self.tree.get(annotation)
            && let Some(Decision::Name(resolution)) = self
                .check
                .decisions
                .get(annotation.into_global_any(self.module))
            && let [symbol] = resolution.symbols()
            && self.check.symbol_kind(*symbol).is_nominal()
        {
            return Ok(dir::ExtensionTarget::Nominal { root: *symbol, ty });
        }

        Ok(dir::ExtensionTarget::Blanket { ty })
    }
}

/// Return the string mapping one language item declares, when any.
fn string_mapping_for_item(item: dir::LanguageItem) -> Option<dir::StringMapping> {
    match item {
        dir::LanguageItem::Uppercase => Some(dir::StringMapping::Uppercase),
        dir::LanguageItem::Lowercase => Some(dir::StringMapping::Lowercase),
        dir::LanguageItem::Capitalize => Some(dir::StringMapping::Capitalize),
        dir::LanguageItem::Uncapitalize => Some(dir::StringMapping::Uncapitalize),
        _ => None,
    }
}
