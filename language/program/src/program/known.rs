use tspp_core::StringId;

use crate::{Program, ProgramLoadError, TypeId};

/// The language item of the string representation.
pub const STRING_ITEM: &str = "string.String";
/// The code unit slice field of the string representation.
pub const STRING_UNITS_FIELD: &str = "codeUnits";
/// The language item of the big integer representation.
pub const BIGINT_ITEM: &str = "math.BigInt";
/// The limb slice field of the big integer representation.
pub const BIGINT_LIMBS_FIELD: &str = "limbs";
/// The sign field of the big integer representation.
pub const BIGINT_SIGN_FIELD: &str = "sign";
/// The limb count field of the big integer representation.
pub const BIGINT_LENGTH_FIELD: &str = "length";

/// The well-known language types whose representation the runtime reads, resolved at load.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KnownTypeTable {
    /// The string representation, when the program declares one.
    pub string: Option<KnownString>,
}

/// The resolved representation of the language string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownString {
    /// The string object type.
    pub ty: TypeId,
    /// The byte offset of the code unit slice inside the object.
    pub units_offset: u32,
}

impl KnownTypeTable {
    /// Resolve the well-known types one program declares.
    pub(crate) fn resolve(program: &Program) -> Result<Self, ProgramLoadError> {
        Ok(Self {
            string: KnownString::resolve(program)?,
        })
    }
}

impl KnownString {
    /// Resolve the string representation, none when the program declares no string type.
    fn resolve(program: &Program) -> Result<Option<Self>, ProgramLoadError> {
        // find the type the string language item names
        let sections = program.sections();
        let item = StringId::for_text(STRING_ITEM);
        let Some(ty) = program.types().language_type(sections, item) else {
            return Ok(None);
        };

        // read the code unit slice field off its layout
        let units = StringId::for_text(STRING_UNITS_FIELD);
        let field = program.layout(ty).and_then(|layout| {
            program
                .layouts()
                .fields(sections, layout)
                .iter()
                .find(|field| field.name.get() == Some(units))
        });
        let Some(field) = field else {
            return Err(ProgramLoadError::InvalidKnownType(STRING_ITEM));
        };

        Ok(Some(Self {
            ty,
            units_offset: field.offset,
        }))
    }
}
