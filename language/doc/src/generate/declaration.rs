use std::path::Path;

use tspp_dir as dir;

use crate::print::{FormattedSignature, Printer};
use crate::{
    DeclarationKind, DeclarationReference, DocError, DocResult, Signature, SourceReference,
    TextRange,
};

use super::{Generator, Module};

impl Generator<'_> {
    /// Generate one checked declaration reference.
    pub(super) fn declaration_reference(
        &self,
        symbol_id: dir::GlobalSymbolId,
        package_path: Option<&Path>,
    ) -> DocResult<DeclarationReference> {
        let module = self.module(symbol_id.module_id)?;
        let symbols = module.bindings();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let name = symbol
            .name()
            .map(|name| module.strings().get(name).to_string());
        let declaration = symbol.declaration.ok_or_else(|| {
            DocError::missing(format!("public declaration source: {symbol_id:?}"))
        })?;
        let view = module.view();
        let kind = symbol_kind(symbol)?;
        let printer = Printer::new(module, self);
        let signature = printer.formatted_symbol_signature(symbol_id)?;
        let signature = signature.into();
        let documentation = self.symbol_documentation(symbol_id)?;
        let members = if let Some(definition) = module.definitions().definition(symbol_id) {
            self.declaration_members(module, definition, package_path)?
        } else {
            Vec::new()
        };
        let span = module.node_span(view, declaration.local_id)?;
        let source =
            SourceReference::build(span, package_path, self.repository(), self.revision())?;

        Ok(DeclarationReference {
            name,
            kind,
            signature,
            documentation,
            members,
            source,
        })
    }

    /// Return the public members of one declaration.
    fn declaration_members(
        &self,
        module: &Module<'_>,
        definition: &dir::Definition,
        package_path: Option<&Path>,
    ) -> DocResult<Vec<DeclarationReference>> {
        let view = module.view();
        let mut members = Vec::new();

        // retain public checked members in authored order
        for member in definition.members() {
            let source = member.source();
            if source.module_id != module.module_id() {
                return Err(DocError::invalid(format!(
                    "member source module: {source:?}, {:?}",
                    module.module_id()
                )));
            }
            if source.local_id.ty == dir::NodeType::Member {
                let member_id = dir::LocalNodeId::<dir::Member>::new(source.local_id.id);
                if member_visibility(view.get(member_id)) == Some(dir::Visibility::Private) {
                    continue;
                }
            }

            members.push(self.member_reference(module, member, package_path)?);
        }

        Ok(members)
    }

    /// Generate one checked declaration member.
    fn member_reference(
        &self,
        module: &Module<'_>,
        member: &dir::DefinitionMember,
        package_path: Option<&Path>,
    ) -> DocResult<DeclarationReference> {
        let printer = Printer::new(module, self);
        let name = printer.member_name(member)?;
        let signature = printer.member_signature(member)?.into();
        let source = member.source();
        let documentation = self.node_documentation(module, source.local_id)?;
        let view = module.view();
        let span = module.node_span(view, source.local_id)?;
        let source =
            SourceReference::build(span, package_path, self.repository(), self.revision())?;

        Ok(DeclarationReference {
            name: Some(name),
            kind: definition_member_kind(member),
            signature,
            documentation,
            members: Vec::new(),
            source,
        })
    }

    /// Return documentation attached directly to one source node.
    fn node_documentation(
        &self,
        module: &Module<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> DocResult<Option<String>> {
        let view = module.view();
        let Some(documentation) = view.get_documentation_any(node_id) else {
            return Ok(None);
        };

        Ok(Some(
            Printer::new(module, self).documentation(documentation)?,
        ))
    }

    /// Return documentation attached to one symbol declaration.
    fn symbol_documentation(&self, symbol_id: dir::GlobalSymbolId) -> DocResult<Option<String>> {
        let module = self.module(symbol_id.module_id)?;
        let symbols = module.bindings();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };
        if let Some(documentation) = self.node_documentation(module, declaration.local_id)? {
            return Ok(Some(documentation));
        }

        // declarator documentation may belong to the declaration statement
        let view = module.view();
        let Some(declarator) = view.ancestor::<dir::Declarator>(declaration.local_id) else {
            return Ok(None);
        };
        let owner = view
            .get_parent_for(declarator)
            .ok_or_else(|| DocError::missing(format!("declarator owner: {symbol_id:?}")))?;
        let documentation = view
            .get_documentation(declarator)
            .or_else(|| view.get_documentation_any(owner));
        let Some(documentation) = documentation else {
            return Ok(None);
        };

        Ok(Some(
            Printer::new(module, self).documentation(documentation)?,
        ))
    }
}

impl From<FormattedSignature> for Signature {
    /// Convert one formatted signature into its stable artifact form.
    fn from(signature: FormattedSignature) -> Self {
        Self {
            text: signature.text,
            name: signature.name.map(|name| TextRange {
                start: name.start,
                end: name.end,
            }),
        }
    }
}

/// Return one symbol's stable public category.
fn symbol_kind(symbol: &dir::Symbol) -> DocResult<DeclarationKind> {
    let kind = match symbol.kind {
        dir::SymbolKind::Class => DeclarationKind::Class,
        dir::SymbolKind::Enum => DeclarationKind::Enum,
        dir::SymbolKind::Function => DeclarationKind::Function,
        dir::SymbolKind::Interface => DeclarationKind::Interface,
        dir::SymbolKind::NewtypeInterface => DeclarationKind::NewtypeInterface,
        dir::SymbolKind::Newtype => DeclarationKind::Newtype,
        dir::SymbolKind::Extension => DeclarationKind::Extension,
        dir::SymbolKind::Struct => DeclarationKind::Struct,
        dir::SymbolKind::TypeAlias => DeclarationKind::TypeAlias,
        dir::SymbolKind::Variable
            if symbol.binding_mutability == Some(dir::Mutability::Immutable) =>
        {
            DeclarationKind::Constant
        }
        dir::SymbolKind::Variable => DeclarationKind::Variable,
        kind => {
            return Err(DocError::invalid(format!(
                "public declaration kind: {kind:?}"
            )));
        }
    };

    Ok(kind)
}

/// Return one checked definition member's stable public category.
fn definition_member_kind(member: &dir::DefinitionMember) -> DeclarationKind {
    match member.kind() {
        dir::MemberKind::AssociatedConst => DeclarationKind::AssociatedConst,
        dir::MemberKind::AssociatedType => DeclarationKind::AssociatedType,
        dir::MemberKind::CallSignature | dir::MemberKind::Method => DeclarationKind::Method,
        dir::MemberKind::Constructor | dir::MemberKind::ConstructSignature => {
            DeclarationKind::Constructor
        }
        dir::MemberKind::Field | dir::MemberKind::IndexSignature => DeclarationKind::Field,
        dir::MemberKind::Property => DeclarationKind::Property,
        dir::MemberKind::Variant => DeclarationKind::EnumMember,
    }
}

/// Return one value member's authored visibility.
fn member_visibility(member: &dir::Member) -> Option<dir::Visibility> {
    match member {
        dir::Member::AssociatedType { visibility, .. }
        | dir::Member::AssociatedConst { visibility, .. }
        | dir::Member::Field { visibility, .. }
        | dir::Member::Method { visibility, .. } => *visibility,
        dir::Member::StaticBlock { .. } | dir::Member::ConstBlock { .. } | dir::Member::Error => {
            None
        }
    }
}
