use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_global_qualified_member, expression_target_symbol};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow weak cryptographic algorithms.
    ///
    /// Algorithms like MD5, SHA1, DES, and RC4 are cryptographically broken
    /// and should not be used for security purposes.
    #[lint(
        id = "no-weak-crypto",
        code = "LS011",
        category = Security,
        level = Dir,
        requires_all = [RequireLibSymbol("crypto", &["dom"])],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoWeakCrypto,
    "Disallow weak cryptographic algorithms"
}

/// Weak hash algorithms that should not be used.
const WEAK_HASH_ALGORITHMS: &[&str] = &["md2", "md4", "md5", "sha1", "ripemd160"];

/// Weak cipher algorithms that should not be used.
const WEAK_CIPHER_ALGORITHMS: &[&str] = &[
    "des",
    "des-cbc",
    "des-ecb",
    "des3",
    "des-ede",
    "des-ede-cbc",
    "des-ede3",
    "des-ede3-cbc",
    "rc2",
    "rc2-cbc",
    "rc4",
    "bf",
    "bf-cbc",
    "bf-ecb",
    "blowfish",
];

impl LintRule for NoWeakCrypto {
    fn meta(&self) -> &'static LintMeta {
        NoWeakCrypto::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoWeakCryptoVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags weak crypto algorithm usage.
struct NoWeakCryptoVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The crypto lib symbol.
    crypto_symbol: dir::GlobalSymbolId,
    /// The `crypto` name.
    crypto_name: StringId,
    /// The `createHash` method name.
    create_hash_name: StringId,
    /// The `createHmac` method name.
    create_hmac_name: StringId,
    /// The `createCipher` method name.
    create_cipher_name: StringId,
    /// The `createCipheriv` method name.
    create_cipher_iv_name: StringId,
    /// The `createDecipher` method name.
    create_decipher_name: StringId,
    /// The `createDecipheriv` method name.
    create_decipher_iv_name: StringId,
    /// Weak hash algorithm string IDs.
    weak_hash_ids: Vec<StringId>,
    /// Weak cipher algorithm string IDs.
    weak_cipher_ids: Vec<StringId>,
    /// Global qualifier symbols for matching `globalThis.crypto`.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoWeakCryptoVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // intern method names
        let crypto_name = ctx.program.strings.intern("crypto");
        let create_hash_name = ctx.program.strings.intern("createHash");
        let create_hmac_name = ctx.program.strings.intern("createHmac");
        let create_cipher_name = ctx.program.strings.intern("createCipher");
        let create_cipher_iv_name = ctx.program.strings.intern("createCipheriv");
        let create_decipher_name = ctx.program.strings.intern("createDecipher");
        let create_decipher_iv_name = ctx.program.strings.intern("createDecipheriv");

        // resolve symbols
        let crypto_symbol = ctx.declared_lib_symbol(crypto_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

        // intern weak algorithm names
        let weak_hash_ids = WEAK_HASH_ALGORITHMS
            .iter()
            .map(|s| ctx.program.strings.intern(s))
            .collect();
        let weak_cipher_ids = WEAK_CIPHER_ALGORITHMS
            .iter()
            .map(|s| ctx.program.strings.intern(s))
            .collect();

        Self {
            ctx,
            meta,
            crypto_symbol,
            crypto_name,
            create_hash_name,
            create_hmac_name,
            create_cipher_name,
            create_cipher_iv_name,
            create_decipher_name,
            create_decipher_iv_name,
            weak_hash_ids,
            weak_cipher_ids,
            global_qualifiers,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module expression roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for weak crypto usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // need at least one argument for the algorithm
        let Some(first_arg) = arguments.first() else {
            return;
        };

        // check if this is a crypto method call
        let Some((method_name, is_cipher)) = self.get_crypto_method(left) else {
            return;
        };

        // resolve the algorithm argument
        let argument = self.ctx.tree.get(*first_arg);
        let arg_expr = self.ctx.tree.get(argument.value());

        // only check string literals
        let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(algo_id),
        } = arg_expr
        else {
            return;
        };

        // check against weak algorithms
        let weak_ids = if is_cipher {
            &self.weak_cipher_ids
        } else {
            &self.weak_hash_ids
        };
        if !weak_ids.contains(algo_id) {
            return;
        }

        // get algorithm name for the message
        let algo_name = {
            let s = self.ctx.program.strings.get(*algo_id);
            s.as_ref().to_string()
        };

        // check effective severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(expression_id);
        let kind = if is_cipher { "cipher" } else { "hash" };
        self.ctx.report(
            LintDiagnostic::new(
                NO_WEAK_CRYPTO.id,
                NO_WEAK_CRYPTO.code,
                NO_WEAK_CRYPTO.category,
                severity,
                format!("weak {kind} algorithm '{algo_name}'"),
                self.ctx.module.file_id,
                span,
            )
            .with_label(format!(
                "crypto.{method_name} uses weak algorithm '{algo_name}'"
            )),
        );
    }

    /// Return the crypto method name and whether it's a cipher method.
    fn get_crypto_method(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<(&'static str, bool)> {
        let expression = self.ctx.tree.get(expression_id);

        // must be a member expression
        let dir::Expression::Member { left, name, .. } = expression else {
            return None;
        };

        // left must be the crypto object
        if !self.is_crypto_object(*left) {
            return None;
        }

        // match hash methods
        if *name == self.create_hash_name {
            return Some(("createHash", false));
        }
        if *name == self.create_hmac_name {
            return Some(("createHmac", false));
        }

        // match cipher methods
        if *name == self.create_cipher_name {
            return Some(("createCipher", true));
        }
        if *name == self.create_cipher_iv_name {
            return Some(("createCipheriv", true));
        }
        if *name == self.create_decipher_name {
            return Some(("createDecipher", true));
        }
        if *name == self.create_decipher_iv_name {
            return Some(("createDecipheriv", true));
        }

        None
    }

    /// Return true when the expression references the crypto object.
    fn is_crypto_object(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol reference
        let target_symbol = expression_target_symbol(self.ctx.tree, expression_id);
        if target_symbol == Some(self.crypto_symbol) {
            return true;
        }

        // match globalThis.crypto
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.crypto_name,
        )
    }
}

impl NodeVisitor for NoWeakCryptoVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_call(id, *left, dynamic_arguments);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag MD5 hash algorithm.
    #[test]
    fn test_flags_md5_hash() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_flags_md5_hash.ds",
            r#"
let hash = crypto.createHash("md5");
"#,
        );
        test.result(result).assert_lint("no-weak-crypto");
    }

    /// Flag SHA1 hash algorithm.
    #[test]
    fn test_flags_sha1_hash() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_flags_sha1_hash.ds",
            r#"
let hash = crypto.createHash("sha1");
"#,
        );
        test.result(result).assert_lint("no-weak-crypto");
    }

    /// Flag DES cipher algorithm.
    #[test]
    fn test_flags_des_cipher() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_flags_des_cipher.ds",
            r#"
let key = "secret";
let cipher = crypto.createCipher("des", key);
"#,
        );
        test.result(result).assert_lint("no-weak-crypto");
    }

    /// Flag RC4 cipher algorithm.
    #[test]
    fn test_flags_rc4_cipher() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_flags_rc4_cipher.ds",
            r#"
let key = "secret";
let iv = "iv";
let cipher = crypto.createCipheriv("rc4", key, iv);
"#,
        );
        test.result(result).assert_lint("no-weak-crypto");
    }

    /// Flag MD5 HMAC algorithm.
    #[test]
    fn test_flags_md5_hmac() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_flags_md5_hmac.ds",
            r#"
let key = "secret";
let hmac = crypto.createHmac("md5", key);
"#,
        );
        test.result(result).assert_lint("no-weak-crypto");
    }

    /// Allow SHA256 hash algorithm.
    #[test]
    fn test_allows_sha256_hash() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_allows_sha256_hash.ds",
            r#"
let hash = crypto.createHash("sha256");
"#,
        );
        test.result(result).assert_no_lint("no-weak-crypto");
    }

    /// Allow AES cipher algorithm.
    #[test]
    fn test_allows_aes_cipher() {
        let test = TestProgram::for_rule_with_prelude(NoWeakCrypto);
        let result = test.lint_dir(
            "no_weak_crypto/test_allows_aes_cipher.ds",
            r#"
let key = "secret";
let iv = "iv";
let cipher = crypto.createCipheriv("aes-256-gcm", key, iv);
"#,
        );
        test.result(result).assert_no_lint("no-weak-crypto");
    }
}
