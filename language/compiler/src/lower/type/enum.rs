use destack_dir as dir;

use destack_artifact::DiagnosticAnchor;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerResult, LowerError};

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
    context: &dyn ProviderContext,
    profile: ProfileId,
    member_symbol: dir::GlobalSymbolId,
    anchor: DiagnosticAnchor,
) -> CompilerResult<Option<EnumFieldValueDescriptor>> {
    // load the bound and checked DIR artifacts for this symbol
    let bound = compiler.dir_bound(context, member_symbol.module_id, profile);
    let bound = match bound {
        Ok(snapshot) => snapshot,
        Err(error) => return Err(error.into()),
    };
    let checked = compiler.dir_checked(context, member_symbol.module_id, profile);
    let checked = match checked {
        Ok(snapshot) => snapshot,
        Err(error) => return Err(error.into()),
    };
    let expanded = compiler.dir_expanded(context, member_symbol.module_id, profile);
    let expanded = match expanded {
        Ok(snapshot) => snapshot,
        Err(error) => return Err(error.into()),
    };
    let symbols = expanded.binding_table(&bound);
    let types = checked.type_table(&bound, &expanded);

    // require the member symbol to be an enum field
    let member_entry = symbols.get_symbol(member_symbol.local_id);
    let primary = member_entry.declaration;
    let Some(primary) = primary else {
        return Ok(None);
    };
    if primary.local_id.ty != dir::NodeType::EnumField {
        return Ok(None);
    }

    // resolve the enum symbol that owns this field
    let scope = symbols.get_scope_by_symbol(member_symbol.local_id);
    let Some(scope_owner) = scope.owner else {
        return Ok(None);
    };
    let scope_owner_entry = symbols.get_symbol(scope_owner);
    if scope_owner_entry.form != dir::SymbolForm::Enum {
        return Ok(None);
    }
    let enum_symbol = scope_owner.into_global(member_symbol.module_id);

    // read the enum backing type
    let backing = types.get_enum_backing_type(enum_symbol).ok_or_else(|| {
        LowerError::UnsupportedConstruct {
            anchor: anchor.clone(),
            message: "enum missing backing type".to_string(),
        }
    })?;

    // resolve the stored enum field value
    let value = types.get_enum_field_value(member_symbol).ok_or_else(|| {
        LowerError::UnsupportedConstruct {
            anchor,
            message: "enum field missing value".to_string(),
        }
    })?;

    Ok(Some(EnumFieldValueDescriptor { backing, value }))
}
