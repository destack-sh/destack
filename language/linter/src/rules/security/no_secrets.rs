use destack_ast as ast;
use destack_workspace::LintSeverity;

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
        level = Ast
    )]
    pub NoSecrets,
    "Disallow hardcoded secrets"
}

// common secret patterns
const SECRET_PATTERNS: &[(&str, &str)] = &[
    ("password", "password"),
    ("passwd", "password"),
    ("pwd", "password"),
    ("secret", "secret"),
    ("api_key", "API key"),
    ("apikey", "API key"),
    ("api-key", "API key"),
    ("access_token", "access token"),
    ("accesstoken", "access token"),
    ("auth_token", "auth token"),
    ("authtoken", "auth token"),
    ("private_key", "private key"),
    ("privatekey", "private key"),
    ("client_secret", "client secret"),
    ("clientsecret", "client secret"),
    ("bearer", "bearer token"),
];

// minimum length for a value to be considered a potential secret
const MIN_SECRET_LENGTH: usize = 8;

impl LintRule for NoSecrets {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSecrets::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // check let bindings with suspicious names and string values
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Let { declarators, .. } = expression else {
                continue;
            };

            for declarator_id in declarators {
                let declarator = ctx.tree.get(*declarator_id);

                // get the binding name
                let pattern = ctx.tree.get(declarator.pattern);
                let name = match pattern {
                    ast::Pattern::Binding { name, .. } => ctx.strings.get(*name).to_lowercase(),
                    _ => continue,
                };

                // check if name matches a secret pattern
                let secret_type = SECRET_PATTERNS
                    .iter()
                    .find(|(pattern, _)| name.contains(pattern))
                    .map(|(_, desc)| *desc);
                let Some(secret_type) = secret_type else {
                    continue;
                };

                // check if value is a non-empty string literal
                let Some(value_id) = declarator.value else {
                    continue;
                };

                let value = ctx.tree.get(value_id);
                let is_suspicious = match value {
                    ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(s)) => {
                        let string_value = ctx.strings.get(*s);
                        string_value.len() >= MIN_SECRET_LENGTH
                    }
                    ast::Expression::TemplateExpression { .. } => true,
                    _ => false,
                };

                if is_suspicious {
                    ctx.report(
                        LintDiagnostic::new(
                            NO_SECRETS.id,
                            NO_SECRETS.code,
                            NO_SECRETS.category,
                            severity,
                            format!("possible hardcoded {secret_type}"),
                            ctx.module.file_id,
                            ctx.tree.get_span(*declarator_id),
                        )
                        .with_label("use environment variables instead"),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_hardcoded_password() {
        let test = TestProgram::for_rule(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = "supersecretpassword123";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_api_key() {
        let test = TestProgram::for_rule(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const apiKey = "sk_live_abc123def456";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_detects_hardcoded_secret() {
        let test = TestProgram::for_rule(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const clientSecret = "verysecretvalue";"#);
        test.result(result).assert_lint("no-secrets");
    }

    #[test]
    fn test_allows_env_variable() {
        let test = TestProgram::for_rule(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = process.env.PASSWORD;"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_short_password() {
        let test = TestProgram::for_rule(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const password = "short";"#);
        test.result(result).assert_no_lint("no-secrets");
    }

    #[test]
    fn test_allows_non_secret_names() {
        let test = TestProgram::for_rule(NoSecrets);
        let result = test.lint_ast("test.ts", r#"const username = "john_doe_123";"#);
        test.result(result).assert_no_lint("no-secrets");
    }
}
