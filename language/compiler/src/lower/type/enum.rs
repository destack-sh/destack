use destack_dir as dir;

use destack_workspace::{ProfileId, Revision};

use crate::{Compiler, LowerError, LowerResult, RequirementError};

/// Enum field value with its backing type.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EnumFieldValueDescriptor {
    /// The enum backing type.
    pub(crate) backing: dir::EnumBackingType,
    /// The enum field value.
    pub(crate) value: dir::EnumFieldValue,
}

/// Resolve the backing type and value for an enum field symbol.
pub(crate) fn enum_field_value_for_symbol(
    compiler: &Compiler,
    revision: Revision,
    profile: ProfileId,
    member_symbol: dir::GlobalSymbolId,
    node: dir::AnchoredGlobalNodeId,
) -> LowerResult<Option<EnumFieldValueDescriptor>> {
    // load the analyzed dir artifact for this symbol
    let snapshot =
        compiler.require_artifact_dir_analyzed(revision, member_symbol.module_id, profile);
    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(RequirementError::NotReady { requirement }) => {
            return Err(LowerError::Yield { requirement });
        }
        Err(RequirementError::Failed { requirement }) => {
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
    if primary.local_id.ty != dir::NodeType::EnumField {
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
