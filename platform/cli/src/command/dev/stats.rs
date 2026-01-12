use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap::Args;
use tiktoken_rs::o200k_base;

use crate::console::{bold, color, dim, style, visible_width};

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

    /// Show file type breakdown per directory.
    #[arg(long, short = 't')]
    pub by_type: bool,
}

/// Statistics for a single file or aggregated directory.
#[derive(Debug, Clone, Default)]
struct Stats {
    /// Number of files
    files: usize,
    /// Total lines of code
    lines: usize,
    /// Total bytes
    bytes: usize,
    /// Total characters
    chars: usize,
    /// Total tokens (when tokenization is enabled)
    tokens: usize,
}

impl Stats {
    /// Add another Stats instance to this one.
    fn add(&mut self, other: &Stats) {
        self.files += other.files;
        self.lines += other.lines;
        self.bytes += other.bytes;
        self.chars += other.chars;
        self.tokens += other.tokens;
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

impl DirNode {
    /// Create a new directory node.
    fn new(name: String) -> Self {
        Self {
            name,
            stats: Stats::default(),
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
            eprintln!(
                "{} failed to resolve path: {error}",
                color("error:", "1;91")
            );
            return 1;
        }
    };

    // initialize tokenizer if needed
    let tokenizer = if args.no_tokens {
        None
    } else {
        match o200k_base() {
            Ok(bpe) => Some(bpe),
            Err(error) => {
                eprintln!(
                    "{} failed to initialize tokenizer: {error}",
                    color("warning:", "1;93")
                );
                None
            }
        }
    };

    // build the stats tree
    let root_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(".")
        .to_string();
    let mut tree = DirNode::new(root_name);

    // walk the directory
    let mut ignore_set = destack_source::IgnoreSet::new();
    walk_directory(&root, &root, &mut tree, &tokenizer, &mut ignore_set);

    // aggregate stats from children to parents
    tree.aggregate();

    // determine effective depth
    let max_depth = if args.full { usize::MAX } else { args.depth };

    // render the tree
    let output = render_tree(&tree, max_depth, args.by_type, tokenizer.is_some());
    println!("{output}");

    0
}

/// Recursively walk a directory and collect stats.
///
/// # Arguments
/// * `root` - Root directory path for gitignore resolution
/// * `dir` - Current directory being processed
/// * `node` - Directory node to populate with stats
/// * `tokenizer` - Optional tokenizer for token counting
/// * `ignore_set` - Gitignore rules for filtering files
fn walk_directory(
    root: &Path,
    dir: &Path,
    node: &mut DirNode,
    tokenizer: &Option<tiktoken_rs::CoreBPE>,
    ignore_set: &mut destack_source::IgnoreSet,
) {
    // load gitignore for this directory
    ignore_set.load_dir(dir);

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        // skip hidden files and directories
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }

        // check gitignore
        if ignore_set.is_ignored(root, &path, file_type.is_dir()) {
            continue;
        }

        if file_type.is_dir() {
            let child_node = node.child(name);
            walk_directory(root, &path, child_node, tokenizer, ignore_set);
        } else if file_type.is_file() {
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
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // count stats
            let lines = content.lines().count();
            let bytes = content.len();
            let chars = content.chars().count();
            let tokens = tokenizer
                .as_ref()
                .map(|t| t.encode_with_special_tokens(&content).len())
                .unwrap_or(0);

            let stats = Stats {
                files: 1,
                lines,
                bytes,
                chars,
                tokens,
            };
            node.add_file(&extension, stats);
        }
    }
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
fn render_tree(tree: &DirNode, max_depth: usize, by_type: bool, show_tokens: bool) -> String {
    let mut output = String::new();

    // calculate dynamic column width (minimum 40, add 2 for padding)
    let dir_width = calc_max_dir_width(tree, 0, 0, max_depth).max(40) + 2;

    // calculate total width for separators
    // columns: Files(8) + Lines(10) + Bytes(10) + Chars(10) + [Tokens(10)] + B/L(5) + [T/L(5)]
    let sep_width = if show_tokens {
        dir_width + 2 + 8 + 2 + 10 + 2 + 10 + 2 + 10 + 2 + 10 + 2 + 5 + 2 + 5
    } else {
        dir_width + 2 + 8 + 2 + 10 + 2 + 10 + 2 + 10 + 2 + 5
    };

    // header
    output.push_str(&render_header(dir_width, show_tokens));
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
        show_tokens,
        dir_width,
    );

    // footer separator
    output.push_str(&dim(&"─".repeat(sep_width)));
    output.push('\n');

    // totals row
    output.push_str(&render_totals(&tree.stats, dir_width, show_tokens));

    // summary section with averages
    output.push('\n');
    output.push_str(&render_summary(&tree.stats, show_tokens));

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
fn render_header(dir_width: usize, show_tokens: bool) -> String {
    let dir_header = bold("Directory");
    let dir_pad = dir_width.saturating_sub(9); // "Directory" is 9 chars
    let padded_dir = format!("{dir_header}{}", " ".repeat(dir_pad));

    if show_tokens {
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}",
            padded_dir,
            dim(&format!("{:>8}", "Files")),
            dim(&format!("{:>10}", "Lines")),
            dim(&format!("{:>10}", "Bytes")),
            dim(&format!("{:>10}", "Chars")),
            dim(&format!("{:>10}", "Tokens")),
            dim(&format!("{:>5}", "B/L")),
            dim(&format!("{:>5}", "T/L"))
        )
    } else {
        format!(
            "{}  {}  {}  {}  {}  {}",
            padded_dir,
            dim(&format!("{:>8}", "Files")),
            dim(&format!("{:>10}", "Lines")),
            dim(&format!("{:>10}", "Bytes")),
            dim(&format!("{:>10}", "Chars")),
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
fn render_totals(stats: &Stats, dir_width: usize, show_tokens: bool) -> String {
    let label = bold("Total");
    let label_pad = dir_width.saturating_sub(5); // "Total" is 5 chars
    let padded_label = format!("{label}{}", " ".repeat(label_pad));

    let files = format_number(stats.files);
    let lines = format_number(stats.lines);
    let bytes = format_number(stats.bytes);
    let chars = format_number(stats.chars);
    let bl = format_ratio(stats.bytes, stats.lines);

    if show_tokens {
        let tokens = format_number(stats.tokens);
        let tl = format_ratio(stats.tokens, stats.lines);
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}\n",
            padded_label,
            style(&format!("{:>8}", files), &["1", "36"]),
            style(&format!("{:>10}", lines), &["1", "33"]),
            style(&format!("{:>10}", bytes), &["1", "32"]),
            style(&format!("{:>10}", chars), &["1", "32"]),
            style(&format!("{:>10}", tokens), &["1", "35"]),
            dim(&format!("{:>5}", bl)),
            dim(&format!("{:>5}", tl))
        )
    } else {
        format!(
            "{}  {}  {}  {}  {}  {}\n",
            padded_label,
            style(&format!("{:>8}", files), &["1", "36"]),
            style(&format!("{:>10}", lines), &["1", "33"]),
            style(&format!("{:>10}", bytes), &["1", "32"]),
            style(&format!("{:>10}", chars), &["1", "32"]),
            dim(&format!("{:>5}", bl))
        )
    }
}

/// Render summary section with context window comparisons.
///
/// # Arguments
/// * `stats` - Statistics to summarize
/// * `show_tokens` - Whether tokens are available
///
/// # Returns
/// Formatted summary string
fn render_summary(stats: &Stats, show_tokens: bool) -> String {
    if !show_tokens || stats.tokens == 0 {
        return String::new();
    }

    let tokens = stats.tokens as f64;

    // context window sizes
    let windows = [
        ("32K", 32_000.0),
        ("128K", 128_000.0),
        ("200K", 200_000.0),
        ("1M", 1_000_000.0),
        ("10M", 10_000_000.0),
    ];

    // format each comparison as "Nx SIZE"
    let comparisons: Vec<String> = windows
        .iter()
        .map(|(name, size)| {
            let ratio = tokens / size;
            if ratio >= 1.0 {
                format!(
                    "{} {}",
                    style(&format!("{:.1}×", ratio), &["35"]),
                    dim(name)
                )
            } else {
                format!(
                    "{} {}",
                    style(&format!("{:.0}%", ratio * 100.0), &["32"]),
                    dim(name)
                )
            }
        })
        .collect();

    format!("{} {}\n", dim("Context:"), comparisons.join(&dim(" · ")))
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
fn render_node(
    output: &mut String,
    node: &DirNode,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: usize,
    by_type: bool,
    show_tokens: bool,
    dir_width: usize,
) {
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

    // pad to column width
    let pad_width = dir_width.saturating_sub(dir_visible_width);
    let padded_dir = format!("{dir_display}{}", " ".repeat(pad_width));

    // format numbers
    let files = format_number(node.stats.files);
    let lines = format_number(node.stats.lines);
    let bytes = format_number(node.stats.bytes);
    let chars = format_number(node.stats.chars);
    let bl = format_ratio(node.stats.bytes, node.stats.lines);

    // render the row - pad numbers BEFORE applying color
    if show_tokens {
        let tokens = format_number(node.stats.tokens);
        let tl = format_ratio(node.stats.tokens, node.stats.lines);
        output.push_str(&format!(
            "{}  {}  {}  {}  {}  {}  {}  {}\n",
            padded_dir,
            color(&format!("{:>8}", files), "36"),
            color(&format!("{:>10}", lines), "33"),
            color(&format!("{:>10}", bytes), "32"),
            color(&format!("{:>10}", chars), "32"),
            color(&format!("{:>10}", tokens), "35"),
            dim(&format!("{:>5}", bl)),
            dim(&format!("{:>5}", tl))
        ));
    } else {
        output.push_str(&format!(
            "{}  {}  {}  {}  {}  {}\n",
            padded_dir,
            color(&format!("{:>8}", files), "36"),
            color(&format!("{:>10}", lines), "33"),
            color(&format!("{:>10}", bytes), "32"),
            color(&format!("{:>10}", chars), "32"),
            dim(&format!("{:>5}", bl))
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
            show_tokens,
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
    sorted_children.sort_by(|a, b| b.stats.lines.cmp(&a.stats.lines));

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
            show_tokens,
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
fn render_extension_breakdown(
    output: &mut String,
    node: &DirNode,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: usize,
    show_tokens: bool,
    dir_width: usize,
) {
    // sort extensions by lines (descending)
    let mut extensions: Vec<_> = node.by_extension.iter().collect();
    extensions.sort_by(|a, b| b.1.lines.cmp(&a.1.lines));

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
        let bytes = format_number(stats.bytes);
        let chars = format_number(stats.chars);
        let bl = format_ratio(stats.bytes, stats.lines);

        if show_tokens {
            let tokens = format_number(stats.tokens);
            let tl = format_ratio(stats.tokens, stats.lines);
            output.push_str(&format!(
                "{}  {}  {}  {}  {}  {}  {}  {}\n",
                padded_ext,
                dim(&format!("{:>8}", files)),
                dim(&format!("{:>10}", lines)),
                dim(&format!("{:>10}", bytes)),
                dim(&format!("{:>10}", chars)),
                dim(&format!("{:>10}", tokens)),
                dim(&format!("{:>5}", bl)),
                dim(&format!("{:>5}", tl))
            ));
        } else {
            output.push_str(&format!(
                "{}  {}  {}  {}  {}  {}\n",
                padded_ext,
                dim(&format!("{:>8}", files)),
                dim(&format!("{:>10}", lines)),
                dim(&format!("{:>10}", bytes)),
                dim(&format!("{:>10}", chars)),
                dim(&format!("{:>5}", bl))
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
        format!("{:.0}", ratio)
    } else if ratio >= 10.0 {
        format!("{:.1}", ratio)
    } else {
        format!("{:.2}", ratio)
    }
}
