use crate::LintMeta;
use aho_corasick::{AhoCorasick, MatchKind};
use destack_core::StringId;
use destack_dir as dir;
use destack_workspace::LintSeverity;
use indexmap::IndexMap;
use regex::bytes::{
    Regex as BytesRegex, RegexBuilder as BytesRegexBuilder, RegexSet as BytesRegexSet,
    RegexSetBuilder as BytesRegexSetBuilder,
};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow hardcoded secrets and credentials.
    ///
    /// Hardcoded secrets in source code can be exposed through version control
    /// or build artifacts. Use environment variables or secret management systems.
    #[lint(
        id = "no-secrets",
        code = "LS009",
        category = Security,
        level = Dir,
        requires_all = [],
        requires_any = [],
        declarations = Exclude,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSecrets,
    "Disallow hardcoded secrets"
}

/// A parsed rule from the Gitleaks configuration file.
#[derive(Debug)]
struct SecretRule {
    /// Unique identifier for the rule (e.g., "aws-access-key").
    id: String,
    /// Human-readable description of the detected secret type.
    description: String,
    /// Regex pattern to match the secret.
    regex: String,
    /// Keywords to pre-filter strings before regex matching.
    keywords: Vec<String>,
}

/// Parse the Gitleaks TOML configuration into a list of rules.
fn parse_gitleaks_config(content: &str) -> Vec<SecretRule> {
    let value: toml::Value = toml::from_str(content).expect("invalid gitleaks.toml syntax");
    let rules = value
        .get("rules")
        .and_then(|r| r.as_array())
        .expect("gitleaks.toml must contain a 'rules' array");
    rules
        .iter()
        .filter_map(|rule| {
            // required fields
            let id = rule.get("id")?.as_str()?.to_string();
            let description = rule.get("description")?.as_str()?.to_string();
            let regex = rule.get("regex")?.as_str()?.to_string();

            // optional fields
            let keywords = rule
                .get("keywords")
                .and_then(|k| k.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Some(SecretRule {
                id,
                description,
                regex,
                keywords,
            })
        })
        .collect()
}

/// A compiled secret pattern ready for runtime matching.
struct SecretPattern {
    /// Human-readable description shown in lint diagnostics.
    description: String,
    /// Compiled regex for efficient matching.
    regex: BytesRegex,
}

impl TryFrom<&SecretRule> for SecretPattern {
    type Error = regex::Error;

    fn try_from(rule: &SecretRule) -> Result<Self, Self::Error> {
        let regex = BytesRegexBuilder::new(&rule.regex)
            // match gitleaks patterns with ascii classes to keep regex size small
            .unicode(false)
            .build()?;
        Ok(Self {
            description: rule.description.clone(),
            regex,
        })
    }
}

/// Keyword matcher and lookup index for secret pattern prefiltering.
struct SecretKeywordIndex {
    /// Multi-pattern matcher built from unique gitleaks keywords.
    matcher: AhoCorasick,
    /// For each keyword index, the patterns that should be tested.
    pattern_indexes: Vec<Vec<usize>>,
    /// Patterns that cannot be prefiltered by keywords.
    fallback_pattern_indexes: Vec<usize>,
    /// Optional matcher for fallback patterns.
    fallback_matcher: Option<BytesRegexSet>,
}

/// Compiled secret patterns with optional keyword prefilter index.
struct SecretPatternSet {
    /// All compiled secret regex patterns.
    patterns: Vec<SecretPattern>,
    /// Keyword matcher index for fast candidate narrowing.
    keyword_index: Option<SecretKeywordIndex>,
}

/// Compiled secret patterns, initialized lazily at runtime.
static SECRET_PATTERNS_PATH: &str = include_str!("gitleaks.toml");
static SECRET_PATTERNS: LazyLock<SecretPatternSet> = LazyLock::new(|| {
    let rules = parse_gitleaks_config(SECRET_PATTERNS_PATH);

    // compile regex patterns, skipping any that fail to compile
    // (some gitleaks patterns may use features not supported by the regex crate)
    let mut patterns = Vec::with_capacity(rules.len());
    let mut keyword_to_pattern_indexes: IndexMap<String, Vec<usize>> = IndexMap::new();
    let mut fallback_pattern_indexes = Vec::new();
    let mut fallback_pattern_regexes = Vec::new();
    for rule in &rules {
        match SecretPattern::try_from(rule) {
            Ok(pattern) => {
                let pattern_index = patterns.len();
                patterns.push(pattern);

                // map each keyword to the patterns it can unlock
                let mut has_keyword = false;
                for keyword in &rule.keywords {
                    if keyword.is_empty() {
                        continue;
                    }

                    has_keyword = true;
                    let keyword = keyword.to_ascii_lowercase();
                    keyword_to_pattern_indexes
                        .entry(keyword)
                        .or_default()
                        .push(pattern_index);
                }

                // rules without keywords still need direct regex checks
                if !has_keyword {
                    fallback_pattern_indexes.push(pattern_index);
                    fallback_pattern_regexes.push(rule.regex.clone());
                }
            }
            Err(e) => eprintln!("warning: skipping rule '{}': {e}", rule.id),
        }
    }

    // build a keyword matcher so each string is scanned once before regex checks
    let keyword_index = if keyword_to_pattern_indexes.is_empty() {
        None
    } else {
        let mut keyword_patterns = Vec::with_capacity(keyword_to_pattern_indexes.len());
        let mut keyword_literals = Vec::with_capacity(keyword_to_pattern_indexes.len());

        for (keyword, pattern_indexes) in keyword_to_pattern_indexes {
            keyword_literals.push(keyword);
            keyword_patterns.push(pattern_indexes);
        }

        let fallback_matcher = if fallback_pattern_regexes.is_empty() {
            None
        } else {
            match BytesRegexSetBuilder::new(fallback_pattern_regexes)
                // keep fallback set aligned with individual regex behavior
                .unicode(false)
                .build()
            {
                Ok(matcher) => Some(matcher),
                Err(e) => {
                    eprintln!("warning: disabling no-secrets fallback regex set: {e}");
                    None
                }
            }
        };

        match AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::Standard)
            .build(keyword_literals)
        {
            Ok(matcher) => Some(SecretKeywordIndex {
                matcher,
                pattern_indexes: keyword_patterns,
                fallback_pattern_indexes,
                fallback_matcher,
            }),
            Err(e) => {
                eprintln!("warning: disabling no-secrets keyword prefilter: {e}");
                None
            }
        }
    };

    SecretPatternSet {
        patterns,
        keyword_index,
    }
});

/// Variable name patterns that suggest secrets (for context-based detection).
const SUSPICIOUS_NAME_PATTERNS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "api_key",
    "apikey",
    "api-key",
    "access_token",
    "accesstoken",
    "auth_token",
    "authtoken",
    "private_key",
    "privatekey",
    "client_secret",
    "clientsecret",
    "bearer",
    "credential",
    "encryption_key",
    "signing_key",
];

/// Check if a variable name suggests it might hold a secret.
fn is_suspicious_name(name: &str) -> bool {
    let words = ascii_identifier_words(name);
    if words.is_empty() {
        return false;
    }

    // match suspicious names on identifier words, not raw substrings
    for start in 0..words.len() {
        let mut sequence = String::new();

        for (index, word) in words[start..].iter().enumerate() {
            if index > 0 {
                sequence.push(' ');
            }
            sequence.push_str(word);

            if suspicious_name_pattern_matches(&sequence) {
                return true;
            }
        }
    }

    false
}

/// Return true when one normalized word sequence matches a suspicious name pattern.
fn suspicious_name_pattern_matches(sequence: &str) -> bool {
    SUSPICIOUS_NAME_PATTERNS
        .iter()
        .any(|pattern| normalize_suspicious_name_pattern(pattern).as_deref() == Some(sequence))
}

/// Return one suspicious name pattern normalized to identifier words.
fn normalize_suspicious_name_pattern(pattern: &str) -> Option<String> {
    let words = ascii_identifier_words(pattern);
    if words.is_empty() {
        return None;
    }

    Some(words.join(" "))
}

/// Split one ascii identifier-like string into lowercase words.
fn ascii_identifier_words(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut words = Vec::new();
    let mut word_start = None;

    for index in 0..bytes.len() {
        let byte = bytes[index];

        // end a word on separators
        if !byte.is_ascii_alphanumeric() {
            push_ascii_identifier_word(&mut words, value, word_start, index);
            word_start = None;
            continue;
        }

        // start a new word on the first identifier character
        let Some(current_start) = word_start else {
            word_start = Some(index);
            continue;
        };

        let previous = bytes[index - 1];
        let next = bytes.get(index + 1).copied();
        let starts_new_word = starts_ascii_identifier_word(previous, byte, next);
        if !starts_new_word {
            continue;
        }

        push_ascii_identifier_word(&mut words, value, Some(current_start), index);
        word_start = Some(index);
    }

    push_ascii_identifier_word(&mut words, value, word_start, bytes.len());
    words
}

/// Return true when one ascii identifier byte starts a new word.
fn starts_ascii_identifier_word(previous: u8, current: u8, next: Option<u8>) -> bool {
    // split camelCase words
    if previous.is_ascii_lowercase() && current.is_ascii_uppercase() {
        return true;
    }

    // split acronym tails like APIKey
    previous.is_ascii_uppercase()
        && current.is_ascii_uppercase()
        && next.is_some_and(|next| next.is_ascii_lowercase())
}

/// Push one lowercase identifier word when present.
fn push_ascii_identifier_word(
    words: &mut Vec<String>,
    value: &str,
    start: Option<usize>,
    end: usize,
) {
    let Some(start) = start else {
        return;
    };
    if start >= end {
        return;
    }

    words.push(value[start..end].to_ascii_lowercase());
}

/// Check if a string value is suspicious when assigned to a secret-named variable.
///
/// This is less strict than `detect_secret` since we already have context from
/// the variable name.
fn is_suspicious_value(value: &str) -> bool {
    value.len() >= MIN_SUSPICIOUS_VALUE_LENGTH && !is_placeholder(value)
}

/// Minimum string length to consider for entropy analysis.
const MIN_ENTROPY_CHECK_LENGTH: usize = 16;

/// Minimum string length for suspicious name detection.
const MIN_SUSPICIOUS_VALUE_LENGTH: usize = 8;

/// Result of checking a string for secret patterns.
#[derive(Clone, Copy)]
enum SecretMatch {
    /// Matched a known secret pattern (e.g., AWS key, GitHub token).
    KnownPattern(&'static str),
    /// High entropy string that looks like a random secret.
    HighEntropy,
}

/// Check if a string value looks like a hardcoded secret.
///
/// Returns `Some(SecretMatch)` if the string matches a known pattern or has
/// suspiciously high entropy, `None` otherwise.
fn detect_secret(value: &str, entropy_threshold: f64) -> Option<SecretMatch> {
    // skip short or placeholder strings
    if value.len() < MIN_SUSPICIOUS_VALUE_LENGTH || is_placeholder(value) {
        return None;
    }

    // check against known secret patterns from gitleaks
    let value_bytes = value.as_bytes();
    if let Some(keyword_matcher) = &SECRET_PATTERNS.keyword_index {
        let mut candidate_pattern_indexes = Vec::new();

        for keyword_match in keyword_matcher.matcher.find_overlapping_iter(value_bytes) {
            let keyword_match_index = keyword_match.pattern().as_usize();

            for pattern_index in &keyword_matcher.pattern_indexes[keyword_match_index] {
                candidate_pattern_indexes.push(*pattern_index);
            }
        }

        // dedupe candidate regex checks without allocating a full bool table
        if candidate_pattern_indexes.len() > 1 {
            candidate_pattern_indexes.sort_unstable();
            candidate_pattern_indexes.dedup();
        }

        for pattern_index in candidate_pattern_indexes {
            let pattern = &SECRET_PATTERNS.patterns[pattern_index];
            if pattern.regex.is_match(value_bytes) {
                return Some(SecretMatch::KnownPattern(&pattern.description));
            }
        }

        if let Some(fallback_matcher) = &keyword_matcher.fallback_matcher {
            if let Some(fallback_match_index) = fallback_matcher.matches(value_bytes).iter().next()
            {
                let pattern_index = keyword_matcher.fallback_pattern_indexes[fallback_match_index];
                let pattern = &SECRET_PATTERNS.patterns[pattern_index];
                return Some(SecretMatch::KnownPattern(&pattern.description));
            }
        } else {
            for pattern_index in &keyword_matcher.fallback_pattern_indexes {
                let pattern = &SECRET_PATTERNS.patterns[*pattern_index];
                if pattern.regex.is_match(value_bytes) {
                    return Some(SecretMatch::KnownPattern(&pattern.description));
                }
            }
        }
    } else {
        for pattern in &SECRET_PATTERNS.patterns {
            if pattern.regex.is_match(value_bytes) {
                return Some(SecretMatch::KnownPattern(&pattern.description));
            }
        }
    }

    if is_safe_string(value) {
        return None;
    }

    // entropy check for longer strings (catches unknown secret formats)
    if value.len() >= MIN_ENTROPY_CHECK_LENGTH {
        let entropy = calculate_entropy(value);
        if entropy >= entropy_threshold {
            // only flag alphanumeric-heavy strings (looks random)
            let mut alnum_count = 0usize;
            for byte in value_bytes {
                if byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-' {
                    alnum_count += 1;
                }
            }
            let alnum_ratio = alnum_count as f64 / value_bytes.len() as f64;
            if alnum_ratio > 0.8 {
                return Some(SecretMatch::HighEntropy);
            }
        }
    }

    None
}

/// Calculate Shannon entropy of a string (bits per character).
fn calculate_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut char_counts = [0u32; 256];
    let mut total = 0u32;

    for byte in s.bytes() {
        char_counts[byte as usize] += 1;
        total += 1;
    }

    let total_f = total as f64;
    let mut entropy = 0.0;

    for &count in &char_counts {
        if count > 0 {
            let p = count as f64 / total_f;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Check if a string looks like a placeholder or example value.
fn is_placeholder(s: &str) -> bool {
    PLACEHOLDER_MATCHER.is_match(s.as_bytes())
        || starts_with_ascii_case_insensitive(s, "test")
        || s.eq_ignore_ascii_case("password")
        || s.eq_ignore_ascii_case("secret")
}

/// Placeholder substring matcher.
static PLACEHOLDER_MATCHER: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .match_kind(MatchKind::Standard)
        .build([
            "example",
            "placeholder",
            "your_",
            "your-",
            "<your",
            "xxx",
            "changeme",
            "fixme",
            "todo",
        ])
        .expect("invalid placeholder patterns")
});

/// Return true when `value` starts with `prefix` using ascii-insensitive comparison.
fn starts_with_ascii_case_insensitive(value: &str, prefix: &str) -> bool {
    value
        .as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// Check if a string looks like a safe path or URL (no embedded secrets).
fn is_safe_string(s: &str) -> bool {
    // absolute or relative file paths
    if s.starts_with('/') || s.starts_with("./") || s.starts_with("../") {
        return true;
    }

    // URLs without credentials (but exclude webhook URLs which may contain secrets)
    if (s.starts_with("http://") || s.starts_with("https://")) && !s.contains('@') {
        if s.contains("/hooks/") || s.contains("/webhook") {
            return false;
        }
        return true;
    }

    false
}

impl LintRule for NoSecrets {
    fn meta(&self) -> &'static LintMeta {
        NoSecrets::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut secret_match_cache: HashMap<StringId, Option<SecretMatch>> = HashMap::new();
        let entropy_threshold =
            no_secrets_entropy_threshold(ctx.options().security.no_secrets_entropy_threshold);

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            match expression {
                // string literals: check for secrets
                dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(s)) => {
                    let secret_match = if let Some(secret_match) = secret_match_cache.get(s) {
                        *secret_match
                    } else {
                        let string_ref = ctx.strings.get(*s);
                        let string_value: &str = string_ref;
                        let secret_match = detect_secret(string_value, entropy_threshold);
                        secret_match_cache.insert(*s, secret_match);
                        secret_match
                    };

                    if let Some(secret_match) = secret_match {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        let message = match secret_match {
                            SecretMatch::KnownPattern(desc) => format!("possible {desc}"),
                            SecretMatch::HighEntropy => {
                                "possible hardcoded secret (high entropy string)".to_string()
                            }
                        };
                        ctx.report(
                            LintReport::new(
                                NO_SECRETS.id,
                                NO_SECRETS.code,
                                NO_SECRETS.category,
                                severity,
                                message,
                                ctx.dir.get_span(node_id),
                            )
                            .label("use environment variables instead"),
                        );
                    }
                }

                // let bindings: check for suspicious variable names with string values
                dir::Expression::Let { declarators, .. }
                | dir::Expression::Using { declarators, .. } => {
                    for declarator_id in declarators {
                        let declarator = ctx.dir.get(*declarator_id);
                        let pattern = ctx.dir.get(declarator.pattern);

                        // check if the name looks suspicious
                        let name: &str = match pattern {
                            dir::Pattern::Binding { name, .. } => ctx.strings.get(*name),
                            _ => continue,
                        };
                        if !is_suspicious_name(name) {
                            continue;
                        }

                        // check if the value looks suspicious
                        let Some(value_id) = declarator.value else {
                            continue;
                        };
                        let value = ctx.dir.get(value_id);
                        let is_suspicious = match value {
                            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(s)) => {
                                let string_value: &str = ctx.strings.get(*s);
                                is_suspicious_value(string_value)
                            }
                            dir::Expression::TemplateExpression { .. } => true,
                            _ => false,
                        };

                        if is_suspicious {
                            let severity = ctx.get_effective_severity(meta, node_id);
                            if !severity.is_enabled() {
                                continue;
                            }
                            ctx.report(
                                LintReport::new(
                                    NO_SECRETS.id,
                                    NO_SECRETS.code,
                                    NO_SECRETS.category,
                                    severity,
                                    format!("possible hardcoded secret in '{name}'"),
                                    ctx.dir.get_span(*declarator_id),
                                )
                                .label("use environment variables instead"),
                            );
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

/// Return the configured no-secrets entropy threshold.
fn no_secrets_entropy_threshold(entropy_threshold_tenths: u32) -> f64 {
    entropy_threshold_tenths as f64 / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    // known pattern detection tests

    #[test]
    fn test_detects_aws_access_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // AWS access key: AKIA + 16 uppercase alphanumeric chars (20 total)
        let result = test.lint(
            "no_secrets/test_detects_aws_access_key.ts",
            r#"const key = "AKIAIOSFODNN7REALKEY";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_github_pat() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // GitHub PAT: ghp_ + 36 alphanumeric chars
        let result = test.lint(
            "no_secrets/test_detects_github_pat.ts",
            r#"const token = "ghp_aB1cD2eF3gH4iJ5kL6mN7oP8qR9sT0uV1wX2";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_stripe_live_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_stripe_live_key.ts",
            r#"const key = "sk_live_abcdefghijklmnopqrstuvwx";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_slack_token() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_slack_token.ts",
            r#"const token = "xoxb-123456789012-1234567890123-abcdefghijklmnopqrstuvwx";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_openai_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // OpenAI: sk- + 48 alphanumeric chars
        let result = test.lint(
            "no_secrets/test_detects_openai_key.ts",
            r#"const key = "sk-aB1cD2eF3gH4iJ5kL6mN7oP8qR9sT0uV1wX2yZ3aB4cD5eF6g";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_private_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // Gitleaks requires full key block with BEGIN, content (64+ chars), and END markers
        let result = test.lint(
            "no_secrets/test_detects_private_key.ts",
            r#"const key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF8PbnGy0AHB7MxszR8GxfRzPCfuK\n-----END RSA PRIVATE KEY-----";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_jwt() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_jwt.ts",
            r#"const token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_sendgrid_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // SendGrid: SG. + 22 chars + . + 43 chars
        let result = test.lint(
            "no_secrets/test_detects_sendgrid_key.ts",
            r#"const key = "SG.aB1cD2eF3gH4iJ5kL6mN7o.P8qR9sT0uV1wX2yZ3aB4cD5eF6gH7iJ8kL9mN0oP1q";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_google_api_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_google_api_key.ts",
            r#"const key = "AIzaSyDaGmWKa4JsXZ-HjGw7ISLn_3namBGewQe";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_generic_api_key_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_generic_api_key_pattern.ts",
            r#"const key = "key=aaaaaaaaaa";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_pypi_upload_token() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let suffix = "a".repeat(50);
        let token = format!("pypi-AgEIcHlwaS5vcmc{suffix}");
        let source = format!("const token = \"{token}\";");
        let result = test.lint("no_secrets/test_detects_pypi_upload_token.ts", &source);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_vault_batch_token() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let suffix = "a".repeat(138);
        let token = format!("hvb.{suffix}");
        let source = format!("const token = \"{token}\";");
        let result = test.lint("no_secrets/test_detects_vault_batch_token.ts", &source);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_slack_webhook_url() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let suffix = "a".repeat(43);
        let url = format!("https://hooks.slack.com/services/{suffix}");
        let source = format!("const url = \"{url}\";");
        let result = test.lint("no_secrets/test_detects_slack_webhook_url.ts", &source);
        test.result(result).assert_lint("no-secrets");
    }

    // suspicious name detection tests

    #[test]
    fn test_detects_hardcoded_password() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_hardcoded_password.ts",
            r#"const password = "supersecretpassword123";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_api_key_by_name() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_hardcoded_api_key_by_name.ts",
            r#"const apiKey = "some_long_api_key_value";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_secret() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_hardcoded_secret.ts",
            r#"const clientSecret = "verysecretvalue1234";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_acronym_api_key_by_name() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_detects_hardcoded_acronym_api_key_by_name.ts",
            r#"const APIKey = "some_long_api_key_value";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    // entropy detection tests

    #[test]
    fn test_detects_high_entropy_string() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // random-looking string with high entropy
        let result = test.lint(
            "no_secrets/test_detects_high_entropy_string.ts",
            r#"const config = "Kj8mNpQrStUvWxYz1234AbCdEfGhIjKl";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_allows_high_entropy_string_with_higher_threshold() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets).with_options(|options| {
            options.security.no_secrets_entropy_threshold = 50;
        });
        let result = test.lint(
            "no_secrets/test_allows_high_entropy_string_with_higher_threshold.ts",
            r#"const config = "Kj8mNpQrStUvWxYz1234AbCdEfGhIjKl";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    // false positive prevention tests

    #[test]
    fn test_allows_env_variable() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_env_variable.ts",
            r#"const password = process.env.PASSWORD;"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_short_password() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_short_password.ts",
            r#"const password = "short";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_placeholder_password() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_placeholder_password.ts",
            r#"const password = "your_password_here";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_example_value() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_example_value.ts",
            r#"const apiKey = "example_api_key_here";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_test_key() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // Stripe test keys are generally safe
        let result = test.lint(
            "no_secrets/test_allows_test_key.ts",
            r#"const key = "sk_test_abcdefghijklmnopqrstuvwx";"#,
        );
        // we do flag test keys since they're still credentials
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_allows_non_secret_names() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_non_secret_names.ts",
            r#"const username = "john_doe_123";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_secretary_name() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_secretary_name.ts",
            r#"const secretary = "monthly_schedule";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_urls_without_credentials() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_urls_without_credentials.ts",
            r#"const url = "https://api.example.com/v1/users";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_file_paths() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        let result = test.lint(
            "no_secrets/test_allows_file_paths.ts",
            r#"const path = "/etc/ssl/certs/ca-certificates.crt";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_low_entropy_strings() {
        let test = TestProgram::for_rule_without_prelude(NoSecrets);
        // repetitive string has low entropy
        let result = test.lint(
            "no_secrets/test_allows_low_entropy_strings.ts",
            r#"const data = "aaaaaaaaaaaaaaaaaaaaaaaaaaaa";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    // entropy calculation unit tests

    #[test]
    fn test_entropy_calculation() {
        // single character repeated: entropy = 0
        assert!(calculate_entropy("aaaa") < 0.1);

        // two characters equally distributed: entropy = 1.0
        let two_char_entropy = calculate_entropy("abab");
        assert!((two_char_entropy - 1.0).abs() < 0.1);

        // random-looking string has moderate-high entropy (depends on character diversity)
        let random_entropy = calculate_entropy("Kj8mNpQrStUvWxYz");
        assert!(random_entropy > 3.5);

        // truly random base64-like string has high entropy
        let high_entropy = calculate_entropy("aB1cD2eF3gH4iJ5kL6mN7oP8qR9sT0uV");
        assert!(high_entropy > 4.0);

        // english-like text has moderate entropy
        let english_entropy = calculate_entropy("hello world this is a test");
        assert!(english_entropy > 2.0 && english_entropy < 4.5);
    }
}
