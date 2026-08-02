use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CauseKind, CheckError, CheckState, DeclarationHeritageObligation, FunctionHeader,
    GenericTemplateId, ImplementationCoherenceObligation, InducedParameterOwner,
    InterfaceConformanceObligation, Obligation, Origin, ParameterUseObligation, Receiver,
    ReceiverBinding, Relation, RepresentationObligation, TypeSubstitution, VariableRole, WalkState,
    Widening,
};
use crate::{CompilerError, CompilerResult};

/// One component template declaration pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TemplatePass {
    /// Declare template and parameter identities.
    Declare,
    /// Walk parameter bounds, defaults, and where predicates.
    Walk,
}

/// Result of walking one enum variant declaration.
enum WalkedEnumVariant {
    /// The variant is absent under its static guard.
    Absent,
    /// The variant is present but invalid.
    Invalid,
    /// The variant has one valid scalar value.
    Present(dir::EnumVariantDefinition),
}

impl CheckState<'_> {
    /// Bind nominal type definition symbols as declaration references.
    pub(in crate::check) fn bind_module_reference_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        let binding_table = self.module(module).binding_table();
        let candidates = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .collect::<Vec<_>>();
        let mut symbols = Vec::with_capacity(candidates.len());
        for symbol in candidates {
            let kind = self.symbol_kind(symbol)?;
            if kind.is_type_definition() && !kind.is_type_alias() {
                symbols.push(symbol);
            }
        }

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

        let ty = self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
        self.commit_declaration_type(symbol, ty)?;

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
    pub(in crate::check) fn visit_expression_templates(
        &mut self,
        _id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
        pass: TemplatePass,
    ) -> CompilerResult<()> {
        if let dir::Expression::Declaration(declaration) = expression {
            let declaration = *declaration;
            self.visit_declaration_templates(declaration, self.tree.get(declaration), pass)?;
        }

        Ok(())
    }

    /// Walk generic headers introduced by one declaration.
    ///
    /// Example:
    /// ```ds
    /// function value<T>(input: T): T { input }
    /// ```
    fn visit_declaration_templates(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
        pass: TemplatePass,
    ) -> CompilerResult<()> {
        if !self.decide_static_presence(id.into_any())? {
            return Ok(());
        }

        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                for expression in &declaration.expressions {
                    self.visit_expression_templates(*expression, self.tree.get(*expression), pass)?;
                }
            }
            // module M { ... }
            dir::Declaration::Module(declaration) => {
                for expression in &declaration.expressions {
                    self.visit_expression_templates(*expression, self.tree.get(*expression), pass)?;
                }
            }
            // type X<T> = T
            dir::Declaration::Type(declaration) => {
                self.visit_declaration_template(
                    id,
                    &declaration.generic_parameters,
                    &declaration.where_clauses,
                    pass,
                )?;
            }
            // struct S<T> {}
            dir::Declaration::Struct(declaration) => {
                self.visit_declaration_template(
                    id,
                    &declaration.generic_parameters,
                    &declaration.where_clauses,
                    pass,
                )?;
            }
            // class C<T> {}
            dir::Declaration::Class(declaration) => {
                self.visit_declaration_template(
                    id,
                    &declaration.generic_parameters,
                    &declaration.where_clauses,
                    pass,
                )?;
            }
            // enum E<T> {}
            dir::Declaration::Enum(declaration) => {
                self.visit_declaration_template(
                    id,
                    &declaration.generic_parameters,
                    &declaration.where_clauses,
                    pass,
                )?;
            }
            // interface I<T> {}
            dir::Declaration::Interface(declaration) => {
                self.visit_declaration_template(
                    id,
                    &declaration.generic_parameters,
                    &declaration.where_clauses,
                    pass,
                )?;
            }
            // extension T<U> {}
            dir::Declaration::Extension(declaration) => {
                self.visit_declaration_template(
                    id,
                    &declaration.generic_parameters,
                    &declaration.where_clauses,
                    pass,
                )?;
            }
            // function f<T>() {}
            dir::Declaration::Function(declaration) => {
                if pass == TemplatePass::Declare {
                    self.open_function_generic_template(id, &declaration.signature)?;
                }
            }
        }

        Ok(())
    }

    /// Visit one type-level declaration's template in one pass.
    fn visit_declaration_template(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
        pass: TemplatePass,
    ) -> CompilerResult<()> {
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(());
        };
        let source = id.into_global_any(self.module);

        // declare identities first so bounds may reference any template
        if pass == TemplatePass::Declare {
            let template = self.open_generic_template(source, parameters)?;

            // hypotheses need a template: where clauses, heritage
            //  assumptions, and interfaces assuming their own application
            let assumes = !where_clauses.is_empty()
                || self.check.symbol_kind(symbol)?.is_interface()
                || self
                    .tree
                    .get(id)
                    .implements_types()
                    .is_some_and(|types| !types.is_empty());
            if template.is_none() && assumes {
                self.check.open_generic_template(source)?;
            }

            return Ok(());
        }
        let template = self.walk_generic_template(source, parameters)?;

        // interfaces assume this satisfies their own application
        if self.check.symbol_kind(symbol)?.is_interface()
            && let Some(template) = template
        {
            self.push_this_predicate(source, symbol, template)?;
        }

        // where clauses resolve under the declaration's own template
        let _scope = self.enter_template_scope(template);
        for where_clause in where_clauses {
            self.walk_where_clause(template, *where_clause)?;
        }

        Ok(())
    }

    /// Assume `this` satisfies one implemented heritage application.
    fn push_this_heritage_predicate(
        &mut self,
        source: dir::GlobalNodeIdAny,
        template: GenericTemplateId,
        heritage: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let left = self.intern_type(dir::Type::This)?;
        let predicate = dir::WherePredicate {
            source,
            relation: dir::WhereRelation::Satisfies,
            left,
            right: heritage,
        };

        self.check.push_template_predicate(template, predicate)
    }

    /// Assume `this` satisfies one interface's own application.
    pub(in crate::check) fn push_this_predicate(
        &mut self,
        source: dir::GlobalNodeIdAny,
        interface: dir::GlobalSymbolId,
        template: GenericTemplateId,
    ) -> CompilerResult<()> {
        let instance = self.check.declaration_instance(interface)?;
        let left = self.intern_type(dir::Type::This)?;
        let right = self.intern_type(dir::Type::Application(instance))?;
        let predicate = dir::WherePredicate {
            source,
            relation: dir::WhereRelation::Satisfies,
            left,
            right,
        };

        self.check.push_template_predicate(template, predicate)
    }

    /// Open the generic template owned by one function declaration.
    fn open_function_generic_template(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<()> {
        // symbol-less declarations own no template
        if self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
            .is_none()
        {
            return Ok(());
        }
        let source = id.into_global_any(self.module);
        let Some(template) = self.open_signature_template(source, signature)? else {
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
        // each declaration walks exactly once, on demand or in root order
        let node = id.into_global_any(self.module);
        if !self.check.walked_declarations.insert(node) {
            return Ok(());
        }

        self.check.walking_declarations.push(node);
        let walked = self.walk_declaration_kind(id, declaration);
        self.check.walking_declarations.pop();

        walked
    }

    /// Walk one declaration's kind-specific structure.
    fn walk_declaration_kind(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        if !self.declare_decorators(id.into_any())? {
            return Ok(());
        }

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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.check.generics.template_by_source(source);
        let _scope = self.enter_template_scope(template);

        // handle intrinsic declarations separately from ordinary aliases
        if matches!(
            self.tree.get(declaration.value),
            dir::TypeExpression::Intrinsic
        ) {
            self.walk_intrinsic_type_declaration(id, declaration, symbol, template)?;
            return Ok(());
        }

        // nominal values bind `this` to their own declaration
        let receiver = match declaration.is_nominal {
            true => Some(self.nominal_receiver(symbol, None)?),
            false => None,
        };
        let _receiver = receiver.map(|receiver| self.enter_receiver_scope(Some(receiver)));

        // walk the written value
        let value = self.walk_type_expression(declaration.value)?;
        self.push_induced_parameter_site(induction, value);

        // transparent aliases expand to their value, newtypes wrap it
        let definition = if receiver.is_some() {
            let template = self.induced_owner_template(induction, template)?;
            dir::Definition::Newtype(dir::NewtypeDefinition {
                space: declaration.place.map(dir::PlaceModifier::space),
                template: template.map(|template| template.local_id),
                representation: dir::Representation::default(),
                backing: value,
                is_tagged: false,
                tagged_options: None,
                constructors: Vec::new(),
                discriminator: None,
                members: Vec::new(),
            })
        } else {
            self.bind_symbol_type(symbol, value)?;

            // commit the induced owner template, whose arity matches interned applications
            let template = self.induced_owner_template(induction, template)?;

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

        let value = self.intern_type(dir::Type::Intrinsic)?;
        self.commit_node_type(declaration.value, value)?;

        // intrinsic newtypes stay opaque
        if declaration.is_nominal {
            let definition = dir::Definition::Newtype(dir::NewtypeDefinition {
                space: declaration.place.map(dir::PlaceModifier::space),
                template: template.map(|template| template.local_id),
                representation: dir::Representation::default(),
                backing: value,
                is_tagged: false,
                tagged_options: None,
                constructors: Vec::new(),
                discriminator: None,
                members: Vec::new(),
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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.check.generics.template_by_source(source);
        let _scope = self.enter_template_scope(template);
        let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk implemented interfaces
        let implements = self.walk_nominal_implements(
            symbol,
            receiver,
            template,
            induction,
            &declaration.implements_types,
        )?;

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            if let Some(definition) = self.walk_member_header(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )? {
                members.push(definition);
            }
        }

        let template = self.induced_owner_template(induction, template)?;
        let definition = dir::Definition::Struct(dir::StructDefinition {
            space: declaration.place.map(dir::PlaceModifier::space),
            template: template.map(|template| template.local_id),
            representation: dir::Representation::default(),
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.check.generics.template_by_source(source);
        let _scope = self.enter_template_scope(template);
        let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Managed))?;
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk superclass type
        let mut extends = None;
        let mut super_ty = None;
        if let Some(extends_type) = declaration.extends_type {
            let ty = self.walk_type_expression(extends_type)?;
            self.push_induced_parameter_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(extends_type, ty)? {
                if self
                    .check
                    .symbol_kind_maybe(instance.symbol)?
                    .is_none_or(|kind| kind == dir::SymbolKind::Class)
                {
                    self.relate_heritage_clause(extends_type, Relation::Extends, receiver.ty, ty);
                    extends = Some(dir::NominalHeritage { source, ty });
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
        let implements = self.walk_nominal_implements(
            symbol,
            receiver,
            template,
            induction,
            &declaration.implements_types,
        )?;

        // members see the superclass through the receiver
        let receiver = Receiver {
            super_ty,
            ..receiver
        };

        // walk members
        let mut members = Vec::new();
        for member in &declaration.members {
            if let Some(definition) = self.walk_member_header(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )? {
                members.push(definition);
            }
        }
        let constructors =
            self.class_construct_candidates(receiver.ty, extends.is_some(), &members)?;

        let template = self.induced_owner_template(induction, template)?;
        let definition = dir::Definition::Class(dir::ClassDefinition {
            space: declaration.place.map(dir::PlaceModifier::space),
            template: template.map(|template| template.local_id),
            representation: dir::Representation::default(),
            is_abstract: declaration.is_abstract,
            is_final: declaration.is_final,
            extends,
            implements,
            constructors,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        Ok(())
    }

    /// Walk one nominal declaration's implemented interfaces.
    fn walk_nominal_implements(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Receiver,
        template: Option<GenericTemplateId>,
        induction: InducedParameterOwner,
        implements_types: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CompilerResult<Vec<dir::InterfaceImplementation>> {
        let mut implements = Vec::new();
        for implemented_type in implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.push_induced_parameter_site(induction, ty);

            // require a written interface instance
            let Some((source, instance)) = self.heritage_instance(*implemented_type, ty)? else {
                self.check.report_implementation_target_not_interface_type(
                    self.check.format_symbol(symbol),
                    ty,
                    implemented_type.into_global_any(self.module),
                );

                continue;
            };
            if self
                .check
                .symbol_kind_maybe(instance.symbol)?
                .is_some_and(|kind| !kind.is_interface())
            {
                self.check
                    .report_implementation_target_not_interface_symbol(
                        self.check.format_symbol(symbol),
                        instance.symbol,
                        source,
                    );

                continue;
            }
            self.relate_heritage_clause(*implemented_type, Relation::Implements, receiver.ty, ty);

            // members assume this satisfies the implemented interface
            if let Some(template) = template {
                self.push_this_heritage_predicate(source, template, ty)?;
            }
            implements.push(dir::InterfaceImplementation {
                interface: dir::NominalHeritage { source, ty },
                members: Vec::new(),
            });
        }

        Ok(implements)
    }

    /// Return direct construct candidates for one class.
    fn class_construct_candidates(
        &mut self,
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

        self.default_class_construct_candidate(receiver)
    }

    /// Return explicitly declared class construct candidates.
    fn declared_class_construct_candidates(
        &mut self,
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
                ty: self.symbol_type_slot(method.symbol)?,
            });
        }

        Ok(constructors)
    }

    /// Return a base class default construct candidate.
    fn default_class_construct_candidate(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::ClassConstructorDefinition>> {
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters: dir::TypeListId::EMPTY,
            return_type: Some(receiver),
            is_generator: false,
        };
        let ty = self.intern_signature(function)?;

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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.check.generics.template_by_source(source);
        let _scope = self.enter_template_scope(template);
        let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk implemented interfaces
        let implements = self.walk_nominal_implements(
            symbol,
            receiver,
            template,
            induction,
            &declaration.implements_types,
        )?;

        // evaluate variants and establish one scalar backing domain
        let mut members = Vec::new();
        let mut next_value = Some(Ok(dir::EnumVariantValue::Integer(0)));
        let mut backing = None;
        let mut values = FxIndexMap::default();
        for field in &declaration.fields {
            let variant = self.walk_enum_field(*field, self.tree.get(*field), next_value)?;
            let variant = match variant {
                WalkedEnumVariant::Absent => continue,
                WalkedEnumVariant::Invalid => {
                    next_value = None;

                    continue;
                }
                WalkedEnumVariant::Present(variant) => variant,
            };
            let variant_backing = variant.value.default_backing();
            if backing.is_some_and(|backing| backing != variant_backing) {
                let anchor = self.check.diagnostic_anchor(self.module, field.into_any());
                let error = CheckError::MixedEnumVariantDomain {
                    anchor,
                    module: self.module,
                };
                self.check.report(self.module, error);

                let error = self.intern_type(dir::Type::Error)?;
                self.bind_symbol_type(variant.symbol, error)?;
                next_value = None;

                continue;
            }

            // preserve one nominal member per runtime value
            if let Some(previous) = values.get(&variant.value).copied() {
                let literal = dir::ScalarLiteral::from(variant.value);
                let value = self.check.format_scalar_literal(&literal);
                self.check
                    .report_duplicate_enum_variant_value(variant.source, previous, value);
                let error = self.intern_type(dir::Type::Error)?;
                self.bind_symbol_type(variant.symbol, error)?;
                next_value = Some(variant.value.increment());

                continue;
            }
            values.insert(variant.value, variant.source);

            // commit the accepted singleton and its scalar value
            let ty = self.intern_type(dir::Type::Variant(dir::VariantType {
                owner: receiver.ty,
                variant: variant.symbol,
            }))?;
            self.bind_symbol_type(variant.symbol, ty)?;
            let literal = dir::ScalarLiteral::from(variant.value);
            let static_type = self.intern_type(dir::Type::Literal(literal))?;
            self.commit_static_value(variant.symbol, static_type)?;

            backing = Some(variant_backing);
            next_value = Some(variant.value.increment());
            members.push(dir::DefinitionMember::EnumVariant(variant));
        }
        for member in &declaration.members {
            if let Some(definition) = self.walk_member_header(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )? {
                members.push(definition);
            }
        }

        let template = self.induced_owner_template(induction, template)?;
        let definition = dir::Definition::Enum(dir::EnumDefinition {
            space: declaration.place.map(dir::PlaceModifier::space),
            template: template.map(|template| template.local_id),
            representation: dir::Representation::default(),
            backing: backing.unwrap_or(dir::EnumBackingType::DEFAULT),
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.check.generics.template_by_source(source);
        let _scope = self.enter_template_scope(template);
        let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Managed))?;
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk inherited interfaces
        let mut extends = Vec::new();
        for extends_type in &declaration.extends_types {
            let ty = self.walk_type_expression(*extends_type)?;
            self.push_induced_parameter_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*extends_type, ty)? {
                if self
                    .check
                    .symbol_kind_maybe(instance.symbol)?
                    .is_none_or(|kind| kind.is_interface())
                {
                    extends.push(dir::NominalHeritage { source, ty });
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

        let template = self.induced_owner_template(induction, template)?;
        let definition = dir::Definition::Interface(dir::InterfaceDefinition {
            space: declaration.place.map(dir::PlaceModifier::space),
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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.check.generics.template_by_source(source);
        let _scope = self.enter_template_scope(template);

        // expose members under the extended receiver
        let target_type = self.walk_type_expression(declaration.target_type)?;
        self.push_induced_parameter_site(induction, target_type);
        let origin = Origin::Node(source, self.flow().template_scope());
        let target = self.walk_extension_target(origin, target_type)?;
        let target_name = match &target {
            dir::ExtensionTarget::Rooted { root, .. } => self.check.format_symbol(*root),
            _ => self.check.format_type(target_type),
        };
        let ownership = match self.check.default_ownership(origin, target_type)? {
            Answer::Ready(ownership) => ownership,
            Answer::Pending(_) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "extension target {} has unsettled ownership",
                        self.check.format_type(target_type),
                    ),
                });
            }
        };
        let receiver = Receiver {
            declaration: Some(symbol),
            ownership,
            ty: target_type,
            super_ty: None,
        };
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk implemented interfaces
        let mut implements = Vec::new();
        for implemented_type in &declaration.implements_types {
            let ty = self.walk_type_expression(*implemented_type)?;
            self.push_induced_parameter_site(induction, ty);
            if let Some((source, instance)) = self.heritage_instance(*implemented_type, ty)? {
                // skip kind validation on foreign symbols while declaring, checking reads their kind
                if self
                    .check
                    .symbol_kind_maybe(instance.symbol)?
                    .is_none_or(|kind| kind.is_interface())
                {
                    implements.push(dir::InterfaceImplementation {
                        interface: dir::NominalHeritage { source, ty },
                        members: Vec::new(),
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
            if let Some(definition) = self.walk_member_header(
                *member,
                self.tree.get(*member),
                Some(receiver),
                Some(induction),
                declaration.is_ambient,
            )? {
                members.push(definition);
            }
        }

        // exported extensions are visible outside this module
        let form = if declaration.export.is_some() {
            dir::ExtensionForm::Exported
        } else {
            dir::ExtensionForm::Local
        };
        let template = self.induced_owner_template(induction, template)?;
        let definition = dir::Definition::Extension(dir::ExtensionDefinition {
            symbol,
            form,
            template: template.map(|template| template.local_id),
            target,
            implements,
            members,
        });
        self.check.insert_definition(symbol, source, definition)?;

        Ok(())
    }

    /// Queue one extension conformance obligation.
    pub(in crate::check) fn queue_extension_conformance_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let scope = self.check.symbol_template(symbol)?;
        self.check.push_obligation(
            Obligation::InterfaceConformance(InterfaceConformanceObligation { source, symbol }),
            scope,
        );

        Ok(())
    }

    /// Queue one implementation coherence obligation.
    pub(in crate::check) fn queue_implementation_coherence_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let scope = self.check.symbol_template(symbol)?;
        self.check.push_obligation(
            Obligation::ImplementationCoherence(ImplementationCoherenceObligation {
                source,
                symbol,
            }),
            scope,
        );

        Ok(())
    }

    /// Queue one generic parameter use obligation.
    pub(in crate::check) fn queue_parameter_use_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let scope = self.check.symbol_template(symbol)?;
        if scope.is_none() {
            return Ok(());
        }
        self.check.push_obligation(
            Obligation::ParameterUse(ParameterUseObligation { source, symbol }),
            scope,
        );

        Ok(())
    }

    /// Queue one heritage obligation.
    pub(in crate::check) fn queue_heritage_obligation(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let scope = self.check.symbol_template(symbol)?;
        self.check.push_obligation(
            Obligation::DeclarationHeritage(DeclarationHeritageObligation { source, symbol }),
            scope,
        );

        Ok(())
    }

    /// Queue one concrete declaration's layout check.
    pub(in crate::check) fn queue_declaration_layout_obligation(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Receiver,
    ) -> CompilerResult<()> {
        // generic declarations lay out per instantiation
        if self.check.symbol_template(symbol)?.is_some() {
            return Ok(());
        }
        let source = self
            .check
            .module(self.module)
            .symbol_declaration_node(symbol.local_id)?;
        let scope = self.check.symbol_template(symbol)?;
        self.check.push_obligation(
            Obligation::Representation(RepresentationObligation {
                source: source.into_global(self.module),
                ty: receiver.ty,
            }),
            scope,
        );

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
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        let template = self.open_signature_template(source, &declaration.signature)?;

        let (header, result, tracked) = self.walk_signature_header(
            id.into_any(),
            template,
            &declaration.signature,
            declaration.body,
        )?;

        // require a body unless the declaration is ambient
        if declaration.body.is_none() && !declaration.is_ambient {
            let source = id.into_global_any(self.module);
            self.check
                .report_missing_declaration_body(source, self.check.format_symbol(symbol));
        }

        // write the function symbol type
        let signature = self.walk_function_signature_type(
            id.into_any(),
            &declaration.signature,
            header,
            Some(induction),
            None,
            result,
            tracked,
        )?;
        let is_function_value =
            declaration.signature.form == dir::FunctionForm::Lambda || declaration.name.is_none();
        let function = if is_function_value {
            self.push_function_value_type(signature)?
        } else {
            signature
        };
        self.push_induced_parameter_site(induction, function);
        self.bind_symbol_type(symbol, function)?;

        Ok(())
    }

    /// Walk one enum field and return its scalar variant.
    ///
    /// Example:
    /// ```ds
    /// enum Status { Ready = 1 }
    /// ```
    fn walk_enum_field(
        &mut self,
        id: dir::LocalNodeId<dir::EnumField>,
        enum_field: &dir::EnumField,
        implicit: Option<Result<dir::EnumVariantValue, dir::EnumVariantIncrementError>>,
    ) -> CompilerResult<WalkedEnumVariant> {
        if !self.declare_decorators(id.into_any())? {
            return Ok(WalkedEnumVariant::Absent);
        }
        let (name, value) = (enum_field.name, enum_field.value);
        let Some(symbol) = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any())
        else {
            return Ok(WalkedEnumVariant::Invalid);
        };

        // evaluate the scalar value before committing its member state
        let Some(value) = self.evaluate_enum_variant_value(id, value, implicit)? else {
            let error = self.intern_type(dir::Type::Error)?;
            self.bind_symbol_type(symbol, error)?;

            return Ok(WalkedEnumVariant::Invalid);
        };

        Ok(WalkedEnumVariant::Present(dir::EnumVariantDefinition {
            symbol,
            source: id.into_global_any(self.module),
            key: name.static_key(),
            value,
        }))
    }

    /// Evaluate one enum variant's explicit or implicit scalar value.
    fn evaluate_enum_variant_value(
        &mut self,
        id: dir::LocalNodeId<dir::EnumField>,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
        implicit: Option<Result<dir::EnumVariantValue, dir::EnumVariantIncrementError>>,
    ) -> CompilerResult<Option<dir::EnumVariantValue>> {
        match expression {
            Some(expression) => {
                let static_type = self.walk_static_term(expression)?;
                let origin = Origin::Node(
                    expression.into_global_any(self.module),
                    self.flow().template_scope(),
                );
                let reduced = self.check.reduce_type_head(origin, static_type)?;
                let static_type = match reduced {
                    Answer::Ready(static_type) => static_type,
                    Answer::Pending(_) => {
                        self.check
                            .report_undecidable_static_value(self.module, expression.into_any());

                        return Ok(None);
                    }
                };
                let value = match self.check.ty(static_type)? {
                    dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                        dir::EnumVariantValue::Integer(value)
                    }
                    dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                        dir::EnumVariantValue::String(value)
                    }
                    dir::Type::Error => return Ok(None),
                    _ => {
                        let ty = self.check.format_type(static_type);
                        let anchor = self
                            .check
                            .diagnostic_anchor(self.module, expression.into_any());
                        let error = CheckError::InvalidEnumVariantType {
                            anchor,
                            module: self.module,
                            ty,
                        };
                        self.check.report(self.module, error);

                        return Ok(None);
                    }
                };

                Ok(Some(value))
            }
            None => {
                let value = match implicit {
                    Some(Ok(value)) => value,
                    Some(Err(dir::EnumVariantIncrementError::ExplicitValueRequired)) => {
                        let anchor = self.check.diagnostic_anchor(self.module, id.into_any());
                        let error = CheckError::ImplicitStringEnumVariant {
                            anchor,
                            module: self.module,
                        };
                        self.check.report(self.module, error);

                        return Ok(None);
                    }
                    Some(Err(dir::EnumVariantIncrementError::Overflow)) => {
                        let anchor = self.check.diagnostic_anchor(self.module, id.into_any());
                        let error = CheckError::EnumVariantValueOverflow {
                            anchor,
                            module: self.module,
                        };
                        self.check.report(self.module, error);

                        return Ok(None);
                    }
                    None => return Ok(None),
                };
                Ok(Some(value))
            }
        }
    }

    /// Walk one where clause onto its declaring template.
    ///
    /// Instantiation sites prove recorded predicates and the template's
    /// own scope assumes them.
    /// Clauses without a template check satisfaction at the declaration.
    ///
    /// Example:
    /// ```ds
    /// where T: Copy
    /// ```
    pub(in crate::check) fn walk_where_clause(
        &mut self,
        template: Option<GenericTemplateId>,
        id: dir::LocalNodeId<dir::WhereClause>,
    ) -> CompilerResult<()> {
        // walk operands
        let clause = self.tree.get(id);
        let (relation, left, right) = (clause.relation, clause.left, clause.right);
        let left = self.walk_type_expression(left)?;
        let right = self.walk_type_expression(right)?;

        // a lifetime outlives one lifetime; a union bound has no sound reading
        if self.check.is_lifetime_term(left)?
            && matches!(self.check.ty(right)?, dir::Type::Union(_))
        {
            self.check
                .report_disjunctive_lifetime_bound(id.into_global_any(self.module))?;

            return Ok(());
        }

        // check clauses without a template through current bound logic
        let Some(template) = template else {
            let origin = Origin::Node(
                id.into_global_any(self.module),
                self.flow().template_scope(),
            );
            self.relate_type(
                origin,
                CauseKind::Expression,
                Relation::Satisfies,
                left,
                right,
            );

            return Ok(());
        };

        // preserve template predicate relation for later proof
        self.check.push_template_predicate(
            template,
            dir::WherePredicate {
                source: id.into_global_any(self.module),
                relation,
                left,
                right,
            },
        )?;

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
    ) -> CompilerResult<FunctionHeader> {
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

        // declared callable parameters need value representation
        let is_annotation_required = signature.form != dir::FunctionForm::Lambda;
        let this_parameter = if let Some(parameter) = signature.this_parameter {
            self.walk_parameter(
                parameter,
                self.tree.get(parameter),
                is_annotation_required,
                is_annotation_required,
            )?
        } else {
            None
        };

        let mut parameters = Vec::new();
        for parameter in &signature.parameters {
            let Some(ty) = self.walk_parameter(
                *parameter,
                self.tree.get(*parameter),
                is_annotation_required,
                is_annotation_required,
            )?
            else {
                continue;
            };
            if let Some(parameter) = self.function_parameter_type(*parameter, ty)? {
                parameters.push(parameter);
            }
        }

        // walk where clauses
        for where_clause in &signature.where_clauses {
            self.walk_where_clause(template, *where_clause)?;
        }

        Ok(FunctionHeader {
            template,
            this_parameter,
            parameters,
        })
    }

    /// Open the generic template owned by one function signature.
    pub(in crate::check) fn open_signature_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if !signature.declares_generic_scope() {
            return Ok(None);
        }

        self.check.open_generic_template(source).map(Some)
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
        ty: dir::GlobalTypeId,
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

        let ty = self.apply_receiver_scope(scope, ty)?;

        Ok(ReceiverBinding {
            symbol,
            receiver: Receiver {
                declaration: scope.and_then(|scope| scope.declaration),
                ownership: scope.and_then(|scope| scope.ownership),
                ty,
                super_ty: scope.and_then(|scope| scope.super_ty),
            },
        })
    }

    /// Apply the active receiver scope to one receiver parameter type.
    pub(in crate::check) fn apply_receiver_scope(
        &mut self,
        scope: Option<Receiver>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(scope) = scope else {
            return Ok(ty);
        };

        let substitution = TypeSubstitution::default().with_receiver(scope.ty);

        self.check.substitute_type(ty, &substitution)
    }

    /// Walk one callable header under the template it declares.
    pub(in crate::check) fn walk_signature_header(
        &mut self,
        source: dir::LocalNodeIdAny,
        template: Option<GenericTemplateId>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<(
        FunctionHeader,
        Option<dir::GlobalTypeId>,
        Vec<dir::TypeVariableId>,
    )> {
        // walk the whole header under the template it declares
        let _scope = self.enter_template_scope(template);
        let header = self.walk_function_signature(template, signature)?;
        let (result, tracked) = self.walk_function_result_type(source, signature, body)?;

        Ok((header, result, tracked))
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
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Vec<dir::TypeVariableId>)> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            let (result, tracked) = self.walk_return_type_expression(return_type)?;

            return Ok((Some(result), tracked));
        }

        // setters return void by role, never written or inferred
        if signature.role == Some(dir::FunctionRole::Setter) {
            return Ok((Some(self.intern_type(dir::Type::Void)?), Vec::new()));
        }

        // skip ambient signatures
        if body.is_none() {
            return Ok((None, Vec::new()));
        }

        // open the inferred result: lambdas hold a contextual slot,
        //  declarations hold a producer cell
        let role = match signature.form {
            dir::FunctionForm::Lambda => VariableRole::Parameter,
            _ => VariableRole::Return,
        };

        Ok((
            Some(self.open_type_hole(source, Widening::Never, role)?),
            Vec::new(),
        ))
    }

    /// Return one nominal declaration receiver scope.
    ///
    /// Example:
    /// ```ds
    /// struct Box { value: number }
    /// ```
    pub(in crate::check) fn nominal_receiver(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ownership: Option<dir::Ownership>,
    ) -> CompilerResult<Receiver> {
        // apply the declaration's own parameters as arguments
        let parameters = match self.check.generics.template_by_symbol(symbol) {
            Some(template) => self.check.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let mut arguments = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            arguments.push(self.intern_type(dir::Type::Parameter(parameter))?);
        }
        let arguments = self.intern_type_ids(&arguments)?;
        let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;

        Ok(Receiver {
            declaration: Some(symbol),
            ownership,
            ty,
            super_ty: None,
        })
    }

    /// Return the nominal instance written in one heritage annotation.
    fn heritage_instance(
        &mut self,
        source: dir::LocalNodeId<dir::TypeExpression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalNodeIdAny, dir::GenericApplication)>> {
        let global_source = source.into_global_any(self.module);

        let dir::Type::Application(instance) = self.check.ty(ty)? else {
            return Ok(None);
        };

        Ok(Some((global_source, instance)))
    }

    /// Relate one written heritage clause.
    fn relate_heritage_clause(
        &mut self,
        source: dir::LocalNodeId<dir::TypeExpression>,
        relation: Relation,
        declared: dir::GlobalTypeId,
        heritage: dir::GlobalTypeId,
    ) {
        let origin = Origin::Node(
            source.into_global_any(self.module),
            self.flow().template_scope(),
        );
        let clause = source.into_global_any(self.module);
        self.relate_type(
            origin,
            CauseKind::Heritage { clause },
            relation,
            declared,
            heritage,
        );
    }

    /// Return one extension target from a walked target annotation.
    fn walk_extension_target(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ExtensionTarget> {
        // prefer the concrete nominal beneath memory forms
        let target_instance = self.check.apparent_instance(ty)?;
        let chain = self.check.form_chain(origin, ty)?;
        let value_instance = self.check.apparent_instance(chain.base())?;
        if let Some(instance) = value_instance.or(target_instance) {
            let root = instance.symbol;

            return Ok(dir::ExtensionTarget::Rooted { root, ty });
        }

        Ok(dir::ExtensionTarget::Blanket { ty })
    }
}

/// Return whether one named function declaration must write its result type.
pub(in crate::check) fn function_needs_written_result(
    declaration: &dir::FunctionDeclaration,
) -> bool {
    declaration.signature.return_type.is_none()
        && !declaration.signature.is_generator
        && declaration.name.is_some()
        && declaration.signature.form != dir::FunctionForm::Lambda
}
