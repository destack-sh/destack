//! Formatter roundtrip tests.

use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::Parser;
use destack_source::{
    File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use crate::harness::{
    TestCase, TestOptions, TestResult, check_diagnostics, discover_test_files, fixtures_dir,
    run_tests,
};

/// Run all formatter roundtrip tests.
pub fn run_formatter_tests(options: &TestOptions) -> std::process::ExitCode {
    let formatter_dir = fixtures_dir().join("formatter");
    let tests = discover_test_files(&formatter_dir, &["ds", ".d.ds"], "destack_test::formatter")
        .expect("failed to discover tests");
    run_tests(tests, options, run_roundtrip_test)
}

/// Run a single formatter roundtrip test.
fn run_roundtrip_test(test: &TestCase) -> TestResult {
    // set up a minimal program for diagnostics
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let language = LanguageOptions::default();
    let program = Arc::new(Program::new(language, cwd, fs, files));

    // read the original file
    let original = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };

    // create file
    let uri = Uri::from_path(&test.path);
    let file_type = if test.path.to_string_lossy().ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else {
        FileType::Destack
    };
    let file_id = FileId::new(0);
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let path = Some(test.path.clone());
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        path,
        file_type,
        original.clone(),
    ));
    program.files.insert((*file).clone());

    // parse the file
    let mut parser = Parser::lex_file(file.clone(), program.language);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);
    // bail on parse errors
    let parse_result = check_diagnostics(test, &program.files, &program.diagnostics);
    if parse_result.is_failed() {
        return parse_result;
    }

    // format the file
    let formatted = format_expressions(&parser, &expressions, &file, program.language);

    // compare to original
    if formatted == original {
        TestResult::Passed
    } else {
        // print a nice diff
        print_diff(&original, &formatted);
        TestResult::Failed {
            message: "formatted output differs from original".to_string(),
        }
    }
}

/// Format parsed expressions back to a string.
fn format_expressions(
    parser: &Parser,
    expressions: &[destack_ast::LocalNodeId<destack_ast::Expression>],
    file: &File,
    language: LanguageOptions,
) -> String {
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions::from(language);

    let context = DestackFormatContext {
        options: format_options,
        file,
        tree: &parser.tree,
        source_map: &parser.tree.source_map,
        parents,
        tokens: &parser.tokens,
        side_tokens: &parser.side_tokens,
        side_span: &side_span,
        strings: &strings,
    };

    let mut result = String::new();
    for (i, expr) in expressions.iter().enumerate() {
        let formatted = fir_format!(context.clone(), [expr]).unwrap();
        let printed = formatted.print().unwrap();
        result.push_str(printed.as_str());
        if i < expressions.len() - 1 {
            result.push('\n');
        }
    }

    // ensure trailing newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Print a unified diff between original and formatted.
fn print_diff(old: &str, new: &str) {
    use crate::harness::print::color;

    eprintln!();
    eprintln!("{}:", color::red("diff"));
    eprintln!();

    // show byte lengths for debugging
    eprintln!(
        "  {} bytes: {}, {} bytes: {}",
        color::red("old"),
        old.len(),
        color::green("new"),
        new.len()
    );

    // check trailing newline differences
    let old_has_newline = old.ends_with('\n');
    let new_has_newline = new.ends_with('\n');
    if old_has_newline != new_has_newline {
        eprintln!(
            "  (trailing newline: {} -> {})",
            if old_has_newline { "yes" } else { "no" },
            if new_has_newline { "yes" } else { "no" }
        );
    }

    eprintln!();

    // split into lines, preserving info about final newline
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // compute simple LCS-based diff
    let mut i = 0;
    let mut j = 0;
    let mut context_before: Vec<(usize, &str)> = Vec::new();
    let context_size = 2;

    while i < old_lines.len() || j < new_lines.len() {
        // matching lines
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            context_before.push((i + 1, old_lines[i]));
            if context_before.len() > context_size {
                context_before.remove(0);
            }
            i += 1;
            j += 1;
            continue;
        }

        // print context before diff
        for (line_no, line) in &context_before {
            eprintln!("{} {}", color::dim(&format!(" {line_no:>4}|")), line);
        }
        context_before.clear();

        // find next matching line
        let mut old_skip = 0;
        let mut new_skip = 0;
        'outer: for look_ahead in 1..=10 {
            for oi in 0..=look_ahead {
                let ni = look_ahead - oi;
                if i + oi < old_lines.len()
                    && j + ni < new_lines.len()
                    && old_lines[i + oi] == new_lines[j + ni]
                {
                    old_skip = oi;
                    new_skip = ni;
                    break 'outer;
                }
            }
        }

        // if no match found, consume rest
        if old_skip == 0 && new_skip == 0 {
            old_skip = old_lines.len().saturating_sub(i);
            new_skip = new_lines.len().saturating_sub(j);
        }

        // print removed lines
        for k in 0..old_skip {
            let line = old_lines[i + k];
            let escaped = escape_special(line);
            eprintln!("{} {}", color::red(&format!("-{:>4}|", i + k + 1)), escaped);
        }

        // print added lines
        for k in 0..new_skip {
            let line = new_lines[j + k];
            let escaped = escape_special(line);
            eprintln!(
                "{} {}",
                color::green(&format!("+{:>4}|", j + k + 1)),
                escaped
            );
        }

        i += old_skip;
        j += new_skip;
    }

    // show if files are identical except for whitespace
    if old.trim() == new.trim() && old != new {
        eprintln!();
        eprintln!(
            "  {}",
            color::yellow("note: files differ only in whitespace")
        );
    }

    eprintln!();
}

/// Escape special characters for display.
fn escape_special(s: &str) -> String {
    s.replace('\t', "→").replace('\r', "⏎").replace(' ', "·")
}
