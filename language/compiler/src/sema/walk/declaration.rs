use smallvec::SmallVec;
use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    CauseKind, CheckError, CheckState, ElisionSite, FunctionHeader, GenericTemplateId,
    InducedParameterOwner, Origin, Receiver, ReceiverBinding, Relation, TypeSubstitution,
    VariableKind, WalkState,
};
use crate::{CompilerError, CompilerResult};

/// One module template declaration pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum TemplatePass {
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
    /// Commit nominal type definition symbols as declaration references.
    pub(in crate::sema) fn commit_module_reference_types(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // gather every nominal type definition the module binds
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

        // commit each one as its own reference type
        for symbol in symbols {
            self.commit_nominal_reference_type(symbol)?;
        }

        Ok(())
    }

    /// Commit one nominal type definition symbol as its own reference type.
    fn commit_nominal_reference_type(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<()> {
        if self.declaration_type_maybe(symbol).is_some() {
            return Ok(());
        }

        let ty = self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;
        self.commit_declaration_type(symbol, ty)?;

        Ok(())
    }
}

impl WalkState<'_, '_> {
    /// Walk generic headers introduced by one expression.
    pub(in crate::sema) fn visit_expression_templates(
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
    fn visit_declaration_templates(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
        pass: TemplatePass,
    ) -> CompilerResult<()> {
        if !self.decide_static_presence(id.into_any())? {
            return Ok(());
        }

        // declare by the declaration kind
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
            // type X<T> = T, struct S<T> {}, class C<T> {}, enum E<T> {}, interface I<T> {}
            dir::Declaration::Type(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_)
            | dir::Declaration::Extension(_) => {
                let generic_parameters = declaration.generic_parameters().unwrap_or_default();
                let where_clauses = declaration.where_clauses().unwrap_or_default();
                self.visit_declaration_template(id, generic_parameters, where_clauses, pass)?;
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
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };
        let source = id.into_global_any(self.module);

        // declare identities first so bounds may reference any template
        if pass == TemplatePass::Declare {
            let template = self.open_generic_template(source, parameters)?;

            // open a template for where clauses, heritage assumptions, and interfaces
            let assumes = !where_clauses.is_empty()
                || self.check.symbol_kind(symbol)?.is_interface()
                || self
                    .tree
                    .get(id)
                    .implements_types()
                    .is_some_and(|types| !types.is_empty());
            if template.is_none() && assumes {
                self.check
                    .open_generic_template(source, self.flow().template_scope())?;
            }

            // an interface declares its receiver as an implicit parameter
            if self.check.symbol_kind(symbol)?.is_interface() {
                let template = self
                    .check
                    .open_generic_template(source, self.flow().template_scope())?;
                self.check
                    .push_receiver_parameter(template, source, symbol)?;
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
        self.with_template_scope(template, |walk| {
            for where_clause in where_clauses {
                walk.walk_where_clause(template, *where_clause)?;
            }

            Ok(())
        })
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
    pub(in crate::sema) fn push_this_predicate(
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
        // skip declarations without a symbol, which have no template
        if self.declared_symbol(id.into_any()).is_none() {
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

    /// Walk one module or global block member like a module root.
    fn walk_block_member(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // walk a declaration statement, entering its node first
        match self.tree.get(expression) {
            dir::Expression::Declaration(declaration) => {
                let declaration = *declaration;
                self.enter_node(expression)?;
                let void = self.intern_type(dir::Type::Void)?;
                self.commit_node_type(expression, void)?;

                self.walk_declaration(declaration, &self.tree.get(declaration).clone())
            }
            dir::Expression::Let { .. } => {
                self.enter_node(expression)?;

                // drop the whole member on a static gate
                if !self.decide_static_presence(expression.into_any())? {
                    return Ok(());
                }

                self.walk_let_bindings(expression)
            }
            // every other member belongs to the checking pass
            _ => Ok(()),
        }
    }

    /// Walk one declaration.
    pub(in crate::sema) fn walk_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        // walk each declaration exactly once, when asked or in root order
        let node = id.into_global_any(self.module);
        if !self.check.walked_declarations.insert(node) {
            return Ok(());
        }

        // walk the declaration, marking it in flight
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

        // walk by the declaration kind
        match declaration {
            // global { ... }
            dir::Declaration::Global(declaration) => {
                for expression in declaration.expressions.clone() {
                    self.walk_block_member(expression)?;
                }
            }
            // module { ... }
            dir::Declaration::Module(declaration) => {
                for expression in declaration.expressions.clone() {
                    self.walk_block_member(expression)?;
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
    fn walk_type_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::TypeDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };

        // reject a space on an alias, which names no type of its own
        if declaration.is_shared && !declaration.is_nominal {
            self.check.report_space_on_alias(self.module, id.into_any());
        }

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.check.template_by_source(source);
        self.with_template_scope(template, |walk| {
            // walk intrinsic declarations apart from ordinary aliases
            if matches!(
                walk.tree.get(declaration.value),
                dir::TypeExpression::Intrinsic
            ) {
                walk.walk_intrinsic_type_declaration(id, declaration, symbol, template)?;

                return Ok(());
            }

            // nominal values bind `this` to their own declaration
            let receiver = match declaration.is_nominal {
                true => Some(walk.nominal_receiver(symbol, None)?),
                false => None,
            };
            walk.with_receiver_scope(receiver, |walk| {
                // walk the written value under the declaration owner
                let value = walk.walk_type_expression(declaration.value)?;

                // transparent aliases expand to their value, newtypes wrap it
                let definition = if receiver.is_some() {
                    let template = walk.induced_owner_template(induction, template)?;
                    dir::Definition::Newtype(dir::NewtypeDefinition {
                        space: declaration.is_shared.then_some(dir::Space::Shared),
                        template: template.map(|template| template.local_id),
                        derives: walk.declared_derives(id)?.0,
                        backing: value,
                        backing_visibility: declaration
                            .backing_visibility
                            .unwrap_or(dir::Visibility::Public),
                        members: Vec::new(),
                    })
                } else {
                    walk.commit_symbol_type(symbol, value)?;

                    // commit the induced owner template
                    let template = walk.induced_owner_template(induction, template)?;

                    dir::Definition::TypeAlias(dir::TypeAliasDefinition {
                        template: template.map(|template| template.local_id),
                        value,
                    })
                };
                walk.check.insert_definition(symbol, source, definition)?;

                Ok(())
            })
        })
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

        // type an intrinsic declaration as opaque
        let value = self.intern_type(dir::Type::Intrinsic)?;
        self.commit_node_type(declaration.value, value)?;

        // intrinsic newtypes stay opaque
        if declaration.is_nominal {
            let definition = dir::Definition::Newtype(dir::NewtypeDefinition {
                space: declaration.is_shared.then_some(dir::Space::Shared),
                template: template.map(|template| template.local_id),
                derives: None,
                backing: value,
                backing_visibility: dir::Visibility::Public,
                members: Vec::new(),
            });
            self.check.insert_definition(symbol, source, definition)?;

            return Ok(());
        }

        // transparent intrinsic aliases reduce when applied
        if self.check.is_transparent_intrinsic_alias(symbol)? {
            self.commit_symbol_type(symbol, value)?;
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

    /// Read the derive list and conformance entries one declaration writes.
    fn declared_derives(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<(
        Option<Vec<dir::AutoInterface>>,
        Vec<dir::NominalConformance>,
    )> {
        let owner = id.into_any().into_global(self.module);

        // read the declaration's derive application
        let mut arguments = None;
        for application in &self.check.decorators {
            if application.owner != owner {
                continue;
            }
            if self
                .check
                .environment_bound
                .language
                .item(application.symbol)
                != Some(dir::LanguageItem::Derive)
            {
                continue;
            }
            arguments = Some(application.expression.arguments.clone());

            break;
        }
        let Some(arguments) = arguments else {
            return Ok((None, Vec::new()));
        };

        // name each derivable interface and its nominal conformance
        let mut selected = FxIndexSet::default();
        let mut interfaces = Vec::new();
        let mut conformances = Vec::new();
        for argument in arguments {
            // resolve the written argument reference
            let Some(expression) = self.tree.get(argument).value() else {
                return Ok((Some(Vec::new()), Vec::new()));
            };
            let expression = expression.into_global_any(self.module);
            let Some(symbol) = self.check.reference_symbol(expression)? else {
                return Ok((Some(Vec::new()), Vec::new()));
            };

            // require a compiler-known derivable interface
            let Some(interface) = self
                .check
                .environment_bound
                .language
                .item(symbol)
                .and_then(dir::AutoInterface::from_language_item)
                .filter(|interface| interface.is_derivable())
            else {
                return Ok((Some(Vec::new()), Vec::new()));
            };

            // reject duplicates without retaining a valid prefix
            if !selected.insert(interface) {
                return Ok((Some(Vec::new()), Vec::new()));
            }

            // record the conformance against the naming argument
            let arguments = if interface.has_receiver_argument() {
                vec![self.check.intern_type(dir::Type::This)?]
            } else {
                Vec::new()
            };
            let applied = self
                .check
                .language_type(dir::LanguageItem::from(interface), &arguments)?;
            interfaces.push(interface);
            conformances.push(dir::NominalConformance {
                source: expression,
                interface: applied,
                polarity: dir::Polarity::Positive,
            });
        }

        Ok((Some(interfaces), conformances))
    }

    /// Walk one struct declaration.
    fn walk_struct_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::StructDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.check.template_by_source(source);
        self.with_template_scope(template, |walk| {
            let receiver = walk.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
            walk.with_receiver_scope(Some(receiver), |walk| {
                // walk implemented interfaces
                let implements = walk.walk_nominal_implements(
                    symbol,
                    template,
                    induction,
                    &declaration.implements_types,
                )?;

                // walk members
                let mut members = Vec::new();
                for member in &declaration.members {
                    if let Some(definition) = walk.walk_member_header(
                        *member,
                        walk.tree.get(*member),
                        Some(receiver),
                        Some(induction),
                        declaration.is_ambient,
                    )? {
                        members.push(definition);
                    }
                }

                // commit the struct definition
                let template = walk.induced_owner_template(induction, template)?;
                let (derives, conformances) = walk.declared_derives(id)?;
                let mut implements = implements;
                implements.extend(conformances);
                let definition = dir::Definition::Struct(dir::StructDefinition {
                    space: declaration.is_shared.then_some(dir::Space::Shared),
                    template: template.map(|template| template.local_id),
                    derives,
                    implements,
                    members,
                });
                walk.check.insert_definition(symbol, source, definition)?;

                Ok(())
            })
        })
    }

    /// Walk one class declaration.
    fn walk_class_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ClassDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.check.template_by_source(source);
        self.with_template_scope(template, |walk| {
            let receiver = walk.nominal_receiver(symbol, Some(dir::Ownership::Managed))?;
            walk.with_receiver_scope(Some(receiver), |walk| {
                // walk superclass type
                let mut extends = None;
                let mut super_ty = None;
                if let Some(extends_type) = declaration.extends_type {
                    // store the base subobject inside the derived instance
                    let ty = walk.walk_type_expression_in(extends_type, ElisionSite::Member)?;
                    if let Some((source, instance)) = walk.heritage_instance(extends_type, ty)? {
                        if walk
                            .check
                            .symbol_kind(instance.symbol)
                            .map(|kind| kind == dir::SymbolKind::Class)?
                        {
                            walk.relate_heritage_clause(
                                extends_type,
                                Relation::Subtype,
                                receiver.ty,
                                ty,
                            )?;
                            extends = Some(dir::NominalHeritage { source, ty });
                            super_ty = Some(ty);
                        } else {
                            walk.check.report_does_not_extend_symbol(
                                receiver.ty,
                                instance.symbol,
                                source,
                            );
                        }
                    } else {
                        walk.check.report_does_not_extend_type(
                            receiver.ty,
                            ty,
                            extends_type.into_global_any(walk.module),
                        );
                    }
                }

                // walk implemented interfaces
                let implements = walk.walk_nominal_implements(
                    symbol,
                    template,
                    induction,
                    &declaration.implements_types,
                )?;

                // expose the superclass through the receiver
                let receiver = Receiver {
                    super_ty,
                    ..receiver
                };

                // walk members
                let mut members = Vec::new();
                for member in &declaration.members {
                    if let Some(definition) = walk.walk_member_header(
                        *member,
                        walk.tree.get(*member),
                        Some(receiver),
                        Some(induction),
                        declaration.is_ambient,
                    )? {
                        members.push(definition);
                    }
                }

                // commit the class definition
                let template = walk.induced_owner_template(induction, template)?;
                let (derives, conformances) = walk.declared_derives(id)?;
                let mut implements = implements;
                implements.extend(conformances);
                let definition = dir::Definition::Class(dir::ClassDefinition {
                    space: declaration.is_shared.then_some(dir::Space::Shared),
                    template: template.map(|template| template.local_id),
                    derives,
                    is_abstract: declaration.is_abstract,
                    is_final: declaration.is_final,
                    extends,
                    implements,
                    members,
                });
                walk.check.insert_definition(symbol, source, definition)?;

                Ok(())
            })
        })
    }

    /// Walk one nominal declaration's implemented interfaces.
    fn walk_nominal_implements(
        &mut self,
        symbol: dir::GlobalSymbolId,
        template: Option<GenericTemplateId>,
        induction: InducedParameterOwner,
        implements_types: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CompilerResult<Vec<dir::NominalConformance>> {
        let mut implements = Vec::new();
        for implemented_type in implements_types {
            // read the interface beneath a negated clause's operator
            let negated = self.negated_implements(*implemented_type);
            let heritage_annotation = negated.unwrap_or(*implemented_type);

            // induce elided heritage borrows on the declaring template
            let previous_owner = self.induced_owner;
            self.induced_owner = Some(induction);
            let ty = self.walk_type_expression_in(heritage_annotation, ElisionSite::Signature);
            self.induced_owner = previous_owner;
            let ty = ty?;

            // classify the clause, assuming `this` satisfies a positive interface
            let name = self.check.format_symbol(symbol);
            let Some(conformance) = self.implemented_conformance(
                symbol,
                &name,
                *implemented_type,
                heritage_annotation,
                ty,
                negated.is_some(),
            )?
            else {
                continue;
            };
            if conformance.polarity == dir::Polarity::Positive
                && let Some(template) = template
            {
                self.push_this_heritage_predicate(conformance.source, template, ty)?;
            }
            implements.push(conformance);
        }

        Ok(implements)
    }

    /// Classify one walked implements clause, reporting a clause that names no interface.
    fn implemented_conformance(
        &mut self,
        symbol: dir::GlobalSymbolId,
        name: &str,
        implemented_type: dir::LocalNodeId<dir::TypeExpression>,
        heritage_annotation: dir::LocalNodeId<dir::TypeExpression>,
        ty: dir::GlobalTypeId,
        is_negated: bool,
    ) -> CompilerResult<Option<dir::NominalConformance>> {
        // require a written interface instance
        let Some((source, instance)) = self.heritage_instance(heritage_annotation, ty)? else {
            self.check.report_implementation_target_not_interface_type(
                name.to_string(),
                ty,
                implemented_type.into_global_any(self.module),
            );

            return Ok(None);
        };

        // require an interface declaration
        if self
            .check
            .symbol_kind(instance.symbol)
            .map(|kind| !kind.is_interface())?
        {
            self.check
                .report_implementation_target_not_interface_symbol(
                    name.to_string(),
                    instance.symbol,
                    source,
                );

            return Ok(None);
        }

        // refuse a negated clause over an interface without a compiler rule
        let polarity = match is_negated {
            true if !self.require_auto_interface(symbol, instance.symbol, source)? => {
                return Ok(None);
            }
            true => dir::Polarity::Negative,
            false => dir::Polarity::Positive,
        };

        Ok(Some(dir::NominalConformance {
            source,
            interface: ty,
            polarity,
        }))
    }

    /// Return the interface one negated implements clause names, like `!Copy`.
    fn negated_implements(
        &self,
        implemented_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
        match self.tree.get(implemented_type) {
            dir::TypeExpression::Not { target_type } => Some(*target_type),
            _ => None,
        }
    }

    /// Require one negated clause to name an auto interface, reporting it otherwise.
    fn require_auto_interface(
        &mut self,
        symbol: dir::GlobalSymbolId,
        interface: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        let is_auto = self
            .check
            .language_item(interface)?
            .and_then(dir::AutoInterface::from_language_item)
            .is_some();
        if !is_auto {
            let name = self.check.format_symbol(symbol);
            self.check
                .report_negative_implementation_not_auto(name, interface, source);
        }

        Ok(is_auto)
    }

    /// Walk one enum declaration.
    fn walk_enum_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::EnumDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.check.template_by_source(source);
        self.with_template_scope(template, |walk| {
            let receiver = walk.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
            walk.with_receiver_scope(Some(receiver), |walk| {
                // walk implemented interfaces
                let implements = walk.walk_nominal_implements(
                    symbol,
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
                    let variant =
                        walk.walk_enum_field(*field, walk.tree.get(*field), next_value)?;
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
                        let anchor = walk.check.diagnostic_anchor(walk.module, field.into_any());
                        let error = CheckError::MixedEnumVariantDomain {
                            anchor,
                            module: walk.module,
                        };
                        walk.check.report(walk.module, error);

                        let error = walk.intern_type(dir::Type::Error)?;
                        walk.commit_symbol_type(variant.symbol, error)?;
                        next_value = None;

                        continue;
                    }

                    // preserve one nominal member per runtime value
                    if let Some(previous) = values.get(&variant.value).copied() {
                        let literal = dir::Literal::from(variant.value);
                        let value = walk.check.format_scalar_literal(&literal);
                        walk.check.report_duplicate_enum_variant_value(
                            variant.source,
                            previous,
                            value,
                        );
                        let error = walk.intern_type(dir::Type::Error)?;
                        walk.commit_symbol_type(variant.symbol, error)?;
                        next_value = Some(variant.value.increment());

                        continue;
                    }
                    values.insert(variant.value, variant.source);

                    // commit the accepted singleton and its scalar value
                    let ty = walk.intern_type(dir::Type::Variant(dir::VariantType {
                        owner: receiver.ty,
                        variant: variant.symbol,
                    }))?;
                    walk.commit_symbol_type(variant.symbol, ty)?;
                    let literal = dir::Literal::from(variant.value);
                    let static_type = walk.intern_type(dir::Type::Literal(literal))?;
                    walk.commit_static_value(variant.symbol, static_type)?;

                    backing = Some(variant_backing);
                    next_value = Some(variant.value.increment());
                    members.push(dir::DefinitionMember::EnumVariant(variant));
                }

                // declare the enum ahead of its members, which read its variants
                let declared = dir::Definition::Enum(dir::EnumDefinition {
                    space: declaration.is_shared.then_some(dir::Space::Shared),
                    template: None,
                    derives: Default::default(),
                    backing: backing.unwrap_or(dir::EnumBackingType::DEFAULT),
                    implements: implements.clone(),
                    members: members.clone(),
                });
                walk.check.insert_definition(symbol, source, declared)?;

                // walk the members declared beside the variants
                for member in &declaration.members {
                    if let Some(definition) = walk.walk_member_header(
                        *member,
                        walk.tree.get(*member),
                        Some(receiver),
                        Some(induction),
                        declaration.is_ambient,
                    )? {
                        members.push(definition);
                    }
                }

                // commit the enum definition
                let template = walk.induced_owner_template(induction, template)?;
                let (derives, conformances) = walk.declared_derives(id)?;
                let mut implements = implements;
                implements.extend(conformances);
                let definition = dir::Definition::Enum(dir::EnumDefinition {
                    space: declaration.is_shared.then_some(dir::Space::Shared),
                    template: template.map(|template| template.local_id),
                    derives,
                    backing: backing.unwrap_or(dir::EnumBackingType::DEFAULT),
                    implements,
                    members,
                });
                walk.check.insert_definition(symbol, source, definition)?;

                Ok(())
            })
        })
    }

    /// Walk one interface declaration.
    fn walk_interface_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::InterfaceDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.check.template_by_source(source);
        self.with_template_scope(template, |walk| {
            let receiver = walk.nominal_receiver(symbol, Some(dir::Ownership::Managed))?;
            walk.with_receiver_scope(Some(receiver), |walk| {
                // bind this to the interface receiver for its members
                if let Some(template) = template {
                    walk.push_this_predicate(source, symbol, template)?;
                }

                // walk inherited interfaces
                let mut extends = Vec::new();
                for extends_type in &declaration.extends_types {
                    let ty = walk.walk_type_expression(*extends_type)?;
                    if let Some((source, instance)) = walk.heritage_instance(*extends_type, ty)? {
                        if walk
                            .check
                            .symbol_kind(instance.symbol)
                            .map(|kind| kind.is_interface())?
                        {
                            extends.push(dir::NominalHeritage { source, ty });
                        } else {
                            walk.check.report_interface_base_not_interface_symbol(
                                symbol,
                                instance.symbol,
                                source,
                            );
                        }
                    } else {
                        walk.check.report_interface_base_not_interface_type(
                            symbol,
                            ty,
                            extends_type.into_global_any(walk.module),
                        );
                    }
                }

                // walk members
                let mut members = Vec::new();
                for member in &declaration.members {
                    members.extend(walk.walk_type_member(
                        *member,
                        walk.tree.get(*member),
                        Some(receiver),
                        Some(induction),
                    )?);
                }

                // commit the interface definition
                let template = walk.induced_owner_template(induction, template)?;
                let definition = dir::Definition::Interface(dir::InterfaceDefinition {
                    space: declaration.is_shared.then_some(dir::Space::Shared),
                    template: template.map(|template| template.local_id),
                    is_nominal: declaration.is_nominal,
                    extends,
                    members,
                });
                walk.check.insert_definition(symbol, source, definition)?;

                Ok(())
            })
        })
    }

    /// Walk one extension declaration.
    fn walk_extension_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::ExtensionDeclaration,
    ) -> CompilerResult<()> {
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(());
        };

        // walk generic header
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.check.template_by_source(source);
        self.with_template_scope(template, |walk| {
            // expose members under the extended receiver
            let target_type = walk.walk_type_expression(declaration.target_type)?;
            let origin = Origin::Node(source, walk.flow().template_scope());
            let target = walk.walk_extension_target(origin, target_type)?;

            // name the target for the extension's diagnostics
            let target_name = match &target {
                dir::ExtensionTarget::Rooted { root, .. } => match root {
                    dir::TypeRoot::Declaration(symbol) => walk.check.format_symbol(*symbol),
                    dir::TypeRoot::Primitive(primitive) => primitive.as_str(),
                    dir::TypeRoot::Tuple => "tuple".into(),
                },
                dir::ExtensionTarget::Blanket { .. } => walk.check.format_type(target_type),
            };
            let ownership = walk.check.default_ownership(origin, target_type)?;
            let receiver = Receiver {
                declaration: Some(symbol),
                ownership,
                ty: target_type,
                super_ty: None,
            };
            walk.with_receiver_scope(Some(receiver), |walk| {
                // walk implemented interfaces
                let mut implements = Vec::new();
                for implemented_type in &declaration.implements_types {
                    let negated = walk.negated_implements(*implemented_type);
                    let heritage_annotation = negated.unwrap_or(*implemented_type);
                    let ty = walk.walk_type_expression(heritage_annotation)?;

                    // classify the clause against the extension target
                    if let Some(conformance) = walk.implemented_conformance(
                        symbol,
                        &target_name,
                        *implemented_type,
                        heritage_annotation,
                        ty,
                        negated.is_some(),
                    )? {
                        implements.push(conformance);
                    }
                }

                // walk members
                let mut members = Vec::new();
                for member in &declaration.members {
                    if let Some(definition) = walk.walk_member_header(
                        *member,
                        walk.tree.get(*member),
                        Some(receiver),
                        Some(induction),
                        declaration.is_ambient,
                    )? {
                        members.push(definition);
                    }
                }

                // export the extension outside this module
                let form = if declaration.export.is_some() {
                    dir::ExtensionForm::Exported
                } else {
                    dir::ExtensionForm::Local
                };

                // commit the extension definition
                let template = walk.induced_owner_template(induction, template)?;
                let definition = dir::Definition::Extension(dir::ExtensionDefinition {
                    symbol,
                    form,
                    template: template.map(|template| template.local_id),
                    target,
                    implements,
                    members,
                });
                walk.check.insert_definition(symbol, source, definition)?;

                Ok(())
            })
        })
    }

    /// Walk one function declaration.
    fn walk_function_item_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
    ) -> CompilerResult<()> {
        let symbol = self.declared_symbol(id.into_any());
        let Some(symbol) = symbol else {
            // validate local signatures without a declaration symbol
            let previous = self.induced_owner.take();
            let result =
                self.walk_function_signature(None, &declaration.signature, declaration.is_ambient);
            self.induced_owner = previous;
            result?;

            return Ok(());
        };

        // open signature parameters before building the function type
        let source = id.into_global_any(self.module);
        let induction = InducedParameterOwner::new(source, None, Some(symbol));
        self.induced_owner = Some(induction);
        let template = self.open_signature_template(source, &declaration.signature)?;

        // walk the declared signature header
        let (header, result, tracked) = self.walk_signature_header(
            id.into_any(),
            template,
            &declaration.signature,
            declaration.body,
            declaration.is_ambient,
        )?;

        // require a body unless the declaration is ambient
        if declaration.body.is_none() && !declaration.is_ambient {
            let source = id.into_global_any(self.module);
            let name = self.check.format_symbol(symbol);
            self.check.report_missing_declaration_body(source, name);
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
            // infer a lambda's receiver access from its body
            let receiver = match declaration.signature.form {
                dir::FunctionForm::Lambda => {
                    let origin = Origin::Node(id.into_global_any(self.module), None);
                    self.check
                        .open_memory_type(origin, dir::MemoryParameter::Access)?
                }
                dir::FunctionForm::Function => {
                    self.check.receiver_literal(dir::ReceiverMode::Borrowed {
                        access: dir::Access::Readonly,
                    })?
                }
            };

            self.push_function_value_type(signature, receiver)?
        } else {
            signature
        };
        self.commit_symbol_type(symbol, function)?;

        Ok(())
    }

    /// Walk one enum field and return its scalar variant.
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
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(WalkedEnumVariant::Invalid);
        };

        // evaluate the scalar value before committing its member state
        let Some(value) = self.evaluate_enum_variant_value(id, value, implicit)? else {
            let error = self.intern_type(dir::Type::Error)?;
            self.commit_symbol_type(symbol, error)?;

            return Ok(WalkedEnumVariant::Invalid);
        };

        Ok(WalkedEnumVariant::Present(dir::EnumVariantDefinition {
            symbol,
            source: id.into_global_any(self.module),
            key: name.into(),
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
            // evaluate a written value
            Some(expression) => {
                let static_type = self.walk_static_term(expression)?;
                let origin = Origin::Node(
                    expression.into_global_any(self.module),
                    self.flow().template_scope(),
                );
                let static_type = self.check.normalize(origin, static_type)?;
                let value = match self.check.ty(static_type)? {
                    dir::Type::Literal(dir::Literal::Integer(value)) => {
                        dir::EnumVariantValue::Integer(value)
                    }
                    dir::Type::Literal(dir::Literal::String(value)) => {
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
            // take the value implied by the preceding variant
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

    /// Walk one where clause onto its declaring template, or check it at the declaration.
    pub(in crate::sema) fn walk_where_clause(
        &mut self,
        template: Option<GenericTemplateId>,
        id: dir::LocalNodeId<dir::WhereClause>,
    ) -> CompilerResult<()> {
        // walk the operands, elided bound borrows inducing on the declaring template
        let clause = self.tree.get(id);
        let (relation, left, right) = (clause.relation, clause.left, clause.right);
        let (left, right) = match template {
            Some(template) => self.with_template_owner(template, |walk| {
                let left = walk.walk_type_expression_in(left, ElisionSite::Signature)?;
                let right = walk.walk_type_expression_in(right, ElisionSite::Signature)?;

                Ok((left, right))
            })?,
            None => (
                self.walk_type_expression(left)?,
                self.walk_type_expression(right)?,
            ),
        };

        // require a lifetime bound to name exactly one lifetime
        if self.check.memory_kind(left)? == Some(dir::MemoryParameter::Region)
            && matches!(self.check.ty(right)?, dir::Type::Union(_))
        {
            self.check
                .report_disjunctive_lifetime_bound(id.into_global_any(self.module))?;

            return Ok(());
        }

        // require the clause to bound one parameter of the declaration
        let bounds_parameter = self.check.type_flags(left)?.has_parameter()
            || self.check.type_flags(right)?.has_parameter()
            || self.check.memory_kind(left)? == Some(dir::MemoryParameter::Region);
        let Some(template) = template.filter(|_| bounds_parameter) else {
            self.check
                .report_where_clause_without_parameter(id.into_global_any(self.module))?;

            return Ok(());
        };

        // record the predicate relation for the template to check later
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
    pub(in crate::sema) fn walk_function_signature(
        &mut self,
        template: Option<GenericTemplateId>,
        signature: &dir::FunctionSignature,
        is_ambient: bool,
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

        // require annotations on every declared callable parameter
        let is_annotation_required = signature.form != dir::FunctionForm::Lambda;

        // walk the receiver and value parameters
        let this_parameter = match signature.this_parameter {
            Some(parameter) => self.walk_parameter(
                parameter,
                self.tree.get(parameter),
                is_annotation_required,
                is_ambient,
            )?,
            None => None,
        };
        let mut parameters = Vec::new();
        for parameter in &signature.parameters {
            let Some(ty) = self.walk_parameter(
                *parameter,
                self.tree.get(*parameter),
                is_annotation_required,
                is_ambient,
            )?
            else {
                continue;
            };
            parameters.push(self.function_parameter_type(*parameter, ty)?);
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
    pub(in crate::sema) fn open_signature_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        signature: &dir::FunctionSignature,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if !signature.declares_generic_scope() {
            return Ok(None);
        }

        self.check
            .open_generic_template(source, self.flow().template_scope())
            .map(Some)
    }

    /// Return the receiver introduced by one `this` parameter.
    pub(in crate::sema) fn this_parameter_receiver_binding(
        &mut self,
        parameter: dir::LocalNodeId<dir::Parameter>,
        scope: Option<Receiver>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<ReceiverBinding> {
        // read the receiver binding
        let Some(symbol) = self.declared_symbol(parameter.into_any()) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "this parameter {:?} has no declaration symbol",
                    parameter.into_global(self.module)
                ),
            });
        };

        // apply the surrounding receiver scope to the written type
        let origin = Origin::Node(
            parameter.into_global_any(self.module),
            self.flow().template_scope(),
        );
        let ty = self.apply_receiver_scope(origin, scope, ty)?;
        let receiver = self.receiver_with_super(Receiver {
            declaration: scope.and_then(|scope| scope.declaration),
            ownership: scope.and_then(|scope| scope.ownership),
            ty,
            super_ty: scope.and_then(|scope| scope.super_ty),
        })?;
        self.commit_receiver_symbol_type(symbol, receiver.ty)?;

        Ok(ReceiverBinding { symbol, receiver })
    }

    /// Resolve `this` in one member type to the object the active receiver scope names.
    pub(in crate::sema) fn apply_receiver_scope(
        &mut self,
        origin: Origin,
        scope: Option<Receiver>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(scope) = scope else {
            return Ok(ty);
        };
        let object = self.check.strip_form(origin, scope.ty)?;
        let substitution = TypeSubstitution::default().with_receiver(object);

        self.check.substitute_type(ty, &substitution)
    }

    /// Walk one callable header under the template it declares.
    pub(in crate::sema) fn walk_signature_header(
        &mut self,
        source: dir::LocalNodeIdAny,
        template: Option<GenericTemplateId>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        is_ambient: bool,
    ) -> CompilerResult<(
        FunctionHeader,
        Option<dir::GlobalTypeId>,
        Vec<dir::GlobalTypeId>,
    )> {
        // walk the whole header under the template it declares
        self.with_template_scope(template, |walk| {
            let header = walk.walk_function_signature(template, signature, is_ambient)?;
            let (result, tracked) = walk.walk_function_result_type(source, signature, body)?;

            Ok((header, result, tracked))
        })
    }

    /// Walk one function return annotation or open its inferred result.
    pub(in crate::sema) fn walk_function_result_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Vec<dir::GlobalTypeId>)> {
        // use explicit return annotations
        if let Some(return_type) = signature.return_type {
            let (result, tracked) = self.walk_return_type_expression(return_type)?;

            return Ok((Some(result), tracked));
        }

        // setters return void by role
        if signature.role == Some(dir::FunctionRole::Setter) {
            return Ok((Some(self.intern_type(dir::Type::Void)?), Vec::new()));
        }

        // skip ambient signatures
        if body.is_none() {
            return Ok((None, Vec::new()));
        }

        // open a contextual result slot for lambdas and constructor roles
        let infers = signature.form == dir::FunctionForm::Lambda
            || matches!(
                signature.role,
                Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
            );
        if infers {
            let result = self.open_type_hole(source, VariableKind::Type)?;

            // join the values an inferred result's returns produce
            if let Some(variable) = self.check.root_variable(result)? {
                self.check.infer.variable_mut(variable)?.is_join = true;
            }

            return Ok((Some(result), Vec::new()));
        }

        // require a written result type on every named declaration
        if self.check.is_declaring() {
            let anchor = self.check.diagnostic_anchor(self.module, source);
            let error = CheckError::MissingResultType {
                anchor,
                module: self.module,
            };
            self.check.report(self.module, error);
        }

        Ok((Some(self.intern_type(dir::Type::Error)?), Vec::new()))
    }

    /// Return one nominal declaration receiver scope.
    pub(in crate::sema) fn nominal_receiver(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ownership: Option<dir::Ownership>,
    ) -> CompilerResult<Receiver> {
        // apply the declaration's own parameters as arguments
        let parameters = match self.check.template_by_symbol(symbol)? {
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

    /// Return the nominal instance one heritage annotation names.
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
    ) -> CompilerResult<()> {
        let origin = Origin::Node(
            source.into_global_any(self.module),
            self.flow().template_scope(),
        );
        let clause = source.into_global_any(self.module);

        // relate the declaration to its heritage clause
        self.relate_type(
            origin,
            CauseKind::Heritage { clause },
            relation,
            declared,
            heritage,
        )
    }

    /// Return one extension target from a walked target annotation.
    fn walk_extension_target(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ExtensionTarget> {
        // root annotated memory forms at their payload's default ownership
        let mut payload = ty;
        let mut form_root = None;
        loop {
            // peel form heads, keeping the outermost constructor as the root fallback
            if let dir::Type::Form(form) = self.check.ty(payload)? {
                let symbol = self.check.language_symbol(form.form.language_item())?;
                form_root.get_or_insert(symbol);
                payload = form.value;

                continue;
            }

            // peel memory accessor applications to their inspected target
            let dir::Type::Application(instance) = self.check.ty(payload)? else {
                break;
            };
            let Some(item) = self.check.language_item(instance.symbol)? else {
                break;
            };
            if !matches!(item, dir::LanguageItem::WithAccess) {
                break;
            }
            let Some(carried) = self
                .check
                .type_ids(payload.module_id, instance.arguments)?
                .first()
                .copied()
            else {
                break;
            };

            form_root.get_or_insert(instance.symbol);
            payload = carried;
        }

        // prefer the concrete nominal beneath memory forms
        let target_instance = self.check.apparent_instance(payload)?;
        let chain = self.check.form_chain(origin, payload)?;
        let value_instance = self.check.apparent_instance(chain.base())?;
        if let Some(root) = value_instance
            .or(target_instance)
            .map(|instance| instance.symbol)
            .or(form_root)
        {
            return Ok(dir::ExtensionTarget::Rooted {
                root: dir::TypeRoot::Declaration(root),
                ty,
            });
        }

        // root structural constructors at their language items
        if let Some(item) = self.check.ty(chain.base())?.member_owner_item() {
            let root = self.check.language_symbol(item)?;

            return Ok(dir::ExtensionTarget::Rooted {
                root: dir::TypeRoot::Declaration(root),
                ty,
            });
        }

        // root the remaining primitives and tuples at their structural constructor
        if let Some(root) = self.check.structural_root(chain.base())? {
            return Ok(dir::ExtensionTarget::Rooted { root, ty });
        }

        // require a root declaration, or a blanket over a bare parameter
        if !matches!(self.check.ty(chain.base())?, dir::Type::Parameter(_)) {
            self.report_invalid_extension_target(origin, ty)?;
        }
        let coverage = self.blanket_coverage(origin, ty)?;

        Ok(dir::ExtensionTarget::Blanket { ty, coverage })
    }

    /// Classify the receiver coverage one blanket's declared bound decides.
    fn blanket_coverage(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<dir::BlanketCoverage> {
        // defer where-predicated blankets to their use sites
        let template = self.check.origin_scope(origin)?;
        if !self.check.template_predicates(template)?.is_empty() {
            return Ok(dir::BlanketCoverage::Deferred);
        }
        let dir::Type::Parameter(parameter) = self.check.ty(target)? else {
            return Ok(dir::BlanketCoverage::Deferred);
        };

        // read the target parameter's bound
        let Some(template) = template else {
            return Ok(dir::BlanketCoverage::Deferred);
        };
        let Some(binding) = self.check.generic_parameter(parameter)? else {
            return Ok(dir::BlanketCoverage::Deferred);
        };
        let constraint = binding.constraint;

        // defer a blanket whose target bound leaves a constrained secondary parameter free
        for secondary in self.check.generic_template_parameters(template)? {
            if secondary == parameter {
                continue;
            }
            let constrained = self
                .check
                .generic_parameter(secondary)?
                .is_none_or(|binding| binding.constraint.is_some() && binding.default.is_none());
            let determined = match constraint {
                Some(constraint) => self.check.has_parameter_occurrence(constraint, secondary)?,
                None => false,
            };
            if constrained && !determined {
                return Ok(dir::BlanketCoverage::Deferred);
            }
        }

        // unbounded targets cover every receiver
        let Some(constraint) = constraint else {
            return Ok(dir::BlanketCoverage::Every);
        };

        // interface bounds cover their conforming receivers
        let dir::Type::Application(instance) = self.check.ty(constraint)? else {
            return Ok(dir::BlanketCoverage::Deferred);
        };
        let interface = instance.symbol;
        if !self.check.symbol_kind(interface)?.is_interface() {
            return Ok(dir::BlanketCoverage::Deferred);
        }

        Ok(dir::BlanketCoverage::Interface(interface))
    }
}
