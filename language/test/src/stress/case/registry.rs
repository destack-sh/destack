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
    damaged_delimiters, deep_block, deep_parentheses, deep_tree, massive_file, trivia_flood,
    wide_call,
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
use super::weave::{woven_destack_forms, woven_script_forms, woven_tsx_forms};

const DEFAULT_WIDTH: usize = 96;
const REGULAR_LARGE: usize = 1_024;
const REGULAR_HUGE: usize = 2_048;
const REGULAR_MASSIVE: usize = 8_192;
const REGULAR_DENSE: usize = 4_096;
const REGULAR_WIDE: usize = 1_024;
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
const RECOVERY: StressExpectation = StressExpectation::Recovery;
const ALL_MODES: &[StressMode] = &[
    StressMode::Destack,
    StressMode::DestackDeclaration,
    StressMode::TypeScript,
    StressMode::TypeScriptXml,
    StressMode::TypeScriptDeclaration,
];
const SCRIPT_MODES: &[StressMode] = &[
    StressMode::Destack,
    StressMode::TypeScript,
    StressMode::TypeScriptXml,
];
const DESTACK_MODES: &[StressMode] = &[StressMode::Destack];
const TSX_MODES: &[StressMode] = &[StressMode::Destack, StressMode::TypeScriptXml];

const CASES: &[StressSpec] = &[
    StressSpec::new("large_declaration", ALL_MODES, VALID, large_declaration),
    StressSpec::new("large_function", ALL_MODES, VALID, large_signature),
    StressSpec::new("large_class", ALL_MODES, VALID, large_class),
    StressSpec::new("large_interface", ALL_MODES, VALID, large_interface),
    StressSpec::new("large_type", ALL_MODES, VALID, large_type),
    StressSpec::new("large_import_export", ALL_MODES, VALID, large_import_export),
    StressSpec::new("large_trivia", SCRIPT_MODES, VALID, large_trivia),
    StressSpec::new("large_array", SCRIPT_MODES, VALID, large_array),
    StressSpec::new("large_object", SCRIPT_MODES, VALID, large_object),
    StressSpec::new("nested_block", SCRIPT_MODES, VALID, nested_block),
    StressSpec::new("nested_ternary", SCRIPT_MODES, VALID, nested_ternary),
    StressSpec::new("nested_match", DESTACK_MODES, VALID, nested_match),
    StressSpec::new("nested_try", DESTACK_MODES, VALID, nested_try),
    StressSpec::new("nested_tsx", TSX_MODES, VALID, nested_tsx),
    StressSpec::new("deep_call", SCRIPT_MODES, VALID, deep_call),
    StressSpec::new("deep_member", SCRIPT_MODES, VALID, deep_member),
    StressSpec::new("deep_type", ALL_MODES, VALID, deep_type),
    StressSpec::new("control_flow", SCRIPT_MODES, VALID, control_flow),
    StressSpec::new("trivia_wall", SCRIPT_MODES, VALID, trivia_wall),
    StressSpec::new(
        "convoluted_expressions",
        SCRIPT_MODES,
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
    StressSpec::new("operator_forms", SCRIPT_MODES, VALID, operator_forms),
    StressSpec::new("decorator_forms", DESTACK_MODES, VALID, decorator_forms),
    StressSpec::new("module_forms", DESTACK_MODES, VALID, module_forms),
    StressSpec::new("comptime_forms", DESTACK_MODES, VALID, comptime_forms),
    StressSpec::new("memory_forms", DESTACK_MODES, VALID, memory_forms),
    StressSpec::new("error_forms", DESTACK_MODES, VALID, error_forms),
    StressSpec::new("using_forms", DESTACK_MODES, VALID, using_forms),
    StressSpec::new("woven_script", SCRIPT_MODES, VALID, woven_script_forms),
    StressSpec::new("woven_destack", DESTACK_MODES, VALID, woven_destack_forms),
    StressSpec::new("woven_tsx", TSX_MODES, VALID, woven_tsx_forms),
    StressSpec::new(
        "ambiguous_generics",
        SCRIPT_MODES,
        VALID,
        ambiguous_generics,
    ),
    StressSpec::new("ambiguous_tsx", TSX_MODES, VALID, ambiguous_tsx),
    StressSpec::new("ambiguous_objects", SCRIPT_MODES, VALID, ambiguous_objects),
    StressSpec::pathological("massive_file", ALL_MODES, VALID, massive_file),
    StressSpec::deep("deep_parentheses", SCRIPT_MODES, VALID, deep_parentheses),
    StressSpec::deep("deep_block", SCRIPT_MODES, VALID, deep_block),
    StressSpec::deep("deep_tree", TSX_MODES, VALID, deep_tree),
    StressSpec::pathological("wide_call", SCRIPT_MODES, VALID, wide_call),
    StressSpec::pathological("long_binary_chain", SCRIPT_MODES, VALID, long_binary_chain),
    StressSpec::pathological(
        "long_logical_chain",
        SCRIPT_MODES,
        VALID,
        long_logical_chain,
    ),
    StressSpec::pathological(
        "long_nullish_chain",
        SCRIPT_MODES,
        VALID,
        long_nullish_chain,
    ),
    StressSpec::pathological(
        "long_postfix_chain",
        SCRIPT_MODES,
        VALID,
        long_postfix_chain,
    ),
    StressSpec::deep(
        "nested_lambda_chain",
        SCRIPT_MODES,
        VALID,
        nested_lambda_chain,
    ),
    StressSpec::deep(
        "parenthesized_binary_chain",
        SCRIPT_MODES,
        VALID,
        parenthesized_binary_chain,
    ),
    StressSpec::pathological("trivia_flood", SCRIPT_MODES, VALID, trivia_flood),
    StressSpec::new(
        "damaged_declaration",
        ALL_MODES,
        RECOVERY,
        damaged_declaration,
    ),
    StressSpec::new(
        "damaged_expression",
        SCRIPT_MODES,
        RECOVERY,
        damaged_expression,
    ),
    StressSpec::new("damaged_type", ALL_MODES, RECOVERY, damaged_type),
    StressSpec::new("damaged_tsx", TSX_MODES, RECOVERY, damaged_tsx),
    StressSpec::new("damaged_trivia", SCRIPT_MODES, RECOVERY, damaged_trivia),
    StressSpec::recovery_pathological("damaged_delimiters", SCRIPT_MODES, damaged_delimiters),
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
    /// Deep nesting fixtures.
    Deep,
    /// Large recovery fixtures.
    RecoveryPathological,
}

/// One generated file inside a stress case family.
#[derive(Debug, Clone)]
struct StressVariant {
    /// The generated file stem.
    name: &'static str,
    /// The generated source text.
    source: String,
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
        if self.size == StressSize::RecoveryPathological {
            return vec![
                StressVariant::new(
                    "large",
                    (self.generate)(mode, RECOVERY_PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "massive",
                    (self.generate)(mode, RECOVERY_PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "brutal",
                    (self.generate)(mode, RECOVERY_PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
                ),
            ];
        }

        if self.expectation == StressExpectation::Recovery {
            return vec![
                StressVariant::new("baseline", (self.generate)(mode, 1, DEFAULT_WIDTH)),
                StressVariant::new("large", (self.generate)(mode, 128, DEFAULT_WIDTH)),
                StressVariant::new("huge", (self.generate)(mode, 512, 120)),
                StressVariant::new("wide", (self.generate)(mode, 256, 180)),
                StressVariant::new("dense", (self.generate)(mode, 1_024, 56)),
            ];
        }

        if self.size == StressSize::Pathological {
            return vec![
                StressVariant::new(
                    "large",
                    (self.generate)(mode, PATHOLOGICAL_LARGE, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "massive",
                    (self.generate)(mode, PATHOLOGICAL_MASSIVE, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "brutal",
                    (self.generate)(mode, PATHOLOGICAL_BRUTAL, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "monster",
                    (self.generate)(mode, PATHOLOGICAL_MONSTER, DEFAULT_WIDTH),
                ),
                StressVariant::new("dense", (self.generate)(mode, PATHOLOGICAL_MASSIVE, 56)),
            ];
        }

        if self.size == StressSize::Deep {
            return vec![
                StressVariant::new(
                    "deep",
                    (self.generate)(mode, PATHOLOGICAL_DEEP, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "deeper",
                    (self.generate)(mode, PATHOLOGICAL_DEEPER, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "deepest",
                    (self.generate)(mode, PATHOLOGICAL_DEEPEST, DEFAULT_WIDTH),
                ),
                StressVariant::new(
                    "absurd",
                    (self.generate)(mode, PATHOLOGICAL_ABSURD, DEFAULT_WIDTH),
                ),
            ];
        }

        vec![
            StressVariant::new("large", (self.generate)(mode, REGULAR_LARGE, DEFAULT_WIDTH)),
            StressVariant::new("huge", (self.generate)(mode, REGULAR_HUGE, 120)),
            StressVariant::new("massive", (self.generate)(mode, REGULAR_MASSIVE, 140)),
            StressVariant::new("wide", (self.generate)(mode, REGULAR_WIDE, 180)),
            StressVariant::new("dense", (self.generate)(mode, REGULAR_DENSE, 56)),
        ]
    }
}

impl StressVariant {
    /// Create one generated variant.
    fn new(name: &'static str, source: String) -> Self {
        Self { name, source }
    }
}

impl StressCase {
    /// Load one stress case from a generated fixture path.
    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        let file_type = FileType::from_path(&path)
            .ok_or_else(|| format!("unsupported stress file type: {}", path.display()))?;
        let name = stress_case_name(&path);
        let expectation = if name.starts_with("damaged_") {
            StressExpectation::Recovery
        } else {
            StressExpectation::Valid
        };

        Ok(Self {
            name,
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

/// Materialize and return the parser stress corpus.
pub fn materialize_parser_cases() -> Result<Vec<StressCase>, String> {
    let directory = stress_generated_dir("parser");
    materialize_cases(&directory, true)
}

/// Materialize and return the formatter stress corpus.
pub fn materialize_formatter_cases() -> Result<Vec<StressCase>, String> {
    let directory = stress_generated_dir("formatter");
    materialize_cases(&directory, false)
}

/// Generate one deterministic parser fuzz input from arbitrary bytes.
pub fn generate_parser_fuzz_case(data: &[u8]) -> (String, FileType, StressExpectation) {
    generate_fuzz_case(data, true)
}

/// Generate one deterministic formatter fuzz input from arbitrary bytes.
pub fn generate_formatter_fuzz_case(data: &[u8]) -> (String, FileType, StressExpectation) {
    generate_fuzz_case(data, false)
}

fn materialize_cases(directory: &Path, include_recovery: bool) -> Result<Vec<StressCase>, String> {
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
                let file_name = format!("{}.{}.{}", variant.name, mode.label(), mode.extension());
                let path = case_directory.join(file_name);
                fs::write(&path, variant.source)
                    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

                cases.push(StressCase {
                    name: format!("{}::{}::{}", spec.name, variant.name, mode.label()),
                    path,
                    file_type: mode.file_type(),
                    expectation: spec.expectation,
                });
            }
        }
    }

    Ok(cases)
}

fn stress_generated_dir(kind: &str) -> PathBuf {
    fixtures_dir().join("stress").join(kind).join("generated")
}

/// Return the stable generated case name for one fixture path.
fn stress_case_name(path: &Path) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .zip(path.file_stem().and_then(|stem| stem.to_str()))
        .map(|(family, variant)| format!("{family}::{variant}"))
        .unwrap_or_else(|| path.display().to_string())
}

fn generate_fuzz_case(
    data: &[u8],
    include_recovery: bool,
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
    let variants = spec.variants(mode);
    let variant = &variants[(seed as usize / specs.len() / spec.modes.len()) % variants.len()];

    (variant.source.clone(), mode.file_type(), spec.expectation)
}
