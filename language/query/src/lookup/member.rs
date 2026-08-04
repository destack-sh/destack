use destack_dir as dir;
use destack_dir::HeritageKind;
use destack_source::ModuleId;

use crate::{CompletionItemKind, ModuleQueryContext, ProgramQueryContext, QueryResult};

/// One member of a receiver's apparent type.
#[derive(Debug, Clone)]
pub(crate) struct ApparentMember {
    /// The member display name.
    pub(crate) name: String,
    /// The completion kind of the member.
    pub(crate) kind: CompletionItemKind,
    /// The declaring member symbol, when the member has one.
    pub(crate) symbol: Option<dir::GlobalSymbolId>,
}

impl ProgramQueryContext<'_> {
    /// Return the apparent members of one receiver type in selection precedence.
    pub(crate) fn apparent_members(
        &self,
        origin: ModuleId,
        type_id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> QueryResult<Vec<ApparentMember>> {
        let mut members = Vec::new();
        let ownership = self.ownership_form(type_id)?;
        self.collect_apparent_members(origin, type_id, ownership, is_optional, &mut members)?;

        Ok(members)
    }

    /// Return the effective ownership of one receiver type.
    fn ownership_form(&self, type_id: dir::GlobalTypeId) -> QueryResult<dir::Form> {
        self.read_type(type_id, |ty, _| match ty {
            dir::Type::Form(form) => match form.form {
                dir::Form::Readonly | dir::Form::Placed { .. } => self.ownership_form(form.value),
                form => Ok(form),
            },
            _ => Ok(dir::Form::Managed),
        })
    }

    /// Collect the apparent members of one receiver type.
    fn collect_apparent_members(
        &self,
        origin: ModuleId,
        type_id: dir::GlobalTypeId,
        ownership: dir::Form,
        is_optional: bool,
        members: &mut Vec<ApparentMember>,
    ) -> QueryResult<()> {
        self.read_type(type_id, |ty, module| match ty {
            // nominal receivers list their declared members
            dir::Type::Application(instance) => {
                self.collect_nominal_members(origin, instance.symbol, &ownership, members)?;
                self.collect_blanket_members(
                    origin,
                    dir::FamilyKey::Nominal(instance.symbol),
                    &ownership,
                    members,
                )
            }

            // primitive receivers list their covered blanket members
            dir::Type::Primitive(primitive) => self.collect_blanket_members(
                origin,
                dir::FamilyKey::Primitive(*primitive),
                &ownership,
                members,
            ),

            // literal receivers complete as their widened carrier
            dir::Type::Literal(literal) => {
                if let dir::Type::Primitive(primitive) = literal.widen() {
                    self.collect_blanket_members(
                        origin,
                        dir::FamilyKey::Primitive(primitive),
                        &ownership,
                        members,
                    )?;
                }

                Ok(())
            }

            // structural receivers list their shape properties
            dir::Type::Shape(shape) => {
                for property in module.types().properties(shape.properties) {
                    let dir::StaticKey::Name(name) = property.key else {
                        continue;
                    };
                    members.push(ApparentMember {
                        name: module.strings().get(name).to_string(),
                        kind: CompletionItemKind::Field,
                        symbol: None,
                    });
                }

                Ok(())
            }

            // erased receivers list their constraint members
            dir::Type::Dynamic(dynamic) => {
                self.collect_apparent_members(origin, dynamic.constraint, ownership, false, members)
            }

            // memory forms complete through their pointee under their own form
            dir::Type::Form(form) => {
                let ownership = self.ownership_form(type_id)?;

                self.collect_apparent_members(origin, form.value, ownership, false, members)
            }

            // unions offer the members common to every runtime arm
            dir::Type::Union(union) => {
                let elements = module.types().type_ids(union.elements).to_vec();
                let mut common: Option<Vec<ApparentMember>> = None;
                for element in elements {
                    // optional chains complete past the nullish arms
                    let is_nullish = self.read_type(element, |element, _| {
                        Ok(matches!(element, dir::Type::Null | dir::Type::Undefined))
                    })?;
                    if is_optional && is_nullish {
                        continue;
                    }

                    let mut arm = Vec::new();
                    self.collect_apparent_members(origin, element, ownership, false, &mut arm)?;
                    common = Some(match common {
                        None => arm,
                        Some(common) => common
                            .into_iter()
                            .filter(|member| {
                                arm.iter().any(|candidate| candidate.name == member.name)
                            })
                            .collect(),
                    });
                }
                members.extend(common.unwrap_or_default());

                Ok(())
            }

            // intersections offer the members of every part
            dir::Type::Intersection(intersection) => {
                let elements = module.types().type_ids(intersection.elements).to_vec();
                for element in elements {
                    self.collect_apparent_members(origin, element, ownership, false, members)?;
                }

                Ok(())
            }
            _ => Ok(()),
        })
    }

    /// Collect one nominal declaration's own, extension, and inherited members.
    fn collect_nominal_members(
        &self,
        origin: ModuleId,
        symbol: dir::GlobalSymbolId,
        ownership: &dir::Form,
        members: &mut Vec<ApparentMember>,
    ) -> QueryResult<()> {
        let module = self.module(symbol.module_id)?;
        let Some(definition) = module.definitions().definition(symbol) else {
            return Ok(());
        };

        // own members precede every other family
        for member in definition.members() {
            self.collect_definition_member(&module, member, members)?;
        }

        // reachable visible extensions follow in phase order
        for extension in self.visible_extensions(origin, symbol)? {
            self.collect_extension_members(extension, ownership, members)?;
        }

        // inherited members follow last
        if let dir::Definition::Class(class) = definition
            && let Some(extends) = &class.extends
            && let Some((_, instance)) = self.nominal_application_head(extends.ty)?
        {
            self.collect_nominal_members(origin, instance, ownership, members)?;
        }

        Ok(())
    }

    /// Collect the covering blanket extension members of one receiver family.
    fn collect_blanket_members(
        &self,
        _origin: ModuleId,
        family: dir::FamilyKey,
        ownership: &dir::Form,
        members: &mut Vec<ApparentMember>,
    ) -> QueryResult<()> {
        // offer ground extensions declared over a primitive format
        let environment = self.environment_declared()?;
        if let dir::FamilyKey::Primitive(primitive) = family
            && let Some(extensions) = environment.extensions_by_primitive.get(&primitive)
        {
            for extension in extensions.clone() {
                self.collect_extension_members(extension, ownership, members)?;
            }
        }

        // offer blankets whose declared coverage includes the family
        let blankets = environment.blanket_extensions.clone();
        for extension in blankets {
            if self.blanket_covers_family(extension, family)? {
                self.collect_extension_members(extension, ownership, members)?;
            }
        }

        Ok(())
    }

    /// Return whether one blanket's declared coverage includes a family.
    fn blanket_covers_family(
        &self,
        extension: dir::GlobalSymbolId,
        family: dir::FamilyKey,
    ) -> QueryResult<bool> {
        let module = self.module(extension.module_id)?;
        let Some(declaration) = module.definitions().extension_definition(extension) else {
            return Ok(false);
        };

        match (declaration.target.coverage(), family) {
            (dir::BlanketCoverage::Every, _) => Ok(true),
            // marker bounds cover their scalar domain
            (dir::BlanketCoverage::Interface(interface), dir::FamilyKey::Primitive(primitive)) => {
                let environment = self.environment_bound()?;
                let marker = environment
                    .language
                    .item(interface)
                    .and_then(|item| item.scalar_domain());

                Ok(marker == Some(primitive.scalar_domain()))
            }
            // interface bounds cover declaring nominals
            (dir::BlanketCoverage::Interface(interface), dir::FamilyKey::Nominal(symbol)) => {
                self.declares_interface(symbol, interface, 0)
            }
            (dir::BlanketCoverage::Deferred, _) => Ok(false),
        }
    }

    /// Return whether one nominal declares an interface conformance.
    fn declares_interface(
        &self,
        symbol: dir::GlobalSymbolId,
        interface: dir::GlobalSymbolId,
        depth: usize,
    ) -> QueryResult<bool> {
        if depth > 8 {
            return Ok(false);
        }
        let home = self.module(symbol.module_id)?;
        let Some(definition) = home.definitions().definition(symbol) else {
            return Ok(false);
        };

        // gather the declaration's own and extension conformance rows
        let mut conformances = definition
            .implementations()
            .iter()
            .map(|implementation| implementation.ty)
            .collect::<Vec<_>>();
        for extension in self.visible_extensions(symbol.module_id, symbol)? {
            let extension_module = self.module(extension.module_id)?;
            let Some(dir::Definition::Extension(declaration)) =
                extension_module.definitions().definition(extension)
            else {
                continue;
            };
            if declaration.target.root() != Some(symbol) {
                continue;
            }
            conformances.extend(
                declaration
                    .implements
                    .iter()
                    .map(|implementation| implementation.ty),
            );
        }

        // match any conformance head against the interface
        for conformance in conformances {
            let head = self.read_type(conformance, |ty, _| match ty {
                dir::Type::Application(instance) => Ok(Some(instance.symbol)),
                _ => Ok(None),
            })?;
            if head == Some(interface) {
                return Ok(true);
            }
        }

        // walk the extends chain of class declarations
        if let dir::Definition::Class(class) = definition
            && let Some(extends) = &class.extends
            && let Some((_, base)) = self.nominal_application_head(extends.ty)?
        {
            return self.declares_interface(base, interface, depth + 1);
        }

        Ok(false)
    }

    /// Collect one reachable extension's members into the apparent set.
    fn collect_extension_members(
        &self,
        extension: dir::GlobalSymbolId,
        ownership: &dir::Form,
        members: &mut Vec<ApparentMember>,
    ) -> QueryResult<()> {
        let extension_module = self.module(extension.module_id)?;
        let Some(dir::Definition::Extension(definition)) =
            extension_module.definitions().definition(extension)
        else {
            return Ok(());
        };
        let target = self.ownership_form(definition.target.r#type())?;
        if !ownership.adjusts_to(target) {
            return Ok(());
        }

        for member in &definition.members {
            self.collect_definition_member(&extension_module, member, members)?;
        }

        Ok(())
    }

    /// Collect extension symbols visible from one module for one target.
    fn visible_extensions(
        &self,
        origin: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut symbols = Vec::new();

        // collect extensions declared beside the looking module
        let origin_module = self.module(origin)?;
        symbols.extend(origin_module.definitions().target_extensions(target));

        // collect the implicit environment's extensions over the root
        let environment = self.environment_declared()?;
        if let Some(implicit) = environment.extensions_by_root.get(&target) {
            for extension in implicit {
                if !symbols.contains(extension) {
                    symbols.push(*extension);
                }
            }
        }

        // collect inherent extensions beside the target declaration
        if target.module_id != origin {
            let target_module = self.module(target.module_id)?;
            symbols.extend(target_module.definitions().target_extensions(target));
        }

        // collect explicitly imported extension symbols targeting the nominal
        for (_, imported) in origin_module.resolved().imports.symbol_targets() {
            let Ok(imported_module) = self.module(imported.module_id) else {
                continue;
            };
            let Some(dir::Definition::Extension(extension)) =
                imported_module.definitions().definition(imported)
            else {
                continue;
            };
            if extension.target.root() == Some(target) && !symbols.contains(&imported) {
                symbols.push(imported);
            }
        }

        Ok(symbols)
    }

    /// Collect one displayable definition member.
    fn collect_definition_member(
        &self,
        module: &ModuleQueryContext<'_>,
        member: &dir::DefinitionMember,
        members: &mut Vec<ApparentMember>,
    ) -> QueryResult<()> {
        // signatures and derived identities never complete after a dot
        if matches!(
            member,
            dir::DefinitionMember::CallSignature(_)
                | dir::DefinitionMember::ConstructSignature(_)
                | dir::DefinitionMember::IndexSignature(_)
                | dir::DefinitionMember::TaggedKey(_)
        ) {
            return Ok(());
        }
        let Some(name) = member.name(module.strings()) else {
            return Ok(());
        };

        let kind = match member {
            dir::DefinitionMember::Field(_) => CompletionItemKind::Field,
            dir::DefinitionMember::Method(_) => CompletionItemKind::Method,
            dir::DefinitionMember::AssociatedType(_) => CompletionItemKind::TypeAlias,
            dir::DefinitionMember::AssociatedConst(_) => CompletionItemKind::AssociatedConst,
            dir::DefinitionMember::EnumVariant(_) | dir::DefinitionMember::TaggedVariant(_) => {
                CompletionItemKind::EnumMember
            }
            _ => CompletionItemKind::Field,
        };
        members.push(ApparentMember {
            name,
            kind,
            symbol: member.symbol(),
        });

        Ok(())
    }

    /// Return the nominal application head symbol of one type.
    fn nominal_application_head(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<Option<(dir::GlobalTypeId, dir::GlobalSymbolId)>> {
        self.read_type(type_id, |ty, _| match ty {
            dir::Type::Application(instance) => Ok(Some((type_id, instance.symbol))),
            _ => Ok(None),
        })
    }
}

impl ModuleQueryContext<'_> {
    /// Return the declarations renamed together with one member symbol.
    pub(crate) fn member_rename_siblings(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut siblings = Vec::new();
        let Some((owner, definition, member)) = self.definitions().member(symbol_id) else {
            return Ok(siblings);
        };
        let Some(key) = member.key() else {
            return Ok(siblings);
        };

        // rename same-key members of the owner together
        for candidate in definition.members() {
            if candidate.key() == Some(key)
                && let Some(candidate_symbol) = candidate.symbol()
            {
                siblings.push(candidate_symbol);
            }
        }

        // rename interface requirements with their implementing declarations
        let requirement = member.source();
        self.member_implementation_symbols(program, owner, requirement, &mut siblings)?;

        Ok(siblings)
    }

    /// Collect the declaration symbols implementing one interface member.
    pub(crate) fn member_implementation_symbols(
        &self,
        program: &ProgramQueryContext<'_>,
        owner: dir::GlobalSymbolId,
        requirement: dir::GlobalNodeIdAny,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) -> QueryResult<()> {
        // resolve the requirement to its declaring member
        let home = program.module(owner.module_id)?;
        let Some(definition) = home.definitions().definition(owner) else {
            return Ok(());
        };
        let Some(required) = definition
            .members()
            .iter()
            .find(|member| member.source() == requirement)
        else {
            return Ok(());
        };

        // read the required member's space and key
        let space = required.space();
        let Some(key) = required.key() else {
            return Ok(());
        };

        // walk implementers reached through the owner's heritage edges
        for entry in program.base_heritage(owner)? {
            if entry.kind != HeritageKind::Implements {
                continue;
            }
            let derived_module = program.module(entry.declaration.module_id)?;
            let Some(definition) = derived_module.definitions().definition(entry.declaration)
            else {
                continue;
            };

            // collect the implementer's declarations under the same member key
            for member in definition.members_with_key(space, key) {
                if let Some(symbol) = member.symbol() {
                    symbols.push(symbol);
                }
            }
        }

        Ok(())
    }
}
