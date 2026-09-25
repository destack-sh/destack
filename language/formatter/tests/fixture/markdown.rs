use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

/// One formatter transformation declared in Markdown.
#[derive(Debug)]
pub(super) struct Transform {
    /// The complete test name.
    pub(super) name: String,
    /// The virtual source path.
    pub(super) path: PathBuf,
    /// The unformatted source.
    pub(super) source: String,
    /// The canonical formatted source.
    pub(super) expected: String,
    /// The formatter options declared on the source fence.
    pub(super) options: HashMap<String, String>,
}

/// Return the formatter fixture root.
pub(super) fn fixture_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixture")
}

/// Discover Markdown files recursively in stable path order.
pub(super) fn markdown_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![directory.to_path_buf()];
    let mut files = Vec::new();

    // visit every directory without depending on filesystem order
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("failed to inspect '{}': {error}", path.display()))?;
            if file_type.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    files.sort();

    Ok(files)
}

/// Parse one formatter transformation document.
pub(super) fn parse_transforms(path: &Path, root: &Path) -> Result<Vec<Transform>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let relative = path.strip_prefix(root).map_err(|error| {
        format!(
            "fixture '{}' is outside '{}': {error}",
            path.display(),
            root.display()
        )
    })?;
    let mut parser = Parser::new(&source);
    let mut transforms = Vec::new();
    let mut section = String::new();
    let mut name = None;
    let mut heading = None;
    let mut heading_text = String::new();
    let mut fence = None;
    let mut fence_source = String::new();
    let mut input = None;
    let mut expected = None;

    // consume the small formatter-specific Markdown grammar
    for event in parser.by_ref() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                finish_transform(
                    relative,
                    &section,
                    &mut name,
                    &mut input,
                    &mut expected,
                    &mut transforms,
                )?;
                heading = Some(level);
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                let text = heading_text.trim();
                match heading.take() {
                    Some(HeadingLevel::H2) => section = text.to_string(),
                    Some(HeadingLevel::H3 | HeadingLevel::H4) => {
                        name = Some(text.to_string());
                    }
                    _ => {}
                }
            }
            Event::Text(text) | Event::Code(text) if heading.is_some() => {
                heading_text.push_str(&text);
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) => {
                fence = Some(language.to_string());
                fence_source.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                let language = fence.take().ok_or_else(|| {
                    format!(
                        "fixture '{}' closes a fenced block that was not opened",
                        relative.display()
                    )
                })?;
                let parsed = parse_fence(&language)?;
                if parsed.is_expected {
                    if expected.replace(fence_source.clone()).is_some() {
                        return Err(format!(
                            "fixture '{}' repeats its expected block",
                            relative.display()
                        ));
                    }
                } else if parsed.is_source
                    && input
                        .replace((parsed.path, fence_source.clone(), parsed.options))
                        .is_some()
                {
                    return Err(format!(
                        "fixture '{}' repeats its source block",
                        relative.display()
                    ));
                }
            }
            Event::Text(text) if fence.is_some() => fence_source.push_str(&text),
            Event::SoftBreak | Event::HardBreak if fence.is_some() => fence_source.push('\n'),
            _ => {}
        }
    }
    finish_transform(
        relative,
        &section,
        &mut name,
        &mut input,
        &mut expected,
        &mut transforms,
    )?;

    Ok(transforms)
}

/// One parsed formatter fence tag.
struct Fence {
    /// Whether the fence is an input source.
    is_source: bool,
    /// Whether the fence is an expected output.
    is_expected: bool,
    /// The virtual source path.
    path: PathBuf,
    /// The formatter options.
    options: HashMap<String, String>,
}

/// Parse one formatter fence tag.
fn parse_fence(language: &str) -> Result<Fence, String> {
    let mut parts = language.split_whitespace();
    let first = parts.next().unwrap_or_default();
    let (language, path) = first
        .split_once(':')
        .map_or((first, None), |(language, path)| (language, Some(path)));
    let mut path = path.map(PathBuf::from);
    let mut is_expected = false;
    let mut options = HashMap::new();

    // parse positional markers and key-value formatter options
    for part in parts {
        if part == "expected" {
            is_expected = true;
        } else if let Some((key, value)) = part.split_once('=') {
            if options.insert(key.to_string(), value.to_string()).is_some() {
                return Err(format!("formatter fence repeats option '{key}'"));
            }
        } else if path.replace(PathBuf::from(part)).is_some() {
            return Err(format!("formatter fence has extra marker '{part}'"));
        }
    }

    Ok(Fence {
        is_source: language.eq_ignore_ascii_case("tspp") && !is_expected,
        is_expected: language.eq_ignore_ascii_case("tspp") && is_expected,
        path: path.unwrap_or_else(|| PathBuf::from("main.tspp")),
        options,
    })
}

/// Materialize one pending transformation.
fn finish_transform(
    document: &Path,
    section: &str,
    name: &mut Option<String>,
    input: &mut Option<(PathBuf, String, HashMap<String, String>)>,
    expected: &mut Option<String>,
    transforms: &mut Vec<Transform>,
) -> Result<(), String> {
    let Some(name) = name.take() else {
        return Ok(());
    };
    let (path, source, options) = input
        .take()
        .ok_or_else(|| format!("formatter fixture '{name}' has no source block"))?;
    let expected = expected
        .take()
        .ok_or_else(|| format!("formatter fixture '{name}' has no expected block"))?;
    let full_name = format!("{}/{section}/{name}", document.display());
    transforms.push(Transform {
        name: full_name,
        path,
        source,
        expected,
        options,
    });

    Ok(())
}
