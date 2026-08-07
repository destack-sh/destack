use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{Module, Package, RepositoryError};
use rustc_hash::FxHashSet;

use crate::source::strip_module_extension;
use crate::{QueryError, QueryResult};

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format one symbol type.
    pub(crate) fn symbol_type(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<String> {
        let module = self.program.module(symbol_id.module_id)?;
        let type_id = module
            .types()?
            .get_symbol_type_id(symbol_id)
            .ok_or(QueryError::missing(format!("symbol type: {symbol_id:?}")))?;
        let formatter = Formatter::new(&module, self.program);

        // retain authored parameter names on callable types
        let type_text =
            if let Some(parameter_names) = self.program.symbol_parameter_names(symbol_id)? {
                formatter.callable_type(type_id, Some(&parameter_names))
            } else {
                formatter.global_type(type_id)
            }?;

        Ok(type_text)
    }

    /// Format one generic parameter type.
    pub(super) fn generic_parameter_type(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> QueryResult<String> {
        let module = self.program.module(parameter.module_id)?;
        let parameter = module.generics()?.get_parameter(parameter.local_id);

        let name = match parameter.key {
            dir::GenericParameterKey::Symbol(symbol) => {
                let symbol = module.bindings()?.get_symbol(symbol.local_id);

                static_key_segment(symbol.key, module.strings())
            }
            dir::GenericParameterKey::Generated(name) => {
                Some(module.strings().get(name).to_string())
            }
        };

        let name = name.ok_or(QueryError::missing(format!(
            "generic parameter name: {parameter:?}"
        )))?;

        Ok(name)
    }

    /// Format one symbol path.
    pub(super) fn symbol(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<String> {
        let module = self.program.module(symbol_id.module_id)?;
        let symbols = module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.into_local());
        let Some(symbol_name) = static_key_segment(symbol.key, module.strings()) else {
            return Err(QueryError::missing(format!(
                "symbol path segment: {symbol_id:?}"
            )));
        };
        let mut segments = vec![symbol_name];

        let mut scope_id = symbol.scope.id;
        let mut seen_scopes = FxHashSet::default();
        loop {
            if !seen_scopes.insert(scope_id) {
                return Err(QueryError::cycle(format!("symbol scope: {symbol_id:?}")));
            }

            let scope = symbols.get_scope_by_id(scope_id);

            // include named containers but omit anonymous namespace owners
            if let Some(owner_id) = scope.owner
                && owner_id != symbol_id.into_local()
            {
                let owner = symbols.get_symbol(owner_id);
                if let Some(owner_name) = static_key_segment(owner.key, module.strings()) {
                    segments.push(owner_name);
                } else if owner.role != dir::SymbolRole::Namespace {
                    return Err(QueryError::missing(format!(
                        "symbol owner path segment: {symbol_id:?}, {owner_id:?}"
                    )));
                }
            }

            let Some(parent) = scope.parent else {
                break;
            };
            scope_id = parent.id;
        }

        segments.reverse();

        Ok(segments.join("."))
    }

    /// Format one symbol name with its package and module.
    fn qualified_symbol(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<String> {
        let source_module = self
            .module
            .repository()
            .module(self.module.revision(), symbol_id.module_id)?
            .ok_or(RepositoryError::MissingModule {
                module: symbol_id.module_id,
            })?;
        let package = self
            .module
            .repository()
            .package(self.module.revision(), source_module.package_id)?
            .ok_or(RepositoryError::MissingPackage {
                package: source_module.package_id,
            })?;

        let Some(package_name) = package.name.as_ref() else {
            return Err(QueryError::missing(format!(
                "symbol package name: {symbol_id:?}"
            )));
        };
        if package_name.is_empty() {
            return Err(QueryError::missing(format!(
                "symbol package name: {symbol_id:?}"
            )));
        }
        let module_path = module_path_without_extension(source_module.as_ref(), package.as_ref())?;
        let symbol_path = self.symbol(symbol_id)?;
        let module_prefix = if module_path.is_empty() {
            package_name.to_string()
        } else {
            format!("{package_name}/{module_path}")
        };

        Ok(format!("{module_prefix}:{symbol_path}"))
    }

    /// Format the qualified name of a unique symbol.
    pub(super) fn unique_symbol(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<String> {
        let name = self.qualified_symbol(symbol_id)?;

        Ok(format!("{name}#unique"))
    }

    /// Format one symbol from its exact declaration.
    pub(crate) fn symbol_signature(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<String> {
        let module = self.program.module(symbol_id.module_id)?;
        let formatter = Formatter::new(&module, self.program);
        let symbols = module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.into_local());
        let Some(declaration) = symbol.declaration else {
            return formatter.member_symbol_signature(symbol_id);
        };
        let name = symbol
            .name()
            .map(|name| module.strings().get(name).to_string());

        // format the exact authored declaration kind
        let signature = match declaration.local_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id = declaration
                    .local_id
                    .try_into_typed::<dir::Declaration>()
                    .map_err(|_| QueryError::invalid(format!("signature symbol: {symbol_id:?}")))?;
                let declaration = module.view()?.get::<dir::Declaration>(declaration_id);

                formatter.declaration_signature(declaration)?
            }
            dir::NodeType::Parameter => {
                let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(declaration.local_id.id);

                formatter.parameter_symbol_signature(parameter_id)?
            }
            dir::NodeType::GenericParameter => {
                let parameter_id =
                    dir::LocalNodeId::<dir::GenericParameter>::new(declaration.local_id.id);

                formatter.generic_parameter(parameter_id)?
            }
            dir::NodeType::Pattern => {
                let name = name.as_deref().ok_or(QueryError::missing(format!(
                    "signature name: {symbol_id:?}"
                )))?;

                formatter.binding_symbol_signature(name, symbol_id)?
            }
            dir::NodeType::Member | dir::NodeType::TypeMember | dir::NodeType::EnumField => {
                formatter.member_symbol_signature(symbol_id)?
            }
            node_type => {
                return Err(QueryError::invalid(format!(
                    "signature declaration: {symbol_id:?}, {node_type:?}"
                )));
            }
        };

        Ok(signature)
    }

    /// Format one member symbol.
    fn member_symbol_signature(&self, symbol_id: dir::GlobalSymbolId) -> QueryResult<String> {
        let (declaring, definition, member) = self
            .module
            .definition_member(self.program, symbol_id)?
            .ok_or(QueryError::missing(format!(
                "signature member: {symbol_id:?}"
            )))?;
        let owner = definition.member_owner(declaring);
        let container = owner
            .map(|owner| self.program.symbol_name(owner))
            .transpose()?
            .flatten();
        let name = self.member_name(member)?;
        let name = match container {
            Some(container) => format!("{container}.{name}"),
            None => name,
        };

        let signature = match member {
            dir::DefinitionMember::Field(_) => {
                let type_text = self.member_type(member)?;

                format!("{name}: {type_text}")
            }
            dir::DefinitionMember::Method(method) => {
                let signature = self.authored_member_signature(member.source())?;
                let signature = self.call_signature(&name, signature, false)?;
                let static_prefix = if method.space == dir::MemberSpace::Static {
                    "static "
                } else {
                    ""
                };
                let role_prefix = match method.role {
                    Some(dir::FunctionRole::Getter) => "get ",
                    Some(dir::FunctionRole::Setter) => "set ",
                    _ => "",
                };

                format!("{static_prefix}{role_prefix}{signature}")
            }
            dir::DefinitionMember::AssociatedType(associated) => {
                let constraint = associated
                    .constraint
                    .map(|constraint| self.global_type(constraint))
                    .transpose()?;
                let value = associated
                    .value
                    .map(|value| self.global_type(value))
                    .transpose()?;

                match (constraint, value) {
                    (None, None) => name,
                    (Some(constraint), None) => format!("{name}: {constraint}"),
                    (None, Some(value)) => format!("{name} = {value}"),
                    (Some(constraint), Some(value)) => {
                        format!("{name}: {constraint} = {value}")
                    }
                }
            }
            dir::DefinitionMember::AssociatedConst(_) => {
                let type_text = self.member_type(member)?;

                format!("{name}: {type_text}")
            }
            dir::DefinitionMember::EnumVariant(_) => {
                let type_text = self.member_type(member)?;

                format!("{name}: {type_text}")
            }
            dir::DefinitionMember::TaggedKey(_) => {
                return Err(QueryError::invalid(format!(
                    "tagged member has no selected variant: {symbol_id:?}"
                )));
            }
            dir::DefinitionMember::TaggedVariant(variant) => {
                self.tagged_variant_signature(declaring, variant)?
            }
            dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_) => {
                let signature = self.authored_member_signature(member.source())?;

                self.call_signature(&name, signature, false)?
            }
            dir::DefinitionMember::IndexSignature(_) => {
                let type_text = self.member_type(member)?;

                format!("{name}: {type_text}")
            }
        };

        Ok(signature)
    }

    /// Format one generated tagged variant constructor.
    pub(crate) fn tagged_variant_signature(
        &self,
        declaring: dir::GlobalSymbolId,
        variant: &dir::TaggedVariantDefinition,
    ) -> QueryResult<String> {
        let owner = self.symbol(declaring)?;
        let generic_parameters = self.symbol_generics(declaring)?.unwrap_or_default();
        let name = self.property_key(variant.key)?;
        let parameters = variant
            .argument
            .map(|argument| self.global_type(argument))
            .transpose()?
            .unwrap_or_default();

        Ok(format!(
            "{owner}.{name}{generic_parameters}({parameters}): {owner}{generic_parameters}"
        ))
    }

    /// Format one parameter symbol.
    fn parameter_symbol_signature(
        &self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> QueryResult<String> {
        let parameter = self.module.view()?.get::<dir::Parameter>(parameter_id);
        let name = self.module.parameter_name(parameter)?;
        let node_id = parameter_id.into_global(self.module.module_id()).into_any();
        let type_text = self.node_type(node_id)?;

        Ok(format!("{name}: {type_text}"))
    }

    /// Format one local binding symbol.
    fn binding_symbol_signature(
        &self,
        name: &str,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<String> {
        let symbol = self.module.bindings()?.get_symbol(symbol_id.local_id);
        let Some(type_id) = self.types()?.get_symbol_type_id(symbol_id) else {
            return Err(QueryError::missing(format!(
                "binding symbol type: {symbol_id:?}"
            )));
        };
        let type_text = self.global_type(type_id)?;
        let keyword = if symbol.binding_mutability == Some(dir::Mutability::Immutable) {
            "const"
        } else {
            "let"
        };

        Ok(format!("{keyword} {name}: {type_text}"))
    }

    /// Return one authored member function signature.
    fn authored_member_signature(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> QueryResult<&dir::FunctionSignature> {
        let view = self.module.view()?;

        let signature = match source.local_id.ty {
            dir::NodeType::Member => {
                let member_id = dir::LocalNodeId::<dir::Member>::new(source.local_id.id);

                view.get(member_id).signature()
            }
            dir::NodeType::TypeMember => {
                let member_id = dir::LocalNodeId::<dir::TypeMember>::new(source.local_id.id);

                view.get(member_id).signature()
            }
            _ => {
                return Err(QueryError::invalid(format!(
                    "member signature source: {source:?}"
                )));
            }
        };

        signature.ok_or(QueryError::missing(format!("member signature: {source:?}")))
    }

    /// Format the type carried by one member.
    fn member_type(&self, member: &dir::DefinitionMember) -> QueryResult<String> {
        let type_id = if let Some(symbol) = member.type_symbol() {
            self.types()?
                .get_symbol_type_id(symbol)
                .ok_or(QueryError::missing(format!("member type: {symbol:?}")))?
        } else {
            member.value_type().ok_or(QueryError::missing(format!(
                "member type: {:?}",
                member.source()
            )))?
        };

        self.global_type(type_id)
    }

    /// Format the authored generic parameters for one declaration symbol.
    pub(crate) fn symbol_generics(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let module = self.program.module(symbol_id.module_id)?;
        let symbols = module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let source = symbol.declaration.ok_or(QueryError::invalid(format!(
            "type item symbol: {symbol_id:?}"
        )))?;
        if source.local_id.ty != dir::NodeType::Declaration {
            return Err(QueryError::invalid(format!(
                "type item symbol: {symbol_id:?}"
            )));
        }

        // read generic parameters from the exact declaration node
        let declaration_id = source
            .local_id
            .try_into_typed::<dir::Declaration>()
            .map_err(|_| QueryError::invalid(format!("type item symbol: {symbol_id:?}")))?;
        let declaration = module.view()?.get::<dir::Declaration>(declaration_id);
        let parameters = declaration
            .generic_parameters()
            .ok_or(QueryError::invalid(format!(
                "type item symbol: {symbol_id:?}"
            )))?;
        let formatted = Formatter::new(&module, self.program).generics(parameters)?;

        Ok((!formatted.is_empty()).then_some(formatted))
    }
}

/// Resolve the package relative module path without extension.
fn module_path_without_extension(module: &Module, package: &Package) -> QueryResult<String> {
    let Some(module_path) = module.path.as_ref() else {
        return Err(QueryError::missing(format!("module path: {:?}", module.id)));
    };
    let Some(package_path) = package.path.as_ref() else {
        return Err(QueryError::missing(format!(
            "package path: {:?}",
            package.id
        )));
    };
    let relative = module_path
        .strip_prefix(package_path)
        .map_err(|_| QueryError::invalid(format!("module path: {:?}", module.id)))?;
    let relative = relative
        .to_str()
        .ok_or_else(|| QueryError::invalid(format!("non-Unicode module path: {relative:?}")))?;
    let module_path = normalize_path_separators(relative);

    Ok(strip_module_extension(&module_path))
}

/// Normalize a module path to use forward slashes.
fn normalize_path_separators(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    normalized.trim_start_matches('/').to_string()
}

/// Convert a static key into a symbol path segment.
fn static_key_segment(key: Option<dir::StaticKey>, strings: &StringPool) -> Option<String> {
    let key = key?;
    match key {
        dir::StaticKey::Name(name) => Some(strings.get(name).to_string()),
        dir::StaticKey::Index(index) => Some(index.to_string()),
        dir::StaticKey::Symbol(_) => None,
    }
}
