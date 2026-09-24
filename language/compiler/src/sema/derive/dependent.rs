use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::reduce::TypeSubstitution;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Record the dependents of every declaration.
    pub(in crate::sema) fn derive_module_dependents(&mut self) -> CompilerResult<()> {
        // gather every template, every definition, and every definition method of this module
        let module = self.module_id;
        let mut symbols: FxIndexSet<_> = self
            .module
            .generics_tail
            .iter_templates()
            .chain(self.module.generics.iter_templates())
            .filter_map(|(_, template)| template.symbol)
            .collect();
        for (symbol, definition) in self.module.iter_definitions() {
            symbols.insert(symbol);
            for member in definition.members() {
                if let dir::DefinitionMember::Method(method) = member {
                    symbols.insert(method.symbol);
                }
            }
        }
        let functions: Vec<_> = self
            .module
            .types
            .with_tail(&self.module.types_tail)
            .symbol_types()
            .map(|(symbol, _)| symbol)
            .collect();
        for symbol in functions {
            if self.symbol_kind(symbol)? == dir::SymbolKind::Function {
                symbols.insert(symbol);
            }
        }

        // record the dependents of each declaration of this module
        for symbol in symbols {
            if symbol.module_id == module {
                self.symbol_dependents(symbol)?;
            }
        }

        Ok(())
    }

    /// Return the dependents one declaration writes, recorded on first use.
    pub(in crate::sema) fn symbol_dependents(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut collecting = FxIndexSet::default();

        self.collect_symbol_dependents(symbol, &mut collecting)
    }

    /// Collect one declaration's dependents, a recursive application contributing none.
    fn collect_symbol_dependents(
        &mut self,
        symbol: dir::GlobalSymbolId,
        collecting: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // read a recorded list
        let recorded = match self.is_own_module(symbol.module_id) {
            true => self
                .module
                .symbol_dependents(symbol.local_id)
                .map(<[_]>::to_vec),
            false => self
                .external(symbol.module_id)?
                .and_then(|external| external.generics().symbol_dependents(symbol.local_id))
                .map(<[_]>::to_vec)
                .or_else(|| self.foreign_dependents.get(&symbol).cloned()),
        };
        if let Some(dependents) = recorded {
            return Ok(dependents);
        }
        if !collecting.insert(symbol) {
            return Ok(Vec::new());
        }

        // collect over the evaluated type lowering reads
        let scope = match self.symbol_template(symbol)? {
            Some(template) => Some(template),
            None => match self.member_owner(symbol)? {
                Some(owner) => self.symbol_template(owner)?,
                None => None,
            },
        };
        let receiver_is_open = self.is_interface_member(symbol)?;
        let keeps_parameter_projections = self.symbol_kind(symbol)?.is_type_definition();
        let anchor = self.module.bound.module_node.into_global(self.module_id);
        let origin = Origin::Node(anchor, scope);
        let mut dependents = Vec::new();
        for ty in self.declared_types(symbol)? {
            let ty = self.deeply_resolve(origin, ty)?;
            let ty = self.evaluate_type(origin, ty)?;
            self.collect_dependents(
                origin,
                receiver_is_open,
                keeps_parameter_projections,
                ty,
                &mut dependents,
                collecting,
            )?;
        }

        // record the list, the module's own declarations into its artifact
        match self.is_own_module(symbol.module_id) {
            true => self
                .module
                .generics_tail
                .set_symbol_dependents(symbol.local_id, dependents.clone()),
            false => {
                self.foreign_dependents.insert(symbol, dependents.clone());
            }
        }

        Ok(dependents)
    }

    /// Close one dependent at an application's arguments, evaluated as far as they allow.
    pub(in crate::sema) fn close_dependent(
        &mut self,
        origin: Origin,
        dependent: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let substituted = self.substitute_type(dependent, substitution)?;

        self.evaluate_type(origin, substituted)
    }

    /// Return whether one declaration is an interface or one of its members, `this` open there.
    fn is_interface_member(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let mut current = Some(symbol);
        while let Some(symbol) = current {
            if self.symbol_kind(symbol)?.is_interface() {
                return Ok(true);
            }
            current = self.member_owner(symbol)?;
        }

        Ok(false)
    }

    /// Return the declared types one declaration's signature writes.
    fn declared_types(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(vec![self.symbol_type(symbol)?]);
        };

        // a transparent alias expands into the signatures applying it
        match &*definition {
            dir::Definition::TypeAlias(alias) if self.is_shaped_alias_body(alias.value)? => {
                return Ok(vec![alias.value]);
            }
            dir::Definition::TypeAlias(_) => return Ok(Vec::new()),
            dir::Definition::Newtype(newtype) => return Ok(vec![newtype.backing]),
            _ => {}
        }

        // collect the types the definition's fields write
        let mut types = Vec::new();
        for member in definition.members() {
            if let dir::DefinitionMember::Field(field) = member {
                types.push(self.symbol_type(field.symbol)?);
            }
        }

        Ok(types)
    }

    /// Return whether one alias body is a shape lowering keeps the alias identity for.
    pub(in crate::sema) fn is_shaped_alias_body(
        &self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        Ok(matches!(
            self.ty(value)?,
            dir::Type::Object(_)
                | dir::Type::Union(_)
                | dir::Type::Tuple(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_)
                | dir::Type::Function(_)
        ))
    }

    /// Return whether one type only closes at an instance.
    fn is_dependent(
        &mut self,
        origin: Origin,
        keeps_parameter_projections: bool,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let kind = self.ty(id)?;
        Ok(match &kind {
            // leave unqualified, memory-const, and type definition parameter projections open
            dir::Type::Member(member) => {
                let member = self.type_member(id.module_id, *member)?;
                let owner = self.shallow_resolve(member.owner)?;
                let on_parameter = matches!(self.ty(owner)?, dir::Type::Parameter(_));

                member.qualifier.is_none()
                    || self.is_memory_const_projection(&member)?
                    || (keeps_parameter_projections && on_parameter)
            }
            dir::Type::Operation(_) => true,
            // an intersection of open value operands merges at instantiation
            dir::Type::Intersection(intersection) => {
                let elements = self.type_ids(id.module_id, intersection.elements)?;
                let mut values = 0;
                for element in elements {
                    let is_interface = match self.ty(*element)? {
                        dir::Type::Application(application) => {
                            self.symbol_kind(application.symbol)?.is_interface()
                        }
                        _ => false,
                    };
                    if !is_interface {
                        values += 1;
                    }
                }

                values > 1
            }
            // a stuck application evaluates at instantiation
            dir::Type::Application(application) => {
                self.spreads_open_parameters(id.module_id, application)?
                    || self.is_stuck_head(origin, id)?
            }
            _ => false,
        })
    }

    /// Return whether one qualified projection names a memory-kind associated const.
    fn is_memory_const_projection(&mut self, member: &dir::MemberType) -> CompilerResult<bool> {
        let qualifier = member
            .qualifier
            .map(|qualifier| self.ty(qualifier))
            .transpose()?;
        let Some(symbol) = qualifier.and_then(|qualifier| qualifier.symbol()) else {
            return Ok(false);
        };
        let Some(definition) = self.definition(symbol)? else {
            return Ok(false);
        };
        let constant = definition
            .members()
            .iter()
            .find_map(|declared| match declared {
                dir::DefinitionMember::AssociatedConst(constant) if constant.key == member.key => {
                    Some(constant.symbol)
                }
                _ => None,
            });
        let Some(constant) = constant else {
            return Ok(false);
        };
        let ty = self.symbol_type(constant)?;

        Ok(self.memory_kind(ty)?.is_some())
    }

    /// Return whether one application is a callable spreading an open parameter tuple.
    fn spreads_open_parameters(
        &self,
        module: ModuleId,
        application: &dir::GenericApplication,
    ) -> CompilerResult<bool> {
        if self.language_item(application.symbol)? != Some(dir::LanguageItem::Function) {
            return Ok(false);
        }
        let Some(parameters) = self
            .type_ids(module, application.arguments)?
            .first()
            .copied()
        else {
            return Ok(false);
        };

        Ok(!matches!(self.ty(parameters)?, dir::Type::Tuple(_)))
    }

    /// Return the normalized expansion of one transparent alias application.
    fn transparent_alias_expansion(
        &mut self,
        origin: Origin,
        module: ModuleId,
        application: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let definition = self.definition(application.symbol)?;
        let Some(dir::Definition::TypeAlias(alias)) = definition.as_deref() else {
            return Ok(None);
        };
        let value = alias.value;
        if self.language_item(application.symbol)?.is_some()
            || matches!(self.ty(value)?, dir::Type::Intrinsic)
            || self.is_shaped_alias_body(value)?
        {
            return Ok(None);
        }
        let Some(body) = self.type_alias_body(origin, module, application)? else {
            return Ok(None);
        };

        Ok(Some(self.normalize(origin, body)?))
    }

    /// Collect the dependents one type reaches, an applied template's closing at the arguments.
    fn collect_dependents(
        &mut self,
        origin: Origin,
        receiver_is_open: bool,
        keeps_parameter_projections: bool,
        ty: dir::GlobalTypeId,
        dependents: &mut Vec<dir::GlobalTypeId>,
        collecting: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        let mut pending = vec![ty];
        let mut visited = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            // a closed type closes at the declaration already
            let flags = self.type_flags(id)?;
            if !flags.has_parameter() && !(receiver_is_open && flags.has_this()) {
                continue;
            }

            // record a dependent once and stop descending into it
            let kind = self.ty(id)?;
            if self.is_dependent(origin, keeps_parameter_projections, id)? {
                if !dependents.contains(&id) {
                    dependents.push(id);
                }
                continue;
            }

            // a transparent alias application expands into this signature
            if let dir::Type::Application(application) = &kind
                && let Some(expansion) =
                    self.transparent_alias_expansion(origin, id.module_id, application)?
            {
                pending.push(expansion);

                continue;
            }

            // an applied declaration's dependents close at the arguments into this signature
            if let dir::Type::Application(application) = &kind
                && self.symbol_template(application.symbol)?.is_some()
            {
                let applied = self.collect_symbol_dependents(application.symbol, collecting)?;
                if !applied.is_empty() {
                    let substitution = self.instance_substitution(id.module_id, application)?;
                    for dependent in applied {
                        pending.push(self.close_dependent(origin, dependent, &substitution)?);
                    }
                }
            }

            // descend into the type's children
            self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
        }

        Ok(())
    }
}
