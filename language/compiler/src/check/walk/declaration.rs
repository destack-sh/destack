use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, DeclarationHeritageObligation, ExtensionConformanceObligation,
    GenericInductionDeclaration, GenericTemplateId, ImplementationCoherenceObligation, Obligation,
    Origin, Receiver, ReceiverBinding, Relation, RepresentationObligation, TypeSubstitution,
    WalkState,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Bind nominal type definition symbols as declaration references.
    pub(in crate::check) fn bind_module_reference_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| {
                let kind = self.symbol_kind(*symbol);

                kind.is_type_definition() && !kind.is_type_alias()
            })
            .collect::<Vec<_>>();

        for symbol in symbols {
            self.bind_nominal_reference_type(symbol)?;
        }

        Ok(())
    }

    /// Bind one nominal type definition symbol as its own reference type.
    fn bind_nominal_reference_type(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<()> {
        if self.declaration_type_maybe(symbol).is_some() {
            return Ok(());
        }

        let origin = Origin::Symbol(symbol);
        let source = self.origin_source_node(origin)?;
        let ty = self.push_type(
            symbol.module_id,
            dir::Type::Reference(dir::TypeReference { symbol }),
            source,
        )?;
        self.set_declaration_type(symbol, ty)?;

        Ok(())
    }
}

impl WalkState<'_, '_> {
    /// Walk generic headers introduced by one expression.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn walk_expression_header(
        &mut self,
        _id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) -> CompilerResult<()> {
        if let dir::Expression::Declaration(declaration) = expression {
            let declaration = *declaration;
            self.walk_declaration_header(declaration, self.tree.get(declaration))?;
        }

        Ok(())
    }

    /// Walk generic headers introduced by one declaration.
    ///
    /// Example:
    /// ```ds
    /// function value<T>(input: T): T { input }
    /// ```
    fn walk_declaration_header(
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
                    self.walk_expression_header(*expression, self.tree.get(*expression))?;
                }
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.walk_expression_header(*expression, self.tree.get(*expression))?;
                }
            }
            // type X<T> = T
            dir::Declaration::Type(declaration) => {
                self.open_declaration_generic_template(id, &declaration.generic_parameters)?;
            }
            // struct S<T> {}
            dir::Declaration::Struct(declaration) => {
                self.open_declaration_generic_template(id, &declaration.generic_parameters)?;
            }
            // class C<T> {}
            dir::Declaration::Class(declaration) => {
                self.open_declaration_generic_template(id, &declaration.generic_parameters)?;
            }
            // enum E<T> {}
            dir::Declaration::Enum(declaration) => {
                self.open_declaration_generic_template(id, &declaration.generic_parameters)?;
            }
            // interface I<T> {}
            dir::Declaration::Interface(declaration) => {
                self.open_declaration_generic_template(id, &declaration.generic_parameters)?;
            }
            // extension T<U> {}
            dir::Declaration::Extension(declaration) => {
                self.open_declaration_generic_template(id, &declaration.generic_parameters)?;
            }
            // function f<T>() {}
            dir::Declaration::Function(declaration) => {
                self.open_function_generic_template(id, &declaration.signature)?;
            }
        }

        Ok(())
    }

    /// Open the generic template owned by one symbol declaration.
    fn open_declaration_generic_template(
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
        self.open_generic_template(source, None, Some(symbol), parameters)?;

        Ok(())
    }

    /// Open the generic template owned by one function declaration.
    fn open_function_generic_template(
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
        let Some(template) = self.open_signature_template(source, None, Some(symbol), signature)?
        else {
            return Ok(());
        };

        // open explicit signature parameters before walking bodies
        for parameter in &signature.generic_parameters {
            self.open_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
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
            // module { ... }
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

        // handle intrinsic declarations separately from ordinary aliases
        if matches!(
            self.tree.get(declaration.value),
            dir::TypeExpression::Intrinsic
        ) {
            self.walk_intrinsic_type_declaration(id, declaration, symbol, template)?;
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
            self.bind_symbol_type(symbol, value)?;

            dir::Definition::TypeAlias(dir::TypeAliasDefinition {
                template: template.map(|template| template.local_id),
                value,
            })
        };
        self.check.insert_definition(symbol, source, definition)?;

        Ok(())
    }

    /// Walk one type declaration whose value is `intrinsic`.
    fn walk_intrinsic_type_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::TypeDeclaration,
        symbol: dir::GlobalSymbolId,
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<()> {
        let source = id.into_global_any(self.module);

        let value = self.push_type(dir::Type::Intrinsic, declaration.value.into_any())?;

        // intrinsic newtypes stay opaque
        if declaration.is_nominal {
            let definition = dir::Definition::Newtype(dir::NewtypeDefinition {
                template: template.map(|template| template.local_id),
                value,
            });
            self.check.insert_definition(symbol, source, definition)?;

            return Ok(());
        }

        // transparent intrinsic aliases reduce when applied
        if self.check.is_transparent_intrinsic_alias(symbol)? {
            self.bind_symbol_type(symbol, value)?;
            let definition = dir::Definition::TypeAlias(dir::TypeAliasDefinition {
                template: template.map(|template| template.local_id),
                value,
            });
            self.check.insert_definition(symbol, source, definition)?;

            return Ok(());
        }

        self.check
            .report_invalid_intrinsic_type(self.module, declaration.value.into_any());

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

        // walk implemented interfaces
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*implemented_type, ty)? {
                if self.check.symbol_kind(instance.symbol).is_interface() {
                    self.relate_heritage_clause(
                        *implemented_type,
                        Relation::Implements,
                        receiver.ty,
                        ty,
                    );
                    implements.push(dir::NominalHeritage {
                        source,
                        symbol: instance.symbol,
                        arguments: instance.arguments,
                    });
                } else {
                    self.check
                        .report_implementation_target_not_interface_symbol(
                            self.check.format_symbol(symbol),
                            instance.symbol,
                            source,
                        );
                }
            } else {
                self.check.report_implementation_target_not_interface_type(
                    self.check.format_symbol(symbol),
                    ty,
                    implemented_type.into_global_any(self.module),
                );
            }
        }

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )?);
        }

        let definition = dir::Definition::Struct(dir::StructDefinition {
            template: template.map(|template| template.local_id),
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // heritage rules check once the inherited declarations close
        self.queue_heritage_obligation(source, symbol);

        // concrete structs need one fixed representation
        self.queue_declaration_layout_obligation(symbol, receiver, template)?;

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
            if let Some((source, instance)) = self.heritage_instance(extends_type, ty)? {
                if self.check.symbol_kind(instance.symbol) == dir::SymbolKind::Class {
                    self.relate_heritage_clause(extends_type, Relation::Extends, receiver.ty, ty);
                    extends = Some(dir::NominalHeritage {
                        source,
                        symbol: instance.symbol,
                        arguments: instance.arguments,
                    });
                    super_ty = Some(ty);
                } else {
                    self.check
                        .report_does_not_extend_symbol(receiver.ty, instance.symbol, source);
                }
            } else {
                self.check.report_does_not_extend_type(
                    receiver.ty,
                    ty,
                    extends_type.into_global_any(self.module),
                );
            }
        }

        // walk implemented interfaces
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*implemented_type, ty)? {
                if self.check.symbol_kind(instance.symbol).is_interface() {
                    self.relate_heritage_clause(
                        *implemented_type,
                        Relation::Implements,
                        receiver.ty,
                        ty,
                    );
                    implements.push(dir::NominalHeritage {
                        source,
                        symbol: instance.symbol,
                        arguments: instance.arguments,
                    });
                } else {
                    self.check
                        .report_implementation_target_not_interface_symbol(
                            self.check.format_symbol(symbol),
                            instance.symbol,
                            source,
                        );
                }
            } else {
                self.check.report_implementation_target_not_interface_type(
                    self.check.format_symbol(symbol),
                    ty,
                    implemented_type.into_global_any(self.module),
                );
            }
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
                declaration.is_ambient,
            )?);
        }
        let constructors = self.class_construct_candidates(
            id.into_any(),
            receiver.ty,
            extends.is_some(),
            &members,
        )?;

        let definition = dir::Definition::Class(dir::ClassDefinition {
            template: template.map(|template| template.local_id),
            is_abstract: declaration.is_abstract,
            is_final: declaration.is_final,
            extends,
            implements,
            constructors,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // heritage rules check once the inherited declarations close
        self.queue_heritage_obligation(source, symbol);

        // concrete classes need one fixed representation
        self.queue_declaration_layout_obligation(symbol, receiver, template)?;

        Ok(())
    }

    /// Return direct construct candidates for one class.
    fn class_construct_candidates(
        &mut self,
        source: dir::LocalNodeIdAny,
        receiver: dir::GlobalTypeId,
        is_derived: bool,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        let declared = self.declared_class_construct_candidates(members)?;
        if !declared.is_empty() {
            return Ok(declared);
        }
        if is_derived {
            return Ok(Vec::new());
        }

        self.default_class_construct_candidate(source, receiver)
    }

    /// Return explicitly declared class construct candidates.
    fn declared_class_construct_candidates(
        &self,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        let mut constructors = Vec::new();
        for member in members {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            if method.slot != dir::MemberSlot::Constructor {
                continue;
            }

            constructors.push(dir::ClassConstructorDefinition {
                constructor: dir::ClassConstructor::Declared {
                    symbol: method.symbol,
                },
                ty: method.ty,
            });
        }

        Ok(constructors)
    }

    /// Return a base class default construct candidate.
    fn default_class_construct_candidate(
        &mut self,
        source: dir::LocalNodeIdAny,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters: Vec::new(),
            return_type: Some(receiver),
            is_generator: false,
        };
        let ty = self.push_type(dir::Type::FunctionSignature(function), source)?;

        Ok(vec![dir::ClassConstructorDefinition {
            constructor: dir::ClassConstructor::Default,
            ty,
        }])
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

        // walk implemented interfaces
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*implemented_type, ty)? {
                if self.check.symbol_kind(instance.symbol).is_interface() {
                    self.relate_heritage_clause(
                        *implemented_type,
                        Relation::Implements,
                        receiver.ty,
                        ty,
                    );
                    implements.push(dir::NominalHeritage {
                        source,
                        symbol: instance.symbol,
                        arguments: instance.arguments,
                    });
                } else {
                    self.check
                        .report_implementation_target_not_interface_symbol(
                            self.check.format_symbol(symbol),
                            instance.symbol,
                            source,
                        );
                }
            } else {
                self.check.report_implementation_target_not_interface_type(
                    self.check.format_symbol(symbol),
                    ty,
                    implemented_type.into_global_any(self.module),
                );
            }
        }

        // walk variants and members
        let mut members = Vec::new();
        for field in &declaration.fields {
            members.extend(self.walk_enum_field(*field, self.tree.get(*field), receiver.ty)?);
        }
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )?);
        }

        let definition = dir::Definition::Enum(dir::EnumDefinition {
            template: template.map(|template| template.local_id),
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        // heritage rules check once the inherited declarations close
        self.queue_heritage_obligation(source, symbol);

        // enums need one fixed backing representation
        self.queue_declaration_layout_obligation(symbol, receiver, template)?;

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

        // walk inherited interfaces
        let mut extends = Vec::new();
        for extends_type in &declaration.extends_types {
            let ty = self.walk_type_expression(*extends_type)?;
            self.record_type_induction_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*extends_type, ty)? {
                if self.check.symbol_kind(instance.symbol).is_interface() {
                    extends.push(dir::NominalHeritage {
                        source,
                        symbol: instance.symbol,
                        arguments: instance.arguments,
                    });
                } else {
                    self.check.report_interface_base_not_interface_symbol(
                        symbol,
                        instance.symbol,
                        source,
                    );
                }
            } else {
                self.check.report_interface_base_not_interface_type(
                    symbol,
                    ty,
                    extends_type.into_global_any(self.module),
                );
            }
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

        // heritage rules check once the inherited declarations close
        self.queue_heritage_obligation(source, symbol);

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

        // expose members under the extended receiver
        let target_type = self.walk_type_expression(declaration.target_type)?;
        self.record_type_induction_site(induction, target_type);
        let target = self.walk_extension_target(target_type)?;
        let target_name = match &target {
            dir::ExtensionTarget::Rooted { root, .. } => self.check.format_symbol(*root),
            _ => self.check.format_type(target_type),
        };
        let receiver = Receiver {
            declaration: Some(symbol),
            ty: target_type,
            super_ty: None,
        };
        let _receiver = self.enter_receiver_maybe(Some(receiver));

        // walk implemented interfaces
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.record_type_induction_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*implemented_type, ty)? {
                if self.check.symbol_kind(instance.symbol).is_interface() {
                    implements.push(dir::NominalHeritage {
                        source,
                        symbol: instance.symbol,
                        arguments: instance.arguments,
                    });
                } else {
                    self.check
                        .report_implementation_target_not_interface_symbol(
                            target_name.clone(),
                            instance.symbol,
                            source,
                        );
                }
            } else {
                self.check.report_implementation_target_not_interface_type(
                    target_name.clone(),
                    ty,
                    implemented_type.into_global_any(self.module),
                );
            }
        }

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            members.extend(self.walk_member(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )?);
        }

        // named extensions import explicitly, inherent ones travel with their target declaration
        let form = match &target {
            dir::ExtensionTarget::Rooted { root, .. } if root.module_id == self.module => {
                dir::ExtensionForm::Inherent
            }
            _ if declaration.name.is_some() => dir::ExtensionForm::Named,
            _ => dir::ExtensionForm::Local,
        };
        let definition = dir::Definition::Extension(dir::ExtensionDefinition {
            symbol,
            form,
            template: template.map(|template| template.local_id),
            target,
            implements,
            where_clauses,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        self.queue_extension_conformance_obligation(source, symbol);
        self.queue_implementation_coherence_obligation(source, symbol);
        self.queue_heritage_obligation(source, symbol);

        Ok(())
    }

    /// Queue one extension conformance obligation under the active guard.
    fn queue_extension_conformance_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let condition = self.active_static_guard();
        self.check.push_obligation(Obligation::ExtensionConformance(
            ExtensionConformanceObligation {
                source,
                condition,
                symbol,
            },
        ));
    }

    /// Queue one implementation coherence obligation under the active guard.
    fn queue_implementation_coherence_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::ImplementationCoherence(
                ImplementationCoherenceObligation {
                    source,
                    condition,
                    symbol,
                },
            ));
    }

    /// Queue one heritage obligation under the active guard.
    fn queue_heritage_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let condition = self.active_static_guard();
        self.check.push_obligation(Obligation::DeclarationHeritage(
            DeclarationHeritageObligation {
                source,
                condition,
                symbol,
            },
        ));
    }

    /// Queue one concrete declaration's layout check.
    /// Generic declarations lay out per instantiation instead.
    fn queue_declaration_layout_obligation(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Receiver,
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<()> {
        if template.is_some() {
            return Ok(());
        }
        let source = self
            .check
            .module(self.module)
            .symbol_declaration_node(symbol.local_id)?;
        let condition = self.active_static_guard();
        self.check
            .push_obligation(Obligation::Representation(RepresentationObligation {
                source: source.into_global(self.module),
                condition,
                ty: receiver.ty,
            }));

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
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any());
        let Some(symbol) = symbol else {
            // validate local signatures without a declaration symbol
            self.walk_function_signature(None, &declaration.signature)?;

            return Ok(());
        };

        // open signature parameters before building the function type
        let source = id.into_global_any(self.module);
        let induction = GenericInductionDeclaration::new(source, None, Some(symbol));
        let template =
            self.open_signature_template(source, None, Some(symbol), &declaration.signature)?;
        self.walk_function_signature(template, &declaration.signature)?;
        if declaration.body.is_none() && !declaration.is_ambient {
            let source = id.into_global_any(self.module);
            self.check
                .report_missing_declaration_body(source, self.check.format_symbol(symbol));
        }
        let result = self.walk_function_result_type(
            id.into_any(),
            &declaration.signature,
            declaration.body,
        )?;

        // write the function symbol type
        let signature = self.walk_function_signature_type(
            id.into_any(),
            &declaration.signature,
            template,
            Some(induction),
            None,
            result,
        )?;
        let is_function_value =
            declaration.signature.form == dir::FunctionForm::Lambda || declaration.name.is_none();
        let function = if is_function_value {
            self.push_function_value_type(id.into_any(), signature)?
        } else {
            signature
        };
        self.record_type_induction_site(induction, function);
        self.bind_symbol_type(symbol, function)?;

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

    /// Walk one enum field and return its variant member.
    ///
    /// Example:
    /// ```ds
    /// Some(value)
    /// ```
    pub(in crate::check) fn walk_enum_field(
        &mut self,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
        owner: dir::GlobalTypeId,
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
                let written = self.walk_static_term(value)?;
                self.set_static_value(symbol, written)?;
            }
        }

        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(None);
        };

        let ty = self.push_type(
            dir::Type::EnumMember(dir::EnumMemberType {
                owner,
                member: symbol,
            }),
            id.into_any(),
        )?;

        Ok(Some(dir::DefinitionMember::Variant(
            dir::VariantDefinition {
                symbol,
                source: id.into_global_any(self.module),
                key: name.static_key(),
                ty,
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
                self.open_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
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

        // walk where clauses
        for where_clause in &signature.where_clauses {
            self.walk_where_clause(*where_clause)?;
        }

        Ok(())
    }

    /// Open the generic template owned by one function signature.
    pub(in crate::check) fn open_signature_template(
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
            .open_generic_template(source, parent, symbol)
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
        scope: Option<Receiver>,
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
        let Some(ty) = self.walk_parameter_type(parameter)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "this parameter {:?} has no type",
                    parameter.into_global(self.module)
                ),
            });
        };

        let ty = self.apply_receiver_scope(parameter.into_any(), scope, ty)?;

        Ok(ReceiverBinding {
            symbol,
            receiver: Receiver {
                declaration: scope.and_then(|scope| scope.declaration),
                ty,
                super_ty: scope.and_then(|scope| scope.super_ty),
            },
        })
    }

    /// Apply the active receiver scope to one receiver parameter type.
    pub(in crate::check) fn apply_receiver_scope(
        &mut self,
        source: dir::LocalNodeIdAny,
        scope: Option<Receiver>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(scope) = scope else {
            return Ok(ty);
        };

        let substitution = TypeSubstitution::default().with_receiver(scope.ty);

        self.check
            .fold_type(self.module, source, ty, substitution.rewrite())
    }

    /// Walk one function return annotation or open its inferred result.
    ///
    /// Example:
    /// ```ds
    /// function value(): number { 1 }
    /// ```
    pub(in crate::check) fn walk_function_result_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            let result = self.walk_return_type_expression(source, return_type, body.is_some())?;

            return Ok(Some(result));
        }

        // skip ambient signatures
        if body.is_none() {
            return Ok(None);
        }

        // open the inferred result
        Ok(Some(self.infer_type(source)?))
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
            dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
            source,
        )?;

        Ok(Receiver {
            declaration: Some(symbol),
            ty,
            super_ty: None,
        })
    }

    /// Return the nominal instance written in one heritage annotation.
    fn heritage_instance(
        &mut self,
        source: dir::LocalNodeId<dir::TypeExpression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalNodeIdAny, dir::GenericInstance)>> {
        let global_source = source.into_global_any(self.module);
        let dir::Type::Instance(instance) = self.check.ty(ty)? else {
            return Ok(None);
        };

        Ok(Some((global_source, instance.clone())))
    }

    /// Relate one written heritage clause.
    fn relate_heritage_clause(
        &mut self,
        source: dir::LocalNodeId<dir::TypeExpression>,
        relation: Relation,
        declared: dir::GlobalTypeId,
        heritage: dir::GlobalTypeId,
    ) {
        let origin = Origin::Node(source.into_global_any(self.module));
        self.relate_type(origin, relation, declared, heritage);
    }

    /// Return one extension target from a walked target annotation.
    fn walk_extension_target(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ExtensionTarget> {
        // root extensions under their lookup owner
        if let Some(root) = self.check.extension_root(ty)? {
            return Ok(dir::ExtensionTarget::Rooted { root, ty });
        }

        Ok(dir::ExtensionTarget::Blanket { ty })
    }
}
