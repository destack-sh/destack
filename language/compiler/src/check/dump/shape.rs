use crate::check::{Dump, DumpContext, ShapeMember, TupleElement};

use super::format::{dump_list, dump_record};

impl Dump for TupleElement {
    /// Render one tuple element.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TupleElement",
            [
                ("label", context.optional_string(self.label)),
                ("type", self.ty.dump(context)),
                ("optional", self.is_optional.to_string()),
                ("readonly", self.is_readonly.to_string()),
                ("rest", self.is_rest.to_string()),
            ],
        )
    }
}

impl Dump for ShapeMember {
    /// Render one shape member.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Field {
                key,
                ty,
                is_optional,
                is_readonly,
            } => dump_record(
                "ShapeMember.Field",
                [
                    ("key", key.dump(context)),
                    ("type", ty.dump(context)),
                    ("optional", is_optional.to_string()),
                    ("readonly", is_readonly.to_string()),
                ],
            ),
            Self::Spread { origin, source } => dump_record(
                "ShapeMember.Spread",
                [
                    ("source", origin.dump(context)),
                    ("value", source.dump(context)),
                ],
            ),
            Self::CallSignature { ty } => {
                dump_record("ShapeMember.CallSignature", [("type", ty.dump(context))])
            }
            Self::ConstructSignature { ty } => dump_record(
                "ShapeMember.ConstructSignature",
                [("type", ty.dump(context))],
            ),
            Self::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => dump_record(
                "ShapeMember.IndexSignature",
                [
                    ("name", name.dump(context)),
                    ("key_type", key_type.dump(context)),
                    ("value_type", value_type.dump(context)),
                    ("optional", is_optional.to_string()),
                    ("readonly", is_readonly.to_string()),
                ],
            ),
        }
    }
}

/// Render already formatted members as one dump list.
pub(super) fn dump_members(members: String) -> String {
    dump_list(members)
}
