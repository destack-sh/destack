use destack_core::StringPool;
use destack_dir as dir;
use rustc_hash::FxHashSet;

use crate::{DocError, DocResult};

use super::{FormattedSignature, Printer};

impl Printer<'_, '_, '_> {
    /// Format one generic parameter type.
    pub(super) fn generic_parameter_type(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> DocResult<String> {
        let module = self.program.module(parameter.module_id)?;
        let binding = module.generics().get_parameter(parameter.local_id);

        let name = match binding.key {
            dir::GenericParameterKey::Symbol(symbol) => {
                let symbol = module.bindings().get_symbol(symbol.local_id);

                static_key_segment(symbol.key, module.strings())
            }
            dir::GenericParameterKey::Anonymous => {
                let generics = module.generics();
                let position = generics.parameter_position(parameter.local_id);
                let regions = region_names_before(generics, module, parameter.local_id);

                Some(binding.canonical_name(position, regions.iter().map(String::as_str)))
            }
        };

        let name = name
            .ok_or_else(|| DocError::missing(format!("generic parameter name: {parameter:?}")))?;

        Ok(name)
    }

    /// Format one symbol path.
    pub(super) fn symbol(&self, symbol_id: dir::GlobalSymbolId) -> DocResult<String> {
        let module = self.program.module(symbol_id.module_id)?;
        let symbols = module.bindings();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        let Some(symbol_name) = static_key_segment(symbol.key, module.strings()) else {
            return Err(DocError::missing(format!(
                "symbol path segment: {symbol_id:?}"
            )));
        };
        let mut segments = vec![symbol_name];

        let mut scope_id = symbol.scope.id;
        let mut seen_scopes = FxHashSet::default();
        loop {
            if !seen_scopes.insert(scope_id) {
                return Err(DocError::cycle(format!("symbol scope: {symbol_id:?}")));
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
                    return Err(DocError::missing(format!(
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

    /// Format one symbol and retain its declared name interval.
    pub(crate) fn formatted_symbol_signature(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> DocResult<FormattedSignature> {
        let module = self.program.module(symbol_id.module_id)?;
        let formatter = Printer::new(module, self.program);
        let symbols = module.bindings();
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
                    .map_err(|_| DocError::invalid(format!("signature symbol: {symbol_id:?}")))?;
                let declaration = module.view().get::<dir::Declaration>(declaration_id);

                formatter.declaration_signature(declaration)?
            }
            dir::NodeType::Parameter => {
                let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(declaration.local_id.id);

                let signature = formatter.parameter_symbol_signature(parameter_id)?;

                FormattedSignature::plain(signature)
            }
            dir::NodeType::GenericParameter => {
                let parameter_id =
                    dir::LocalNodeId::<dir::GenericParameter>::new(declaration.local_id.id);

                let signature = formatter.generic_parameter(parameter_id)?;

                FormattedSignature::plain(signature)
            }
            dir::NodeType::Pattern => {
                let name = name
                    .as_deref()
                    .ok_or_else(|| DocError::missing(format!("signature name: {symbol_id:?}")))?;

                formatter.binding_symbol_signature(name, symbol_id)?
            }
            dir::NodeType::Member | dir::NodeType::TypeMember | dir::NodeType::EnumField => {
                formatter.member_symbol_signature(symbol_id)?
            }
            node_type => {
                return Err(DocError::invalid(format!(
                    "signature declaration: {symbol_id:?}, {node_type:?}"
                )));
            }
        };

        Ok(signature)
    }

    /// Format one member symbol.
    fn member_symbol_signature(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> DocResult<FormattedSignature> {
        let (declaring, definition, member) = self
            .module
            .definition_member(symbol_id)?
            .ok_or_else(|| DocError::missing(format!("signature member: {symbol_id:?}")))?;
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

        self.format_member_signature(member, &name)
    }

    /// Format one member as declared inside its owner.
    pub(crate) fn member_signature(
        &self,
        member: &dir::DefinitionMember,
    ) -> DocResult<FormattedSignature> {
        let name = self.member_name(member)?;

        self.format_member_signature(member, &name)
    }

    /// Format one resolved member under the supplied display name.
    fn format_member_signature(
        &self,
        member: &dir::DefinitionMember,
        name: &str,
    ) -> DocResult<FormattedSignature> {
        let signature = match member {
            dir::DefinitionMember::Field(_) => {
                let type_text = self.member_type(member)?;

                FormattedSignature::named(String::new(), name, format!(": {type_text}"))
            }
            dir::DefinitionMember::Method(method) => {
                let signature = self.authored_member_signature(member.source())?;
                let signature = self.named_call_signature(name, signature, false)?;
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

                signature.prepend(&format!("{static_prefix}{role_prefix}"))
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
                    (None, None) => FormattedSignature::named(String::new(), name, String::new()),
                    (Some(constraint), None) => {
                        FormattedSignature::named(String::new(), name, format!(": {constraint}"))
                    }
                    (None, Some(value)) => {
                        FormattedSignature::named(String::new(), name, format!(" = {value}"))
                    }
                    (Some(constraint), Some(value)) => FormattedSignature::named(
                        String::new(),
                        name,
                        format!(": {constraint} = {value}"),
                    ),
                }
            }
            dir::DefinitionMember::AssociatedConst(_) => {
                let type_text = self.member_type(member)?;

                FormattedSignature::named(String::new(), name, format!(": {type_text}"))
            }
            dir::DefinitionMember::EnumVariant(variant) => {
                let suffix = self.enum_variant_suffix(variant)?;

                FormattedSignature::named(String::new(), name, suffix)
            }
            dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_) => {
                let source = member.source();
                if source.local_id.ty != dir::NodeType::TypeMember {
                    return Err(DocError::invalid(format!(
                        "callable type member source: {source:?}"
                    )));
                }
                let member_id = dir::LocalNodeId::<dir::TypeMember>::new(source.local_id.id);

                self.type_member_signature(self.module.view().get(member_id))?
            }
            dir::DefinitionMember::IndexSignature(_) => {
                let type_text = self.member_type(member)?;

                FormattedSignature::named(String::new(), name, format!(": {type_text}"))
            }
        };

        Ok(signature)
    }

    /// Format one parameter symbol.
    fn parameter_symbol_signature(
        &self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> DocResult<String> {
        let parameter = self.module.view().get::<dir::Parameter>(parameter_id);
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
    ) -> DocResult<FormattedSignature> {
        let symbol = self.module.bindings().get_symbol(symbol_id.local_id);
        let Some(type_id) = self.types().get_symbol_type_id(symbol_id) else {
            return Err(DocError::missing(format!(
                "binding symbol type: {symbol_id:?}"
            )));
        };
        let type_text = self.global_type(type_id)?;
        let keyword = if symbol.binding_mutability == Some(dir::Mutability::Immutable) {
            "const"
        } else {
            "let"
        };

        Ok(FormattedSignature::named(
            format!("{keyword} "),
            name,
            format!(": {type_text}"),
        ))
    }

    /// Return one authored member function signature.
    fn authored_member_signature(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> DocResult<&dir::FunctionSignature> {
        let view = self.module.view();

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
                return Err(DocError::invalid(format!(
                    "member signature source: {source:?}"
                )));
            }
        };

        signature.ok_or_else(|| DocError::missing(format!("member signature: {source:?}")))
    }

    /// Format the type carried by one member.
    fn member_type(&self, member: &dir::DefinitionMember) -> DocResult<String> {
        let type_id = if let Some(symbol) = member.type_symbol() {
            self.types()
                .get_symbol_type_id(symbol)
                .ok_or_else(|| DocError::missing(format!("member type: {symbol:?}")))?
        } else {
            member
                .value_type()
                .ok_or_else(|| DocError::missing(format!("member type: {:?}", member.source())))?
        };

        self.global_type(type_id)
    }

    /// Format an enum variant's authored value.
    fn enum_variant_suffix(&self, variant: &dir::EnumVariantDefinition) -> DocResult<String> {
        if variant.source.module_id != self.module.module_id() {
            return Err(DocError::invalid(format!(
                "enum variant source module: {:?}",
                variant.source
            )));
        }

        // read an explicitly authored discriminant from the source tree
        let field_id = variant
            .source
            .local_id
            .try_into_typed::<dir::EnumField>()
            .map_err(DocError::invalid)?;
        let view = self.module.view();
        let field = view.get::<dir::EnumField>(field_id);
        let Some(value_id) = field.value else {
            return Ok(String::new());
        };

        // retain the exact expression instead of printing the resolved scalar value
        let value_span = self.module.node_span(view, value_id.into_any())?;
        let value = self.module.source_text(value_span)?;

        Ok(format!(" = {value}"))
    }
}

/// Convert a static key into a symbol path segment.
fn static_key_segment(key: Option<dir::StaticKey>, strings: &StringPool) -> Option<String> {
    let key = key?;
    match key {
        dir::StaticKey::Name(name) => Some(strings.get(name).to_string()),
        dir::StaticKey::Index(index) => Some(index.to_string()),
    }
}

/// Return the region names one template declares ahead of one parameter.
fn region_names_before(
    generics: &dir::GenericTable<'_>,
    module: &crate::generate::Module<'_>,
    parameter: dir::LocalGenericParameterId,
) -> Vec<String> {
    generics.region_names_before(parameter, |symbol| {
        let symbol = module.bindings().get_symbol(symbol.local_id);

        static_key_segment(symbol.key, module.strings()).unwrap_or_default()
    })
}
