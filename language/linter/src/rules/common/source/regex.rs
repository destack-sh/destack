use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use regex::Regex;

/// Cached compiled regex patterns for `no-fallthrough`.
static NO_FALLTHROUGH_COMMENT_PATTERN_CACHE: LazyLock<Mutex<HashMap<String, Arc<Regex>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Return a compiled comment pattern for `no-fallthrough`.
pub fn compiled_no_fallthrough_comment_pattern(pattern: Option<&str>) -> Option<Arc<Regex>> {
    let pattern = pattern?;

    let mut cache = NO_FALLTHROUGH_COMMENT_PATTERN_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(compiled_pattern) = cache.get(pattern) {
        return Some(compiled_pattern.clone());
    }

    // validated config invariant
    let compiled_pattern = Arc::new(
        Regex::new(pattern)
            .expect("validated config invariant: no-fallthrough comment pattern compiles"),
    );
    cache.insert(pattern.to_string(), compiled_pattern.clone());
    Some(compiled_pattern)
}
