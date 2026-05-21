use std::fs;
use std::path::{Path, PathBuf};

use destack_source::FileType;

use crate::core::fixtures_dir;

use super::array::large_array;
use super::block::{control_flow, nested_block};
use super::call::deep_call;
use super::class::large_class;
use super::comptime::comptime_forms;
use super::declaration::{damaged_declaration, large_declaration};
use super::decorator::decorator_forms;
use super::dependency::large_import_export;
use super::error::error_forms;
use super::expression::{convoluted_expressions, damaged_expression, nested_try};
use super::generic::ambiguous_generics;
use super::interface::large_interface;
use super::member::deep_member;
use super::memory::memory_forms;
use super::module::module_forms;
use super::object::{ambiguous_objects, large_object};
use super::operator::operator_forms;
use super::pathology::{
    damaged_arrow_return_heads, damaged_delimiters, damaged_function_type_heads,
    damaged_generic_heads, damaged_infix_chains, damaged_parenthesized_heads, deep_block,
    deep_parentheses, deep_tree, massive_file, trivia_flood, wide_call,
};
use super::pattern::{convoluted_patterns, damaged_type, nested_match};
use super::range::range_forms;
use super::sequence::{sequence_pattern_forms, sequence_type_forms};
use super::signature::large_signature;
use super::ternary::nested_ternary;
use super::torture::{
    long_binary_chain, long_logical_chain, long_nullish_chain, long_postfix_chain,
    nested_lambda_chain, parenthesized_binary_chain,
};
use super::tree::{ambiguous_tsx, damaged_tsx, nested_tsx};
use super::trivia::{damaged_trivia, large_trivia, trivia_wall};
use super::ty::{convoluted_types, deep_type, large_type};
use super::using::using_forms;
use super::weave::{woven_destack_forms, woven_source_forms, woven_tsx_forms};

const DEFAULT_WIDTH: usize = 96;
const REGULAR_LARGE: usize = 1_024;
const REGULAR_HUGE: usize = 2_048;
const REGULAR_MASSIVE: usize = 8_192;
const REGULAR_DENSE: usize = 4_096;
const REGULAR_WIDE: usize = 1_024;
const RECURSIVE_VALID_LARGE: usize = 256;
const RECURSIVE_EXPRESSION_VALID_LARGE: usize = 128;
const RECURSIVE_BOUNDED_HUGE: usize = 2_048;
const RECURSIVE_BOUNDED_MASSIVE: usize = 8_192;
const RECURSIVE_BOUNDED_DENSE: usize = 4_096;
const PATHOLOGICAL_LARGE: usize = 1_024;
const PATHOLOGICAL_MASSIVE: usize = 8_192;
const PATHOLOGICAL_BRUTAL: usize = 32_768;
const PATHOLOGICAL_MONSTER: usize = 262_144;
const PATHOLOGICAL_DEEP: usize = 2_048;
const PATHOLOGICAL_DEEPER: usize = 8_192;
const PATHOLOGICAL_DEEPEST: usize = 16_384;
const PATHOLOGICAL_ABSURD: usize = 32_768;
const RECOVERY_PATHOLOGICAL_LARGE: usize = 128;
const RECOVERY_PATHOLOGICAL_MASSIVE: usize = 1_024;
const RECOVERY_PATHOLOGICAL_BRUTAL: usize = 8_192;
const VALID: StressExpectation = StressExpectation::Valid;
const BOUNDED: StressExpectation = StressExpectation::Bounded;
const RECOVERY: StressExpectation = StressExpectation::Recovery;
const ALL_MODES: &[StressMode] = &[
    StressMode::Destack,
    StressMode::DestackDeclaration,
    StressMode::TypeScript,
    StressMode::TypeScriptXml,
    StressMode::TypeScriptDeclaration,
];
const SOURCE_MODES: &[StressMode] = &[
    StressMode::Destack,
    StressMode::TypeScript,
    StressMode::TypeScriptXml,
];
const DESTACK_MODES: &[StressMode] = &[StressMode::Destack];
const TSX_MODES: &[StressMode] = &[StressMode::Destack, StressMode::TypeScriptXml];
const REGULAR_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", REGULAR_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("huge", REGULAR_HUGE, 120),
    StressVariantShape::new("massive", REGULAR_MASSIVE, 140),
    StressVariantShape::new("wide", REGULAR_WIDE, 180),
    StressVariantShape::new("dense", REGULAR_DENSE, 56),
];
const RECOVERY_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("baseline", 1, DEFAULT_WIDTH),
    StressVariantShape::new("large", 128, DEFAULT_WIDTH),
    StressVariantShape::new("huge", 512, 120),
    StressVariantShape::new("wide", 256, 180),
    StressVariantShape::new("dense", 1_024, 56),
];
const PATHOLOGICAL_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("massive", PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariantShape::new("brutal", PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
    StressVariantShape::new("monster", PATHOLOGICAL_MONSTER, DEFAULT_WIDTH),
    StressVariantShape::new("dense", PATHOLOGICAL_MASSIVE, 56),
];
const PATHOLOGICAL_CAPPED_AT_BRUTAL_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("massive", PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariantShape::new("brutal", PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
    StressVariantShape::new("dense", PATHOLOGICAL_MASSIVE, 56),
];
const PATHOLOGICAL_CAPPED_AT_MASSIVE_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("massive", PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariantShape::new("dense", PATHOLOGICAL_MASSIVE, 56),
];
const DEEP_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("deep", PATHOLOGICAL_DEEP, DEFAULT_WIDTH),
    StressVariantShape::expect("deeper", PATHOLOGICAL_DEEPER, DEFAULT_WIDTH, BOUNDED),
    StressVariantShape::expect("deepest", PATHOLOGICAL_DEEPEST, DEFAULT_WIDTH, BOUNDED),
    StressVariantShape::expect("absurd", PATHOLOGICAL_ABSURD, DEFAULT_WIDTH, BOUNDED),
];
const RECURSIVE_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", RECURSIVE_VALID_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("wide", RECURSIVE_VALID_LARGE, 180),
    StressVariantShape::expect("bounded_huge", RECURSIVE_BOUNDED_HUGE, 120, BOUNDED),
    StressVariantShape::expect("bounded_massive", RECURSIVE_BOUNDED_MASSIVE, 140, BOUNDED),
    StressVariantShape::expect("bounded_dense", RECURSIVE_BOUNDED_DENSE, 56, BOUNDED),
];
const RECURSIVE_EXPRESSION_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", RECURSIVE_EXPRESSION_VALID_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("wide", RECURSIVE_EXPRESSION_VALID_LARGE, 180),
    StressVariantShape::expect("bounded_huge", RECURSIVE_BOUNDED_HUGE, 120, BOUNDED),
    StressVariantShape::expect("bounded_massive", RECURSIVE_BOUNDED_MASSIVE, 140, BOUNDED),
    StressVariantShape::expect("bounded_dense", RECURSIVE_BOUNDED_DENSE, 56, BOUNDED),
];
const RECOVERY_PATHOLOGICAL_VARIANTS: &[StressVariantShape] = &[
    StressVariantShape::new("large", RECOVERY_PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariantShape::new("massive", RECOVERY_PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariantShape::new("brutal", RECOVERY_PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
];

const CASES: &[StressSpec] = &[
    StressSpec::new("large_declaration", ALL_MODES, VALID, large_declaration),
    StressSpec::new("large_function", ALL_MODES, VALID, large_signature),
    StressSpec::new("large_class", ALL_MODES, VALID, large_class),
    StressSpec::new("large_interface", ALL_MODES, VALID, large_interface),
    StressSpec::new("large_type", ALL_MODES, VALID, large_type),
    StressSpec::new("large_import_export", ALL_MODES, VALID, large_import_export),
    StressSpec::new("large_trivia", SOURCE_MODES, VALID, large_trivia),
    StressSpec::new("large_array", SOURCE_MODES, VALID, large_array),
    StressSpec::new("large_object", SOURCE_MODES, VALID, large_object),
    StressSpec::new("nested_block", SOURCE_MODES, VALID, nested_block),
    StressSpec::recursive_expression("nested_ternary", SOURCE_MODES, nested_ternary),
    StressSpec::new("nested_match", DESTACK_MODES, VALID, nested_match),
    StressSpec::new("nested_try", DESTACK_MODES, VALID, nested_try),
    StressSpec::new("nested_tsx", TSX_MODES, VALID, nested_tsx),
    StressSpec::recursive("deep_call", SOURCE_MODES, deep_call),
    StressSpec::recursive("deep_member", SOURCE_MODES, deep_member),
    StressSpec::new("deep_type", ALL_MODES, VALID, deep_type),
    StressSpec::new("control_flow", SOURCE_MODES, VALID, control_flow),
    StressSpec::new("trivia_wall", SOURCE_MODES, VALID, trivia_wall),
    StressSpec::new(
        "convoluted_expressions",
        SOURCE_MODES,
        VALID,
        convoluted_expressions,
    ),
    StressSpec::new("convoluted_types", ALL_MODES, VALID, convoluted_types),
    StressSpec::new(
        "convoluted_patterns",
        DESTACK_MODES,
        VALID,
        convoluted_patterns,
    ),
    StressSpec::new("sequence_types", DESTACK_MODES, VALID, sequence_type_forms),
    StressSpec::new(
        "sequence_patterns",
        DESTACK_MODES,
        VALID,
        sequence_pattern_forms,
    ),
    StressSpec::new("range_forms", DESTACK_MODES, VALID, range_forms),
    StressSpec::new("operator_forms", SOURCE_MODES, VALID, operator_forms),
    StressSpec::new("decorator_forms", DESTACK_MODES, VALID, decorator_forms),
    StressSpec::new("module_forms", DESTACK_MODES, VALID, module_forms),
    StressSpec::new("comptime_forms", DESTACK_MODES, VALID, comptime_forms),
    StressSpec::new("memory_forms", DESTACK_MODES, VALID, memory_forms),
    StressSpec::new("error_forms", DESTACK_MODES, VALID, error_forms),
    StressSpec::new("using_forms", DESTACK_MODES, VALID, using_forms),
    StressSpec::new("woven_source", SOURCE_MODES, VALID, woven_source_forms),
    StressSpec::new("woven_destack", DESTACK_MODES, VALID, woven_destack_forms),
    StressSpec::new("woven_tsx", TSX_MODES, VALID, woven_tsx_forms),
    StressSpec::new(
        "ambiguous_generics",
        SOURCE_MODES,
        VALID,
        ambiguous_generics,
    ),
    StressSpec::new("ambiguous_tsx", TSX_MODES, VALID, ambiguous_tsx),
    StressSpec::new("ambiguous_objects", SOURCE_MODES, VALID, ambiguous_objects),
    StressSpec::pathological_capped_at_brutal("massive_file", ALL_MODES, VALID, massive_file),
    StressSpec::deep("deep_parentheses", SOURCE_MODES, VALID, deep_parentheses),
    StressSpec::deep("deep_block", SOURCE_MODES, VALID, deep_block),
    StressSpec::deep("deep_tree", TSX_MODES, VALID, deep_tree),
    StressSpec::pathological("wide_call", SOURCE_MODES, VALID, wide_call),
    StressSpec::pathological("long_binary_chain", SOURCE_MODES, VALID, long_binary_chain),
    StressSpec::pathological(
        "long_logical_chain",
        SOURCE_MODES,
        VALID,
        long_logical_chain,
    ),
    StressSpec::pathological(
        "long_nullish_chain",
        SOURCE_MODES,
        VALID,
        long_nullish_chain,
    ),
    StressSpec::pathological(
        "long_postfix_chain",
        SOURCE_MODES,
        VALID,
        long_postfix_chain,
    ),
    StressSpec::recursive("nested_lambda_chain", SOURCE_MODES, nested_lambda_chain),
    StressSpec::deep(
        "parenthesized_binary_chain",
        SOURCE_MODES,
        VALID,
        parenthesized_binary_chain,
    ),
    StressSpec::pathological_capped_at_massive("trivia_flood", SOURCE_MODES, VALID, trivia_flood),
    StressSpec::new(
        "damaged_declaration",
        ALL_MODES,
        RECOVERY,
        damaged_declaration,
    ),
    StressSpec::new(
        "damaged_expression",
        SOURCE_MODES,
        RECOVERY,
        damaged_expression,
    ),
    StressSpec::new("damaged_type", ALL_MODES, RECOVERY, damaged_type),
    StressSpec::new("damaged_tsx", TSX_MODES, RECOVERY, damaged_tsx),
    StressSpec::new("damaged_trivia", SOURCE_MODES, RECOVERY, damaged_trivia),
    StressSpec::recovery_pathological("damaged_delimiters", SOURCE_MODES, damaged_delimiters),
    StressSpec::recovery_pathological(
        "damaged_parenthesized_heads",
        SOURCE_MODES,
        damaged_parenthesized_heads,
    ),
    StressSpec::recovery_pathological("damaged_generic_heads", SOURCE_MODES, damaged_generic_heads),
    StressSpec::recovery_pathological(
        "damaged_arrow_return_heads",
        SOURCE_MODES,
        damaged_arrow_return_heads,
    ),
    StressSpec::recovery_pathological(
        "damaged_function_type_heads",
        SOURCE_MODES,
        damaged_function_type_heads,
    ),
    StressSpec::recovery_pathological("damaged_infix_chains", SOURCE_MODES, damaged_infix_chains),
];

/// One generated stress fixture.
#[derive(Debug, Clone)]
pub struct StressCase {
    /// The stable case name.
    pub name: String,
    /// The generated file path.
    pub path: PathBuf,
    /// The generated file type.
    pub file_type: FileType,
    /// The expected parser result shape.
    pub expectation: StressExpectation,
}

/// Expected parser result shape for one stress fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StressExpectation {
    /// The source should parse without errors.
    Valid,
    /// The source may exceed parser limits but must not crash or time out.
    Bounded,
    /// The source is damaged but should recover and keep later roots.
    Recovery,
}

/// One generated stress file mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StressMode {
    /// `.ds` source.
    Destack,
    /// `.d.ds` declaration source.
    DestackDeclaration,
    /// `.ts` source.
    TypeScript,
    /// `.tsx` source.
    TypeScriptXml,
    /// `.d.ts` declaration source.
    TypeScriptDeclaration,
}

/// One generated stress case family.
#[derive(Debug, Clone, Copy)]
struct StressSpec {
    /// The generated folder name.
    name: &'static str,
    /// The file modes covered by this family.
    modes: &'static [StressMode],
    /// The expected parser result shape.
    expectation: StressExpectation,
    /// The generated size ladder.
    size: StressSize,
    /// The source builder for this family.
    generate: fn(StressMode, usize, usize) -> String,
}

/// Generated case size ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StressSize {
    /// Regular coverage fixtures.
    Regular,
    /// Large pathological fixtures.
    Pathological,
    /// Pathological fixtures capped at brutal scale.
    PathologicalCappedAtBrutal,
    /// Pathological fixtures capped at massive scale.
    PathologicalCappedAtMassive,
    /// Deep nesting fixtures.
    Deep,
    /// Recursive descent limit fixtures.
    Recursive,
    /// Recursive expression limit fixtures.
    RecursiveExpression,
    /// Large recovery fixtures.
    RecoveryPathological,
}

/// One generated file inside a stress case family.
#[derive(Debug, Clone, Copy)]
struct StressVariantShape {
    /// The generated file stem.
    name: &'static str,
    /// The generated fixture scale.
    scale: usize,
    /// The generated line width.
    width: usize,
    /// The expectation for this specific variant.
    expectation: Option<StressExpectation>,
}

/// One materialized file inside a stress case family.
#[derive(Debug, Clone)]
struct StressVariant {
    /// The generated file stem.
    name: &'static str,
    /// The generated source text.
    source: String,
    /// The expectation for this specific variant.
    expectation: StressExpectation,
}

impl StressSpec {
    /// Create one stress case family.
    const fn new(
        name: &'static str,
        modes: &'static [StressMode],
        expectation: StressExpectation,
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation,
            size: StressSize::Regular,
            generate,
        }
    }

    /// Create one pathological stress case family.
    const fn pathological(
        name: &'static str,
        modes: &'static [StressMode],
        expectation: StressExpectation,
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation,
            size: StressSize::Pathological,
            generate,
        }
    }

    /// Create one pathological stress case family capped at brutal scale.
    const fn pathological_capped_at_brutal(
        name: &'static str,
        modes: &'static [StressMode],
        expectation: StressExpectation,
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation,
            size: StressSize::PathologicalCappedAtBrutal,
            generate,
        }
    }

    /// Create one pathological stress case family capped at massive scale.
    const fn pathological_capped_at_massive(
        name: &'static str,
        modes: &'static [StressMode],
        expectation: StressExpectation,
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation,
            size: StressSize::PathologicalCappedAtMassive,
            generate,
        }
    }

    /// Create one deep nesting stress case family.
    const fn deep(
        name: &'static str,
        modes: &'static [StressMode],
        expectation: StressExpectation,
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation,
            size: StressSize::Deep,
            generate,
        }
    }

    /// Create one recursive descent stress case family.
    const fn recursive(
        name: &'static str,
        modes: &'static [StressMode],
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation: StressExpectation::Valid,
            size: StressSize::Recursive,
            generate,
        }
    }

    /// Create one recursive expression stress case family.
    const fn recursive_expression(
        name: &'static str,
        modes: &'static [StressMode],
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation: StressExpectation::Valid,
            size: StressSize::RecursiveExpression,
            generate,
        }
    }

    /// Create one large recovery stress case family.
    const fn recovery_pathological(
        name: &'static str,
        modes: &'static [StressMode],
        generate: fn(StressMode, usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            modes,
            expectation: StressExpectation::Recovery,
            size: StressSize::RecoveryPathological,
            generate,
        }
    }

    /// Generate all variants for one file mode.
    fn variants(self, mode: StressMode) -> Vec<StressVariant> {
        self.shapes()
            .iter()
            .map(|shape| shape.variant(mode, self.generate, self.expectation))
            .collect()
    }

    /// Return all variant shapes for this family.
    fn shapes(self) -> &'static [StressVariantShape] {
        if self.size == StressSize::RecoveryPathological {
            return RECOVERY_PATHOLOGICAL_VARIANTS;
        }

        if self.expectation == StressExpectation::Recovery {
            return RECOVERY_VARIANTS;
        }

        match self.size {
            StressSize::Regular => REGULAR_VARIANTS,
            StressSize::Pathological => PATHOLOGICAL_VARIANTS,
            StressSize::PathologicalCappedAtBrutal => PATHOLOGICAL_CAPPED_AT_BRUTAL_VARIANTS,
            StressSize::PathologicalCappedAtMassive => PATHOLOGICAL_CAPPED_AT_MASSIVE_VARIANTS,
            StressSize::Deep => DEEP_VARIANTS,
            StressSize::Recursive => RECURSIVE_VARIANTS,
            StressSize::RecursiveExpression => RECURSIVE_EXPRESSION_VARIANTS,
            StressSize::RecoveryPathological => RECOVERY_PATHOLOGICAL_VARIANTS,
        }
    }

    /// Return the expectation for one generated variant name.
    fn variant_expectation(self, variant: &str) -> Option<StressExpectation> {
        self.shapes()
            .iter()
            .find(|shape| shape.name == variant)
            .map(|shape| shape.expectation.unwrap_or(self.expectation))
    }
}

impl StressVariantShape {
    /// Create one generated variant.
    const fn new(name: &'static str, scale: usize, width: usize) -> Self {
        Self {
            name,
            scale,
            width,
            expectation: None,
        }
    }

    /// Create one generated variant with a custom expectation.
    const fn expect(
        name: &'static str,
        scale: usize,
        width: usize,
        expectation: StressExpectation,
    ) -> Self {
        Self {
            name,
            scale,
            width,
            expectation: Some(expectation),
        }
    }

    /// Generate one materialized variant.
    fn variant(
        self,
        mode: StressMode,
        generate: fn(StressMode, usize, usize) -> String,
        default_expectation: StressExpectation,
    ) -> StressVariant {
        StressVariant {
            name: self.name,
            source: generate(mode, self.scale, self.width),
            expectation: self.expectation.unwrap_or(default_expectation),
        }
    }
}

impl StressCase {
    /// Load one stress case from a generated fixture path.
    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        let file_type = FileType::from_path(&path)
            .ok_or_else(|| format!("unsupported stress file type: {}", path.display()))?;
        let case = StressCaseDescriptor::from_path(&path)?;
        let expectation = case.expectation()?;

        Ok(Self {
            name: case.name(),
            path,
            file_type,
            expectation,
        })
    }

    /// Return the parser case category.
    pub fn parser_category() -> &'static str {
        "destack_test::stress::parser"
    }

    /// Return the formatter case category.
    pub fn formatter_category() -> &'static str {
        "destack_test::stress::formatter"
    }

    /// Load the generated source.
    pub fn source(&self) -> Result<String, String> {
        fs::read_to_string(&self.path)
            .map_err(|error| format!("failed to read {}: {error}", self.path.display()))
    }

    /// Create a source file name for diagnostics.
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("stress.ds")
            .to_string()
    }

    /// Create a stable logical path for file ids.
    pub fn logical_path(&self) -> PathBuf {
        self.path
            .strip_prefix(fixtures_dir())
            .unwrap_or(&self.path)
            .to_path_buf()
    }
}

impl StressMode {
    /// Return the file type for this mode.
    pub(super) fn file_type(self) -> FileType {
        match self {
            Self::Destack => FileType::Destack,
            Self::DestackDeclaration => FileType::DestackDeclaration,
            Self::TypeScript => FileType::TypeScript,
            Self::TypeScriptXml => FileType::TypeScriptXml,
            Self::TypeScriptDeclaration => FileType::TypeScriptDeclaration,
        }
    }

    /// Return the stable file name label for this mode.
    fn label(self) -> &'static str {
        match self {
            Self::Destack => "destack",
            Self::DestackDeclaration => "destack_declaration",
            Self::TypeScript => "typescript",
            Self::TypeScriptXml => "tsx",
            Self::TypeScriptDeclaration => "typescript_declaration",
        }
    }

    /// Return the file extension for this mode.
    fn extension(self) -> &'static str {
        match self {
            Self::Destack => "ds",
            Self::DestackDeclaration => "d.ds",
            Self::TypeScript => "ts",
            Self::TypeScriptXml => "tsx",
            Self::TypeScriptDeclaration => "d.ts",
        }
    }

    /// Return whether this mode is declaration-only.
    pub(super) fn is_declaration(self) -> bool {
        matches!(self, Self::DestackDeclaration | Self::TypeScriptDeclaration)
    }

    /// Return whether this mode uses Destack syntax.
    pub(super) fn is_destack(self) -> bool {
        matches!(self, Self::Destack | Self::DestackDeclaration)
    }

    /// Return whether this mode can parse TSX trees.
    pub(super) fn is_tsx(self) -> bool {
        matches!(self, Self::Destack | Self::TypeScriptXml)
    }
}

/// Generated stress case identity parsed from a fixture path.
struct StressCaseDescriptor<'a> {
    /// The fixture family name.
    family: &'a str,
    /// The variant name within the family.
    variant: &'a str,
    /// The generated file mode.
    mode: StressMode,
}

impl<'a> StressCaseDescriptor<'a> {
    /// Parse a generated stress fixture path.
    fn from_path(path: &'a Path) -> Result<Self, String> {
        let family = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("stress path has no case family: {}", path.display()))?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("stress path has no file name: {}", path.display()))?;

        for spec in CASES {
            if spec.name != family {
                continue;
            }

            if let Some((variant, mode)) = Self::variant_and_mode(file_name, spec.modes) {
                return Ok(Self {
                    family,
                    variant,
                    mode,
                });
            }
        }

        Err(format!("unknown generated stress case: {}", path.display()))
    }

    /// Return the generated case name.
    fn name(&self) -> String {
        format!("{}::{}::{}", self.family, self.variant, self.mode.label())
    }

    /// Return the expected parser result shape.
    fn expectation(&self) -> Result<StressExpectation, String> {
        let spec = CASES
            .iter()
            .find(|spec| spec.name == self.family)
            .ok_or_else(|| format!("unknown stress family: {}", self.family))?;
        let expectation = spec.variant_expectation(self.variant).ok_or_else(|| {
            format!(
                "unknown stress variant: {}::{}::{}",
                self.family,
                self.variant,
                self.mode.label()
            )
        })?;

        Ok(expectation)
    }

    /// Return the variant name and mode encoded in one file name.
    fn variant_and_mode(
        file_name: &'a str,
        modes: &'static [StressMode],
    ) -> Option<(&'a str, StressMode)> {
        for mode in modes {
            let Some(variant) = strip_mode_suffix(file_name, *mode) else {
                continue;
            };

            if !variant.is_empty() {
                return Some((variant, *mode));
            }
        }

        None
    }
}

/// Strip one generated mode suffix from a stress file name.
fn strip_mode_suffix(file_name: &str, mode: StressMode) -> Option<&str> {
    let file_name = file_name.strip_suffix(mode.extension())?;
    let file_name = file_name.strip_suffix('.')?;
    let file_name = file_name.strip_suffix(mode.label())?;
    let variant = file_name.strip_suffix('.')?;

    Some(variant)
}

/// Materialize and return the parser stress corpus.
pub fn materialize_parser_cases() -> Result<Vec<StressCase>, String> {
    let directory = stress_generated_dir("parser");
    materialize_cases(&directory, true, true)
}

/// Materialize and return the formatter stress corpus.
pub fn materialize_formatter_cases() -> Result<Vec<StressCase>, String> {
    let directory = stress_generated_dir("formatter");
    materialize_cases(&directory, false, false)
}

/// Generate one deterministic parser fuzz input from arbitrary bytes.
pub fn generate_parser_fuzz_case(data: &[u8]) -> (String, FileType, StressExpectation) {
    generate_fuzz_case(data, true, true)
}

/// Generate one deterministic formatter fuzz input from arbitrary bytes.
pub fn generate_formatter_fuzz_case(data: &[u8]) -> (String, FileType, StressExpectation) {
    generate_fuzz_case(data, false, false)
}

fn materialize_cases(
    directory: &Path,
    include_recovery: bool,
    include_bounded: bool,
) -> Result<Vec<StressCase>, String> {
    if directory.exists() {
        fs::remove_dir_all(directory)
            .map_err(|error| format!("failed to clear {}: {error}", directory.display()))?;
    }
    fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;

    let mut cases = Vec::new();

    // write each generated case into its topical directory
    for spec in CASES {
        if spec.expectation == StressExpectation::Recovery && !include_recovery {
            continue;
        }

        for mode in spec.modes {
            let case_directory = directory.join(spec.name);
            fs::create_dir_all(&case_directory).map_err(|error| {
                format!("failed to create {}: {error}", case_directory.display())
            })?;

            for variant in spec.variants(*mode) {
                let expectation = variant.expectation;
                if expectation == StressExpectation::Recovery && !include_recovery {
                    continue;
                }

                if expectation == StressExpectation::Bounded && !include_bounded {
                    continue;
                }

                let file_name = format!("{}.{}.{}", variant.name, mode.label(), mode.extension());
                let path = case_directory.join(file_name);
                fs::write(&path, variant.source)
                    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

                cases.push(StressCase {
                    name: format!("{}::{}::{}", spec.name, variant.name, mode.label()),
                    path,
                    file_type: mode.file_type(),
                    expectation,
                });
            }
        }
    }

    Ok(cases)
}

fn stress_generated_dir(kind: &str) -> PathBuf {
    fixtures_dir().join("stress").join(kind).join("generated")
}

fn generate_fuzz_case(
    data: &[u8],
    include_recovery: bool,
    include_bounded: bool,
) -> (String, FileType, StressExpectation) {
    let seed = data.iter().fold(0_u64, |seed, byte| {
        seed.wrapping_mul(131).wrapping_add(u64::from(*byte))
    });

    let specs = CASES
        .iter()
        .filter(|spec| include_recovery || spec.expectation == StressExpectation::Valid)
        .collect::<Vec<_>>();
    let spec = specs[seed as usize % specs.len()];
    let mode = spec.modes[(seed as usize / specs.len()) % spec.modes.len()];
    let variants = spec
        .variants(mode)
        .into_iter()
        .filter(|variant| include_bounded || variant.expectation != StressExpectation::Bounded)
        .collect::<Vec<_>>();
    let variant = &variants[(seed as usize / specs.len() / spec.modes.len()) % variants.len()];

    (
        variant.source.clone(),
        mode.file_type(),
        variant.expectation,
    )
}
