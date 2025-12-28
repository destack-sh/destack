use destack_ast as ast;
use destack_workspace::LintSeverity;
use regex::bytes::{Regex as BytesRegex, RegexBuilder as BytesRegexBuilder};
use std::sync::LazyLock;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow hardcoded secrets and credentials.
    ///
    /// Hardcoded secrets in source code can be exposed through version control
    /// or build artifacts. Use environment variables or secret management systems.
    #[lint(
        id = "no-secrets",
        code = "LS003",
        category = Security,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSecrets,
    "Disallow hardcoded secrets"
}

/// A parsed rule from the Gitleaks configuration file.
#[derive(Debug)]
#[allow(dead_code)]
struct SecretRule {
    /// Unique identifier for the rule (e.g., "aws-access-key").
    id: String,
    /// Human-readable description of the detected secret type.
    description: String,
    /// Regex pattern to match the secret.
    regex: String,
    /// Keywords to pre-filter strings before regex matching.
    keywords: Vec<String>,
    /// Minimum entropy threshold for the match.
    entropy: Option<f64>,
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
            let entropy = rule.get("entropy").and_then(|e| e.as_float());

            Some(SecretRule {
                id,
                description,
                regex,
                keywords,
                entropy,
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

impl SecretPattern {
    /// Check if the given string matches this secret pattern.
    fn matches(&self, s: &str) -> bool {
        self.regex.is_match(s.as_bytes())
    }
}

/// Compiled secret patterns, initialized lazily at runtime.
static SECRET_PATTERNS_PATH: &str = include_str!("gitleaks.toml");
static SECRET_PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(|| {
    let rules = parse_gitleaks_config(SECRET_PATTERNS_PATH);

    // compile regex patterns, skipping any that fail to compile
    // (some gitleaks patterns may use features not supported by the regex crate)
    let mut patterns = Vec::with_capacity(rules.len());
    for rule in &rules {
        match SecretPattern::try_from(rule) {
            Ok(pattern) => patterns.push(pattern),
            Err(e) => eprintln!("warning: skipping rule '{}': {e}", rule.id),
        }
    }
    patterns
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
    let lower = name.to_lowercase();
    SUSPICIOUS_NAME_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
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

/// Entropy threshold for high-entropy strings (bits per character).
/// English text is ~4.0, random alphanumeric ~5.7, random base64 ~6.0.
const ENTROPY_THRESHOLD: f64 = 4.5;

/// Minimum string length for suspicious name detection.
const MIN_SUSPICIOUS_VALUE_LENGTH: usize = 8;

/// Result of checking a string for secret patterns.
enum SecretMatch<'a> {
    /// Matched a known secret pattern (e.g., AWS key, GitHub token).
    KnownPattern(&'a str),
    /// High entropy string that looks like a random secret.
    HighEntropy,
}

/// Check if a string value looks like a hardcoded secret.
///
/// Returns `Some(SecretMatch)` if the string matches a known pattern or has
/// suspiciously high entropy, `None` otherwise.
fn detect_secret(value: &str) -> Option<SecretMatch<'_>> {
    // skip short or placeholder strings
    if value.len() < MIN_SUSPICIOUS_VALUE_LENGTH || is_placeholder(value) {
        return None;
    }

    // check against known secret patterns from gitleaks
    for pattern in SECRET_PATTERNS.iter() {
        if pattern.matches(value) {
            return Some(SecretMatch::KnownPattern(&pattern.description));
        }
    }

    if is_safe_string(value) {
        return None;
    }

    // entropy check for longer strings (catches unknown secret formats)
    if value.len() >= MIN_ENTROPY_CHECK_LENGTH {
        let entropy = calculate_entropy(value);
        if entropy >= ENTROPY_THRESHOLD {
            // only flag alphanumeric-heavy strings (looks random)
            let alnum_count = value
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .count();
            let alnum_ratio = alnum_count as f64 / value.len() as f64;
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
    let lower = s.to_lowercase();
    lower.contains("example")
        || lower.contains("placeholder")
        || lower.contains("your_")
        || lower.contains("your-")
        || lower.contains("<your")
        || lower.contains("xxx")
        || lower.contains("changeme")
        || lower.contains("fixme")
        || lower.contains("todo")
        || lower.starts_with("test")
        || lower == "password"
        || lower == "secret"
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoSecrets::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            match expression {
                // string literals: check for secrets
                ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(s)) => {
                    let string_ref = ctx.strings.get(*s);
                    let string_value: &str = &string_ref;
                    if let Some(secret_match) = detect_secret(string_value) {
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
                            LintDiagnostic::new(
                                NO_SECRETS.id,
                                NO_SECRETS.code,
                                NO_SECRETS.category,
                                severity,
                                message,
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("use environment variables instead"),
                        );
                    }
                }

                // let bindings: check for suspicious variable names with string values
                ast::Expression::Let { declarators, .. } => {
                    for declarator_id in declarators {
                        let declarator = ctx.tree.get(*declarator_id);
                        let pattern = ctx.tree.get(declarator.pattern);

                        // check if the name looks suspicious
                        let name: &str = match pattern {
                            ast::Pattern::Binding { name, .. } => &ctx.strings.get(*name),
                            _ => continue,
                        };
                        if !is_suspicious_name(name) {
                            continue;
                        }

                        // check if the value looks suspicious
                        let Some(value_id) = declarator.value else {
                            continue;
                        };
                        let value = ctx.tree.get(value_id);
                        let is_suspicious = match value {
                            ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(s)) => {
                                let string_value: &str = &ctx.strings.get(*s);
                                is_suspicious_value(string_value)
                            }
                            ast::Expression::TemplateExpression { .. } => true,
                            _ => false,
                        };

                        if is_suspicious {
                            let severity = ctx.get_effective_severity(meta, node_id);
                            if !severity.is_enabled() {
                                continue;
                            }
                            ctx.report(
                                LintDiagnostic::new(
                                    NO_SECRETS.id,
                                    NO_SECRETS.code,
                                    NO_SECRETS.category,
                                    severity,
                                    format!("possible hardcoded secret in '{name}'"),
                                    ctx.module.file_id,
                                    ctx.tree.get_span(*declarator_id),
                                )
                                .with_label("use environment variables instead"),
                            );
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    // known pattern detection tests

    #[test]
    fn test_detects_aws_access_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // AWS access key: AKIA + 16 uppercase alphanumeric chars (20 total)
        let result = test.lint_ast("test.ts", r#"const key = "AKIAIOSFODNN7REALKEY";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_github_pat() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // GitHub PAT: ghp_ + 36 alphanumeric chars
        let result = test.lint_ast(
            "test.ts",
            r#"const token = "ghp_aB1cD2eF3gH4iJ5kL6mN7oP8qR9sT0uV1wX2";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_stripe_live_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast(
            "test.ts",
            r#"const key = "sk_live_abcdefghijklmnopqrstuvwx";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_slack_token() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast(
            "test.ts",
            r#"const token = "xoxb-123456789012-1234567890123-abcdefghijklmnopqrstuvwx";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_openai_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // OpenAI: sk- + 48 alphanumeric chars
        let result = test.lint_ast(
            "test.ts",
            r#"const key = "sk-aB1cD2eF3gH4iJ5kL6mN7oP8qR9sT0uV1wX2yZ3aB4cD5eF6g";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_private_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // Gitleaks requires full key block with BEGIN, content (64+ chars), and END markers
        let result = test.lint_ast(
            "test.ts",
            r#"const key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF8PbnGy0AHB7MxszR8GxfRzPCfuK\n-----END RSA PRIVATE KEY-----";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_jwt() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast(
            "test.ts",
            r#"const token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_sendgrid_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // SendGrid: SG. + 22 chars + . + 43 chars
        let result = test.lint_ast(
            "test.ts",
            r#"const key = "SG.aB1cD2eF3gH4iJ5kL6mN7o.P8qR9sT0uV1wX2yZ3aB4cD5eF6gH7iJ8kL9mN0oP1q";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_google_api_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast(
            "test.ts",
            r#"const key = "AIzaSyDaGmWKa4JsXZ-HjGw7ISLn_3namBGewQe";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_generic_api_key_pattern() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const key = "key=aaaaaaaaaa";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_pypi_upload_token() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let suffix = "a".repeat(50);
        let token = format!("pypi-AgEIcHlwaS5vcmc{suffix}");
        let source = format!("const token = \"{token}\";");
        let result = test.lint_ast("test.ts", &source);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_vault_batch_token() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let suffix = "a".repeat(138);
        let token = format!("hvb.{suffix}");
        let source = format!("const token = \"{token}\";");
        let result = test.lint_ast("test.ts", &source);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_slack_webhook_url() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let suffix = "a".repeat(43);
        let url = format!("https://hooks.slack.com/services/{suffix}");
        let source = format!("const url = \"{url}\";");
        let result = test.lint_ast("test.ts", &source);
        test.result(result).assert_lint("no-secrets");
    }

    // suspicious name detection tests

    #[test]
    fn test_detects_hardcoded_password() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = "supersecretpassword123";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_api_key_by_name() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const apiKey = "some_long_api_key_value";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_secret() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const clientSecret = "verysecretvalue1234";"#);
        test.result(result).assert_lint("no-secrets");
    }

    // entropy detection tests

    #[test]
    fn test_detects_high_entropy_string() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // random-looking string with high entropy
        let result = test.lint_ast(
            "test.ts",
            r#"const config = "Kj8mNpQrStUvWxYz1234AbCdEfGhIjKl";"#,
        );
        test.result(result).assert_lint("no-secrets");
    }

    // false positive prevention tests

    #[test]
    fn test_allows_env_variable() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = process.env.PASSWORD;"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_short_password() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = "short";"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_placeholder_password() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = "your_password_here";"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_example_value() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const apiKey = "example_api_key_here";"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_test_key() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // Stripe test keys are generally safe
        let result = test.lint_ast(
            "test.ts",
            r#"const key = "sk_test_abcdefghijklmnopqrstuvwx";"#,
        );
        // we do flag test keys since they're still credentials
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_allows_non_secret_names() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const username = "john_doe_123";"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_urls_without_credentials() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast(
            "test.ts",
            r#"const url = "https://api.example.com/v1/users";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_file_paths() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        let result = test.lint_ast(
            "test.ts",
            r#"const path = "/etc/ssl/certs/ca-certificates.crt";"#,
        );
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_low_entropy_strings() {
        let test = TestProgram::for_rule_without_builtins(NoSecrets);
        // repetitive string has low entropy
        let result = test.lint_ast("test.ts", r#"const data = "aaaaaaaaaaaaaaaaaaaaaaaaaaaa";"#);
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
