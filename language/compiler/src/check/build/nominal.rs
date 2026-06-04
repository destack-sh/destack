use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    AssociatedConstDefinition, AssociatedTypeDefinition, CheckState, ClassDefinition,
    EnumDefinition, FieldDefinition, GenericInstanceKey, InterfaceDefinition, MethodDefinition,
    NewtypeDefinition, NominalDefinition, NominalHeritage, SignatureDefinition, StructDefinition,
    TypeOperand, VariantDefinition,
};
use crate::{CompilerError, CompilerResult};

/// Member definitions built from one nominal body.
#[derive(Debug, Default)]
struct NominalMembers {
    /// The instance fields.
    fields: Vec<FieldDefinition>,
    /// The static fields.
    static_fields: Vec<FieldDefinition>,
    /// The nominal methods.
    methods: Vec<MethodDefinition>,
    /// The static methods.
    static_methods: Vec<MethodDefinition>,
    /// The symbol-free call signatures.
    call_signatures: Vec<SignatureDefinition>,
    /// The symbol-free construct signatures.
    construct_signatures: Vec<SignatureDefinition>,
    /// The symbol-free index signatures.
    index_signatures: Vec<SignatureDefinition>,
    /// The associated types.
    associated_types: Vec<AssociatedTypeDefinition>,
    /// The associated constants.
    associated_consts: Vec<AssociatedConstDefinition>,
}

impl CheckState<'_> {
    /// Build nominal declarations from walked operands.
    pub(in crate::check) fn build_nominal_table(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // build definitions in component order
        for module in modules {
            let symbols = self.nominal_symbols(module);
            for symbol in symbols {
                let definition = self.build_nominal_definition(module, symbol)?;
                self.nominals.insert_definition(symbol, definition);
            }
        }

        Ok(())
    }

    /// Build one nominal definition.
    fn build_nominal_definition(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<NominalDefinition> {
        let source = self
            .module(module)
            .symbol_declaration_node(symbol.local_id)
            .into_global(module);
        let template = self.inference.owner_generic_template(symbol).cloned();
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

                Ok(NominalDefinition::Struct(StructDefinition {
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

                Ok(NominalDefinition::Class(ClassDefinition {
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
            // build nominal interface declaration
            dir::Declaration::Interface(declaration) if declaration.is_nominal => {
                let members = self.build_type_members(module, &declaration.members)?;
                let extends = self.build_nominal_heritages(
                    module,
                    declaration.extends_types.iter().map(|node| node.into_any()),
                );

                Ok(NominalDefinition::Interface(InterfaceDefinition {
                    source,
                    template,
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

                Ok(NominalDefinition::Enum(EnumDefinition {
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
            // build newtype declaration
            dir::Declaration::Type(declaration) if declaration.is_nominal => {
                let value = self.node_type_operand(declaration.value.into_global_any(module))?;

                Ok(NominalDefinition::Newtype(NewtypeDefinition {
                    source,
                    template,
                    value,
                }))
            }
            _ => Err(CompilerError::Internal {
                message: format!(
                    "nominal symbol {symbol:?} does not point at a nominal declaration"
                ),
            }),
        }
    }

    /// Return nominal symbols declared in one component module.
    fn nominal_symbols(&self, module: ModuleId) -> Vec<dir::GlobalSymbolId> {
        let state = self.module(module);
        let bindings = state.binding_table();
        let mut symbols = Vec::new();

        // collect nominal declaration symbols in source order
        for (source, symbol) in bindings.declaration_symbols() {
            if source.module_id != module || source.local_id.ty != dir::NodeType::Declaration {
                continue;
            }
            let symbol = symbol.into_global(module);
            let kind = bindings.get_symbol(symbol.local_id).kind;

            if kind.is_nominal() {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Build nominal heritages from source nodes.
    fn build_nominal_heritages(
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
        let key = GenericInstanceKey {
            source,
            owner: symbol,
        };
        let instance = self.inference.generic_instance(key).cloned();

        Some(NominalHeritage {
            source,
            symbol,
            instance,
        })
    }

    /// Build declaration-body nominal members.
    fn build_members(
        &mut self,
        module: ModuleId,
        members: &[dir::LocalNodeId<dir::Member>],
    ) -> CompilerResult<NominalMembers> {
        let mut definitions = NominalMembers::default();

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
        definitions: &mut NominalMembers,
    ) -> CompilerResult<()> {
        match member {
            // build associated type
            dir::Member::AssociatedType {
                constraint, value, ..
            } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let constraint = constraint
                    .map(|constraint| self.node_type_operand(constraint.into_global_any(module)))
                    .transpose()?;
                let value = value
                    .map(|value| self.node_type_operand(value.into_global_any(module)))
                    .transpose()?;

                definitions.associated_types.push(AssociatedTypeDefinition {
                    symbol,
                    source,
                    constraint,
                    value,
                });
            }
            // build associated const
            dir::Member::AssociatedConst { .. } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.import_symbol_type_operand(module, symbol)?;
                let value = self.inputs.symbol_static(symbol);

                definitions
                    .associated_consts
                    .push(AssociatedConstDefinition {
                        symbol,
                        source,
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
                let ty = self.import_symbol_type_operand(module, symbol)?;
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
            dir::Member::Method { is_static, .. } => {
                let Some(slot) = member.slot() else {
                    return Ok(());
                };
                let symbol = self.module(module).declaration_symbol(source.local_id);
                let ty = self.nominal_member_type_operand(module, source, symbol)?;

                let method = MethodDefinition {
                    symbol,
                    source,
                    slot,
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
    ) -> CompilerResult<NominalMembers> {
        let mut definitions = NominalMembers::default();

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
        definitions: &mut NominalMembers,
    ) -> CompilerResult<()> {
        match member {
            // build field or static field
            dir::TypeMember::Field { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.import_symbol_type_operand(module, symbol)?;
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
            dir::TypeMember::Method { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.import_symbol_type_operand(module, symbol)?;

                let method = MethodDefinition {
                    symbol: Some(symbol),
                    source,
                    slot: dir::MemberSlot::Key(key),
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
                constraint, value, ..
            } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let constraint = constraint
                    .map(|constraint| self.node_type_operand(constraint.into_global_any(module)))
                    .transpose()?;
                let value = value
                    .map(|value| self.node_type_operand(value.into_global_any(module)))
                    .transpose()?;

                definitions.associated_types.push(AssociatedTypeDefinition {
                    symbol,
                    source,
                    constraint,
                    value,
                });
            }
            // build associated const
            dir::TypeMember::AssociatedConst { .. } => {
                let symbol = self.declaration_symbol_at(module, source.local_id)?;
                let ty = self.import_symbol_type_operand(module, symbol)?;
                let value = self.inputs.symbol_static(symbol);

                definitions
                    .associated_consts
                    .push(AssociatedConstDefinition {
                        symbol,
                        source,
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
            return Ok(self.import_symbol_type_operand(module, symbol)?);
        }

        self.node_type_operand(source)
    }
}
