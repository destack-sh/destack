use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition, Definition,
    EnumDefinition, ExtensionDefinition, ExtensionTarget, ExtensionWhereClause, FieldDefinition,
    GenericInstanceKey, InterfaceDefinition, MethodDefinition, NewtypeDefinition, NominalHeritage,
    SignatureDefinition, StructDefinition, TypeAliasDefinition, TypeOperand, VariantDefinition,
};
use crate::{CompilerError, CompilerResult};

/// Member definitions built from one declaration body.
#[derive(Debug, Default)]
pub(in crate::check) struct MemberDefinitions {
    /// The instance fields.
    pub(in crate::check) fields: Vec<FieldDefinition>,
    /// The static fields.
    pub(in crate::check) static_fields: Vec<FieldDefinition>,
    /// The nominal methods.
    pub(in crate::check) methods: Vec<MethodDefinition>,
    /// The static methods.
    pub(in crate::check) static_methods: Vec<MethodDefinition>,
    /// The symbol-free call signatures.
    pub(in crate::check) call_signatures: Vec<SignatureDefinition>,
    /// The symbol-free construct signatures.
    pub(in crate::check) construct_signatures: Vec<SignatureDefinition>,
    /// The symbol-free index signatures.
    pub(in crate::check) index_signatures: Vec<SignatureDefinition>,
    /// The associated types.
    pub(in crate::check) associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    pub(in crate::check) associated_consts: Vec<AssociatedConstDefinition>,
}

impl CheckState<'_> {
    /// Build checked declaration definitions from walked operands.
    pub(in crate::check) fn build_definition_table(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // build definitions in component order
        for module in modules {
            let symbols = self.definition_symbols(module);
            for symbol in symbols {
                let definition = self.build_definition(module, symbol)?;
                self.definitions.insert(symbol, definition);
            }
        }

        Ok(())
    }

    /// Build one checked declaration definition.
    fn build_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Definition> {
        let source = self
            .module(module)
            .symbol_declaration_node(symbol.local_id)
            .into_global(module);
        let template = self.inference.symbol_generic_template(symbol);
        let source_id = source.local_id.into_typed::<dir::Declaration>();
        let declaration = self.module(module).view().get(source_id).clone();

        match declaration {
            // build struct declaration
            dir::Declaration::Struct(declaration) => {
                let members = self.build_members(module, &declaration.members)?;
                let implements = self.build_nominal_heritages(
                    module,
                    declaration
                        .implements_types
                        .iter()
                        .map(|node| node.into_any()),
                );

                Ok(Definition::Struct(StructDefinition {
                    source,
                    template,
                    implements,
                    fields: members.fields,
                    static_fields: members.static_fields,
                    methods: members.methods,
                    static_methods: members.static_methods,
                    associated_types: members.associated_types,
                    associated_consts: members.associated_consts,
                }))
            }
            // build class declaration
            dir::Declaration::Class(declaration) => {
                let members = self.build_members(module, &declaration.members)?;
                let extends = declaration
                    .extends_type
                    .and_then(|node| self.build_nominal_heritage(module, node.into_any()));
                let implements = self.build_nominal_heritages(
                    module,
                    declaration
                        .implements_types
                        .iter()
                        .map(|node| node.into_any()),
                );

                Ok(Definition::Class(ClassDefinition {
                    source,
                    template,
                    extends,
                    implements,
                    fields: members.fields,
                    static_fields: members.static_fields,
                    methods: members.methods,
                    static_methods: members.static_methods,
                    associated_types: members.associated_types,
                    associated_consts: members.associated_consts,
                }))
            }
            // build interface declaration
            dir::Declaration::Interface(declaration) => {
                let members = self.build_type_members(module, &declaration.members)?;
                let extends = self.build_nominal_heritages(
                    module,
                    declaration.extends_types.iter().map(|node| node.into_any()),
                );

                Ok(Definition::Interface(InterfaceDefinition {
                    source,
                    template,
                    is_nominal: declaration.is_nominal,
                    extends,
                    fields: members.fields,
                    static_fields: members.static_fields,
                    methods: members.methods,
                    static_methods: members.static_methods,
                    call_signatures: members.call_signatures,
                    construct_signatures: members.construct_signatures,
                    index_signatures: members.index_signatures,
                    associated_types: members.associated_types,
                    associated_consts: members.associated_consts,
                }))
            }
            // build enum declaration
            dir::Declaration::Enum(declaration) => {
                let members = self.build_members(module, &declaration.members)?;
                let implements = self.build_nominal_heritages(
                    module,
                    declaration
                        .implements_types
                        .iter()
                        .map(|node| node.into_any()),
                );
                let variants = self.build_enum_variants(module, &declaration.fields)?;

                Ok(Definition::Enum(EnumDefinition {
                    source,
                    template,
                    implements,
                    variants,
                    static_fields: members.static_fields,
                    methods: members.methods,
                    static_methods: members.static_methods,
                    associated_types: members.associated_types,
                    associated_consts: members.associated_consts,
                }))
            }
            // build extension declaration
            dir::Declaration::Extension(_) => {
                let definition: ExtensionDefinition =
                    self.build_extension_definition(module, symbol)?;

                Ok(Definition::Extension(definition))
            }
            // build type alias or newtype declaration
            dir::Declaration::Type(declaration) => {
                let value = self.node_type_operand(declaration.value.into_global_any(module))?;

                if declaration.is_nominal {
                    Ok(Definition::Newtype(NewtypeDefinition {
                        source,
                        template,
                        value,
                    }))
                } else {
                    Ok(Definition::TypeAlias(TypeAliasDefinition {
                        source,
                        template,
                        value,
                    }))
                }
            }
            _ => Err(CompilerError::Internal {
                message: format!(
                    "definition symbol {symbol:?} does not point at a declaration definition"
                ),
            }),
        }
    }

    /// Build one extension definition.
    fn build_extension_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ExtensionDefinition> {
        let source = self
            .module(module)
            .symbol_declaration_node(symbol.local_id)
            .into_global(module);
        let declaration_id = source.local_id.into_typed::<dir::Declaration>();
        let declaration = self.module(module).view().get(declaration_id).clone();
        let dir::Declaration::Extension(declaration) = declaration else {
            return Err(CompilerError::Internal {
                message: format!(
                    "extension symbol {symbol:?} does not point at an extension declaration"
                ),
            });
        };

        // build the receiver target
        let ty = self.node_type_operand(declaration.target_type.into_global_any(module))?;
        let target = match self.type_operand_nominal_symbol(ty)? {
            Some(root) => ExtensionTarget::Nominal { root, ty },
            None => ExtensionTarget::Blanket { ty },
        };
        let form = Self::extension_form(module, &declaration, target);

        // build clauses and members
        let where_clauses =
            self.build_extension_where_clauses(module, &declaration.where_clauses)?;
        let implements = self.build_nominal_heritages(
            module,
            declaration
                .implements_types
                .iter()
                .map(|node| node.into_any()),
        );
        let members = self.build_members(module, &declaration.members)?;

        Ok(ExtensionDefinition {
            source,
            form,
            target,
            implements,
            where_clauses,
            fields: members.fields,
            static_fields: members.static_fields,
            methods: members.methods,
            static_methods: members.static_methods,
            associated_types: members.associated_types,
            associated_consts: members.associated_consts,
        })
    }

    /// Build extension where clauses.
    fn build_extension_where_clauses(
        &self,
        module: ModuleId,
        clauses: &[dir::LocalNodeId<dir::WhereClause>],
    ) -> CompilerResult<Vec<ExtensionWhereClause>> {
        let mut where_clauses = Vec::with_capacity(clauses.len());
        let view = self.module(module).view();

        // collect checked where clause operands
        for clause in clauses {
            let source = clause.into_global_any(module);
            let clause = view.get(*clause);
            let left = self.node_type_operand(clause.left.into_global_any(module))?;
            let right = self.node_type_operand(clause.right.into_global_any(module))?;

            where_clauses.push(ExtensionWhereClause {
                source,
                left,
                right,
            });
        }

        Ok(where_clauses)
    }

    /// Return how one extension should be made visible.
    fn extension_form(
        module: ModuleId,
        declaration: &dir::ExtensionDeclaration,
        target: ExtensionTarget,
    ) -> dir::ExtensionForm {
        // same module extensions are inherent
        if target
            .nominal_root()
            .is_some_and(|symbol| symbol.module_id == module)
        {
            return dir::ExtensionForm::Inherent;
        }

        // named foreign extensions are imported explicitly
        if declaration.name.is_some() {
            return dir::ExtensionForm::Named;
        }

        dir::ExtensionForm::Local
    }

    /// Return definition symbols declared in one component module.
    fn definition_symbols(&self, module: ModuleId) -> Vec<dir::GlobalSymbolId> {
        let state = self.module(module);
        let bindings = state.binding_table();
        let mut symbols = Vec::new();

        // collect declaration definition symbols in source order
        for (source, symbol) in bindings.declaration_symbols() {
            if source.module_id != module || source.local_id.ty != dir::NodeType::Declaration {
                continue;
            }
            let symbol = symbol.into_global(module);
            let kind = bindings.get_symbol(symbol.local_id).kind;

            if kind.is_definition() {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Build nominal heritages from source nodes.
    pub(in crate::check) fn build_nominal_heritages(
        &self,
        module: ModuleId,
        nodes: impl Iterator<Item = dir::LocalNodeIdAny>,
    ) -> Vec<NominalHeritage> {
        nodes
            .filter_map(|node| self.build_nominal_heritage(module, node))
            .collect()
    }

    /// Build one nominal heritage from a source node.
    fn build_nominal_heritage(
        &self,
        module: ModuleId,
        node: dir::LocalNodeIdAny,
    ) -> Option<NominalHeritage> {
        let source = node.into_global(module);
        let symbol = self.inference.name(source)?.symbol();
        let instance = self
            .inference
            .symbol_generic_template(symbol)
            .and_then(|template| {
                let key = GenericInstanceKey::new(source, template);

                self.inference.generic_instance(key).cloned()
            });

        Some(NominalHeritage {
            source,
            symbol,
            instance,
        })
    }

    /// Build declaration-body nominal members.
    pub(in crate::check) fn build_members(
        &mut self,
        module: ModuleId,
        members: &[dir::LocalNodeId<dir::Member>],
    ) -> CompilerResult<MemberDefinitions> {
        let mut definitions = MemberDefinitions::default();

        // build members in source order
        for member_id in members {
            let member = self.module(module).view().get(*member_id).clone();
            let source = member_id.into_global_any(module);

            self.build_member(module, source, &member, &mut definitions)?;
        }

        Ok(definitions)
    }

    /// Build one declaration-body nominal member.
    fn build_member(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        member: &dir::Member,
        definitions: &mut MemberDefinitions,
    ) -> CompilerResult<()> {
        match member {
            // build associated type
            dir::Member::AssociatedType {
                name,
                constraint,
                value,
                ..
            } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let key = dir::StaticKey::Name(*name);
                let constraint = constraint
                    .map(|constraint| self.node_type_operand(constraint.into_global_any(module)))
                    .transpose()?;
                let value = value
                    .map(|value| self.node_type_operand(value.into_global_any(module)))
                    .transpose()?;

                definitions.associated_types.push(AssociatedTypeDefinition {
                    symbol,
                    source,
                    key,
                    constraint,
                    value,
                });
            }
            // build associated const
            dir::Member::AssociatedConst { name, .. } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let key = dir::StaticKey::Name(*name);
                let ty = self.symbol_type_operand(module, symbol)?;
                let value = self.inputs.symbol_static(symbol);

                definitions
                    .associated_consts
                    .push(AssociatedConstDefinition {
                        symbol,
                        source,
                        key,
                        ty,
                        value,
                    });
            }
            // build field or static field
            dir::Member::Field { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.symbol_type_operand(module, symbol)?;
                let field = FieldDefinition {
                    symbol,
                    source,
                    key,
                    ty,
                };

                if *is_static {
                    definitions.static_fields.push(field);
                } else {
                    definitions.fields.push(field);
                }
            }
            // build method
            dir::Member::Method {
                is_static,
                signature,
                ..
            } => {
                let Some(slot) = member.slot() else {
                    return Ok(());
                };
                let symbol = self.module(module).declaration_symbol(source.local_id);
                let ty = self.nominal_member_type_operand(module, source, symbol)?;

                let method = MethodDefinition {
                    symbol,
                    source,
                    slot,
                    role: signature.role,
                    ty,
                };

                if *is_static {
                    definitions.static_methods.push(method);
                } else {
                    definitions.methods.push(method);
                }
            }
            // skip blocks and parse recovery
            dir::Member::StaticBlock { .. }
            | dir::Member::ComptimeBlock { .. }
            | dir::Member::Error => {}
        }

        Ok(())
    }

    /// Build interface members.
    fn build_type_members(
        &mut self,
        module: ModuleId,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<MemberDefinitions> {
        let mut definitions = MemberDefinitions::default();

        // build members in source order
        for member_id in members {
            let member = self.module(module).view().get(*member_id).clone();
            let source = member_id.into_global_any(module);

            self.build_type_member(module, source, &member, &mut definitions)?;
        }

        Ok(definitions)
    }

    /// Build one interface member.
    fn build_type_member(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        member: &dir::TypeMember,
        definitions: &mut MemberDefinitions,
    ) -> CompilerResult<()> {
        match member {
            // build field or static field
            dir::TypeMember::Field { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.symbol_type_operand(module, symbol)?;
                let field = FieldDefinition {
                    symbol,
                    source,
                    key,
                    ty,
                };

                if *is_static {
                    definitions.static_fields.push(field);
                } else {
                    definitions.fields.push(field);
                }
            }
            // build method
            dir::TypeMember::Method {
                key,
                is_static,
                signature,
                ..
            } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.symbol_type_operand(module, symbol)?;

                let method = MethodDefinition {
                    symbol: Some(symbol),
                    source,
                    slot: dir::MemberSlot::Key(key),
                    role: signature.role,
                    ty,
                };

                if *is_static {
                    definitions.static_methods.push(method);
                } else {
                    definitions.methods.push(method);
                }
            }
            // build call signature
            dir::TypeMember::CallSignature { .. } => {
                let ty = self.node_type_operand(source)?;

                definitions
                    .call_signatures
                    .push(SignatureDefinition { source, ty });
            }
            // build construct signature
            dir::TypeMember::ConstructSignature { .. } => {
                let ty = self.node_type_operand(source)?;

                definitions
                    .construct_signatures
                    .push(SignatureDefinition { source, ty });
            }
            // build index signature
            dir::TypeMember::IndexSignature { .. } => {
                let ty = self.node_type_operand(source)?;

                definitions
                    .index_signatures
                    .push(SignatureDefinition { source, ty });
            }
            // build associated type
            dir::TypeMember::AssociatedType {
                name,
                constraint,
                value,
                ..
            } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let key = dir::StaticKey::Name(*name);
                let constraint = constraint
                    .map(|constraint| self.node_type_operand(constraint.into_global_any(module)))
                    .transpose()?;
                let value = value
                    .map(|value| self.node_type_operand(value.into_global_any(module)))
                    .transpose()?;

                definitions.associated_types.push(AssociatedTypeDefinition {
                    symbol,
                    source,
                    key,
                    constraint,
                    value,
                });
            }
            // build associated const
            dir::TypeMember::AssociatedConst { name, .. } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let key = dir::StaticKey::Name(*name);
                let ty = self.symbol_type_operand(module, symbol)?;
                let value = self.inputs.symbol_static(symbol);

                definitions
                    .associated_consts
                    .push(AssociatedConstDefinition {
                        symbol,
                        source,
                        key,
                        ty,
                        value,
                    });
            }
            // skip parse recovery
            dir::TypeMember::Error => {}
        }

        Ok(())
    }

    /// Build enum variants in source order.
    fn build_enum_variants(
        &self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::EnumField>],
    ) -> CompilerResult<Vec<VariantDefinition>> {
        let mut variants = Vec::new();

        // build variants in source order
        for field in fields {
            let source = field.into_global_any(module);
            let symbol = self.declaration_symbol_at(module, source.local_id)?;
            let key = self.module(module).view().get(*field).name.static_key();
            let value = self.inputs.symbol_static(symbol);

            variants.push(VariantDefinition {
                symbol,
                source,
                key,
                value,
            });
        }

        Ok(variants)
    }

    /// Return the symbol declared by one source node.
    fn declaration_symbol_at(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let Some(symbol) = self.module(module).declaration_symbol(source) else {
            return Err(CompilerError::Internal {
                message: format!("nominal source node {source:?} declares no symbol"),
            });
        };

        Ok(symbol)
    }

    /// Return one nominal member type operand.
    fn nominal_member_type_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<TypeOperand> {
        if let Some(symbol) = symbol {
            return Ok(self.symbol_type_operand(module, symbol)?);
        }

        self.node_type_operand(source)
    }
}
