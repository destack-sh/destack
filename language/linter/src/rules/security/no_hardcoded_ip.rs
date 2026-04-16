use crate::LintMeta;
use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Disallow hardcoded IP addresses.
    ///
    /// Hardcoded IP addresses make code less portable and harder to configure.
    /// They can also be a security risk if they expose internal network topology.
    /// Use configuration, environment variables, or DNS names instead.
    ///
    /// ## Allowed IPs
    /// - `127.0.0.1` and `::1` (localhost)
    /// - `0.0.0.0` (bind to all interfaces)
    /// - `255.255.255.255` (broadcast)
    /// - Documentation IPs: `192.0.2.*`, `198.51.100.*`, `203.0.113.*`, `2001:db8::/32`
    #[lint(
        id = "no-hardcoded-ip",
        code = "LS002",
        category = Security,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoHardcodedIp,
    "Disallow hardcoded IP addresses"
}

impl LintRule for NoHardcodedIp {
    fn meta(&self) -> &'static LintMeta {
        NoHardcodedIp::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check string literals
            let expression = ctx.tree.get(node_id);
            let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = expression else {
                continue;
            };
            let string_value = ctx.strings.get(*string_id);
            let string_str = string_value.as_ref();

            // check for IPv4 addresses
            if let Some(ip) = find_ipv4_address(string_str)
                && !is_allowed_ipv4(&ip)
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_HARDCODED_IP.id,
                        NO_HARDCODED_IP.code,
                        NO_HARDCODED_IP.category,
                        severity,
                        format!("hardcoded IP address: {ip}"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use configuration or DNS instead"),
                );
            }

            // check for IPv6 addresses
            if let Some(ip) = find_ipv6_address(string_str)
                && !is_allowed_ipv6(&ip)
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_HARDCODED_IP.id,
                        NO_HARDCODED_IP.code,
                        NO_HARDCODED_IP.category,
                        severity,
                        format!("hardcoded IP address: {ip}"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use configuration or DNS instead"),
                );
            }
        }
    }
}

/// Find an IPv4 address in a string.
fn find_ipv4_address(s: &str) -> Option<String> {
    // simple regex-free pattern matching for IPv4
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if let Some((ip, end)) = try_parse_ipv4_at(bytes, i) {
            // make sure it's not part of a larger number/identifier
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'.';
            let after_ok =
                end >= bytes.len() || !bytes[end].is_ascii_alphanumeric() && bytes[end] != b'.';

            if before_ok && after_ok {
                return Some(ip);
            }
        }
        i += 1;
    }
    None
}

/// Try to parse an IPv4 address starting at position `start`.
fn try_parse_ipv4_at(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut parts = Vec::new();
    for _ in 0..4 {
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return None;
        }

        let num_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }

        let num_str = std::str::from_utf8(&bytes[num_start..i]).ok()?;
        let num: u32 = num_str.parse().ok()?;
        if num > 255 {
            return None;
        }
        // check for leading zeros (not valid in standard notation, but 0 alone is ok)
        if num_str.len() > 1 && num_str.starts_with('0') {
            return None;
        }
        parts.push(num.to_string());

        if parts.len() < 4 {
            if i >= bytes.len() || bytes[i] != b'.' {
                return None;
            }
            i += 1;
        }
    }

    if parts.len() == 4 {
        Some((parts.join("."), i))
    } else {
        None
    }
}

/// Check if an IPv4 address is allowed (common non-hardcoded IPs).
fn is_allowed_ipv4(ip: &str) -> bool {
    matches!(
        ip,
        // localhost
        "127.0.0.1" |
        // bind to all
        "0.0.0.0" |
        // broadcast
        "255.255.255.255"
    ) || ip.starts_with("192.0.2.")    // TEST-NET-1 (RFC 5737)
        || ip.starts_with("198.51.100.") // TEST-NET-2 (RFC 5737)
        || ip.starts_with("203.0.113.") // TEST-NET-3 (RFC 5737)
}

/// Find an IPv6 address in a string.
fn find_ipv6_address(s: &str) -> Option<String> {
    // look for patterns that look like IPv6: contain :: or multiple : with hex digits
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if let Some((ip, end)) = try_parse_ipv6_at(bytes, i) {
            // make sure it's not part of a larger identifier
            let before_ok = i == 0 || !is_ipv6_char(bytes[i - 1]);
            let after_ok = end >= bytes.len() || !is_ipv6_char(bytes[end]);

            if before_ok && after_ok {
                return Some(ip);
            }
        }
        i += 1;
    }
    None
}

/// Check if a byte is a valid IPv6 character.
fn is_ipv6_char(b: u8) -> bool {
    b.is_ascii_hexdigit() || b == b':'
}

/// Try to parse an IPv6 address starting at position `start`.
fn try_parse_ipv6_at(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut has_double_colon = false;
    let mut colon_count = 0;
    let mut group_count = 0;

    // check if it starts with hex digit or colon
    if i >= bytes.len() || (!bytes[i].is_ascii_hexdigit() && bytes[i] != b':') {
        return None;
    }

    let ip_start = i;

    while i < bytes.len() {
        if bytes[i] == b':' {
            if i + 1 < bytes.len() && bytes[i + 1] == b':' {
                if has_double_colon {
                    // only one :: allowed
                    break;
                }
                has_double_colon = true;
                i += 2;
                colon_count += 2;
                continue;
            }
            colon_count += 1;
            i += 1;
        } else if bytes[i].is_ascii_hexdigit() {
            let group_start = i;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() && i - group_start < 4 {
                i += 1;
            }
            group_count += 1;
        } else {
            break;
        }
    }

    // valid IPv6 needs at least 2 groups or a ::
    if group_count < 2 && !has_double_colon {
        return None;
    }

    // need at least some colons
    if colon_count < 2 {
        return None;
    }

    let ip_str = std::str::from_utf8(&bytes[ip_start..i]).ok()?;

    // don't match things like "1:2" which could be time
    // Require either :: or at least 3 colons for a proper IPv6
    if !has_double_colon && colon_count < 3 {
        return None;
    }

    Some((ip_str.to_string(), i))
}

/// Check if an IPv6 address is allowed.
fn is_allowed_ipv6(ip: &str) -> bool {
    // localhost
    ip == "::1"
        || ip == "0:0:0:0:0:0:0:1"
        || ip.starts_with("2001:db8:")
        || ip.starts_with("2001:DB8:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_ipv4_address() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_detects_ipv4_address.ds",
            r#"
let server = "192.168.1.100"
"#,
        );
        test.result(result).assert_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_detects_ipv4_in_url() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_detects_ipv4_in_url.ds",
            r#"
let url = "http://10.0.0.1:8080/api"
"#,
        );
        test.result(result).assert_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_localhost() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_localhost.ds",
            r#"
let server = "127.0.0.1"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_bind_all() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_bind_all.ds",
            r#"
let bind = "0.0.0.0"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_broadcast() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_broadcast.ds",
            r#"
let broadcast = "255.255.255.255"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_documentation_ip() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_documentation_ip.ds",
            r#"
let example = "192.0.2.1"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_normal_string() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_normal_string.ds",
            r#"
let msg = "hello world"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_version_number() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_version_number.ds",
            r#"
let version = "1.2.3"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_detects_ipv6_address() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_detects_ipv6_address.ds",
            r#"
let server = "2001:4860:4860::8888"
"#,
        );
        test.result(result).assert_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_ipv6_localhost() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_ipv6_localhost.ds",
            r#"
let localhost = "::1"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }

    #[test]
    fn test_allows_documentation_ipv6_address() {
        let test = TestProgram::for_rule_without_prelude(NoHardcodedIp);
        let result = test.lint_ast(
            "no_hardcoded_ip/test_allows_documentation_ipv6_address.ds",
            r#"
let server = "2001:db8::1"
"#,
        );
        test.result(result).assert_no_lint("no-hardcoded-ip");
    }
}
