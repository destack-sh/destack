use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::CheckState;

use super::CheckModuleOutput;

/// Member definitions collected from one nominal body.
#[derive(Debug, Default)]
struct NominalMembers {
    /// The instance fields.
    fields: Vec<dir::FieldDefinition>,
    /// The nominal methods.
    methods: Vec<dir::MethodDefinition>,
    /// The symbol-free call signatures.
    call_signatures: Vec<dir::SignatureDefinition>,
    /// The symbol-free construct signatures.
    construct_signatures: Vec<dir::SignatureDefinition>,
    /// The symbol-free index signatures.
    index_signatures: Vec<dir::SignatureDefinition>,
    /// The associated types.
    associated_types: Vec<dir::AssociatedTypeDefinition>,
    /// The associated static values.
    associated_statics: Vec<dir::AssociatedStaticDefinition>,
}

impl CheckState<'_> {
    /// Commit nominal declarations into the checked nominal table.
    pub(super) fn commit_nominal_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::NominalSegment> {
        let mut table = dir::NominalSegment::new(module);
        let declarations = self.nominal_symbols(module);

        // commit definitions in walk order
        for symbol in declarations {
            let definition = self.commit_nominal_definition(module, output, environment, symbol)?;

            table.insert_definition(symbol, definition);
        }

        Ok(table)
    }

    /// Commit one nominal declaration.
    fn commit_nominal_definition(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::NominalDefinition> {
        let source = self.symbol_source_node(symbol).into_global(module);
        let template = self.generic_template_for_owner(output, symbol);
        let source_id = source.local_id.into_typed::<dir::Declaration>();
        let source_declaration = self.module(module).view().get(source_id).clone();

        match source_declaration {
            // struct S { ... }
            dir::Declaration::Struct(declaration) => {
                let members =
                    self.commit_members(module, output, environment, &declaration.members)?;
                let implements = self.commit_nominal_targets(
                    module,
                    output,
                    declaration
                        .implements_types
                        .iter()
                        .map(|node| node.into_any()),
                );

                Ok(dir::NominalDefinition::Struct(dir::StructDefinition {
                    source,
                    template,
                    implements,
                    fields: members.fields,
                    methods: members.methods,
                    associated_types: members.associated_types,
                    associated_statics: members.associated_statics,
                }))
            }
            // class C extends B { ... }
            dir::Declaration::Class(declaration) => {
                let members =
                    self.commit_members(module, output, environment, &declaration.members)?;
                let extends = declaration
                    .extends_type
                    .and_then(|node| self.commit_nominal_target(module, output, node.into_any()));
                let implements = self.commit_nominal_targets(
                    module,
                    output,
                    declaration
                        .implements_types
                        .iter()
                        .map(|node| node.into_any()),
                );

                Ok(dir::NominalDefinition::Class(dir::ClassDefinition {
                    source,
                    template,
                    extends,
                    implements,
                    fields: members.fields,
                    methods: members.methods,
                    associated_types: members.associated_types,
                    associated_statics: members.associated_statics,
                }))
            }
            // nominal interface I { ... }
            dir::Declaration::Interface(declaration) if declaration.is_nominal => {
                let members =
                    self.commit_type_members(module, output, environment, &declaration.members)?;
                let extends = self.commit_nominal_targets(
                    module,
                    output,
                    declaration
                        .extends_types
                        .iter()
                        .map(|extends_type| extends_type.into_any()),
                );

                Ok(dir::NominalDefinition::Interface(
                    dir::InterfaceDefinition {
                        source,
                        template,
                        extends,
                        fields: members.fields,
                        methods: members.methods,
                        call_signatures: members.call_signatures,
                        construct_signatures: members.construct_signatures,
                        index_signatures: members.index_signatures,
                        associated_types: members.associated_types,
                        associated_statics: members.associated_statics,
                    },
                ))
            }
            // enum E { A, B }
            dir::Declaration::Enum(declaration) => {
                let members =
                    self.commit_members(module, output, environment, &declaration.members)?;
                let implements = self.commit_nominal_targets(
                    module,
                    output,
                    declaration
                        .implements_types
                        .iter()
                        .map(|node| node.into_any()),
                );
                let variants =
                    self.commit_enum_variants(module, output, environment, &declaration.fields)?;

                Ok(dir::NominalDefinition::Enum(dir::EnumDefinition {
                    source,
                    template,
                    implements,
                    variants,
                    methods: members.methods,
                    associated_types: members.associated_types,
                    associated_statics: members.associated_statics,
                }))
            }
            // newtype Id = T
            dir::Declaration::Type(declaration) if declaration.is_nominal => {
                Ok(dir::NominalDefinition::Newtype(dir::NewtypeDefinition {
                    source,
                    template,
                    implements: Vec::new(),
                    methods: Vec::new(),
                    associated_types: Vec::new(),
                    associated_statics: Vec::new(),
                }))
            }
            _ => {
                panic!("nominal symbol {symbol:?} does not point at a nominal declaration")
            }
        }
    }

    /// Return nominal symbols declared in one checked module.
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

            if nominal_symbol_kind(kind) {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Return the generic template declared by one owner node.
    fn generic_template_for_owner(
        &self,
        output: &CheckModuleOutput,
        owner: dir::GlobalSymbolId,
    ) -> Option<dir::LocalGenericTemplateId> {
        output
            .generics
            .iter_templates()
            .find_map(|(template_id, template)| (template.owner == owner).then_some(template_id))
    }

    /// Commit nominal relation targets from source nodes.
    fn commit_nominal_targets(
        &self,
        module: ModuleId,
        output: &CheckModuleOutput,
        nodes: impl Iterator<Item = dir::LocalNodeIdAny>,
    ) -> Vec<dir::NominalTarget> {
        nodes
            .filter_map(|node| self.commit_nominal_target(module, output, node))
            .collect()
    }

    /// Commit one nominal relation target from a source node.
    fn commit_nominal_target(
        &self,
        module: ModuleId,
        output: &CheckModuleOutput,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::NominalTarget> {
        let node = node.into_global(module);
        let symbol = output.resolutions.symbol_resolution(node)?;
        let application = output.generics.node_application_id(node);

        Some(dir::NominalTarget {
            symbol,
            application,
        })
    }

    /// Commit declaration-body nominal members.
    fn commit_members(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        members: &[dir::LocalNodeId<dir::Member>],
    ) -> CompilerResult<NominalMembers> {
        let mut definitions = NominalMembers::default();

        // commit members in source order
        for member_id in members {
            let member = self.module(module).view().get(*member_id).clone();
            let source = member_id.into_global_any(module);

            self.commit_member(
                module,
                output,
                environment,
                source,
                &member,
                &mut definitions,
            )?;
        }

        Ok(definitions)
    }

    /// Commit one declaration-body nominal member.
    fn commit_member(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::GlobalNodeIdAny,
        member: &dir::Member,
        definitions: &mut NominalMembers,
    ) -> CompilerResult<()> {
        match member {
            // type Name = T
            dir::Member::AssociatedType {
                constraint, value, ..
            } => {
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let constraint = constraint.map(|constraint| {
                    self.commit_node_type(
                        module,
                        output,
                        environment,
                        constraint.into_global_any(module),
                    )
                });
                let value = value.map(|value| {
                    self.commit_node_type(module, output, environment, value.into_global_any(module))
                });

                definitions.associated_types.push(dir::AssociatedTypeDefinition {
                    symbol,
                    source,
                    constraint,
                    value,
                });
            }
            // const Name = value
            dir::Member::AssociatedConst { .. } => {
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let ty = self.commit_symbol_type(module, output, environment, symbol);
                let value = self.commit_symbol_static_maybe(module, output, environment, symbol);

                definitions.associated_statics.push(dir::AssociatedStaticDefinition {
                    symbol,
                    source,
                    ty,
                    value,
                });
            }
            // field: T
            dir::Member::Field { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let ty = self.commit_symbol_type(module, output, environment, symbol);

                if *is_static {
                    let value =
                        self.commit_symbol_static_maybe(module, output, environment, symbol);

                    definitions
                        .associated_statics
                        .push(dir::AssociatedStaticDefinition {
                            symbol,
                            source,
                            ty,
                            value,
                        });
                } else {
                    definitions.fields.push(dir::FieldDefinition {
                        symbol,
                        source,
                        key,
                        ty,
                    });
                }
            }
            // method() {}
            dir::Member::Method { is_static, .. } => {
                let slot = member.slot();
                let Some(slot) = slot else {
                    return Ok(());
                };
                let symbol = self.declaration_symbol(module, source.local_id);
                let ty = self.commit_member_type(module, output, environment, source, symbol);

                definitions.methods.push(dir::MethodDefinition {
                    symbol,
                    source,
                    slot,
                    is_static: *is_static,
                    ty,
                });
            }
            // static { ... }
            dir::Member::StaticBlock { .. }
            // comptime { ... }
            | dir::Member::ComptimeBlock { .. }
            // parse recovery
            | dir::Member::Error => {}
        }

        Ok(())
    }

    /// Commit interface members.
    fn commit_type_members(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        members: &[dir::LocalNodeId<dir::TypeMember>],
    ) -> CompilerResult<NominalMembers> {
        let mut definitions = NominalMembers::default();

        // commit members in source order
        for member_id in members {
            let member = self.module(module).view().get(*member_id).clone();
            let source = member_id.into_global_any(module);

            self.commit_type_member(
                module,
                output,
                environment,
                source,
                &member,
                &mut definitions,
            )?;
        }

        Ok(definitions)
    }

    /// Commit one interface member.
    fn commit_type_member(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::GlobalNodeIdAny,
        member: &dir::TypeMember,
        definitions: &mut NominalMembers,
    ) -> CompilerResult<()> {
        match member {
            // field: T
            dir::TypeMember::Field { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let ty = self.commit_symbol_type(module, output, environment, symbol);

                if *is_static {
                    let value =
                        self.commit_symbol_static_maybe(module, output, environment, symbol);

                    definitions
                        .associated_statics
                        .push(dir::AssociatedStaticDefinition {
                            symbol,
                            source,
                            ty,
                            value,
                        });
                } else {
                    definitions.fields.push(dir::FieldDefinition {
                        symbol,
                        source,
                        key,
                        ty,
                    });
                }
            }
            // method(): T
            dir::TypeMember::Method { key, is_static, .. } => {
                let Some(key) = key.direct_static_key() else {
                    return Ok(());
                };
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let ty = self.commit_symbol_type(module, output, environment, symbol);

                definitions.methods.push(dir::MethodDefinition {
                    symbol: Some(symbol),
                    source,
                    slot: dir::MemberSlot::Key(key),
                    is_static: *is_static,
                    ty,
                });
            }
            // (...): T
            dir::TypeMember::CallSignature { .. } => {
                let ty = self.commit_node_type(module, output, environment, source);

                definitions
                    .call_signatures
                    .push(dir::SignatureDefinition { source, ty });
            }
            // new (...): T
            dir::TypeMember::ConstructSignature { .. } => {
                let ty = self.commit_node_type(module, output, environment, source);

                definitions
                    .construct_signatures
                    .push(dir::SignatureDefinition { source, ty });
            }
            // [key: K]: V
            dir::TypeMember::IndexSignature { .. } => {
                let ty = self.commit_node_type(module, output, environment, source);

                definitions
                    .index_signatures
                    .push(dir::SignatureDefinition { source, ty });
            }
            // type Name = T
            dir::TypeMember::AssociatedType {
                constraint, value, ..
            } => {
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let constraint = constraint.map(|constraint| {
                    self.commit_node_type(
                        module,
                        output,
                        environment,
                        constraint.into_global_any(module),
                    )
                });
                let value = value.map(|value| {
                    self.commit_node_type(
                        module,
                        output,
                        environment,
                        value.into_global_any(module),
                    )
                });

                definitions
                    .associated_types
                    .push(dir::AssociatedTypeDefinition {
                        symbol,
                        source,
                        constraint,
                        value,
                    });
            }
            // const Name = value
            dir::TypeMember::AssociatedConst { .. } => {
                let symbol = self.required_declaration_symbol(module, source.local_id);
                let ty = self.commit_symbol_type(module, output, environment, symbol);
                let value = self.commit_symbol_static_maybe(module, output, environment, symbol);

                definitions
                    .associated_statics
                    .push(dir::AssociatedStaticDefinition {
                        symbol,
                        source,
                        ty,
                        value,
                    });
            }
            // parse recovery
            dir::TypeMember::Error => {}
        }

        Ok(())
    }

    /// Commit enum variants in source order.
    fn commit_enum_variants(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        fields: &[dir::LocalNodeId<dir::EnumField>],
    ) -> CompilerResult<Vec<dir::VariantDefinition>> {
        let mut variants = Vec::new();

        // commit variants in source order
        for field in fields {
            let source = field.into_global_any(module);
            let symbol = self.required_declaration_symbol(module, source.local_id);
            let key = self.module(module).view().get(*field).name.static_key();
            let value = self.commit_symbol_static_maybe(module, output, environment, symbol);

            variants.push(dir::VariantDefinition {
                symbol,
                source,
                key,
                value,
            });
        }

        Ok(variants)
    }

    /// Return the symbol declared by one source node.
    fn required_declaration_symbol(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> dir::GlobalSymbolId {
        self.declaration_symbol(module, source)
            .unwrap_or_else(|| panic!("nominal source node {source:?} declares no symbol"))
    }

    /// Commit one symbol type and return its DIR id.
    fn commit_symbol_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
    ) -> dir::GlobalTypeId {
        self.commit_symbol_type_maybe(module, output, environment, symbol)
            .unwrap_or_else(|| panic!("nominal symbol {symbol:?} has no checked type"))
    }

    /// Commit one optional symbol type.
    fn commit_symbol_type_maybe(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        if let Some(type_id) = output.types.get_symbol_type_id(symbol) {
            return Some(type_id);
        }

        let operand = self.operands.symbol_types.get(&symbol).copied()?;
        let source = self.symbol_source_node(symbol);
        let type_id = self.commit_type_operand(module, output, environment, operand, source)?;

        output.types.set_symbol_type(symbol, type_id);

        Some(type_id)
    }

    /// Commit one member type from either symbol or source node output.
    fn commit_member_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::GlobalNodeIdAny,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> dir::GlobalTypeId {
        if let Some(symbol) = symbol {
            return self.commit_symbol_type(module, output, environment, symbol);
        }

        self.commit_node_type(module, output, environment, source)
    }

    /// Commit one source node type and return its DIR id.
    fn commit_node_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::GlobalNodeIdAny,
    ) -> dir::GlobalTypeId {
        if let Some(type_id) = output.types.get_node_type_id(source) {
            return type_id;
        }

        let operand = self
            .operands
            .node_types
            .get(&source)
            .copied()
            .unwrap_or_else(|| panic!("nominal source node {source:?} has no checked type"));
        let type_id = self
            .commit_type_operand(module, output, environment, operand, source.local_id)
            .unwrap_or_else(|| panic!("nominal source node {source:?} has unresolved type"));

        output.types.set_node_type(source, type_id);

        type_id
    }

    /// Commit one optional symbol static value.
    fn commit_symbol_static_maybe(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalStaticId> {
        if let Some(static_id) = output.statics.get_symbol_static_id(symbol) {
            return Some(static_id);
        }

        let operand = self.operands.symbol_statics.get(&symbol).copied()?;
        let static_id = self.commit_static_operand(module, output, environment, operand)?;

        output.statics.set_symbol_static(symbol, static_id);

        Some(static_id)
    }
}

/// Return whether one symbol kind has a nominal definition.
fn nominal_symbol_kind(kind: dir::SymbolKind) -> bool {
    matches!(
        kind,
        dir::SymbolKind::Class
            | dir::SymbolKind::Enum
            | dir::SymbolKind::Newtype
            | dir::SymbolKind::NewtypeInterface
            | dir::SymbolKind::Struct
    )
}
