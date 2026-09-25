use destack_mir as mir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintOutput, LintResult, MirModule};

const LARGE_VARIANT_EXCESS_BYTES: u32 = 200;

declare_lint! {
    /// Disallow variants that disproportionately enlarge an inline union.
    pub LARGE_VARIANT {
        id: "large-variant",
        summary: "Disallow variants that disproportionately enlarge an inline union",
        explanation: r#"
An unusually large variant determines the storage required by every value of its union type.
Instead, you SHOULD place the large payload behind `Box` when the smaller variants are common enough for the reduced inline size to matter.
Keeping the payload inline can remain faster when the large variant dominates actual use.
"#,
        example: {
            reported: r#"
struct Packet {
    bytes: [uint8; 256];
}

newtype Message = int32 | Packet;
"#,
            accepted: r#"
import { Box } from "destack:memory";

struct Packet {
    bytes: [uint8; 256];
}

newtype Message = int32 | Box<Packet>;
"#,
        },
        provenance: [Clippy("large_enum_variant")],
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Report variant types whose largest case exceeds every other case by more than 200 bytes.
fn check(module: &mut MirModule<'_>, lint: &Lint) -> LintResult {
    let layouts = &module.lowered.layouts;
    let tree = &module.lowered.tree;
    let mut output = LintOutput::default();

    // inspect every declared variant representation
    for (declaration_id, declaration) in tree.iter_nodes::<mir::TypeDeclaration>() {
        // open templates have no single storage size
        if !declaration.generics.is_empty() {
            continue;
        }
        let Some(definition) = declaration.definition else {
            continue;
        };
        let Some(variant) = select_variant_layout(definition, tree, layouts)? else {
            continue;
        };
        let Some(excess) = measure_largest_case_excess(variant, layouts)? else {
            continue;
        };
        if excess <= LARGE_VARIANT_EXCESS_BYTES {
            continue;
        }

        let anchor = module.anchor(declaration_id.into_any())?;
        let diagnostic = lint
            .diagnostic(
                format!("largest variant exceeds the next-largest by {excess} bytes"),
                anchor,
            )
            .help("place the large payload behind Box when smaller variants are common");
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the variant layout beneath transparent nominal storage.
fn select_variant_layout<'a>(
    ty: mir::TypeId,
    tree: &mir::Tree,
    layouts: &'a mir::LayoutTable,
) -> Result<Option<&'a mir::VariantLayout>, ProviderError> {
    let mut represented = ty;

    // unwrap nominal types to identify the represented type
    while let mir::Type::Newtype { value, .. } = tree.get(represented) {
        represented = *value;
    }
    if !matches!(tree.get(represented), mir::Type::Variant { .. }) {
        return Ok(None);
    }

    // require the canonical layout for every concrete variant
    let mut layout = layouts.type_layout(ty).ok_or_else(|| {
        ProviderError::internal(format!("variant type {ty:?} has no concrete layout"))
    })?;

    // unwrap transparent nominal representations
    while let mir::LayoutShape::Newtype(newtype) = &layout.shape {
        layout = layouts.layout(newtype.backing_layout);
    }

    match &layout.shape {
        mir::LayoutShape::Variant(variant) => Ok(Some(variant)),
        shape => Err(ProviderError::internal(format!(
            "variant type {ty:?} has unexpected layout {shape:?}"
        ))),
    }
}

/// Measure the byte difference between the largest and second-largest cases.
fn measure_largest_case_excess(
    variant: &mir::VariantLayout,
    layouts: &mir::LayoutTable,
) -> Result<Option<u32>, ProviderError> {
    let mut largest = 0;
    let mut second_largest = 0;

    // retain the two largest concrete payload layouts
    for case in &variant.cases {
        let layout = layouts.type_layout(case.ty).ok_or_else(|| {
            ProviderError::internal(format!(
                "variant case type {:?} has no concrete layout",
                case.ty
            ))
        })?;

        // promote the new largest case
        if layout.size > largest {
            second_largest = largest;
            largest = layout.size;
        }
        // retain the new runner-up
        else if layout.size > second_largest {
            second_largest = layout.size;
        }
    }

    Ok((variant.cases.len() >= 2).then(|| largest - second_largest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report one variant case that dominates the union representation.
    #[test]
    fn test_reports_large_variant() {
        let session = TestSession::mir(
            &LARGE_VARIANT,
            r#"
type Message = newtype<variant<uint1> { 0uint1 = int32; 1uint1 = [uint8; 256]; }>;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[large-variant]: largest variant exceeds the next-largest by 252 bytes
 ──▶ main.mir:1:1
  │
1 │ type Message = newtype<variant<uint1> { 0uint1 = int32; 1uint1 = [uint8; 256]; }>;
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │

 = help: place the large payload behind Box when smaller variants are common
"#,
        );
    }

    /// Accept variant cases whose representation sizes remain close.
    #[test]
    fn test_accepts_similar_variants() {
        let session = TestSession::mir(
            &LARGE_VARIANT,
            r#"
type Message = variant<uint2> {
    0uint2 = int32;
    1uint2 = [uint8; 128];
    2uint2 = [uint8; 256];
};
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a large single-case representation.
    #[test]
    fn test_accepts_single_variant() {
        let session = TestSession::mir(
            &LARGE_VARIANT,
            r#"
type Message = variant<uint1> { 0uint1 = [uint8; 256]; };
"#,
        );

        session.assert_no_diagnostics();
    }
}
