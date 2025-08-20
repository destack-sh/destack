//! Tokei-like line counter command.

use crate::console::parser::{CommandApp, CommandArguments};
use crate::console::{console, table};
use destack_tokei as tokei;

const DEFAULT_EXTENSIONS: &str = "rs,ds,py,ts,js,tsx,jsx,toml,json";
const DEFAULT_ALIASES: &str =
    "rs=Rust,ds=Destack,py=Python,ts=TypeScript,js=JavaScript,tsx=TSX,jsx=JSX,toml=TOML,json=JSON";
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
            Some(format!(
                "Run counter.
				 --root <dir>
				 --ext \"{DEFAULT_EXTENSIONS}\"
				 --alias \"{DEFAULT_ALIASES}\"
				 --ignore \"<path1,path2,...>\""
            )),
        )
}

/// Run the tokei command.
fn run(ctx: CommandArguments) -> i32 {
    // get options
    let root = ctx.option("root").unwrap_or(".");
    let ext_csv = ctx.option("ext").unwrap_or(DEFAULT_EXTENSIONS);
    let exts: Vec<String> = ext_csv
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    // parse aliases: ext=AliasName
    let alias_csv = ctx.option("alias").unwrap_or(DEFAULT_ALIASES);
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

    // build table (Language, Extension, Files, Lines)
    let headers = vec![
        "Language".to_string(),
        "Extension".to_string(),
        "Files".to_string(),
        "Lines".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    for (lang, lines) in statistics.lines_by_language.iter() {
        let files = statistics.files_by_language.get(lang).copied().unwrap_or(0);
        let extensions = options
            .languages
            .iter()
            .find(|l| &l.name == lang)
            .map(|l| l.endings.join(","))
            .unwrap_or_default();
        rows.push(vec![
            lang.clone(),
            extensions,
            files.to_string(),
            lines.to_string(),
        ]);
    }
    // sort by lines desc
    rows.sort_by(|a, b| {
        b[3].parse::<u64>()
            .unwrap_or(0)
            .cmp(&a[3].parse::<u64>().unwrap_or(0))
    });

    // render table (Language, Extension, Files, Lines)
    let table_str = table::render_table(&headers, &rows, true, 2, None);
    let table_framed_str = console::frame(&table_str, Some("Tokei"), 2);
    println!("{table_framed_str}");

    0 // all good
}
