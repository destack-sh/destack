use destack_dir as dir;
use destack_dir::{
    AnchoredGlobalNodeId, EnumBackingType, EnumFieldValue, GlobalSymbolId, NodeType,
};
use destack_workspace::ProfileId;

use crate::{BuildRequirementError, Compiler, LowerError, LowerResult};

use crate::lower::ModuleLowerer;

/// Enum field value with its backing type.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EnumFieldValueDescriptor {
    /// The enum backing type.
    pub(crate) backing: EnumBackingType,
    /// The enum field value.
    pub(crate) value: EnumFieldValue,
}

/// Resolve the backing type and value for an enum field symbol.
pub(crate) fn enum_field_value_for_symbol(
    compiler: &Compiler,
    profile: ProfileId,
    member_symbol: GlobalSymbolId,
    node: AnchoredGlobalNodeId,
) -> LowerResult<Option<EnumFieldValueDescriptor>> {
    // load the analyzed dir artifact for this symbol
    let snapshot = compiler.require_artifact_dir(destack_workspace::ArtifactKey::dir_analyzed(
        member_symbol.module_id,
        profile,
    ));
    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(BuildRequirementError::NotReady { requirement }) => {
            return Err(LowerError::Yield { requirement });
        }
        Err(BuildRequirementError::Failed { requirement }) => {
            return Err(LowerError::UnsatisfiedRequirement { requirement });
        }
    };
    let symbols = &snapshot.symbols;
    let types = &snapshot.types;

    // require the member symbol to be an enum field
    let member_entry = symbols.get_symbol(member_symbol.local_id);
    let primary = member_entry.primary_declaration;
    let Some(primary) = primary else {
        return Ok(None);
    };
    if primary.local_id.ty != NodeType::EnumField {
        return Ok(None);
    }

    // resolve the enum symbol that owns this field
    let scope = symbols.get_scope_by_symbol(member_symbol.local_id);
    let Some(scope_owner) = scope.owner_id else {
        return Ok(None);
    };
    if scope_owner.ty != dir::SymbolType::Enum {
        return Ok(None);
    }
    let enum_symbol = scope_owner.into_global(member_symbol.module_id);

    // read the enum backing type
    let backing = types.get_enum_backing_type(enum_symbol).ok_or_else(|| {
        LowerError::UnsupportedConstruct {
            node,
            message: "enum missing backing type".to_string(),
        }
    })?;

    // resolve the stored enum field value
    let value = types.get_enum_field_value(member_symbol).ok_or_else(|| {
        LowerError::UnsupportedConstruct {
            node,
            message: "enum field missing value".to_string(),
        }
    })?;

    Ok(Some(EnumFieldValueDescriptor { backing, value }))
}

impl ModuleLowerer<'_> {
    /// Resolve an enum symbol for a type id when available.
    pub(crate) fn enum_symbol_for_type(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::GlobalSymbolId> {
        // check direct enum references
        if let dir::Type::Reference { symbol, .. } = self.types.get_type(type_id)
            && symbol.ty() == dir::SymbolType::Enum
        {
            return Some(*symbol);
        }

        // check enum instance types
        if let Some(symbol) = self.types.symbol_for_instance_type(type_id)
            && symbol.ty() == dir::SymbolType::Enum
        {
            return Some(symbol);
        }

        // require type sources that point at declarations
        let source = self.types.get_type_source(type_id);
        if source.ty != dir::NodeType::Declaration {
            return None;
        }

        // resolve enum declarations from the type source
        let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(source.id);
        let declaration = self.dir_tree.get(declaration_id);
        if let dir::Declaration::Enum { descriptor, .. } = declaration {
            Some(descriptor.symbol.into_global(self.module_id))
        } else {
            None
        }
    }
}
