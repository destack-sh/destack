//! Tokei-like line counter command.

use crate::console::parse::{CommandApp, CommandArguments};
use crate::console::{console, table};
use destack_library_tokei as tokei;

struct LanguageDeclaration<'a> {
    extensions: &'a [&'a str],
    name: Option<&'a str>,
}

const DEFAULT_EXTENSIONS: &[LanguageDeclaration<'static>] = &[
    LanguageDeclaration {
        extensions: &["rs"],
        name: Some("Rust"),
    },
    LanguageDeclaration {
        extensions: &["ds"],
        name: Some("Destack"),
    },
    LanguageDeclaration {
        extensions: &["py"],
        name: Some("Python"),
    },
    LanguageDeclaration {
        extensions: &["ts"],
        name: Some("TypeScript"),
    },
    LanguageDeclaration {
        extensions: &["js"],
        name: Some("JavaScript"),
    },
    LanguageDeclaration {
        extensions: &["tsx"],
        name: Some("TSX"),
    },
    LanguageDeclaration {
        extensions: &["jsx"],
        name: Some("JSX"),
    },
    LanguageDeclaration {
        extensions: &["tf"],
        name: Some("Terraform"),
    },
    LanguageDeclaration {
        extensions: &["sh"],
        name: Some("Shell"),
    },
    LanguageDeclaration {
        extensions: &["sql"],
        name: Some("SQL"),
    },
    LanguageDeclaration {
        extensions: &["toml"],
        name: Some("TOML"),
    },
    LanguageDeclaration {
        extensions: &["json"],
        name: Some("JSON"),
    },
    LanguageDeclaration {
        extensions: &["md"],
        name: Some("Markdown"),
    },
    LanguageDeclaration {
        extensions: &["yaml", "yml"],
        name: Some("YAML"),
    },
];

const DEFAULT_IGNORE_PATHS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "env",
    "ENV",
    "env.bak",
    "venv.bak",
    "dist",
    "build",
    ".idea",
    ".vscode",
    ".vscode-test",
    "out",
    ".next",
    ".nuxt",
    "coverage",
    "htmlcov",
    ".cache",
    ".parcel-cache",
    ".pytest_cache",
    ".mypy_cache",
    ".tox",
    ".nox",
    ".hypothesis",
    ".yarn",
    ".temp",
    "hfuzz_target",
    "fuzz",
];

/// Create the tokei CLI app.
pub fn app() -> CommandApp {
    CommandApp::new("tokei")
        .help("Count source lines fast.")
        .command(
            "run",
            run,
            Some(
                "Run counter.
				 --root <dir>
				 --ext .py,.rs,.ds
				 --alias \"rs=Rust,ds=Destack,py=Python\"
				 --ignore \"<path1,path2,...>\""
                    .to_string(),
            ),
        )
}

/// Run the tokei command.
fn run(ctx: CommandArguments) -> i32 {
    // get options
    let root = ctx.option("root").unwrap_or(".");
    let default_ext_csv = DEFAULT_EXTENSIONS
        .iter()
        .flat_map(|e| e.extensions.iter().copied())
        .collect::<Vec<_>>()
        .join(",");
    let ext_csv = ctx.option("ext").unwrap_or(&default_ext_csv);
    let exts: Vec<String> = ext_csv
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    // parse aliases: ext=AliasName
    let default_alias_csv = DEFAULT_EXTENSIONS
        .iter()
        .flat_map(|e| {
            e.extensions
                .iter()
                .map(|ext| format!("{}={}", ext, e.name.unwrap_or(ext)))
        })
        .collect::<Vec<_>>()
        .join(",");
    let alias_csv = ctx.option("alias").unwrap_or(&default_alias_csv);
    let mut alias_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for part in alias_csv.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            if !k.is_empty() && !v.is_empty() {
                alias_map.insert(k.to_string(), v.to_string());
            }
        }
    }

    // assemble language config by (name, extensions)
    let languages: Vec<tokei::LanguageConfiguration> = exts
        .iter()
        .map(|e| {
            // default to extension name if no alias is found
            let name = alias_map.get(e).cloned().unwrap_or_else(|| e.clone());
            tokei::LanguageConfiguration {
                name,
                endings: vec![format!(".{e}")],
                comment: None,
            }
        })
        .collect();

    // assemble options
    let mut options = tokei::Options {
        root: root.into(),
        languages,
        patterns: vec!["**/*".to_string()],
        ignore_paths: DEFAULT_IGNORE_PATHS.iter().map(|s| s.to_string()).collect(),
    };

    // override patterns if --pattern is set
    if ctx.flag("pattern") {
        if let Some(p) = ctx.option("pattern") {
            options.patterns = p
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    } else {
        options.patterns = vec!["**/*".to_string()];
    }

    // run
    let statistics = match tokei::count(&options) {
        Ok(s) => s,
        Err(e) => {
            console::error(&format!("tokei failed: {e}"));
            return 1;
        }
    };

    struct LanguageStatistic {
        language: String,
        extension: String,
        files: u64,
        lines: u64,
    }

    // build table (Language, Extension, Files, Lines)
    let headers = vec![
        "Language".to_string(),
        "Extension".to_string(),
        "Files".to_string(),
        "Lines".to_string(),
    ];
    let mut rows: Vec<LanguageStatistic> = Vec::new();
    for (lang, lines) in statistics.lines_by_languageuage.iter() {
        let files = statistics
            .files_by_languageuage
            .get(lang)
            .copied()
            .unwrap_or(0);
        let extensions = options
            .languages
            .iter()
            .find(|l| &l.name == lang)
            .map(|l| l.endings.join(","))
            .unwrap_or_default();
        rows.push(LanguageStatistic {
            language: lang.clone(),
            extension: extensions,
            files,
            lines: *lines,
        });
    }
    // sort by lines desc
    rows.sort_by(|a, b| b.lines.cmp(&a.lines));

    // render table (Language, Extension, Files, Lines)
    let rows_str: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.language.clone(),
                r.extension.clone(),
                console::color(&r.files.to_string(), "36"), // cyan
                console::color(&r.lines.to_string(), "32"), // green
            ]
        })
        .collect();
    let table_str = table::render_table(&headers, &rows_str, false, 2, None);
    let table_framed_str = console::frame(&table_str, Some("Tokei"), 2);
    println!("{table_framed_str}");

    0 // all good
}
