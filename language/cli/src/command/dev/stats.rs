use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use clap::Args;
use tiktoken_rs::o200k_base;
use tspp_source::{FileSystem, PhysicalFileSystem};

use crate::console::{self, bold, color, dim, style, visible_width};

/// Marker definitions: (name, ansi_color_code, dot_char)
const MARKERS: &[(&str, &str, char)] = &[
    ("NOTE", "34", '●'), // blue
    ("TODO", "33", '●'), // yellow
    ("FUGU", "31", '●'), // red
];

/// Byte threshold below which estimated token mode still does a full count.
const TOKEN_ESTIMATE_EXACT_BYTES: usize = 8 * 1024;

/// Minimum number of sampled large files per extension bucket.
const TOKEN_ESTIMATE_MIN_SAMPLE_FILES: usize = 3;

/// Maximum number of sampled large files per extension bucket.
const TOKEN_ESTIMATE_MAX_SAMPLE_FILES: usize = 12;

/// Arguments for the stats command.
#[derive(Args, Debug, Clone)]
pub struct StatsArgs {
    /// Path to analyze (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Maximum directory depth to display (default: 2).
    #[arg(long, short = 'd', default_value = "2")]
    pub depth: usize,

    /// Show full tree (unlimited depth).
    #[arg(long, conflicts_with = "depth")]
    pub full: bool,

    /// Hide token counts (faster, lines only).
    #[arg(long)]
    pub no_tokens: bool,

    /// Estimate token counts from sampled content instead of tokenizing full files.
    #[arg(long, conflicts_with = "no_tokens")]
    pub estimate_tokens: bool,

    /// Show file type breakdown per directory.
    #[arg(long, short = 't')]
    pub by_type: bool,
}

/// Statistics for a single file or aggregated directory.
#[derive(Debug, Clone, Default)]
struct Stats {
    /// Number of files
    files: usize,
    /// Total lines
    lines: usize,
    /// Lines containing code (may also have comments)
    lines_code: usize,
    /// Lines containing only comments (no code)
    lines_comment: usize,
    /// Blank lines (whitespace only)
    lines_blank: usize,
    /// Total bytes
    bytes: usize,
    /// Total tokens (when tokenization is enabled)
    tokens: usize,
    /// Marker counts (indexed same as MARKERS constant)
    markers: Vec<usize>,
    /// Tag counts (discovered dynamically, e.g., #Performance, @Cleanup)
    tags: BTreeMap<String, usize>,
}

/// Token counting behavior for the stats command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenCountMode {
    /// Do not count tokens.
    None,
    /// Tokenize every full file.
    Full,
    /// Estimate token counts from sampled content.
    Estimate,
}

impl TokenCountMode {
    /// Return whether token columns should be shown.
    fn shows_tokens(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Return whether token counts are estimated.
    fn is_estimated(self) -> bool {
        matches!(self, Self::Estimate)
    }
}

impl Stats {
    /// Create with proper marker vec size.
    fn new() -> Self {
        Self {
            markers: vec![0; MARKERS.len()],
            ..Default::default()
        }
    }

    /// Add another Stats instance to this one.
    fn add(&mut self, other: &Stats) {
        self.files += other.files;
        self.lines += other.lines;
        self.lines_code += other.lines_code;
        self.lines_comment += other.lines_comment;
        self.lines_blank += other.lines_blank;
        self.bytes += other.bytes;
        self.tokens += other.tokens;
        for (i, count) in other.markers.iter().enumerate() {
            if i < self.markers.len() {
                self.markers[i] += count;
            }
        }
        for (tag, count) in &other.tags {
            *self.tags.entry(tag.clone()).or_default() += count;
        }
    }

    /// Total marker count.
    fn markers_total(&self) -> usize {
        self.markers.iter().sum()
    }

    /// Check if any marker of given index exists.
    fn has_marker(&self, index: usize) -> bool {
        self.markers.get(index).is_some_and(|&c| c > 0)
    }
}

/// Directory node in the stats tree.
#[derive(Debug)]
struct DirNode {
    /// Directory name
    name: String,
    /// Aggregated statistics for this directory
    stats: Stats,
    /// Statistics broken down by file extension
    by_extension: BTreeMap<String, Stats>,
    /// Child directory nodes
    children: BTreeMap<String, DirNode>,
}

/// File metadata collected during the directory walk.
#[derive(Debug, Clone)]
struct FileEntry {
    /// Path relative to the stats root.
    relative_path: PathBuf,
    /// File extension bucket.
    extension: String,
    /// Per-file statistics.
    stats: Stats,
}

impl DirNode {
    /// Create a new directory node.
    fn new(name: String) -> Self {
        Self {
            name,
            stats: Stats::new(),
            by_extension: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }

    /// Get or create a child directory node.
    fn child(&mut self, name: &str) -> &mut DirNode {
        self.children
            .entry(name.to_string())
            .or_insert_with(|| DirNode::new(name.to_string()))
    }

    /// Add file stats to this node and aggregate by extension.
    fn add_file(&mut self, extension: &str, stats: Stats) {
        self.stats.add(&stats);
        self.by_extension
            .entry(extension.to_string())
            .or_default()
            .add(&stats);
    }

    /// Recursively aggregate child stats into parent.
    fn aggregate(&mut self) {
        for child in self.children.values_mut() {
            child.aggregate();
            self.stats.add(&child.stats);

            // merge extension stats from children
            for (ext, ext_stats) in &child.by_extension {
                self.by_extension
                    .entry(ext.clone())
                    .or_default()
                    .add(ext_stats);
            }
        }
    }
}

/// Run the stats command.
///
/// # Arguments
/// * `args` - Command line arguments for the stats command
///
/// # Returns
/// Exit code (0 for success, 1 for error)
pub fn run(args: &StatsArgs) -> i32 {
    // resolve root path
    let root = match args.path.canonicalize() {
        Ok(p) => p,
        Err(error) => {
            console::error(&format!("error: failed to resolve path: {error}"));

            return 1;
        }
    };

    // select token counting mode
    let token_count_mode = if args.no_tokens {
        TokenCountMode::None
    } else if args.estimate_tokens {
        TokenCountMode::Estimate
    } else {
        TokenCountMode::Full
    };

    // initialize tokenizer if needed
    let tokenizer = if matches!(token_count_mode, TokenCountMode::None) {
        None
    } else {
        match o200k_base() {
            Ok(bpe) => Some(bpe),
            Err(error) => {
                console::error(&format!("error: failed to initialize tokenizer: {error}"));

                return 1;
            }
        }
    };

    // walk the directory
    let mut ignore_set = tspp_source::IgnoreSet::new();
    let file_system = PhysicalFileSystem;
    let mut files = Vec::new();
    if let Err(error) = ignore_set.load_root(&file_system, &root) {
        console::error(&format!("error: failed to read ignore rules: {error}"));

        return 1;
    }
    if let Err(error) = walk_directory(&file_system, &root, &root, &mut files, &mut ignore_set) {
        console::error(&format!("error: failed to read source tree: {error}"));

        return 1;
    }

    // count tokens once file metadata is known
    assign_token_counts(&root, &mut files, tokenizer.as_ref(), token_count_mode);

    // build and aggregate the stats tree
    let mut tree = build_tree(&root, files);
    tree.aggregate();

    // determine effective depth
    let max_depth = if args.full { usize::MAX } else { args.depth };

    // render the tree
    let output = render_tree(&tree, max_depth, args.by_type, token_count_mode);
    println!("{output}");

    0
}

/// Recursively walk a directory and collect stats.
///
/// # Arguments
/// * `root` - Root directory path for gitignore resolution
/// * `directory` - Current directory being processed
/// * `files` - Collected file metadata entries
/// * `ignore_set` - Gitignore rules for filtering files
fn walk_directory(
    file_system: &PhysicalFileSystem,
    root: &Path,
    directory: &Path,
    files: &mut Vec<FileEntry>,
    ignore_set: &mut tspp_source::IgnoreSet,
) -> io::Result<()> {
    // load gitignore for this directory
    ignore_set.load(file_system, directory)?;

    // inspect every directory entry
    let paths = file_system.read_dir(directory).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read {}: {error}", directory.display()),
        )
    })?;
    for path in paths {
        let metadata = file_system.symlink_metadata(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to inspect {}: {error}", path.display()),
            )
        })?;

        // skip hidden files and directories
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("path has no UTF-8 file name: {}", path.display()),
                )
            })?;
        if name.starts_with('.') {
            continue;
        }

        // check gitignore
        if ignore_set.is_ignored(&path, metadata.is_directory) {
            continue;
        }

        if metadata.is_directory {
            walk_directory(file_system, root, &path, files, ignore_set)?;
        } else if metadata.is_file {
            // get extension
            let extension = path
                .extension()
                .and_then(OsStr::to_str)
                .map(|s| format!(".{s}"))
                .unwrap_or_else(|| "(no ext)".to_string());

            // skip binary files by extension
            if is_binary_extension(&extension) {
                continue;
            }

            // read file content
            let content = file_system.read_to_string(&path).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("failed to read {}: {error}", path.display()),
                )
            })?;

            // count stats
            let lines = content.lines().count();
            let analysis = analyze_content(&content, &extension);
            let bytes = content.len();
            let mut stats = Stats::new();
            stats.files = 1;
            stats.lines = lines;
            stats.lines_code = analysis.lines_code;
            stats.lines_comment = analysis.lines_comment;
            stats.lines_blank = analysis.lines_blank;
            stats.bytes = bytes;
            stats.markers = analysis.markers;
            stats.tags = analysis.tags;

            let relative_path = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .map_err(|_| {
                    io::Error::other(format!(
                        "walked path is outside source root: {}",
                        path.display()
                    ))
                })?;

            files.push(FileEntry {
                relative_path,
                extension,
                stats,
            });
        }
    }

    Ok(())
}

/// Assign token counts to collected file entries.
fn assign_token_counts(
    root: &Path,
    files: &mut [FileEntry],
    tokenizer: Option<&tiktoken_rs::CoreBPE>,
    token_count_mode: TokenCountMode,
) {
    // skip token counting entirely when disabled
    if matches!(token_count_mode, TokenCountMode::None) {
        return;
    }

    let Some(tokenizer) = tokenizer else {
        return;
    };

    // exact tokenization
    if matches!(token_count_mode, TokenCountMode::Full) {
        for file in files {
            file.stats.tokens = tokenize_file(root, &file.relative_path, tokenizer);
        }

        return;
    }

    // estimate mode
    assign_estimated_token_counts(root, files, tokenizer);
}

/// Assign estimated token counts with per-extension sampling.
fn assign_estimated_token_counts(
    root: &Path,
    files: &mut [FileEntry],
    tokenizer: &tiktoken_rs::CoreBPE,
) {
    let mut extensions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        extensions
            .entry(file.extension.clone())
            .or_default()
            .push(index);
    }

    let mut global_sampled_bytes = 0usize;
    let mut global_sampled_tokens = 0usize;

    // exact small files
    for file in files.iter_mut() {
        if file.stats.bytes <= TOKEN_ESTIMATE_EXACT_BYTES {
            file.stats.tokens = tokenize_file(root, &file.relative_path, tokenizer);
            global_sampled_bytes += file.stats.bytes;
            global_sampled_tokens += file.stats.tokens;
        }
    }

    // per extension ratios
    for indices in extensions.values() {
        let mut sampled_bytes = 0usize;
        let mut sampled_tokens = 0usize;
        let mut large_indices = Vec::new();

        // collect bucket-local exact samples first
        for index in indices {
            let file = &files[*index];

            if file.stats.bytes <= TOKEN_ESTIMATE_EXACT_BYTES {
                sampled_bytes += file.stats.bytes;
                sampled_tokens += file.stats.tokens;
            } else {
                large_indices.push(*index);
            }
        }

        // sample representative larger files by size
        let sample_indices = choose_estimate_sample_indices(files, &large_indices);
        for index in sample_indices {
            let file = &mut files[index];
            file.stats.tokens = tokenize_file(root, &file.relative_path, tokenizer);
            sampled_bytes += file.stats.bytes;
            sampled_tokens += file.stats.tokens;
            global_sampled_bytes += file.stats.bytes;
            global_sampled_tokens += file.stats.tokens;
        }

        let ratio = token_ratio(sampled_tokens, sampled_bytes)
            .or_else(|| token_ratio(global_sampled_tokens, global_sampled_bytes));

        let Some(tokens_per_byte) = ratio else {
            for index in large_indices {
                let file = &mut files[index];
                file.stats.tokens = tokenize_file(root, &file.relative_path, tokenizer);
            }

            continue;
        };

        for index in large_indices {
            let file = &mut files[index];
            if file.stats.tokens > 0 {
                continue;
            }

            let estimated_tokens = (tokens_per_byte * file.stats.bytes as f64).round() as usize;
            file.stats.tokens = estimated_tokens.max(1);
        }
    }
}

/// Return token ratio if there is enough sampled data.
fn token_ratio(tokens: usize, bytes: usize) -> Option<f64> {
    if tokens == 0 || bytes == 0 {
        return None;
    }

    Some(tokens as f64 / bytes as f64)
}

/// Choose a representative sample of larger files for one extension bucket.
fn choose_estimate_sample_indices(files: &[FileEntry], indices: &[usize]) -> Vec<usize> {
    if indices.is_empty() {
        return Vec::new();
    }

    let sample_count = estimate_sample_count(indices.len());
    if indices.len() <= sample_count {
        return indices.to_vec();
    }

    let mut sorted_indices = indices.to_vec();
    sorted_indices.sort_by_key(|index| files[*index].stats.bytes);

    let last_position = sorted_indices.len() - 1;
    let mut sampled = Vec::with_capacity(sample_count);
    for sample_index in 0..sample_count {
        let position = sample_index * last_position / (sample_count - 1);
        sampled.push(sorted_indices[position]);
    }

    sampled.sort_unstable();
    sampled.dedup();
    sampled
}

/// Return the sample count for one extension bucket.
fn estimate_sample_count(file_count: usize) -> usize {
    let sample_count = (file_count as f64).sqrt().round() as usize;
    sample_count.clamp(
        TOKEN_ESTIMATE_MIN_SAMPLE_FILES,
        TOKEN_ESTIMATE_MAX_SAMPLE_FILES,
    )
}

/// Tokenize one file by path.
fn tokenize_file(root: &Path, relative_path: &Path, tokenizer: &tiktoken_rs::CoreBPE) -> usize {
    let path = root.join(relative_path);
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return 0,
    };

    tokenizer.encode_with_special_tokens(&content).len()
}

/// Build the directory tree from collected file entries.
fn build_tree(root: &Path, files: Vec<FileEntry>) -> DirNode {
    let root_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".")
        .to_string();
    let mut tree = DirNode::new(root_name);

    for file in files {
        let mut node = &mut tree;
        let mut components = file.relative_path.components().peekable();

        while let Some(component) = components.next() {
            let component = component.as_os_str().to_string_lossy();

            if components.peek().is_none() {
                break;
            }

            node = node.child(&component);
        }

        node.add_file(&file.extension, file.stats);
    }

    tree
}

/// Check if a file extension typically indicates binary content.
///
/// # Arguments
/// * `extension` - File extension to check (including the dot)
///
/// # Returns
/// `true` if the extension is typically binary, `false` otherwise
fn is_binary_extension(extension: &str) -> bool {
    matches!(
        extension.to_lowercase().as_str(),
        ".png"
            | ".jpg"
            | ".jpeg"
            | ".gif"
            | ".ico"
            | ".webp"
            | ".svg"
            | ".bmp"
            | ".tiff"
            | ".pdf"
            | ".zip"
            | ".tar"
            | ".gz"
            | ".bz2"
            | ".xz"
            | ".7z"
            | ".rar"
            | ".exe"
            | ".dll"
            | ".so"
            | ".dylib"
            | ".bin"
            | ".obj"
            | ".o"
            | ".a"
            | ".lib"
            | ".wasm"
            | ".ttf"
            | ".otf"
            | ".woff"
            | ".woff2"
            | ".eot"
            | ".mp3"
            | ".mp4"
            | ".wav"
            | ".ogg"
            | ".webm"
            | ".mov"
            | ".avi"
            | ".mkv"
            | ".db"
            | ".sqlite"
            | ".lock"
    )
}

/// Comment style for a language.
#[derive(Debug, Clone, Copy)]
enum CommentStyle {
    /// C-style: // and /* */
    CStyle,
    /// Hash-style: #
    Hash,
    /// Dash-style: -- and optionally --[[ ]]
    Dash,
    /// HTML-style: <!-- -->
    Html,
    /// No recognized comment style
    None,
}

/// Get the comment style for a file extension.
fn comment_style_for_extension(extension: &str) -> CommentStyle {
    match extension.to_lowercase().as_str() {
        // c-style comments
        ".rs" | ".js" | ".ts" | ".tsx" | ".jsx" | ".tspp" | ".c" | ".cpp" | ".cc" | ".cxx"
        | ".h" | ".hpp" | ".hxx" | ".java" | ".go" | ".swift" | ".kt" | ".kts" | ".scala"
        | ".cs" | ".m" | ".mm" | ".php" | ".css" | ".scss" | ".sass" | ".less" | ".json"
        | ".jsonc" | ".proto" | ".zig" | ".v" | ".d" | ".vert" | ".frag" | ".glsl" | ".hlsl"
        | ".wgsl" | ".metal" => CommentStyle::CStyle,

        // hash-style comments
        ".py" | ".rb" | ".sh" | ".bash" | ".zsh" | ".fish" | ".pl" | ".pm" | ".r" | ".yml"
        | ".yaml" | ".toml" | ".ini" | ".conf" | ".cfg" | ".makefile" | ".mk" | ".cmake"
        | ".dockerfile" | ".gitignore" | ".gitattributes" | ".env" | ".editorconfig" | ".tf"
        | ".hcl" | ".nix" | ".just" | ".justfile" => CommentStyle::Hash,

        // dash-style comments
        ".lua" | ".sql" | ".hs" | ".lhs" | ".elm" | ".purs" | ".ada" | ".adb" | ".ads" => {
            CommentStyle::Dash
        }

        // html-style comments
        ".html" | ".htm" | ".xml" | ".svg" | ".vue" | ".svelte" | ".astro" => CommentStyle::Html,

        // markdown uses html comments but also has other constructs, treat as html
        ".md" | ".mdx" => CommentStyle::Html,

        _ => CommentStyle::None,
    }
}

/// Results of analyzing file content.
#[derive(Debug, Clone, Default)]
struct LineAnalysis {
    lines_code: usize,
    lines_comment: usize,
    lines_blank: usize,
    /// Marker counts (indexed same as MARKERS constant)
    markers: Vec<usize>,
    /// Tag counts (discovered dynamically)
    tags: BTreeMap<String, usize>,
}

impl LineAnalysis {
    fn new() -> Self {
        Self {
            markers: vec![0; MARKERS.len()],
            ..Default::default()
        }
    }
}

/// Analyze file content for line types and markers.
///
/// A blank line contains only whitespace.
/// A comment line contains only a comment (with optional whitespace).
/// A code line contains any non-comment content (may also have trailing comments).
fn analyze_content(content: &str, extension: &str) -> LineAnalysis {
    let style = comment_style_for_extension(extension);
    let mut result = LineAnalysis::new();
    let mut in_block_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // count markers and tags
        let mut has_marker = false;
        for (i, (marker, _, _)) in MARKERS.iter().enumerate() {
            let count = count_marker(line, marker);
            result.markers[i] += count;
            if count > 0 {
                has_marker = true;
            }
        }

        // discover tags on lines that have markers (both #Tag and @Tag)
        if has_marker {
            extract_tags(line, &mut result.tags);
        }

        // blank line
        if trimmed.is_empty() {
            result.lines_blank += 1;
            continue;
        }

        // classify based on comment style
        let line_type = classify_line(trimmed, style, &mut in_block_comment);
        match line_type {
            LineType::Code => result.lines_code += 1,
            LineType::Comment => result.lines_comment += 1,
        }
    }

    result
}

/// Extract #Tag or @Tag patterns from a line.
fn extract_tags(line: &str, tags: &mut BTreeMap<String, usize>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // look for # or @
        if bytes[i] == b'#' || bytes[i] == b'@' {
            let start = i + 1;

            // tag must start with uppercase letter
            if start < len && bytes[start].is_ascii_uppercase() {
                let mut end = start;

                // collect alphanumeric characters
                while end < len && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                    end += 1;
                }

                // must have at least 2 chars and be PascalCase (start with uppercase)
                if end > start + 1 {
                    let tag = &line[start..end];
                    *tags.entry(tag.to_string()).or_default() += 1;
                }

                i = end;
                continue;
            }
        }
        i += 1;
    }
}

/// Count occurrences of a marker in a line with word boundary checking.
fn count_marker(line: &str, marker: &str) -> usize {
    let mut count = 0;
    let mut search_start = 0;
    let line_bytes = line.as_bytes();
    let marker_len = marker.len();

    while let Some(pos) = line[search_start..].find(marker) {
        let abs_pos = search_start + pos;
        let end_pos = abs_pos + marker_len;

        // check word boundary before (must not be alphanumeric or underscore)
        let valid_before = abs_pos == 0
            || !line_bytes[abs_pos - 1].is_ascii_alphanumeric() && line_bytes[abs_pos - 1] != b'_';

        // check word boundary after (must not be alphanumeric or underscore, or followed by colon/space)
        let valid_after = end_pos >= line.len()
            || !line_bytes[end_pos].is_ascii_alphanumeric() && line_bytes[end_pos] != b'_';

        if valid_before && valid_after {
            count += 1;
        }

        search_start = abs_pos + 1;
    }

    count
}

/// Line classification result.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LineType {
    Code,
    Comment,
}

/// Classify a single line (already trimmed, known non-empty).
fn classify_line(trimmed: &str, style: CommentStyle, in_block: &mut bool) -> LineType {
    match style {
        CommentStyle::CStyle => classify_c_style(trimmed, in_block),
        CommentStyle::Hash => classify_hash_style(trimmed),
        CommentStyle::Dash => classify_dash_style(trimmed, in_block),
        CommentStyle::Html => classify_html_style(trimmed, in_block),
        CommentStyle::None => LineType::Code,
    }
}

/// Classify a line with C-style comments (// and /* */).
fn classify_c_style(trimmed: &str, in_block: &mut bool) -> LineType {
    // if we're in a block comment
    if *in_block {
        if let Some(end_pos) = trimmed.find("*/") {
            *in_block = false;
            let after_comment = trimmed[end_pos + 2..].trim();
            if after_comment.is_empty() {
                return LineType::Comment;
            }
            // there's content after the block comment ends, check if it's code or another comment
            return classify_c_style(after_comment, in_block);
        }
        return LineType::Comment;
    }

    // check for single-line comment (not inside a string)
    if let Some(pos) = find_outside_strings(trimmed, "//") {
        if pos == 0 {
            return LineType::Comment;
        }
        // there's code before the comment
        return LineType::Code;
    }

    // check for block comment start (not inside a string)
    if let Some(start_pos) = find_outside_strings(trimmed, "/*") {
        let before_comment = trimmed[..start_pos].trim();

        // check if block comment ends on same line
        if let Some(end_offset) = trimmed[start_pos + 2..].find("*/") {
            let after_comment = trimmed[start_pos + 2 + end_offset + 2..].trim();
            if before_comment.is_empty() && after_comment.is_empty() {
                return LineType::Comment;
            }
            if before_comment.is_empty() {
                // recurse to check what's after
                return classify_c_style(after_comment, in_block);
            }
            // there's code before the comment
            return LineType::Code;
        }

        // block comment continues to next line
        *in_block = true;
        if before_comment.is_empty() {
            return LineType::Comment;
        }
        return LineType::Code;
    }

    LineType::Code
}

/// Find a pattern outside of string literals (simple heuristic).
///
/// Returns the position of the pattern if found outside strings, None otherwise.
/// Handles double-quoted, single-quoted, and backtick strings.
/// Handles basic escape sequences.
fn find_outside_strings(s: &str, pattern: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    let mut i = 0;
    while i < len {
        let ch = bytes[i];

        // check for string delimiters
        if ch == b'"' || ch == b'\'' || ch == b'`' {
            let delimiter = ch;
            i += 1;

            // skip until closing delimiter (handling escapes)
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2; // skip escape sequence
                } else if bytes[i] == delimiter {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // check for pattern match
        if i + pattern_len <= len && &bytes[i..i + pattern_len] == pattern_bytes {
            return Some(i);
        }

        i += 1;
    }

    None
}

/// Classify a line with hash-style comments (#).
fn classify_hash_style(trimmed: &str) -> LineType {
    // check for shebang as code (it's executable)
    if trimmed.starts_with("#!") {
        return LineType::Code;
    }

    if trimmed.starts_with('#') {
        return LineType::Comment;
    }

    // check for inline comment
    if let Some(hash_pos) = trimmed.find('#') {
        // make sure it's not inside a string (simple heuristic: count quotes before)
        let before = &trimmed[..hash_pos];
        let single_quotes = before.matches('\'').count();
        let double_quotes = before.matches('"').count();
        // if we have an odd number of either quote type, the # is likely in a string
        if single_quotes.is_multiple_of(2) && double_quotes.is_multiple_of(2) {
            // there's code before the comment
            return LineType::Code;
        }
    }

    LineType::Code
}

/// Classify a line with dash-style comments (-- and --[[ ]]).
fn classify_dash_style(trimmed: &str, in_block: &mut bool) -> LineType {
    // lua block comments: --[[ ]]
    if *in_block {
        if trimmed.contains("]]") {
            *in_block = false;
            let end_pos = trimmed.find("]]").unwrap();
            let after = trimmed[end_pos + 2..].trim();
            if after.is_empty() {
                return LineType::Comment;
            }
            return classify_dash_style(after, in_block);
        }
        return LineType::Comment;
    }

    // check for block comment start
    if trimmed.starts_with("--[[") {
        *in_block = true;
        if trimmed.contains("]]") {
            *in_block = false;
        }
        return LineType::Comment;
    }

    // single-line dash comment
    if trimmed.starts_with("--") {
        return LineType::Comment;
    }

    LineType::Code
}

/// Classify a line with HTML-style comments (<!-- -->).
fn classify_html_style(trimmed: &str, in_block: &mut bool) -> LineType {
    if *in_block {
        if let Some(end_pos) = trimmed.find("-->") {
            *in_block = false;
            let after = trimmed[end_pos + 3..].trim();
            if after.is_empty() {
                return LineType::Comment;
            }
            return classify_html_style(after, in_block);
        }
        return LineType::Comment;
    }

    if let Some(start_pos) = trimmed.find("<!--") {
        let before = trimmed[..start_pos].trim();

        // check if comment ends on same line
        if let Some(end_pos) = trimmed[start_pos + 4..].find("-->") {
            let after = trimmed[start_pos + 4 + end_pos + 3..].trim();
            if before.is_empty() && after.is_empty() {
                return LineType::Comment;
            }
            if before.is_empty() {
                return classify_html_style(after, in_block);
            }
            return LineType::Code;
        }

        *in_block = true;
        if before.is_empty() {
            return LineType::Comment;
        }
        return LineType::Code;
    }

    LineType::Code
}

/// Calculate the maximum directory column width needed for the tree.
///
/// # Arguments
/// * `node` - Directory node to analyze
/// * `prefix_len` - Length of the current tree prefix
/// * `depth` - Current depth in the tree
/// * `max_depth` - Maximum depth to consider
///
/// # Returns
/// Maximum width needed for the directory column
fn calc_max_dir_width(node: &DirNode, prefix_len: usize, depth: usize, max_depth: usize) -> usize {
    // connector is 4 chars ("├── " or "└── "), except at root
    let connector_len = if depth == 0 { 0 } else { 4 };
    let this_width = prefix_len + connector_len + node.name.len();

    if depth >= max_depth {
        return this_width;
    }

    // check children
    let child_prefix_len = if depth == 0 { 0 } else { prefix_len + 4 };
    let mut max_width = this_width;
    for child in node.children.values() {
        let child_width = calc_max_dir_width(child, child_prefix_len, depth + 1, max_depth);
        max_width = max_width.max(child_width);
    }

    // extension breakdown uses same indentation as child directories (4-char connector)
    if !node.by_extension.is_empty() && depth < max_depth {
        let ext_prefix_len = if depth == 0 { 0 } else { prefix_len + 4 };
        for ext in node.by_extension.keys() {
            let ext_width = ext_prefix_len + 4 + ext.len(); // "├── " + ext
            max_width = max_width.max(ext_width);
        }
    }

    max_width
}

/// Render the stats tree as a colored string.
///
/// # Arguments
/// * `tree` - Root directory node to render
/// * `max_depth` - Maximum depth to render
/// * `by_type` - Whether to show file type breakdowns
/// * `show_tokens` - Whether to show token counts
///
/// # Returns
/// Formatted string representation of the tree
fn render_tree(
    tree: &DirNode,
    max_depth: usize,
    by_type: bool,
    token_count_mode: TokenCountMode,
) -> String {
    let mut output = String::new();
    let show_tokens = token_count_mode.shows_tokens();

    // calculate dynamic column width (minimum 40, add 2 for padding)
    let dir_width = calc_max_dir_width(tree, 0, 0, max_depth).max(40) + 2;

    // calculate total width for separators
    // columns: Files(8) + Lines(10) + Code(8) + Comment(8) + Blank(8) + Bytes(10) + [Tokens(10)] + L/F(5) + B/L(5) + [T/L(5)]
    let sep_width = if show_tokens {
        dir_width + 2 + 8 + 2 + 10 + 2 + 8 + 2 + 8 + 2 + 8 + 2 + 10 + 2 + 10 + 2 + 5 + 2 + 5 + 2 + 5
    } else {
        dir_width + 2 + 8 + 2 + 10 + 2 + 8 + 2 + 8 + 2 + 8 + 2 + 10 + 2 + 5 + 2 + 5
    };

    // header
    output.push_str(&render_header(dir_width, token_count_mode));
    output.push('\n');

    // separator
    output.push_str(&dim(&"─".repeat(sep_width)));
    output.push('\n');

    // render tree recursively
    render_node(
        &mut output,
        tree,
        "",
        true,
        0,
        max_depth,
        by_type,
        token_count_mode,
        dir_width,
    );

    // footer separator
    output.push_str(&dim(&"─".repeat(sep_width)));
    output.push('\n');

    // totals row
    output.push_str(&render_totals(&tree.stats, dir_width, token_count_mode));

    // summary section with averages
    output.push('\n');
    output.push_str(&render_summary(&tree.stats, token_count_mode));

    output
}

/// Render the header row.
///
/// # Arguments
/// * `dir_width` - Width of the directory column
/// * `show_tokens` - Whether to include token columns
///
/// # Returns
/// Formatted header string
fn render_header(dir_width: usize, token_count_mode: TokenCountMode) -> String {
    let dir_header = bold("Directory");
    let dir_pad = dir_width.saturating_sub(9); // "Directory" is 9 chars
    let padded_dir = format!("{dir_header}{}", " ".repeat(dir_pad));
    let show_tokens = token_count_mode.shows_tokens();
    let tokens_header = if token_count_mode.is_estimated() {
        "Tokens~"
    } else {
        "Tokens"
    };
    let tl_header = if token_count_mode.is_estimated() {
        "T/L~"
    } else {
        "T/L"
    };

    if show_tokens {
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            padded_dir,
            dim(&format!("{:>8}", "Files")),
            dim(&format!("{:>10}", "Lines")),
            dim(&format!("{:>8}", "Code")),
            dim(&format!("{:>8}", "Comment")),
            dim(&format!("{:>8}", "Blank")),
            dim(&format!("{:>10}", "Bytes")),
            dim(&format!("{tokens_header:>10}")),
            dim(&format!("{:>5}", "L/F")),
            dim(&format!("{:>5}", "B/L")),
            dim(&format!("{tl_header:>5}"))
        )
    } else {
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}",
            padded_dir,
            dim(&format!("{:>8}", "Files")),
            dim(&format!("{:>10}", "Lines")),
            dim(&format!("{:>8}", "Code")),
            dim(&format!("{:>8}", "Comment")),
            dim(&format!("{:>8}", "Blank")),
            dim(&format!("{:>10}", "Bytes")),
            dim(&format!("{:>5}", "L/F")),
            dim(&format!("{:>5}", "B/L"))
        )
    }
}

/// Render totals row.
///
/// # Arguments
/// * `stats` - Statistics to display
/// * `dir_width` - Width of the directory column
/// * `show_tokens` - Whether to include token columns
///
/// # Returns
/// Formatted totals string
fn render_totals(stats: &Stats, dir_width: usize, token_count_mode: TokenCountMode) -> String {
    let label = bold("Total");
    let label_pad = dir_width.saturating_sub(5); // "Total" is 5 chars
    let padded_label = format!("{label}{}", " ".repeat(label_pad));
    let show_tokens = token_count_mode.shows_tokens();

    let files = format_number(stats.files);
    let lines = format_number(stats.lines);
    let lines_code = format_number(stats.lines_code);
    let lines_comment = format_number(stats.lines_comment);
    let lines_blank = format_number(stats.lines_blank);
    let bytes = format_number(stats.bytes);
    let lf = format_ratio(stats.lines, stats.files);
    let bl = format_ratio(stats.bytes, stats.lines);

    if show_tokens {
        let tokens = format_number(stats.tokens);
        let tl = format_ratio(stats.tokens, stats.lines);
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}\n",
            padded_label,
            style(&format!("{files:>8}"), &["1", "36"]),
            style(&format!("{lines:>10}"), &["1", "33"]),
            style(&format!("{lines_code:>8}"), &["1", "33"]),
            style(&format!("{lines_comment:>8}"), &["1", "90"]),
            style(&format!("{lines_blank:>8}"), &["1", "90"]),
            style(&format!("{bytes:>10}"), &["1", "32"]),
            style(&format!("{tokens:>10}"), &["1", "35"]),
            dim(&format!("{lf:>5}")),
            dim(&format!("{bl:>5}")),
            dim(&format!("{tl:>5}"))
        )
    } else {
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}\n",
            padded_label,
            style(&format!("{files:>8}"), &["1", "36"]),
            style(&format!("{lines:>10}"), &["1", "33"]),
            style(&format!("{lines_code:>8}"), &["1", "33"]),
            style(&format!("{lines_comment:>8}"), &["1", "90"]),
            style(&format!("{lines_blank:>8}"), &["1", "90"]),
            style(&format!("{bytes:>10}"), &["1", "32"]),
            dim(&format!("{lf:>5}")),
            dim(&format!("{bl:>5}"))
        )
    }
}

/// Render summary section with markers and context window comparisons.
///
/// # Arguments
/// * `stats` - Statistics to summarize
/// * `show_tokens` - Whether tokens are available
///
/// # Returns
/// Formatted summary string
fn render_summary(stats: &Stats, token_count_mode: TokenCountMode) -> String {
    let mut output = String::new();
    let show_tokens = token_count_mode.shows_tokens();

    // markers summary
    if stats.markers_total() > 0 {
        let marker_strs: Vec<String> = MARKERS
            .iter()
            .zip(stats.markers.iter())
            .filter(|(_, count)| **count > 0)
            .map(|((name, color_code, _), count)| {
                format!(
                    "{} {}",
                    color(&format_number(*count), color_code),
                    dim(name)
                )
            })
            .collect();

        output.push_str(&format!(
            "{} {}\n",
            dim("Markers:"),
            marker_strs.join(&dim(" · "))
        ));
    }

    // tags summary (sorted by count descending)
    if !stats.tags.is_empty() {
        let mut tag_vec: Vec<_> = stats.tags.iter().collect();
        tag_vec.sort_by(|a, b| b.1.cmp(a.1));

        let tag_strs: Vec<String> = tag_vec
            .iter()
            .map(|(tag, count)| {
                format!(
                    "{} {}",
                    color(&format_number(**count), "37"),
                    dim(&format!("#{tag}"))
                )
            })
            .collect();

        output.push_str(&format!(
            "{}    {}\n",
            dim("Tags:"),
            tag_strs.join(&dim(" · "))
        ));
    }

    // token mode
    if token_count_mode.is_estimated() {
        output.push_str(&format!(
            "{} {}\n",
            dim("Tokens:"),
            dim("estimated from sampled content")
        ));
    }

    // context window comparisons
    if show_tokens && stats.tokens > 0 {
        let tokens = stats.tokens as f64;

        let windows = [
            ("32K", 32_000.0),
            ("128K", 128_000.0),
            ("200K", 200_000.0),
            ("1M", 1_000_000.0),
            ("10M", 10_000_000.0),
        ];

        let comparisons: Vec<String> = windows
            .iter()
            .map(|(name, size)| {
                let ratio = tokens / size;
                if ratio >= 1.0 {
                    format!("{} {}", style(&format!("{ratio:.1}×"), &["35"]), dim(name))
                } else {
                    format!(
                        "{} {}",
                        style(&format!("{:.0}%", ratio * 100.0), &["32"]),
                        dim(name)
                    )
                }
            })
            .collect();

        output.push_str(&format!(
            "{} {}\n",
            dim(if token_count_mode.is_estimated() {
                "Context~:"
            } else {
                "Context:"
            }),
            comparisons.join(&dim(" · "))
        ));
    }

    output
}

/// Recursively render a directory node.
///
/// # Arguments
/// * `output` - String buffer to append output to
/// * `node` - Directory node to render
/// * `prefix` - Tree prefix for this node
/// * `is_last` - Whether this is the last child at this level
/// * `depth` - Current depth in the tree
/// * `max_depth` - Maximum depth to render
/// * `by_type` - Whether to show file type breakdowns
/// * `show_tokens` - Whether to show token counts
/// * `dir_width` - Width of the directory column
#[allow(clippy::too_many_arguments)]
fn render_node(
    output: &mut String,
    node: &DirNode,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: usize,
    by_type: bool,
    token_count_mode: TokenCountMode,
    dir_width: usize,
) {
    let show_tokens = token_count_mode.shows_tokens();
    // connector characters
    let connector = if depth == 0 {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };

    // build the directory name with tree prefix
    let dir_display = format!("{prefix}{connector}{}", color(&node.name, "1;34"));
    let dir_visible_width = visible_width(&dir_display);

    // format marker dots (right-aligned, fixed width for all marker types)
    let dots = format_marker_dots(&node.stats);
    let dots_display_width = MARKERS.len(); // fixed width slot for dots

    // pad between name and dots, then add dots at the right edge
    let pad_width = dir_width.saturating_sub(dir_visible_width + dots_display_width + 1);
    let padded_dir = format!("{dir_display}{} {dots}", " ".repeat(pad_width));

    // format numbers
    let files = format_number(node.stats.files);
    let lines = format_number(node.stats.lines);
    let lines_code = format_number(node.stats.lines_code);
    let lines_comment = format_number(node.stats.lines_comment);
    let lines_blank = format_number(node.stats.lines_blank);
    let bytes = format_number(node.stats.bytes);
    let lf = format_ratio(node.stats.lines, node.stats.files);
    let bl = format_ratio(node.stats.bytes, node.stats.lines);

    // render the row - pad numbers BEFORE applying color
    if show_tokens {
        let tokens = format_number(node.stats.tokens);
        let tl = format_ratio(node.stats.tokens, node.stats.lines);
        output.push_str(&format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}\n",
            padded_dir,
            color(&format!("{files:>8}"), "36"),
            color(&format!("{lines:>10}"), "33"),
            color(&format!("{lines_code:>8}"), "33"),
            dim(&format!("{lines_comment:>8}")),
            dim(&format!("{lines_blank:>8}")),
            color(&format!("{bytes:>10}"), "32"),
            color(&format!("{tokens:>10}"), "35"),
            dim(&format!("{lf:>5}")),
            dim(&format!("{bl:>5}")),
            dim(&format!("{tl:>5}"))
        ));
    } else {
        output.push_str(&format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}\n",
            padded_dir,
            color(&format!("{files:>8}"), "36"),
            color(&format!("{lines:>10}"), "33"),
            color(&format!("{lines_code:>8}"), "33"),
            dim(&format!("{lines_comment:>8}")),
            dim(&format!("{lines_blank:>8}")),
            color(&format!("{bytes:>10}"), "32"),
            dim(&format!("{lf:>5}")),
            dim(&format!("{bl:>5}"))
        ));
    }

    // render file type breakdown if requested
    if by_type && !node.by_extension.is_empty() && depth < max_depth {
        render_extension_breakdown(
            output,
            node,
            prefix,
            is_last,
            depth,
            max_depth,
            token_count_mode,
            dir_width,
        );
    }

    // stop if we've reached max depth
    if depth >= max_depth {
        return;
    }

    // render children
    let children: Vec<_> = node.children.values().collect();
    let child_count = children.len();

    // sort children by lines (descending) for better visibility of large directories
    let mut sorted_children = children;
    sorted_children.sort_by_key(|child| std::cmp::Reverse(child.stats.lines));

    for (idx, child) in sorted_children.iter().enumerate() {
        let child_is_last = idx + 1 == child_count;
        let child_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{prefix}{}", if is_last { "    " } else { "│   " })
        };

        render_node(
            output,
            child,
            &child_prefix,
            child_is_last,
            depth + 1,
            max_depth,
            by_type,
            token_count_mode,
            dir_width,
        );
    }
}

/// Render file type breakdown for a directory.
///
/// # Arguments
/// * `output` - String buffer to append output to
/// * `node` - Directory node containing extension stats
/// * `prefix` - Tree prefix for this node
/// * `is_last` - Whether the parent node is the last at its level
/// * `depth` - Current depth in the tree
/// * `max_depth` - Maximum depth to render
/// * `show_tokens` - Whether to show token counts
/// * `dir_width` - Width of the directory column
#[allow(clippy::too_many_arguments)]
fn render_extension_breakdown(
    output: &mut String,
    node: &DirNode,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: usize,
    token_count_mode: TokenCountMode,
    dir_width: usize,
) {
    let show_tokens = token_count_mode.shows_tokens();
    // sort extensions by lines (descending)
    let mut extensions: Vec<_> = node.by_extension.iter().collect();
    extensions.sort_by_key(|extension| std::cmp::Reverse(extension.1.lines));

    // extensions are indented under their parent directory (same as child directories)
    let ext_prefix = if depth == 0 {
        String::new()
    } else {
        format!("{prefix}{}", if is_last { "    " } else { "│   " })
    };

    // check if there are children that will be rendered after extensions
    let has_children = !node.children.is_empty() && depth < max_depth;
    let ext_count = extensions.len();

    for (idx, (ext, stats)) in extensions.iter().enumerate() {
        let is_last_ext = idx + 1 == ext_count && !has_children;
        let ext_connector = if is_last_ext {
            "└── "
        } else {
            "├── "
        };

        // dim extension display
        let ext_display = format!("{ext_prefix}{ext_connector}{}", dim(ext));
        let ext_visible_width = visible_width(&ext_display);
        let pad_width = dir_width.saturating_sub(ext_visible_width);
        let padded_ext = format!("{ext_display}{}", " ".repeat(pad_width));

        let files = format_number(stats.files);
        let lines = format_number(stats.lines);
        let lines_code = format_number(stats.lines_code);
        let lines_comment = format_number(stats.lines_comment);
        let lines_blank = format_number(stats.lines_blank);
        let bytes = format_number(stats.bytes);
        let lf = format_ratio(stats.lines, stats.files);
        let bl = format_ratio(stats.bytes, stats.lines);

        if show_tokens {
            let tokens = format_number(stats.tokens);
            let tl = format_ratio(stats.tokens, stats.lines);
            output.push_str(&format!(
                "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}\n",
                padded_ext,
                dim(&format!("{files:>8}")),
                dim(&format!("{lines:>10}")),
                dim(&format!("{lines_code:>8}")),
                dim(&format!("{lines_comment:>8}")),
                dim(&format!("{lines_blank:>8}")),
                dim(&format!("{bytes:>10}")),
                dim(&format!("{tokens:>10}")),
                dim(&format!("{lf:>5}")),
                dim(&format!("{bl:>5}")),
                dim(&format!("{tl:>5}"))
            ));
        } else {
            output.push_str(&format!(
                "{}  {}  {}  {}  {}  {}  {}  {}  {}\n",
                padded_ext,
                dim(&format!("{files:>8}")),
                dim(&format!("{lines:>10}")),
                dim(&format!("{lines_code:>8}")),
                dim(&format!("{lines_comment:>8}")),
                dim(&format!("{lines_blank:>8}")),
                dim(&format!("{bytes:>10}")),
                dim(&format!("{lf:>5}")),
                dim(&format!("{bl:>5}"))
            ));
        }
    }
}

/// Format a number with thousand separators.
///
/// # Arguments
/// * `n` - Number to format
///
/// # Returns
/// Formatted number string with commas as thousand separators
fn format_number(n: usize) -> String {
    if n < 1000 {
        return n.to_string();
    }

    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<_> = s.chars().collect();
    let len = chars.len();

    for (idx, ch) in chars.iter().enumerate() {
        if idx > 0 && (len - idx) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    result
}

/// Format marker dots for a stats entry.
///
/// Returns colored dots indicating which marker types are present.
/// Always outputs fixed width (one char per marker type) for alignment.
fn format_marker_dots(stats: &Stats) -> String {
    let mut dots = String::new();
    for (i, (_, color_code, dot)) in MARKERS.iter().enumerate() {
        if stats.has_marker(i) {
            dots.push_str(&color(&dot.to_string(), color_code));
        } else {
            dots.push(' ');
        }
    }
    dots
}

/// Format a ratio (numerator / denominator) as a short string.
///
/// # Arguments
/// * `numerator` - Numerator value
/// * `denominator` - Denominator value
///
/// # Returns
/// Formatted ratio string with appropriate precision
fn format_ratio(numerator: usize, denominator: usize) -> String {
    if denominator == 0 {
        return "-".to_string();
    }
    let ratio = numerator as f64 / denominator as f64;
    if ratio >= 100.0 {
        format!("{ratio:.0}")
    } else if ratio >= 10.0 {
        format!("{ratio:.1}")
    } else {
        format!("{ratio:.2}")
    }
}
