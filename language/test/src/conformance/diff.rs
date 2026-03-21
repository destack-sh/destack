use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use destack_source::{DiffOptions, print_diff};
use serde::Deserialize;

use super::{CaseStatus, SourceSelectorKind, StatusEntry, StatusSet, status_json_path_for_dir};

const TEST_BEGIN_PREFIX: &str = "// test.begin:";
const TEST_END_MARKER: &str = "// test.end";

/// Options for printing translated test diffs for one conformance suite.
#[derive(Debug, Clone)]
pub struct TranslationDiffOptions {
    /// The suite directory that contains the suite manifest and test files.
    pub suite_dir: PathBuf,
    /// The number of context lines to print around changes.
    pub context: usize,
    /// Whether to show whitespace explicitly in the diff output.
    pub whitespace: bool,
}

/// One suite manifest file for translation diffs.
#[derive(Debug, Deserialize)]
struct SuiteManifest {
    /// The fetch entries declared for one suite.
    #[serde(default)]
    fetch: SuiteFetchManifest,
}

/// One embedded fetch manifest.
#[derive(Debug, Default, Deserialize)]
struct SuiteFetchManifest {
    /// The entries declared for one suite.
    #[serde(default)]
    entries: Vec<FetchEntry>,
}

/// One fetch manifest entry.
#[derive(Debug, Deserialize)]
struct FetchEntry {
    /// The entry kind.
    kind: String,
    /// The listed files for this entry.
    #[serde(default)]
    files: Vec<FetchFile>,
}

/// One file mapping inside one fetch entry.
#[derive(Debug, Deserialize)]
struct FetchFile {
    /// The translated or manual test path.
    path: Option<String>,
    /// The linked source test path.
    source: Option<String>,
}

/// One extracted region from a translated test file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestRegion {
    /// The translated file path.
    path: String,
    /// The linked source file path.
    source: String,
    /// The optional titled source selector.
    title: Option<String>,
    /// The optional ordinal selector for repeated titles.
    ordinal: Option<usize>,
    /// The optional source title selector when it differs from the target title.
    source_title: Option<String>,
    /// The optional semantic source selector kind.
    source_kind: Option<SourceSelectorKind>,
    /// The optional semantic source selector key.
    source_key: Option<String>,
    /// The optional canonical source fragment hash.
    source_hash: Option<String>,
    /// The optional source ordinal selector when it differs from the target ordinal.
    source_ordinal: Option<usize>,
    /// The optional starting line for one explicit source span.
    source_line: Option<usize>,
    /// The optional ending line for one explicit source span.
    source_end_line: Option<usize>,
    /// The region body text without marker lines.
    text: String,
}

/// One extracted titled source test.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TitledSourceTest {
    /// The source test title.
    title: String,
    /// The ordinal for repeated titles in one file.
    ordinal: usize,
    /// The extracted source text.
    text: String,
}

/// One translated or manual target file.
#[derive(Debug, Clone)]
struct TargetFile {
    /// The target-relative file path.
    path: String,
    /// The linked source path from `suite.json`.
    source: Option<String>,
    /// The translated test file text.
    text: String,
    /// The explicit annotated regions inside the file.
    regions: Vec<TestRegion>,
}

/// Print all source versus translated diffs for one suite.
pub fn print_suite_translation_diffs(options: &TranslationDiffOptions) -> Result<usize, String> {
    // load the suite manifest
    let suite_path = options.suite_dir.join("suite.json");
    let status_path = status_json_path_for_dir(&options.suite_dir);
    let test_directory = options.suite_dir.join("tests");
    let suite_manifest = load_suite_manifest(&suite_path)?;
    let statuses = StatusSet::load(&status_path)?;
    let target_files = collect_target_files(&suite_manifest, &test_directory)?;

    // preserve the old file-based mode until one suite carries structured mappings
    if !has_structured_translation_entries(&statuses) {
        return print_manifest_translation_diffs(options, &suite_manifest, &test_directory);
    }

    // validate the double-entry ledger before printing diffs
    validate_translation_ledger(&statuses, &target_files)?;

    // print one diff per declared translated or adapted target
    let mut printed = 0;
    let mut source_cache = BTreeMap::<String, String>::new();

    for entry in translation_status_entries(&statuses) {
        if entry.target_file.is_empty() {
            continue;
        }

        let target_file = target_files
            .iter()
            .find(|target_file| target_file.path == entry.target_file)
            .ok_or_else(|| format!("missing translated target file {}", entry.target_file))?;

        let target_region = find_target_region_for_entry(target_file, entry)?;
        let source_path = source_path_for_entry(target_file, target_region, entry)?;
        let source_text = load_source_text(&source_path, &test_directory, &mut source_cache)?;
        let source_region = extract_source_region_for_entry(target_region, entry, &source_text)?;
        validate_source_hash(entry, target_region, &source_path, &source_region)?;
        let target_text = target_region
            .map(|region| region.text.clone())
            .unwrap_or_else(|| target_file.text.clone());

        eprintln!();
        if let Some(region) = target_region {
            if let Some(title) = &region.title {
                let ordinal = region
                    .ordinal
                    .map(|ordinal| format!(" #{ordinal}"))
                    .unwrap_or_default();
                eprintln!(
                    "=== {} <= {} :: {}{} ===",
                    region.path, source_path, title, ordinal
                );
            } else {
                eprintln!("=== {} <= {} ===", region.path, source_path);
            }
        } else {
            eprintln!("=== {} <= {} ===", target_file.path, source_path);
        }

        let diff_path = entry
            .target_subcase
            .is_empty()
            .then_some(entry.target_file.clone())
            .unwrap_or_else(|| {
                let ordinal = entry
                    .target_ordinal
                    .map(|ordinal| format!("#{ordinal}"))
                    .unwrap_or_default();
                format!(
                    "{} :: {}{}",
                    entry.target_file, entry.target_subcase, ordinal
                )
            });
        let mut diff_options = DiffOptions::new()
            .with_context(options.context)
            .with_path(&diff_path);
        if options.whitespace {
            diff_options = diff_options.with_whitespace();
        }

        print_diff(&source_region, &target_text, &diff_options);
        printed += 1;
    }

    Ok(printed)
}

/// Refresh semantic source hashes in one suite ledger and target annotations.
pub fn refresh_suite_translation_metadata(suite_dir: &Path) -> Result<usize, String> {
    let suite_path = suite_dir.join("suite.json");
    let status_path = status_json_path_for_dir(suite_dir);
    let test_directory = suite_dir.join("tests");
    let suite_manifest = load_suite_manifest(&suite_path)?;
    let mut statuses = StatusSet::load(&status_path)?;
    let target_files = collect_target_files(&suite_manifest, &test_directory)?;
    let mut source_cache = BTreeMap::<String, String>::new();
    let mut refreshed = 0;

    // refresh source hashes on the structured ledger entries
    for entry in &mut statuses.entries {
        if !matches!(
            entry.status,
            CaseStatus::Translated | CaseStatus::Adapted | CaseStatus::Manual
        ) || entry.target_file.is_empty()
        {
            continue;
        }

        let target_file = target_files
            .iter()
            .find(|target_file| target_file.path == entry.target_file)
            .ok_or_else(|| format!("missing translated target file {}", entry.target_file))?;
        let target_region = find_target_region_for_entry(target_file, entry)?;
        let source_path = source_path_for_entry(target_file, target_region, entry)?;
        let source_text = load_source_text(&source_path, &test_directory, &mut source_cache)?;
        let source_region = extract_source_region_for_entry(target_region, entry, &source_text)?;
        entry.source_hash = source_fragment_hash(&source_region);
        refreshed += 1;
    }

    statuses.save(&status_path)?;
    rewrite_target_region_metadata(&statuses, &target_files, &test_directory)?;

    Ok(refreshed)
}

/// Load one suite manifest from disk.
fn load_suite_manifest(path: &Path) -> Result<SuiteManifest, String> {
    // read the manifest file
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    // parse the json payload
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

/// Return whether one status file carries explicit translated target mappings.
fn has_structured_translation_entries(statuses: &StatusSet) -> bool {
    translation_status_entries(statuses).next().is_some()
}

/// Return the translated or adapted status entries for one suite.
fn translation_status_entries(statuses: &StatusSet) -> impl Iterator<Item = &StatusEntry> {
    statuses.entries.iter().filter(|entry| {
        !entry.target_file.is_empty()
            && matches!(
                entry.status,
                CaseStatus::Translated | CaseStatus::Adapted | CaseStatus::Manual
            )
    })
}

/// Collect all translated or manual target files for one suite.
fn collect_target_files(
    suite_manifest: &SuiteManifest,
    test_directory: &Path,
) -> Result<Vec<TargetFile>, String> {
    let mut files = Vec::new();

    // keep only local translated or manual entries
    for entry in &suite_manifest.fetch.entries {
        if !matches!(entry.kind.as_str(), "translated" | "manual") {
            continue;
        }

        for file in &entry.files {
            let Some(path) = &file.path else {
                continue;
            };

            let translated_path = test_directory.join(path);
            let translated_text = read_utf8_file(&translated_path)?;
            let default_source = file.source.clone();
            let regions = parse_test_regions(
                path,
                default_source.as_deref().unwrap_or_default(),
                &translated_text,
            )?;

            files.push(TargetFile {
                path: path.clone(),
                source: default_source,
                text: translated_text,
                regions,
            });
        }
    }

    Ok(files)
}

/// Print diffs with the older manifest-only flow.
fn print_manifest_translation_diffs(
    options: &TranslationDiffOptions,
    suite_manifest: &SuiteManifest,
    test_directory: &Path,
) -> Result<usize, String> {
    let target_files = collect_target_files(suite_manifest, test_directory)?;
    let mut source_cache = BTreeMap::<String, String>::new();
    let mut printed = 0;

    // print one diff per translated file or region
    for target_file in target_files {
        if target_file.regions.is_empty() {
            let Some(source) = &target_file.source else {
                continue;
            };

            let source_text = load_source_text(source, test_directory, &mut source_cache)?;

            eprintln!();
            eprintln!("=== {} <= {} ===", target_file.path, source);

            let mut diff_options = DiffOptions::new()
                .with_context(options.context)
                .with_path(&target_file.path);
            if options.whitespace {
                diff_options = diff_options.with_whitespace();
            }

            print_diff(&source_text, &target_file.text, &diff_options);
            printed += 1;
            continue;
        }

        for region in target_file.regions {
            let source_text = load_source_text(&region.source, test_directory, &mut source_cache)?;
            let source_region = extract_source_region_for_region(&region, &source_text)?;

            eprintln!();
            if let Some(title) = &region.title {
                let ordinal = region
                    .ordinal
                    .map(|ordinal| format!(" #{ordinal}"))
                    .unwrap_or_default();
                eprintln!(
                    "=== {} <= {} :: {}{} ===",
                    region.path, region.source, title, ordinal
                );
            } else {
                eprintln!("=== {} <= {} ===", region.path, region.source);
            }

            let diff_path = region
                .title
                .as_ref()
                .map(|title| format!("{} :: {title}", region.path))
                .unwrap_or_else(|| region.path.clone());
            let mut diff_options = DiffOptions::new()
                .with_context(options.context)
                .with_path(&diff_path);
            if options.whitespace {
                diff_options = diff_options.with_whitespace();
            }

            print_diff(&source_region, &region.text, &diff_options);
            printed += 1;
        }
    }

    Ok(printed)
}

/// Load one cached source file into memory.
fn load_source_text(
    source: &str,
    test_directory: &Path,
    source_cache: &mut BTreeMap<String, String>,
) -> Result<String, String> {
    if let Some(text) = source_cache.get(source) {
        return Ok(text.clone());
    }

    let source_path = test_directory.join(source);
    let source_text = read_utf8_file(&source_path)?;
    source_cache.insert(source.to_string(), source_text.clone());
    Ok(source_text)
}

/// Parse all explicit test regions from one translated file.
fn parse_test_regions(path: &str, source: &str, text: &str) -> Result<Vec<TestRegion>, String> {
    let mut regions = Vec::new();
    let mut lines = text.split_inclusive('\n').peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_end_matches('\n').trim();
        if !trimmed.starts_with(TEST_BEGIN_PREFIX) {
            continue;
        }

        let metadata = parse_region_metadata(trimmed)?;
        let mut region_lines = String::new();

        loop {
            let Some(region_line) = lines.next() else {
                return Err(format!("unterminated test region in {path}"));
            };

            let region_trimmed = region_line.trim_end_matches('\n').trim();
            if region_trimmed == TEST_END_MARKER {
                break;
            }

            region_lines.push_str(region_line);
        }

        let region_source = metadata
            .get("source")
            .cloned()
            .unwrap_or_else(|| source.to_string());
        let region_title = metadata.get("title").cloned();
        let region_ordinal = metadata
            .get("ordinal")
            .and_then(|ordinal| ordinal.parse::<usize>().ok());
        let source_kind = metadata
            .get("source_kind")
            .and_then(|source_kind| parse_source_kind(source_kind));
        let source_key = metadata.get("source_key").cloned();
        let source_hash = metadata.get("source_hash").cloned();
        let source_title = metadata
            .get("source_title")
            .cloned()
            .or_else(|| region_title.clone());
        let source_ordinal = metadata
            .get("source_ordinal")
            .and_then(|ordinal| ordinal.parse::<usize>().ok())
            .or(region_ordinal);
        let source_line = metadata
            .get("source_line")
            .and_then(|line| line.parse::<usize>().ok());
        let source_end_line = metadata
            .get("source_end_line")
            .and_then(|line| line.parse::<usize>().ok());

        regions.push(TestRegion {
            path: path.to_string(),
            source: region_source,
            title: region_title,
            ordinal: region_ordinal,
            source_title,
            source_kind,
            source_key,
            source_hash,
            source_ordinal,
            source_line,
            source_end_line,
            text: region_lines,
        });
    }

    Ok(regions)
}

/// Parse one `test.begin` marker line into key-value metadata.
fn parse_region_metadata(line: &str) -> Result<BTreeMap<String, String>, String> {
    let Some(mut rest) = line.strip_prefix(TEST_BEGIN_PREFIX) else {
        return Err(format!("invalid test region marker: {line}"));
    };

    let mut metadata = BTreeMap::new();
    rest = rest.trim();

    while !rest.is_empty() {
        let Some((key, key_rest)) = rest.split_once('=') else {
            return Err(format!("invalid test region metadata: {line}"));
        };

        let key = key.trim();
        let mut tail = key_rest.trim_start();
        if tail.is_empty() {
            return Err(format!("missing test region value for key '{key}'"));
        }

        let value;
        if let Some(quoted_tail) = tail.strip_prefix('"') {
            let mut escaped = false;
            let mut end_index = None;
            for (index, character) in quoted_tail.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }

                if character == '\\' {
                    escaped = true;
                    continue;
                }

                if character == '"' {
                    end_index = Some(index);
                    break;
                }
            }

            let Some(end_index) = end_index else {
                return Err(format!("unterminated quoted value in test region: {line}"));
            };

            value = quoted_tail[..end_index].replace("\\\"", "\"");
            tail = quoted_tail[end_index + 1..].trim_start();
        } else {
            let mut parts = tail.splitn(2, char::is_whitespace);
            value = parts.next().unwrap_or_default().to_string();
            tail = parts.next().unwrap_or_default().trim_start();
        }

        metadata.insert(key.to_string(), value);
        rest = tail;
    }

    Ok(metadata)
}

/// Parse one serialized source selector kind.
fn parse_source_kind(source_kind: &str) -> Option<SourceSelectorKind> {
    match source_kind {
        "file" => Some(SourceSelectorKind::File),
        "title" => Some(SourceSelectorKind::Title),
        "message" => Some(SourceSelectorKind::Message),
        "statement-group" => Some(SourceSelectorKind::StatementGroup),
        "template-item" => Some(SourceSelectorKind::TemplateItem),
        "table-row" => Some(SourceSelectorKind::TableRow),
        _ => None,
    }
}

/// Return the stable hash for one extracted source fragment.
fn source_fragment_hash(fragment: &str) -> String {
    let normalized = fragment.replace("\r\n", "\n");
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in normalized.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("fnv1a64:{hash:016x}")
}

/// Validate one extracted source fragment hash against the declared metadata.
fn validate_source_hash(
    entry: &StatusEntry,
    region: Option<&TestRegion>,
    source_path: &str,
    source_region: &str,
) -> Result<(), String> {
    let expected_hash = if !entry.source_hash.is_empty() {
        Some(entry.source_hash.as_str())
    } else {
        region.and_then(|region| region.source_hash.as_deref())
    };

    let Some(expected_hash) = expected_hash else {
        return Ok(());
    };

    let actual_hash = source_fragment_hash(source_region);
    if actual_hash == expected_hash {
        return Ok(());
    }

    Err(format!(
        "source hash mismatch for {source_path}: expected {expected_hash}, found {actual_hash}"
    ))
}

/// Extract one matching source region for one translated test region.
fn extract_source_region_for_region(
    region: &TestRegion,
    source_text: &str,
) -> Result<String, String> {
    extract_source_region(
        source_text,
        &region.source,
        region.source_kind,
        region.source_key.as_deref(),
        region.source_title.as_deref(),
        region.source_ordinal,
        region.source_line,
        region.source_end_line,
    )
}

/// Extract one matching source region for one structured status entry.
fn extract_source_region_for_entry(
    region: Option<&TestRegion>,
    entry: &StatusEntry,
    source_text: &str,
) -> Result<String, String> {
    let source_path = source_path_for_entry_placeholder(region, entry);
    let source_kind = entry
        .source_kind
        .or_else(|| region.and_then(|region| region.source_kind));
    let source_key = if !entry.source_key.is_empty() {
        Some(entry.source_key.as_str())
    } else {
        region.and_then(|region| region.source_key.as_deref())
    };
    let source_title = if !entry.source_subcase.is_empty() {
        Some(entry.source_subcase.as_str())
    } else {
        region.and_then(|region| region.source_title.as_deref())
    };
    let source_ordinal = entry
        .source_ordinal
        .or_else(|| region.and_then(|region| region.source_ordinal));
    let source_line = entry
        .source_line
        .or_else(|| region.and_then(|region| region.source_line));
    let source_end_line = entry
        .source_end_line
        .or_else(|| region.and_then(|region| region.source_end_line));

    extract_source_region(
        source_text,
        &source_path,
        source_kind,
        source_key,
        source_title,
        source_ordinal,
        source_line,
        source_end_line,
    )
}

/// Extract one matching source region by selector or explicit line span.
fn extract_source_region(
    source_text: &str,
    source_path: &str,
    source_kind: Option<SourceSelectorKind>,
    source_key: Option<&str>,
    title: Option<&str>,
    ordinal: Option<usize>,
    source_line: Option<usize>,
    source_end_line: Option<usize>,
) -> Result<String, String> {
    if let Some(source_kind) = source_kind {
        let source_key = source_key.ok_or_else(|| {
            format!("missing source_key for {source_kind} selector in {source_path}")
        })?;

        return extract_source_region_by_kind(
            source_text,
            source_path,
            source_kind,
            source_key,
            ordinal,
        );
    }

    if let Some(source_line) = source_line {
        return extract_line_span(source_text, source_path, source_line, source_end_line);
    }

    if let Some(title) = title {
        let ordinal = ordinal.unwrap_or(1);

        if let Some(source_test) = extract_named_test_call(source_text, title, ordinal) {
            return Ok(source_test.text);
        }

        if let Some(source_test) = extract_single_test_call(source_text, ordinal) {
            return Ok(source_test.text);
        }

        if let Some(source_test) = extract_named_statement(source_text, title, ordinal) {
            return Ok(source_test.text);
        }

        return Err(format!(
            "failed to extract source test '{}' from {}",
            title, source_path
        ));
    }

    Ok(source_text.to_string())
}

/// Extract one matching source region by semantic selector kind.
fn extract_source_region_by_kind(
    source_text: &str,
    source_path: &str,
    source_kind: SourceSelectorKind,
    source_key: &str,
    ordinal: Option<usize>,
) -> Result<String, String> {
    let ordinal = ordinal.unwrap_or(1);

    match source_kind {
        SourceSelectorKind::File => Ok(source_text.to_string()),
        SourceSelectorKind::Title => {
            if let Some(source_test) = extract_named_test_call(source_text, source_key, ordinal) {
                return Ok(source_test.text);
            }

            if let Some(source_test) = extract_named_statement(source_text, source_key, ordinal) {
                return Ok(source_test.text);
            }

            Err(format!(
                "failed to extract titled source test '{source_key}' from {source_path}"
            ))
        }
        SourceSelectorKind::Message => extract_named_statement(source_text, source_key, ordinal)
            .map(|source_test| source_test.text)
            .ok_or_else(|| format!("failed to extract message '{source_key}' from {source_path}")),
        SourceSelectorKind::StatementGroup => {
            extract_statement_group(source_text, source_key, ordinal).ok_or_else(|| {
                format!("failed to extract statement group '{source_key}' from {source_path}")
            })
        }
        SourceSelectorKind::TemplateItem => extract_template_item(source_text, source_key)
            .ok_or_else(|| {
                format!("failed to extract template item '{source_key}' from {source_path}")
            }),
        SourceSelectorKind::TableRow => {
            extract_table_row(source_text, source_key).ok_or_else(|| {
                format!("failed to extract table row '{source_key}' from {source_path}")
            })
        }
    }
}

/// Return the effective source path for one status entry and target region.
fn source_path_for_entry(
    target_file: &TargetFile,
    region: Option<&TestRegion>,
    entry: &StatusEntry,
) -> Result<String, String> {
    let source_path = source_path_for_entry_placeholder(region, entry);
    if !source_path.is_empty() {
        return Ok(source_path);
    }

    target_file
        .source
        .clone()
        .ok_or_else(|| format!("missing source mapping for {}", target_file.path))
}

/// Return the preferred source path without validating fallback state.
fn source_path_for_entry_placeholder(region: Option<&TestRegion>, entry: &StatusEntry) -> String {
    if !entry.source_file.is_empty() {
        return entry.source_file.clone();
    }

    region
        .map(|region| region.source.clone())
        .unwrap_or_default()
}

/// Find one translated target region that matches one status entry.
fn find_target_region_for_entry<'a>(
    target_file: &'a TargetFile,
    entry: &StatusEntry,
) -> Result<Option<&'a TestRegion>, String> {
    if entry.target_subcase.is_empty() {
        return Ok(None);
    }

    let target_selector = target_selector_for_entry(entry)?;
    target_file
        .regions
        .iter()
        .find(|region| region_selector(region) == target_selector)
        .map(Some)
        .ok_or_else(|| format!("missing target region {target_selector}"))
}

/// Validate the structured ledger against one suite's target annotations.
fn validate_translation_ledger(
    statuses: &StatusSet,
    target_files: &[TargetFile],
) -> Result<(), String> {
    let mut files_by_path = BTreeMap::<String, &TargetFile>::new();
    let mut status_by_target = BTreeMap::<String, &StatusEntry>::new();
    let mut region_by_target = BTreeMap::<String, &TestRegion>::new();

    // index the known translated target files
    for target_file in target_files {
        files_by_path.insert(target_file.path.clone(), target_file);

        for region in &target_file.regions {
            let selector = region_selector(region);
            if region_by_target.insert(selector.clone(), region).is_some() {
                return Err(format!("duplicate target region {selector}"));
            }
        }
    }

    // index the declared translated or adapted statuses
    for entry in translation_status_entries(statuses) {
        let selector = target_selector_for_entry(entry)?;
        if status_by_target.insert(selector.clone(), entry).is_some() {
            return Err(format!("duplicate status entry for {selector}"));
        }
    }

    // require every declared target to exist and stay in sync with the annotations
    for (selector, entry) in &status_by_target {
        let target_file = files_by_path
            .get(&entry.target_file)
            .ok_or_else(|| format!("missing translated target file {}", entry.target_file))?;

        if entry.target_subcase.is_empty() {
            if !target_file.regions.is_empty() {
                return Err(format!(
                    "status entry {selector} must declare target_subcase because {} uses test.begin regions",
                    target_file.path
                ));
            }

            continue;
        }

        let Some(region) = region_by_target.get(selector) else {
            return Err(format!("missing annotated target region {selector}"));
        };

        if entry.source_kind.is_none() {
            return Err(format!("missing source_kind on status entry {selector}"));
        }

        if entry.source_key.is_empty() {
            return Err(format!("missing source_key on status entry {selector}"));
        }

        if entry.source_hash.is_empty() {
            return Err(format!("missing source_hash on status entry {selector}"));
        }

        validate_region_matches_entry(region, entry)?;
    }

    // require every explicit target region to be represented in the ledger
    for (selector, region) in &region_by_target {
        if !status_by_target.contains_key(selector) {
            return Err(format!(
                "missing status entry for annotated target region {selector} in {}",
                region.path
            ));
        }

        if region.source_kind.is_none() {
            return Err(format!(
                "missing source_kind on annotated target region {selector}"
            ));
        }

        if region.source_key.is_none() {
            return Err(format!(
                "missing source_key on annotated target region {selector}"
            ));
        }

        if region.source_hash.is_none() {
            return Err(format!(
                "missing source_hash on annotated target region {selector}"
            ));
        }
    }

    Ok(())
}

/// Rewrite all target region markers from the structured status ledger.
fn rewrite_target_region_metadata(
    statuses: &StatusSet,
    target_files: &[TargetFile],
    test_directory: &Path,
) -> Result<(), String> {
    let mut entries_by_selector = BTreeMap::<String, &StatusEntry>::new();

    // index the structured translation entries by target selector
    for entry in translation_status_entries(statuses) {
        let selector = target_selector_for_entry(entry)?;
        entries_by_selector.insert(selector, entry);
    }

    // rewrite each translated target file in place
    for target_file in target_files {
        if target_file.regions.is_empty() {
            continue;
        }

        let path = test_directory.join(&target_file.path);
        let text = read_utf8_file(&path)?;
        let mut rewritten = String::new();

        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_end_matches('\n').trim();
            if !trimmed.starts_with(TEST_BEGIN_PREFIX) {
                rewritten.push_str(line);
                continue;
            }

            let metadata = parse_region_metadata(trimmed)?;
            let title = metadata.get("title").cloned().unwrap_or_default();
            let ordinal = metadata
                .get("ordinal")
                .and_then(|ordinal| ordinal.parse::<usize>().ok());
            let selector = build_selector(&target_file.path, Some(&title), ordinal);
            let entry = entries_by_selector.get(&selector).copied().ok_or_else(|| {
                format!("missing status entry for annotated target region {selector}")
            })?;
            let marker = build_test_begin_marker(entry)?;
            let line_ending = if line.ends_with('\n') { "\n" } else { "" };
            rewritten.push_str(&marker);
            rewritten.push_str(line_ending);
        }

        if rewritten != text {
            fs::write(&path, rewritten)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
    }

    Ok(())
}

/// Build one canonical `test.begin` marker from one structured status entry.
fn build_test_begin_marker(entry: &StatusEntry) -> Result<String, String> {
    let source_kind = entry
        .source_kind
        .ok_or_else(|| format!("missing source_kind on {}", entry.target_file))?;
    if entry.source_key.is_empty() {
        return Err(format!("missing source_key on {}", entry.target_file));
    }
    if entry.source_hash.is_empty() {
        return Err(format!("missing source_hash on {}", entry.target_file));
    }
    if entry.target_subcase.is_empty() {
        return Err(format!("missing target_subcase on {}", entry.target_file));
    }

    let mut marker = format!(
        r#"// test.begin: source="{}" title="{}" source_kind={} source_key="{}" source_hash="{}""#,
        escape_marker_value(&entry.source_file),
        escape_marker_value(&entry.target_subcase),
        source_kind,
        escape_marker_value(&entry.source_key),
        escape_marker_value(&entry.source_hash)
    );

    if let Some(target_ordinal) = entry.target_ordinal {
        marker.push_str(&format!(" ordinal={target_ordinal}"));
    }

    if let Some(source_ordinal) = entry.source_ordinal {
        marker.push_str(&format!(" source_ordinal={source_ordinal}"));
    }

    Ok(marker)
}

/// Escape one quoted marker value.
fn escape_marker_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Validate one target annotation against one structured status entry.
fn validate_region_matches_entry(region: &TestRegion, entry: &StatusEntry) -> Result<(), String> {
    if region.path != entry.target_file {
        return Err(format!(
            "target region path mismatch: {} != {}",
            region.path, entry.target_file
        ));
    }

    if region.title.as_deref().unwrap_or_default() != entry.target_subcase {
        return Err(format!(
            "target region title mismatch for {}: {:?} != {:?}",
            region.path, region.title, entry.target_subcase
        ));
    }

    if region.ordinal != entry.target_ordinal {
        return Err(format!(
            "target region ordinal mismatch for {}: {:?} != {:?}",
            region.path, region.ordinal, entry.target_ordinal
        ));
    }

    if !entry.source_file.is_empty() && region.source != entry.source_file {
        return Err(format!(
            "source path mismatch for {}: {} != {}",
            region.path, region.source, entry.source_file
        ));
    }

    if !entry.source_subcase.is_empty()
        && region.source_title.as_deref() != Some(entry.source_subcase.as_str())
    {
        return Err(format!(
            "source title mismatch for {}: {:?} != {:?}",
            region.path, region.source_title, entry.source_subcase
        ));
    }

    if entry.source_ordinal.is_some() && region.source_ordinal != entry.source_ordinal {
        return Err(format!(
            "source ordinal mismatch for {}: {:?} != {:?}",
            region.path, region.source_ordinal, entry.source_ordinal
        ));
    }

    if entry.source_kind.is_some() && region.source_kind != entry.source_kind {
        return Err(format!(
            "source kind mismatch for {}: {:?} != {:?}",
            region.path, region.source_kind, entry.source_kind
        ));
    }

    if !entry.source_key.is_empty()
        && region.source_key.as_deref() != Some(entry.source_key.as_str())
    {
        return Err(format!(
            "source key mismatch for {}: {:?} != {:?}",
            region.path, region.source_key, entry.source_key
        ));
    }

    if !entry.source_hash.is_empty()
        && region.source_hash.as_deref() != Some(entry.source_hash.as_str())
    {
        return Err(format!(
            "source hash mismatch for {}: {:?} != {:?}",
            region.path, region.source_hash, entry.source_hash
        ));
    }

    if entry.source_line.is_some() && region.source_line != entry.source_line {
        return Err(format!(
            "source line mismatch for {}: {:?} != {:?}",
            region.path, region.source_line, entry.source_line
        ));
    }

    if entry.source_end_line.is_some() && region.source_end_line != entry.source_end_line {
        return Err(format!(
            "source end line mismatch for {}: {:?} != {:?}",
            region.path, region.source_end_line, entry.source_end_line
        ));
    }

    Ok(())
}

/// Return the canonical region selector for one target region.
fn region_selector(region: &TestRegion) -> String {
    build_selector(&region.path, region.title.as_deref(), region.ordinal)
}

/// Return the canonical target selector for one status entry.
fn target_selector_for_entry(entry: &StatusEntry) -> Result<String, String> {
    if entry.target_file.is_empty() {
        return Err("missing target_file on translated status entry".to_string());
    }

    Ok(build_selector(
        &entry.target_file,
        (!entry.target_subcase.is_empty()).then_some(entry.target_subcase.as_str()),
        entry.target_ordinal,
    ))
}

/// Build one canonical selector from file, title, and ordinal fields.
fn build_selector(file: &str, title: Option<&str>, ordinal: Option<usize>) -> String {
    let mut selector = file.to_string();

    if let Some(title) = title {
        selector.push_str("::");
        selector.push_str(title);

        if let Some(ordinal) = ordinal {
            selector.push('#');
            selector.push_str(&ordinal.to_string());
        }
    }

    selector
}

/// Extract one explicit 1-based line span from one source file.
fn extract_line_span(
    source_text: &str,
    source_path: &str,
    start_line: usize,
    end_line: Option<usize>,
) -> Result<String, String> {
    let end_line = end_line.unwrap_or(start_line);
    if start_line == 0 || end_line < start_line {
        return Err(format!(
            "invalid source line span {start_line}..={end_line} in {source_path}"
        ));
    }

    let line_ranges = collect_line_ranges(source_text);
    if end_line > line_ranges.len() {
        return Err(format!(
            "source line span {start_line}..={end_line} exceeds {source_path}"
        ));
    }

    let mut extracted = String::new();
    for range in &line_ranges[start_line - 1..end_line] {
        extracted.push_str(&source_text[range.start..range.end]);
    }

    Ok(extracted)
}

/// Extract one expanded loop or template item.
fn extract_template_item(source_text: &str, source_key: &str) -> Option<String> {
    let template_call = extract_single_test_template_call(source_text)?;
    let template_loop = extract_template_loop(source_text)?;

    for item in &template_loop.items {
        let replacement = serde_json::to_string(item).ok()?;
        let expanded = template_call.replace(&template_loop.placeholder, &replacement);
        let expanded = collapse_string_concatenations(&expanded);

        if item == source_key {
            return Some(expanded);
        }

        if let Some(title) = extract_call_title(&expanded)
            && title == source_key
        {
            return Some(expanded);
        }
    }

    None
}

/// One loop template that expands over one string list.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TemplateLoop {
    /// The placeholder expression inside the template body.
    placeholder: String,
    /// The listed string items.
    items: Vec<String>,
}

/// Extract one loop template over a string list.
fn extract_template_loop(source_text: &str) -> Option<TemplateLoop> {
    let (list_name, items) = extract_string_list_assignment(source_text)?;
    let loop_header = source_text.find("for (")?;
    let loop_text = &source_text[loop_header..];
    let loop_end = loop_text.find(')')?;
    let header = &loop_text[..=loop_end];
    let (_, iterable) = header.split_once(" in ")?;
    let iterable = iterable.trim_end_matches(')').trim();
    if iterable != list_name {
        return None;
    }

    let binding = header.split_once('(')?.1.split_once(" in ")?.0.trim();
    let binding = binding
        .strip_prefix("var ")
        .or_else(|| binding.strip_prefix("let "))
        .or_else(|| binding.strip_prefix("const "))
        .unwrap_or(binding)
        .trim();
    let placeholder = format!("{list_name}[{binding}]");

    Some(TemplateLoop { placeholder, items })
}

/// Extract one string list assignment from one source file.
fn extract_string_list_assignment(source_text: &str) -> Option<(String, Vec<String>)> {
    for keyword in ["var ", "let ", "const "] {
        let mut search_start = 0usize;

        while let Some(relative_start) = source_text[search_start..].find(keyword) {
            let binding_start = search_start + relative_start;
            let binding_text = &source_text[binding_start + keyword.len()..];
            let equals = binding_text.find('=')?;
            let list_name = binding_text[..equals].trim().to_string();
            let array_text = binding_text[equals + 1..].trim_start();
            if !array_text.starts_with('[') {
                search_start = binding_start + keyword.len();
                continue;
            }

            let array_end = find_matching_delimiter(array_text.as_bytes(), 0, b'[', b']')?;
            let array_literal = &array_text[..=array_end];
            let items = extract_string_literals(array_literal);
            if items.is_empty() {
                search_start = binding_start + keyword.len();
                continue;
            }

            return Some((list_name, items));
        }
    }

    None
}

/// Extract all string literals from one array literal.
fn extract_string_literals(array_literal: &str) -> Vec<String> {
    let bytes = array_literal.as_bytes();
    let mut values = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if !matches!(byte, b'\'' | b'"') {
            index += 1;
            continue;
        }

        let quote = byte;
        let start = index + 1;
        index += 1;
        let mut escaped = false;

        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }

            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }

            if byte == quote {
                values.push(array_literal[start..index].replace("\\\"", "\""));
                index += 1;
                break;
            }

            index += 1;
        }
    }

    values
}

/// Extract the sole test-like call even when its title is dynamic.
fn extract_single_test_template_call(source_text: &str) -> Option<String> {
    let bytes = source_text.as_bytes();
    let mut calls = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let Some((call_start, call_name_end)) = find_test_call_start(bytes, index) else {
            break;
        };

        let Some(end_index) = find_call_end(bytes, call_name_end) else {
            index = call_name_end.saturating_add(1);
            continue;
        };

        calls.push(source_text[call_start..end_index].to_string());
        index = end_index;
    }

    if calls.len() == 1 {
        return calls.into_iter().next();
    }

    None
}

/// Collapse simple string literal concatenations inside one source fragment.
fn collapse_string_concatenations(text: &str) -> String {
    let mut text = text.to_string();

    loop {
        let Some((start, end, collapsed)) = next_string_concatenation(&text) else {
            return text;
        };

        text.replace_range(start..end, &collapsed);
    }
}

/// Return the next simple string concatenation inside one source fragment.
fn next_string_concatenation(text: &str) -> Option<(usize, usize, String)> {
    let bytes = text.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if !matches!(byte, b'\'' | b'"') {
            index += 1;
            continue;
        }

        let left = parse_string_literal(text, index)?;
        let mut middle = left.end;
        while bytes.get(middle).is_some_and(u8::is_ascii_whitespace) {
            middle += 1;
        }
        if bytes.get(middle) != Some(&b'+') {
            index = left.end;
            continue;
        }

        middle += 1;
        while bytes.get(middle).is_some_and(u8::is_ascii_whitespace) {
            middle += 1;
        }

        let right_byte = *bytes.get(middle)?;
        if !matches!(right_byte, b'\'' | b'"') {
            index = left.end;
            continue;
        }

        let right = parse_string_literal(text, middle)?;
        if left.quote != right.quote {
            index = left.end;
            continue;
        }

        let merged = format!(
            "{}{}{}",
            left.quote as char,
            left.value,
            right
                .value
                .replace('\\', "\\\\")
                .replace(left.quote as char, &format!("\\{}", left.quote as char))
        );
        let collapsed = format!("{merged}{}", left.quote as char);
        return Some((left.start, right.end, collapsed));
    }

    None
}

/// One parsed string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StringLiteral {
    /// The starting byte offset.
    start: usize,
    /// The exclusive ending byte offset.
    end: usize,
    /// The quote byte.
    quote: u8,
    /// The unescaped literal value.
    value: String,
}

/// Parse one string literal at the provided byte offset.
fn parse_string_literal(text: &str, start: usize) -> Option<StringLiteral> {
    let bytes = text.as_bytes();
    let quote = *bytes.get(start)?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }

    let mut index = start + 1;
    let mut escaped = false;
    let mut value = String::new();

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            value.push(byte as char);
            escaped = false;
            index += 1;
            continue;
        }

        if byte == b'\\' {
            escaped = true;
            index += 1;
            continue;
        }

        if byte == quote {
            return Some(StringLiteral {
                start,
                end: index + 1,
                quote,
                value,
            });
        }

        value.push(byte as char);
        index += 1;
    }

    None
}

/// Extract one expanded table row from one source file.
fn extract_table_row(source_text: &str, source_key: &str) -> Option<String> {
    let table = extract_named_table(source_text, "tests")?;
    let row = table.rows.into_iter().find(|row| row.key == source_key)?;
    let loop_template = extract_table_loop_template(source_text, "tests")?;
    let description_literal = serde_json::to_string(&row.key).ok()?;
    let mut body = loop_template.body;

    body = replace_identifier(&body, &loop_template.description_name, &description_literal);
    body = replace_identifier(&body, &loop_template.args_name, &row.args);
    body = replace_identifier(&body, &loop_template.expected_name, &row.expected);

    let mut extracted = String::from("{\n");
    extracted.push_str(body.trim_end_matches('\n'));
    extracted.push_str("\n}\n");
    Some(extracted)
}

/// One named row inside one source table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRow {
    /// The row key.
    key: String,
    /// The row args expression.
    args: String,
    /// The row expected expression.
    expected: String,
}

/// One parsed source table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedTable {
    /// The parsed rows.
    rows: Vec<TableRow>,
}

/// Extract one named table assignment from one source file.
fn extract_named_table(source_text: &str, table_name: &str) -> Option<NamedTable> {
    let table_assignment = format!("{table_name} = [");
    let assignment_start = source_text.find(&table_assignment)?;
    let array_open = assignment_start + table_assignment.len() - 1;
    let array_end = find_matching_delimiter(source_text.as_bytes(), array_open, b'[', b']')?;
    let array_literal = &source_text[array_open..=array_end];
    let row_literals = split_top_level_items(array_literal, b'[', b']')?;
    let mut rows = Vec::new();

    for row_literal in row_literals {
        let row_literal = row_literal.trim();
        if !row_literal.starts_with('[') {
            continue;
        }

        let row_items = split_top_level_row_items(row_literal)?;
        if row_items.len() < 3 {
            continue;
        }

        let key = parse_head_string(&row_items[0])?;
        rows.push(TableRow {
            key,
            args: row_items[1].trim().to_string(),
            expected: row_items[2].trim().to_string(),
        });
    }

    Some(NamedTable { rows })
}

/// One parsed loop template over one source table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TableLoopTemplate {
    /// The description binding name.
    description_name: String,
    /// The args binding name.
    args_name: String,
    /// The expected binding name.
    expected_name: String,
    /// The loop body without its outer braces.
    body: String,
}

/// Extract one loop template over the provided table.
fn extract_table_loop_template(source_text: &str, table_name: &str) -> Option<TableLoopTemplate> {
    let loop_header = format!("of {table_name})");
    let header_start = source_text.find("for (")?;
    let loop_start = source_text[header_start..].find(&loop_header)? + header_start;
    let body_open = source_text[loop_start..].find('{')? + loop_start;
    let body_close = find_matching_delimiter(source_text.as_bytes(), body_open, b'{', b'}')?;
    let header = &source_text[header_start..body_open];
    let binding_text = header.split_once('[')?.1.split_once(']')?.0;
    let bindings = binding_text
        .split(',')
        .map(str::trim)
        .filter(|binding| !binding.is_empty())
        .collect::<Vec<_>>();
    if bindings.len() < 3 {
        return None;
    }

    Some(TableLoopTemplate {
        description_name: bindings[0].to_string(),
        args_name: bindings[1].to_string(),
        expected_name: bindings[2].to_string(),
        body: source_text[body_open + 1..body_close].to_string(),
    })
}

/// Split one top-level array literal into nested row items.
fn split_top_level_items(
    array_literal: &str,
    open_byte: u8,
    close_byte: u8,
) -> Option<Vec<String>> {
    let bytes = array_literal.as_bytes();
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = None::<usize>;
    let mut index = 0;
    let mut quote = None::<u8>;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(quote_byte) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }

            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }

            if byte == quote_byte {
                quote = None;
            }

            index += 1;
            continue;
        }

        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
            index += 1;
            continue;
        }

        if byte == open_byte {
            depth += 1;
            if depth == 2 {
                start = Some(index);
            }
            index += 1;
            continue;
        }

        if byte == close_byte {
            if depth == 2 {
                let item_start = start?;
                items.push(array_literal[item_start..=index].to_string());
                start = None;
            }
            depth = depth.saturating_sub(1);
            index += 1;
            continue;
        }

        index += 1;
    }

    Some(items)
}

/// Split one nested row array into top-level expressions.
fn split_top_level_row_items(row_literal: &str) -> Option<Vec<String>> {
    let row = row_literal.trim().strip_prefix('[')?.strip_suffix(']')?;
    let bytes = row.as_bytes();
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut index = 0usize;
    let mut quote = None::<u8>;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(quote_byte) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }

            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }

            if byte == quote_byte {
                quote = None;
            }

            index += 1;
            continue;
        }

        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
            index += 1;
            continue;
        }

        if matches!(byte, b'[' | b'(' | b'{') {
            depth += 1;
            index += 1;
            continue;
        }

        if matches!(byte, b']' | b')' | b'}') {
            depth = depth.saturating_sub(1);
            index += 1;
            continue;
        }

        if byte == b',' && depth == 0 {
            items.push(row[start..index].trim().to_string());
            start = index + 1;
        }

        index += 1;
    }

    items.push(row[start..].trim().to_string());
    Some(items)
}

/// Parse the leading string literal from one expression.
fn parse_head_string(expression: &str) -> Option<String> {
    let expression = expression.trim();
    let quote = expression.as_bytes().first().copied()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }

    parse_string_literal(expression, 0).map(|literal| literal.value)
}

/// Replace one identifier with one expression.
fn replace_identifier(text: &str, identifier: &str, replacement: &str) -> String {
    let bytes = text.as_bytes();
    let mut replaced = String::new();
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if !is_identifier_start(byte) {
            replaced.push(byte as char);
            index += 1;
            continue;
        }

        let start = index;
        let mut end = index + 1;
        while end < bytes.len() && is_identifier_continue(bytes[end]) {
            end += 1;
        }

        let name = &text[start..end];
        if name == identifier {
            replaced.push_str(replacement);
        } else {
            replaced.push_str(name);
        }

        index = end;
    }

    replaced
}

/// Find the matching closing delimiter for one opening delimiter.
fn find_matching_delimiter(
    bytes: &[u8],
    open_index: usize,
    open_byte: u8,
    close_byte: u8,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None::<u8>;
    let mut escaped = false;
    let mut index = open_index;

    while index < bytes.len() {
        let byte = bytes[index];

        if let Some(quote_byte) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }

            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }

            if byte == quote_byte {
                quote = None;
            }

            index += 1;
            continue;
        }

        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
            index += 1;
            continue;
        }

        if byte == open_byte {
            depth += 1;
            index += 1;
            continue;
        }

        if byte == close_byte {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
            index += 1;
            continue;
        }

        index += 1;
    }

    None
}

/// Extract one top-level statement that carries the provided title.
fn extract_named_statement(
    source_text: &str,
    title: &str,
    ordinal: usize,
) -> Option<TitledSourceTest> {
    let line_ranges = collect_line_ranges(source_text);
    let mut seen = 0;

    for (index, range) in line_ranges.iter().enumerate() {
        let line = &source_text[range.start..range.end];
        if !line.contains(title) {
            continue;
        }

        seen += 1;
        if seen != ordinal {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.ends_with(';') {
            return Some(TitledSourceTest {
                title: title.to_string(),
                ordinal: seen,
                text: line.to_string(),
            });
        }

        let start_index = block_start_index(&line_ranges, source_text, index);
        let end_index = block_end_index(&line_ranges, source_text, index);
        let start = line_ranges[start_index].start;
        let end = line_ranges[end_index].end;

        return Some(TitledSourceTest {
            title: title.to_string(),
            ordinal: seen,
            text: source_text[start..end].to_string(),
        });
    }

    None
}

/// Extract one surrounding statement group keyed by one anchor string.
fn extract_statement_group(source_text: &str, anchor: &str, ordinal: usize) -> Option<String> {
    let line_ranges = collect_line_ranges(source_text);
    let mut seen = 0;

    for (index, range) in line_ranges.iter().enumerate() {
        let line = &source_text[range.start..range.end];
        if !line.contains(anchor) {
            continue;
        }

        seen += 1;
        if seen != ordinal {
            continue;
        }

        let start_index = block_start_index(&line_ranges, source_text, index);
        let end_index = block_end_index(&line_ranges, source_text, index);
        let start = line_ranges[start_index].start;
        let end = line_ranges[end_index].end;

        return Some(source_text[start..end].to_string());
    }

    None
}

/// One indexed line range inside one source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineRange {
    /// The starting byte offset for the line.
    start: usize,
    /// The exclusive ending byte offset for the line.
    end: usize,
}

/// Collect all source line ranges, preserving trailing newlines.
fn collect_line_ranges(source_text: &str) -> Vec<LineRange> {
    let mut ranges = Vec::new();
    let mut start = 0;

    for line in source_text.split_inclusive('\n') {
        let end = start + line.len();
        ranges.push(LineRange { start, end });
        start = end;
    }

    if start < source_text.len() {
        ranges.push(LineRange {
            start,
            end: source_text.len(),
        });
    }

    ranges
}

/// Return the starting line index for one logical block.
fn block_start_index(line_ranges: &[LineRange], source_text: &str, index: usize) -> usize {
    let mut start = index;

    while start > 0 {
        let previous_line = &source_text[line_ranges[start - 1].start..line_ranges[start - 1].end];
        if previous_line.trim().is_empty() {
            break;
        }

        start -= 1;
    }

    start
}

/// Return the ending line index for one logical block.
fn block_end_index(line_ranges: &[LineRange], source_text: &str, index: usize) -> usize {
    let mut end = index;

    while end + 1 < line_ranges.len() {
        let next_line = &source_text[line_ranges[end + 1].start..line_ranges[end + 1].end];
        if next_line.trim().is_empty() {
            break;
        }

        end += 1;
    }

    end
}

/// Extract one `test(...)` call with the provided title.
fn extract_named_test_call(
    source_text: &str,
    title: &str,
    ordinal: usize,
) -> Option<TitledSourceTest> {
    let tests = extract_test_calls(source_text);
    tests
        .into_iter()
        .find(|test| test.title == title && test.ordinal == ordinal)
}

/// Extract the sole titled `test(...)` call from one source file.
fn extract_single_test_call(source_text: &str, ordinal: usize) -> Option<TitledSourceTest> {
    let tests = extract_test_calls(source_text);
    if tests.len() != 1 || ordinal != 1 {
        return None;
    }

    tests.into_iter().next()
}

/// Extract all titled test-like calls from one source file.
fn extract_test_calls(source_text: &str) -> Vec<TitledSourceTest> {
    let bytes = source_text.as_bytes();
    let mut tests = Vec::new();
    let mut title_ordinals = BTreeMap::<String, usize>::new();
    let mut index = 0;

    while index < bytes.len() {
        let Some((call_start, call_name_end)) = find_test_call_start(bytes, index) else {
            break;
        };

        let Some(end_index) = find_call_end(bytes, call_name_end) else {
            index = call_name_end.saturating_add(1);
            continue;
        };

        let text = &source_text[call_start..end_index];
        if let Some(title) = extract_call_title(text) {
            let ordinal = title_ordinals
                .entry(title.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);

            tests.push(TitledSourceTest {
                title,
                ordinal: *ordinal,
                text: text.to_string(),
            });
        }

        index = end_index;
    }

    tests
}

/// Find one test-like call start and its opening parenthesis.
fn find_test_call_start(bytes: &[u8], start_index: usize) -> Option<(usize, usize)> {
    let mut index = start_index;

    while index < bytes.len() {
        if !is_identifier_start(bytes[index]) {
            index += 1;
            continue;
        }

        let identifier_start = index;
        let mut identifier_end = index + 1;
        while identifier_end < bytes.len() && is_identifier_continue(bytes[identifier_end]) {
            identifier_end += 1;
        }

        let name = std::str::from_utf8(&bytes[identifier_start..identifier_end]).ok()?;
        let previous = identifier_start
            .checked_sub(1)
            .and_then(|previous| bytes.get(previous))
            .copied();
        let next = bytes.get(identifier_end).copied();

        if matches_test_call_name(name) && next == Some(b'(') && !matches!(previous, Some(b'.')) {
            return Some((identifier_start, identifier_end));
        }

        index = identifier_end;
    }

    None
}

/// Return whether one byte can start a javascript identifier.
fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

/// Return whether one byte can continue a javascript identifier.
fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

/// Return whether one identifier should be treated as one test-like call.
fn matches_test_call_name(name: &str) -> bool {
    name == "test" || name.ends_with("_test")
}

/// Find the end of one call that starts at the provided opening parenthesis index.
fn find_call_end(bytes: &[u8], open_paren_index: usize) -> Option<usize> {
    let mut index = open_paren_index;
    let mut depth = 0usize;
    let mut quote = None::<u8>;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied();

        if in_line_comment {
            if byte == b'\n' {
                in_line_comment = false;
            }
            index += 1;
            continue;
        }

        if in_block_comment {
            if byte == b'*' && next == Some(b'/') {
                in_block_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(quote_byte) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }

            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }

            if byte == quote_byte {
                quote = None;
            }

            index += 1;
            continue;
        }

        if byte == b'/' && next == Some(b'/') {
            in_line_comment = true;
            index += 2;
            continue;
        }

        if byte == b'/' && next == Some(b'*') {
            in_block_comment = true;
            index += 2;
            continue;
        }

        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
            index += 1;
            continue;
        }

        if byte == b'(' {
            depth += 1;
            index += 1;
            continue;
        }

        if byte == b')' {
            depth = depth.saturating_sub(1);
            index += 1;

            if depth == 0 {
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }

                if bytes.get(index) == Some(&b';') {
                    index += 1;
                }

                if bytes.get(index) == Some(&b'\r') && bytes.get(index + 1) == Some(&b'\n') {
                    index += 2;
                } else if bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }

                return Some(index);
            }

            continue;
        }

        index += 1;
    }

    None
}

/// Extract the title string from one test-like call.
fn extract_call_title(text: &str) -> Option<String> {
    extract_trailing_title(text).or_else(|| extract_leading_title(text))
}

/// Extract the trailing title string from one test-like call.
fn extract_trailing_title(text: &str) -> Option<String> {
    let trimmed = text.trim_end();
    let trimmed = trimmed.strip_suffix(';').unwrap_or(trimmed).trim_end();
    let trimmed = trimmed.strip_suffix(')').unwrap_or(trimmed).trim_end();
    let end_quote = trimmed.chars().last()?;
    if !matches!(end_quote, '"' | '\'') {
        return None;
    }

    let mut escaped = false;
    let mut start_index = None;
    for (index, character) in trimmed[..trimmed.len() - 1].char_indices().rev() {
        if escaped {
            escaped = false;
            continue;
        }

        if character == '\\' {
            escaped = true;
            continue;
        }

        if character == end_quote {
            start_index = Some(index);
            break;
        }
    }

    let start_index = start_index?;
    Some(trimmed[start_index + 1..trimmed.len() - 1].replace("\\\"", "\""))
}

/// Extract the leading title string from one test-like call.
fn extract_leading_title(text: &str) -> Option<String> {
    let open_paren = text.find('(')?;
    let rest = text[open_paren + 1..].trim_start();
    let quote = rest.chars().next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }

    let mut escaped = false;
    for (index, character) in rest[1..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if character == '\\' {
            escaped = true;
            continue;
        }

        if character == quote {
            return Some(rest[1..index + 1].replace("\\\"", "\""));
        }
    }

    None
}

/// Read one UTF-8 file into a string.
fn read_utf8_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{
        extract_named_statement, extract_named_test_call, extract_single_test_call,
        extract_statement_group, extract_table_row, extract_template_item, parse_region_metadata,
        parse_test_regions,
    };

    #[test]
    fn test_parse_region_metadata_supports_key_value_markers() {
        let metadata = parse_region_metadata(
            r#"// test.begin: source="api/headers/headers-basic.any.js" title="Check keys method" ordinal=2 source_kind=title source_key="Check keys method" source_hash="fnv1a64:1234""#,
        )
        .expect("metadata");

        assert_eq!(
            metadata.get("source").map(String::as_str),
            Some("api/headers/headers-basic.any.js")
        );
        assert_eq!(
            metadata.get("title").map(String::as_str),
            Some("Check keys method")
        );
        assert_eq!(metadata.get("ordinal").map(String::as_str), Some("2"));
        assert_eq!(
            metadata.get("source_kind").map(String::as_str),
            Some("title")
        );
        assert_eq!(
            metadata.get("source_key").map(String::as_str),
            Some("Check keys method")
        );
        assert_eq!(
            metadata.get("source_hash").map(String::as_str),
            Some("fnv1a64:1234")
        );
    }

    #[test]
    fn test_parse_test_regions_extracts_region_bodies() {
        let translated = r#"
// test.begin: source="example.js" title="one"
test(function() {
  thing();
}, "one");
// test.end
"#;

        let regions = parse_test_regions("example.ts", "example.js", translated).expect("regions");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].source, "example.js");
        assert_eq!(regions[0].title.as_deref(), Some("one"));
        assert!(regions[0].text.contains("thing();"));
    }

    #[test]
    fn test_extract_named_test_call_matches_titled_source_tests() {
        let source = r#"
test(function() {
  a();
}, "one");

promise_test(async () => {
  b();
}, "two");
"#;

        let test = extract_named_test_call(source, "two", 1).expect("test");
        assert!(test.text.contains("b();"));
    }

    #[test]
    fn test_extract_named_statement_prefers_one_line_statements() {
        let source = r#"
assert.ok(isatty(0), "stdin");
assert.ok(isatty(1), "stdout");
"#;

        let test = extract_named_statement(source, "stdout", 1).expect("test");
        assert_eq!(test.text.trim(), r#"assert.ok(isatty(1), "stdout");"#);
    }

    #[test]
    fn test_extract_single_test_call_matches_dynamic_titled_source_tests() {
        let source = r#"
var methods = ["append", "delete"];
for (var index in methods)
  test(function() {
    assert_true(methods[index] in headers, "headers has " + methods[index] + " method");
  }, "Headers has " + methods[index] + " method");
"#;

        let test = extract_single_test_call(source, 1).expect("test");
        assert!(test.text.contains("methods[index]"));
    }

    #[test]
    fn test_extract_template_item_expands_one_loop_item() {
        let source = r#"
var headers = new Headers();
var methods = ["append", "delete"];
for (var idx in methods)
  test(function() {
    assert_true(methods[idx] in headers, "headers has " + methods[idx] + " method");
  }, "Headers has " + methods[idx] + " method");
"#;

        let extracted = extract_template_item(source, "append").expect("template item");
        assert!(extracted.contains("\"append\" in headers"));
    }

    #[test]
    fn test_extract_statement_group_returns_surrounding_block() {
        let source = r#"
assert.throws(() => {
  setup();
  assert.partialDeepStrictEqual({ a: 1 }, { b: 2 });
}, (error) => {
  assert.strictEqual(error.message, "x");
  return true;
});
"#;

        let extracted = extract_statement_group(
            source,
            "assert.partialDeepStrictEqual({ a: 1 }, { b: 2 })",
            1,
        )
        .expect("statement group");
        assert!(extracted.contains("assert.throws(() => {"));
        assert!(extracted.contains("assert.strictEqual(error.message, \"x\");"));
    }

    #[test]
    fn test_extract_table_row_expands_one_named_case() {
        let source = r#"
const tests = [
  ["alpha", [1, 2], ["x"]],
  ["beta", [3, 4], ["y"]],
];

for (const [description, args, expected] of tests) {
  const actual = [];
  const args_ = args.map((value) => value);
  assert.throws(Error, () => doThing(...args_), description);
  assert.compareArray(actual, expected, `${description} order`);
}
"#;

        let extracted = extract_table_row(source, "beta").expect("table row");
        assert!(extracted.contains("[3, 4]"));
        assert!(extracted.contains("doThing(...args_)"));
        assert!(extracted.contains("\"beta\""));
    }
}
