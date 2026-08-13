use std::path::{Path, PathBuf};

use destack_source::{FileType, LanguageType};

use crate::core::fixtures_dir;
use crate::stress::file::write_complete_file;

use super::array::large_array;
use super::block::{control_flow, nested_block};
use super::call::deep_call;
use super::class::{large_ambient_class, large_class};
use super::r#const::const_forms;
use super::declaration::{
    damaged_ambient_declaration, damaged_declaration, large_ambient_declaration, large_declaration,
};
use super::decorator::decorator_forms;
use super::dependency::{large_ambient_import_export, large_import_export};
use super::error::error_forms;
use super::expression::{convoluted_expressions, damaged_expression, nested_try};
use super::generic::ambiguous_generics;
use super::interface::large_interface;
use super::matrix::{
    damaged_declaration_matrix, damaged_expression_matrix, damaged_tree_matrix,
    damaged_type_matrix, expression_matrix, type_matrix,
};
use super::member::deep_member;
use super::memory::memory_forms;
use super::module::module_forms;
use super::object::{ambiguous_objects, large_object};
use super::operator::operator_forms;
use super::pathology::{
    damaged_argument_lists, damaged_arrow_return_heads, damaged_delimiters,
    damaged_function_type_heads, damaged_generic_heads, damaged_infix_chains,
    damaged_nested_blocks, damaged_parenthesized_heads, damaged_type_member_bodies, deep_block,
    deep_parentheses, deep_tree, massive_ambient_file, massive_file, trivia_flood, wide_call,
};
use super::pattern::{convoluted_patterns, damaged_type, nested_match};
use super::range::range_forms;
use super::recovery::{
    damaged_delimiter_storms, damaged_dependency_attribute_boundaries,
    damaged_dependency_boundaries, damaged_dependency_item_boundaries,
    damaged_dependency_namespace_boundaries, damaged_dependency_target_boundaries,
    damaged_documentation_boundaries, damaged_export_target_boundaries,
    damaged_export_target_only_boundaries, damaged_import_target_boundaries,
    damaged_member_boundaries, damaged_pattern_boundaries, damaged_statement_boundaries,
    damaged_statement_call_boundaries, damaged_statement_object_boundaries,
    damaged_statement_slot_boundaries, damaged_template_boundaries,
};
use super::sequence::{sequence_pattern_forms, sequence_type_forms};
use super::signature::{large_ambient_signature, large_signature};
use super::ternary::nested_ternary;
use super::torture::{
    long_assignment_chain, long_binary_chain, long_conditional_type_chain, long_logical_chain,
    long_nullish_chain, long_pattern_prefix_chain, long_postfix_chain, long_type_operator_chain,
    long_type_prefix_chain, long_value_prefix_chain, nested_lambda_chain,
    parenthesized_binary_chain,
};
use super::tree::{ambiguous_tree, damaged_tree, damaged_tree_nesting, nested_tree};
use super::trivia::{damaged_trivia, large_trivia, trivia_wall};
use super::ty::{convoluted_types, deep_type, large_type};
use super::using::using_forms;
use super::weave::{woven_destack_forms, woven_tree_forms, woven_typescript_forms};

const DEFAULT_WIDTH: usize = 96;
const FUZZ_MAX_SCALE: usize = 64;
const FUZZ_WIDTHS: &[usize] = &[56, DEFAULT_WIDTH, 140, 180];
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
const FORMATTER_SCALE_LIMIT: usize = PATHOLOGICAL_BRUTAL;
const PARSER_SCALE_LIMIT: usize = usize::MAX;
const PATHOLOGICAL_DEEP_VALID: usize = 512;
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
const REGULAR_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", REGULAR_LARGE, DEFAULT_WIDTH),
    StressVariant::new("huge", REGULAR_HUGE, 120),
    StressVariant::new("massive", REGULAR_MASSIVE, 140),
    StressVariant::new("wide", REGULAR_WIDE, 180),
    StressVariant::new("dense", REGULAR_DENSE, 56),
];
const RECOVERY_VARIANTS: &[StressVariant] = &[
    StressVariant::new("baseline", 1, DEFAULT_WIDTH),
    StressVariant::new("large", 128, DEFAULT_WIDTH),
    StressVariant::new("huge", 512, 120),
    StressVariant::new("wide", 256, 180),
    StressVariant::new("dense", 1_024, 56),
];
const PATHOLOGICAL_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariant::new("massive", PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariant::new("brutal", PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
    StressVariant::new("monster", PATHOLOGICAL_MONSTER, DEFAULT_WIDTH),
    StressVariant::new("dense", PATHOLOGICAL_MASSIVE, 56),
];
const PATHOLOGICAL_CAPPED_AT_BRUTAL_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariant::new("massive", PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariant::new("brutal", PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
    StressVariant::new("dense", PATHOLOGICAL_MASSIVE, 56),
];
const PATHOLOGICAL_CAPPED_AT_MASSIVE_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariant::new("massive", PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
    StressVariant::new("dense", PATHOLOGICAL_MASSIVE, 56),
];
const DEEP_VARIANTS: &[StressVariant] = &[
    StressVariant::new("deep", PATHOLOGICAL_DEEP_VALID, DEFAULT_WIDTH),
    StressVariant::expect("bounded_deep", PATHOLOGICAL_DEEP, DEFAULT_WIDTH, BOUNDED),
    StressVariant::expect("deeper", PATHOLOGICAL_DEEPER, DEFAULT_WIDTH, BOUNDED),
    StressVariant::expect("deepest", PATHOLOGICAL_DEEPEST, DEFAULT_WIDTH, BOUNDED),
    StressVariant::expect("absurd", PATHOLOGICAL_ABSURD, DEFAULT_WIDTH, BOUNDED),
];
const RECURSIVE_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", RECURSIVE_VALID_LARGE, DEFAULT_WIDTH),
    StressVariant::new("wide", RECURSIVE_VALID_LARGE, 180),
    StressVariant::expect("bounded_huge", RECURSIVE_BOUNDED_HUGE, 120, BOUNDED),
    StressVariant::expect("bounded_massive", RECURSIVE_BOUNDED_MASSIVE, 140, BOUNDED),
    StressVariant::expect("bounded_dense", RECURSIVE_BOUNDED_DENSE, 56, BOUNDED),
];
const RECURSIVE_EXPRESSION_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", RECURSIVE_EXPRESSION_VALID_LARGE, DEFAULT_WIDTH),
    StressVariant::new("wide", RECURSIVE_EXPRESSION_VALID_LARGE, 180),
    StressVariant::expect("bounded_huge", RECURSIVE_BOUNDED_HUGE, 120, BOUNDED),
    StressVariant::expect("bounded_massive", RECURSIVE_BOUNDED_MASSIVE, 140, BOUNDED),
    StressVariant::expect("bounded_dense", RECURSIVE_BOUNDED_DENSE, 56, BOUNDED),
];
const RECOVERY_PATHOLOGICAL_VARIANTS: &[StressVariant] = &[
    StressVariant::new("large", RECOVERY_PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
    StressVariant::expect(
        "massive",
        RECOVERY_PATHOLOGICAL_MASSIVE,
        DEFAULT_WIDTH,
        BOUNDED,
    ),
    StressVariant::expect(
        "brutal",
        RECOVERY_PATHOLOGICAL_BRUTAL,
        DEFAULT_WIDTH,
        BOUNDED,
    ),
];

const FAMILIES: &[StressFamily] = &[
    StressFamily::new("large_declaration", VALID, large_declaration),
    StressFamily::new(
        "large_ambient_declaration",
        VALID,
        large_ambient_declaration,
    )
    .with_language(LanguageType::DestackDeclaration),
    StressFamily::new("large_function", VALID, large_signature),
    StressFamily::new("large_ambient_function", VALID, large_ambient_signature)
        .with_language(LanguageType::DestackDeclaration),
    StressFamily::new("large_class", VALID, large_class),
    StressFamily::new("large_ambient_class", VALID, large_ambient_class)
        .with_language(LanguageType::DestackDeclaration),
    StressFamily::new("large_interface", VALID, large_interface),
    StressFamily::new("large_type", VALID, large_type),
    StressFamily::new("large_import_export", VALID, large_import_export),
    StressFamily::new(
        "large_ambient_import_export",
        VALID,
        large_ambient_import_export,
    )
    .with_language(LanguageType::DestackDeclaration),
    StressFamily::new("large_trivia", VALID, large_trivia),
    StressFamily::new("large_array", VALID, large_array),
    StressFamily::new("large_object", VALID, large_object),
    StressFamily::new("nested_block", VALID, nested_block),
    StressFamily::recursive_expression("nested_ternary", nested_ternary),
    StressFamily::new("nested_match", VALID, nested_match),
    StressFamily::new("nested_try", VALID, nested_try),
    StressFamily::new("nested_tree", VALID, nested_tree),
    StressFamily::recursive("deep_call", deep_call),
    StressFamily::recursive("deep_member", deep_member),
    StressFamily::new("deep_type", VALID, deep_type),
    StressFamily::new("control_flow", VALID, control_flow),
    StressFamily::new("trivia_wall", VALID, trivia_wall),
    StressFamily::new("convoluted_expressions", VALID, convoluted_expressions),
    StressFamily::new("convoluted_types", VALID, convoluted_types),
    StressFamily::new("convoluted_patterns", VALID, convoluted_patterns),
    StressFamily::new("expression_matrix", VALID, expression_matrix),
    StressFamily::new("type_matrix", VALID, type_matrix),
    StressFamily::new("sequence_types", VALID, sequence_type_forms),
    StressFamily::new("sequence_patterns", VALID, sequence_pattern_forms),
    StressFamily::new("range_forms", VALID, range_forms),
    StressFamily::new("operator_forms", VALID, operator_forms),
    StressFamily::new("decorator_forms", VALID, decorator_forms),
    StressFamily::new("module_forms", VALID, module_forms),
    StressFamily::new("const_forms", VALID, const_forms),
    StressFamily::new("memory_forms", VALID, memory_forms),
    StressFamily::new("error_forms", VALID, error_forms),
    StressFamily::new("using_forms", VALID, using_forms),
    StressFamily::new("woven_typescript", VALID, woven_typescript_forms),
    StressFamily::new("woven_destack", VALID, woven_destack_forms),
    StressFamily::new("woven_tree", VALID, woven_tree_forms),
    StressFamily::new("ambiguous_generics", VALID, ambiguous_generics),
    StressFamily::new("ambiguous_tree", VALID, ambiguous_tree),
    StressFamily::new("ambiguous_objects", VALID, ambiguous_objects),
    StressFamily::pathological_capped_at_brutal("massive_file", VALID, massive_file),
    StressFamily::pathological_capped_at_brutal(
        "massive_ambient_file",
        VALID,
        massive_ambient_file,
    )
    .with_language(LanguageType::DestackDeclaration),
    StressFamily::deep("deep_parentheses", VALID, deep_parentheses),
    StressFamily::deep("deep_block", VALID, deep_block),
    StressFamily::deep("deep_tree", VALID, deep_tree),
    StressFamily::pathological("wide_call", VALID, wide_call),
    StressFamily::pathological("long_binary_chain", VALID, long_binary_chain),
    StressFamily::pathological("long_logical_chain", VALID, long_logical_chain),
    StressFamily::pathological("long_nullish_chain", VALID, long_nullish_chain),
    StressFamily::pathological("long_assignment_chain", VALID, long_assignment_chain),
    StressFamily::recursive_expression("long_value_prefix_chain", long_value_prefix_chain),
    StressFamily::recursive_expression("long_type_prefix_chain", long_type_prefix_chain),
    StressFamily::recursive("long_pattern_prefix_chain", long_pattern_prefix_chain),
    StressFamily::pathological("long_type_operator_chain", VALID, long_type_operator_chain),
    StressFamily::pathological_capped_at_massive(
        "long_conditional_type_chain",
        VALID,
        long_conditional_type_chain,
    ),
    StressFamily::pathological("long_postfix_chain", VALID, long_postfix_chain),
    StressFamily::recursive("nested_lambda_chain", nested_lambda_chain),
    StressFamily::deep(
        "parenthesized_binary_chain",
        VALID,
        parenthesized_binary_chain,
    ),
    StressFamily::pathological_capped_at_massive("trivia_flood", VALID, trivia_flood),
    StressFamily::new("damaged_declaration", RECOVERY, damaged_declaration),
    StressFamily::new(
        "damaged_ambient_declaration",
        RECOVERY,
        damaged_ambient_declaration,
    )
    .with_language(LanguageType::DestackDeclaration),
    StressFamily::new("damaged_expression", RECOVERY, damaged_expression),
    StressFamily::recovery_pathological("damaged_expression_matrix", damaged_expression_matrix),
    StressFamily::new("damaged_type", RECOVERY, damaged_type),
    StressFamily::recovery_pathological("damaged_type_matrix", damaged_type_matrix),
    StressFamily::recovery_pathological("damaged_declaration_matrix", damaged_declaration_matrix),
    StressFamily::new("damaged_tree", RECOVERY, damaged_tree),
    StressFamily::recovery_pathological("damaged_tree_matrix", damaged_tree_matrix),
    StressFamily::new("damaged_tree_nesting", RECOVERY, damaged_tree_nesting),
    StressFamily::new("damaged_trivia", RECOVERY, damaged_trivia),
    StressFamily::recovery_pathological("damaged_argument_lists", damaged_argument_lists),
    StressFamily::recovery_pathological("damaged_type_member_bodies", damaged_type_member_bodies),
    StressFamily::recovery_pathological("damaged_delimiters", damaged_delimiters),
    StressFamily::recovery_pathological("damaged_nested_blocks", damaged_nested_blocks),
    StressFamily::recovery_pathological("damaged_parenthesized_heads", damaged_parenthesized_heads),
    StressFamily::recovery_pathological("damaged_generic_heads", damaged_generic_heads),
    StressFamily::recovery_pathological("damaged_arrow_return_heads", damaged_arrow_return_heads),
    StressFamily::recovery_pathological("damaged_function_type_heads", damaged_function_type_heads),
    StressFamily::recovery_pathological("damaged_infix_chains", damaged_infix_chains),
    StressFamily::recovery_pathological(
        "damaged_dependency_boundaries",
        damaged_dependency_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_dependency_item_boundaries",
        damaged_dependency_item_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_dependency_target_boundaries",
        damaged_dependency_target_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_import_target_boundaries",
        damaged_import_target_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_export_target_boundaries",
        damaged_export_target_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_export_target_only_boundaries",
        damaged_export_target_only_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_dependency_attribute_boundaries",
        damaged_dependency_attribute_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_dependency_namespace_boundaries",
        damaged_dependency_namespace_boundaries,
    ),
    StressFamily::recovery_pathological("damaged_pattern_boundaries", damaged_pattern_boundaries),
    StressFamily::recovery_pathological("damaged_member_boundaries", damaged_member_boundaries),
    StressFamily::recovery_pathological("damaged_template_boundaries", damaged_template_boundaries),
    StressFamily::recovery_pathological(
        "damaged_statement_boundaries",
        damaged_statement_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_statement_slot_boundaries",
        damaged_statement_slot_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_statement_call_boundaries",
        damaged_statement_call_boundaries,
    ),
    StressFamily::recovery_pathological(
        "damaged_statement_object_boundaries",
        damaged_statement_object_boundaries,
    ),
    StressFamily::recovery_pathological("damaged_delimiter_storms", damaged_delimiter_storms),
    StressFamily::recovery_pathological(
        "damaged_documentation_boundaries",
        damaged_documentation_boundaries,
    ),
];

/// One generated stress fixture.
#[derive(Debug, Clone)]
pub struct StressFixture {
    /// The stable fixture name.
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

/// One generated stress fixture family.
#[derive(Debug, Clone, Copy)]
struct StressFamily {
    /// The generated folder name.
    name: &'static str,
    /// The generated source language.
    language: LanguageType,
    /// The expected parser result shape.
    expectation: StressExpectation,
    /// The generated variant ladder.
    ladder: StressLadder,
    /// The source builder for this family.
    generate: fn(usize, usize) -> String,
}

/// Generated fixture variant ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StressLadder {
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

/// One generated fixture variant recipe.
#[derive(Debug, Clone, Copy)]
struct StressVariant {
    /// The generated file stem.
    name: &'static str,
    /// The generated fixture scale.
    scale: usize,
    /// The generated line width.
    width: usize,
    /// The expectation for this specific variant.
    expectation: Option<StressExpectation>,
}

/// One generated fixture source before it is written to disk.
#[derive(Debug, Clone)]
struct StressFixtureSource {
    /// The generated file stem.
    name: &'static str,
    /// The generated source text.
    source: String,
    /// The expectation for this specific variant.
    expectation: StressExpectation,
}

impl StressFamily {
    /// Create one stress fixture family.
    const fn new(
        name: &'static str,
        expectation: StressExpectation,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation,
            ladder: StressLadder::Regular,
            generate,
        }
    }

    /// Create one pathological stress fixture family.
    const fn pathological(
        name: &'static str,
        expectation: StressExpectation,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation,
            ladder: StressLadder::Pathological,
            generate,
        }
    }

    /// Create one pathological stress fixture family capped at brutal scale.
    const fn pathological_capped_at_brutal(
        name: &'static str,
        expectation: StressExpectation,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation,
            ladder: StressLadder::PathologicalCappedAtBrutal,
            generate,
        }
    }

    /// Create one pathological stress fixture family capped at massive scale.
    const fn pathological_capped_at_massive(
        name: &'static str,
        expectation: StressExpectation,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation,
            ladder: StressLadder::PathologicalCappedAtMassive,
            generate,
        }
    }

    /// Create one deep nesting stress fixture family.
    const fn deep(
        name: &'static str,
        expectation: StressExpectation,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation,
            ladder: StressLadder::Deep,
            generate,
        }
    }

    /// Create one recursive descent stress fixture family.
    const fn recursive(name: &'static str, generate: fn(usize, usize) -> String) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation: StressExpectation::Valid,
            ladder: StressLadder::Recursive,
            generate,
        }
    }

    /// Create one recursive expression stress fixture family.
    const fn recursive_expression(
        name: &'static str,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation: StressExpectation::Valid,
            ladder: StressLadder::RecursiveExpression,
            generate,
        }
    }

    /// Create one large recovery stress fixture family.
    const fn recovery_pathological(
        name: &'static str,
        generate: fn(usize, usize) -> String,
    ) -> Self {
        Self {
            name,
            language: LanguageType::Destack,
            expectation: StressExpectation::Recovery,
            ladder: StressLadder::RecoveryPathological,
            generate,
        }
    }

    /// Generate all fixture sources up to one scale limit.
    fn sources(self, scale_limit: usize) -> Vec<StressFixtureSource> {
        self.variants()
            .iter()
            .filter(|variant| variant.scale <= scale_limit)
            .map(|variant| variant.source(self.generate, self.expectation))
            .collect()
    }

    /// Set the generated source language.
    const fn with_language(mut self, language: LanguageType) -> Self {
        self.language = language;

        self
    }

    /// Return the generated source extension.
    const fn file_extension(self) -> &'static str {
        match self.language {
            LanguageType::Destack => "ds",
            LanguageType::DestackDeclaration => "d.ds",
        }
    }

    /// Return all variants for this family.
    fn variants(self) -> &'static [StressVariant] {
        if self.ladder == StressLadder::RecoveryPathological {
            return RECOVERY_PATHOLOGICAL_VARIANTS;
        }

        if self.expectation == StressExpectation::Recovery {
            return RECOVERY_VARIANTS;
        }

        match self.ladder {
            StressLadder::Regular => REGULAR_VARIANTS,
            StressLadder::Pathological => PATHOLOGICAL_VARIANTS,
            StressLadder::PathologicalCappedAtBrutal => PATHOLOGICAL_CAPPED_AT_BRUTAL_VARIANTS,
            StressLadder::PathologicalCappedAtMassive => PATHOLOGICAL_CAPPED_AT_MASSIVE_VARIANTS,
            StressLadder::Deep => DEEP_VARIANTS,
            StressLadder::Recursive => RECURSIVE_VARIANTS,
            StressLadder::RecursiveExpression => RECURSIVE_EXPRESSION_VARIANTS,
            StressLadder::RecoveryPathological => RECOVERY_PATHOLOGICAL_VARIANTS,
        }
    }

    /// Return the expectation for one generated variant name.
    fn variant_expectation(self, variant: &str) -> Option<StressExpectation> {
        self.variants()
            .iter()
            .find(|recipe| recipe.name == variant)
            .map(|recipe| recipe.expectation.unwrap_or(self.expectation))
    }
}

impl StressVariant {
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

    /// Generate one fixture source.
    fn source(
        self,
        generate: fn(usize, usize) -> String,
        default_expectation: StressExpectation,
    ) -> StressFixtureSource {
        StressFixtureSource {
            name: self.name,
            source: generate(self.scale, self.width),
            expectation: self.expectation.unwrap_or(default_expectation),
        }
    }
}

impl StressFixture {
    /// Load one stress fixture from a generated fixture path.
    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        let file_type = FileType::from_path(&path)
            .ok_or_else(|| format!("unsupported stress file type: {}", path.display()))?;
        let descriptor = StressFixtureDescriptor::from_path(&path)?;
        let expectation = descriptor.expectation()?;

        Ok(Self {
            name: descriptor.name(),
            path,
            file_type,
            expectation,
        })
    }

    /// Return the parser fixture category.
    pub const fn parser_category() -> &'static str {
        "destack_test::stress::parser"
    }

    /// Return the formatter fixture category.
    pub const fn formatter_category() -> &'static str {
        "destack_test::stress::formatter"
    }

    /// Load the generated source.
    pub fn source(&self) -> Result<String, String> {
        std::fs::read_to_string(&self.path)
            .map_err(|error| format!("failed to read {}: {error}", self.path.display()))
    }

    /// Create a source file name for diagnostics.
    pub fn file_name(&self) -> Result<String, String> {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .ok_or_else(|| {
                format!(
                    "stress path has no UTF-8 file name: {}",
                    self.path.display()
                )
            })
    }

    /// Create a stable logical path for file ids.
    pub fn logical_path(&self) -> PathBuf {
        self.path
            .strip_prefix(fixtures_dir())
            .unwrap_or(&self.path)
            .to_path_buf()
    }
}

/// Generated stress fixture identity parsed from a fixture path.
struct StressFixtureDescriptor<'a> {
    /// The fixture family name.
    family: &'a str,
    /// The variant name within the family.
    variant: &'a str,
}

impl<'a> StressFixtureDescriptor<'a> {
    /// Parse a generated stress fixture path.
    fn from_path(path: &'a Path) -> Result<Self, String> {
        let family_name = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("stress path has no fixture family: {}", path.display()))?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("stress path has no file name: {}", path.display()))?;

        let family = FAMILIES
            .iter()
            .find(|family| family.name == family_name)
            .ok_or_else(|| format!("unknown stress family: {family_name}"))?;
        let extension = family.file_extension();
        let variant = file_name
            .strip_suffix(extension)
            .and_then(|file_name| file_name.strip_suffix('.'))
            .filter(|variant| !variant.is_empty())
            .ok_or_else(|| format!("invalid stress fixture file: {}", path.display()))?;

        Ok(Self {
            family: family_name,
            variant,
        })
    }

    /// Return the generated fixture name.
    fn name(&self) -> String {
        format!("{}::{}", self.family, self.variant)
    }

    /// Return the expected parser result shape.
    fn expectation(&self) -> Result<StressExpectation, String> {
        let family = FAMILIES
            .iter()
            .find(|family| family.name == self.family)
            .ok_or_else(|| format!("unknown stress family: {}", self.family))?;
        let expectation = family
            .variant_expectation(self.variant)
            .ok_or_else(|| format!("unknown stress variant: {}::{}", self.family, self.variant))?;

        Ok(expectation)
    }
}

/// Materialize and return the parser stress corpus.
pub fn materialize_parser_fixtures() -> Result<Vec<StressFixture>, String> {
    let directory = stress_generated_dir("parser");
    materialize_fixtures(&directory, PARSER_SCALE_LIMIT)
}

/// Materialize and return the formatter stress corpus.
pub fn materialize_formatter_fixtures() -> Result<Vec<StressFixture>, String> {
    let directory = stress_generated_dir("formatter");
    materialize_fixtures(&directory, FORMATTER_SCALE_LIMIT)
}

/// Generate one deterministic stress fuzz input from arbitrary bytes.
pub fn generate_fuzz_case(data: &[u8]) -> (String, FileType, StressExpectation) {
    let seed = data.iter().fold(0_u64, |seed, byte| {
        seed.wrapping_mul(131).wrapping_add(u64::from(*byte))
    });

    let family = &FAMILIES[seed as usize % FAMILIES.len()];
    let scale_seed = seed as usize / FAMILIES.len();
    let scale = 1 + scale_seed % FUZZ_MAX_SCALE;
    let width = FUZZ_WIDTHS[(scale_seed / FUZZ_MAX_SCALE) % FUZZ_WIDTHS.len()];
    let source = (family.generate)(scale, width);

    (source, FileType::from(family.language), family.expectation)
}

fn materialize_fixtures(
    directory: &Path,
    scale_limit: usize,
) -> Result<Vec<StressFixture>, String> {
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;

    let mut fixtures = Vec::new();

    // write each generated fixture into its topical directory
    for family in FAMILIES {
        let family_directory = directory.join(family.name);
        std::fs::create_dir_all(&family_directory)
            .map_err(|error| format!("failed to create {}: {error}", family_directory.display()))?;
        let extension = family.file_extension();

        for source in family.sources(scale_limit) {
            let expectation = source.expectation;
            let file_name = format!("{}.{}", source.name, extension);
            let path = family_directory.join(file_name);
            write_complete_file(&path, source.source)?;

            fixtures.push(StressFixture {
                name: format!("{}::{}", family.name, source.name),
                path,
                file_type: FileType::from(family.language),
                expectation,
            });
        }
    }

    Ok(fixtures)
}

fn stress_generated_dir(kind: &str) -> PathBuf {
    fixtures_dir().join("stress").join(kind).join("generated")
}
