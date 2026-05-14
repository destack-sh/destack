use crate::LintMeta;
use destack_dir::{self as dir, Declaration, FunctionForm, FunctionRole, Key, Name};
use destack_workspace::{LintSeverity, ObjectShorthandMode};
use regex::Regex;

use crate::rules::common::{expression_path_segments, span_has_comment};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer object shorthand form.
    ///
    /// Require or disallow method and property shorthand form for object literals.
    #[lint(
        id = "object-shorthand",
        code = "LY027",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub ObjectShorthand,
    "Prefer object shorthand form"
}

impl LintRule for ObjectShorthand {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        ObjectShorthand::meta()
    }

    /// Check object literal properties for shorthand style.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let methods_ignore_pattern = ctx
            .options
            .style
            .object_shorthand_methods_ignore_pattern
            .as_deref()
            .and_then(|pattern| Regex::new(pattern).ok());

        let object_expression_ids: Vec<_> = ctx
            .dir
            .tree()
            .iter_nodes::<dir::Expression>()
            .filter(|expression_id| {
                matches!(
                    ctx.dir.get(*expression_id),
                    dir::Expression::ObjectExpression { .. }
                )
            })
            .collect();

        for expression_id in object_expression_ids {
            let dir::Expression::ObjectExpression { properties, .. } = ctx.dir.get(expression_id)
            else {
                continue;
            };

            check_object_expression(
                ctx,
                meta,
                expression_id,
                properties,
                methods_ignore_pattern.as_ref(),
            );
        }
    }
}

/// Classification of one property for shorthand policy checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropertyClassification {
    /// Property or method already uses shorthand.
    Shorthand,
    /// Property or method could use shorthand.
    Longform,
    /// Property does not participate in shorthand policy.
    Neutral,
}

/// Check one object expression against the configured shorthand mode.
fn check_object_expression(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
    properties: &[dir::LocalNodeId<dir::Property>],
    methods_ignore_pattern: Option<&Regex>,
) {
    match ctx.options.style.object_shorthand_mode {
        ObjectShorthandMode::Always => {
            for property_id in properties {
                report_longform_property_if_needed(
                    ctx,
                    meta,
                    *property_id,
                    true,
                    true,
                    methods_ignore_pattern,
                );
            }
        }
        ObjectShorthandMode::Methods => {
            for property_id in properties {
                report_longform_property_if_needed(
                    ctx,
                    meta,
                    *property_id,
                    true,
                    false,
                    methods_ignore_pattern,
                );
            }
        }
        ObjectShorthandMode::Properties => {
            for property_id in properties {
                report_longform_property_if_needed(
                    ctx,
                    meta,
                    *property_id,
                    false,
                    true,
                    methods_ignore_pattern,
                );
            }
        }
        ObjectShorthandMode::Never => {
            for property_id in properties {
                report_shorthand_property_if_needed(ctx, meta, *property_id);
            }
        }
        ObjectShorthandMode::Consistent => {
            if object_expression_has_mixed_shorthand(ctx, properties, methods_ignore_pattern, false)
            {
                report_object_mix(ctx, meta, expression_id);
            }
        }
        ObjectShorthandMode::ConsistentAsNeeded => {
            if object_expression_has_mixed_shorthand(ctx, properties, methods_ignore_pattern, false)
            {
                report_object_mix(ctx, meta, expression_id);
                return;
            }

            if object_expression_needs_all_shorthand(ctx, properties, methods_ignore_pattern) {
                report_object_all_shorthand(ctx, meta, expression_id);
            }
        }
    }
}

/// Report one longform property or method that should be shorthand.
fn report_longform_property_if_needed(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    property_id: dir::LocalNodeId<dir::Property>,
    include_methods: bool,
    include_properties: bool,
    methods_ignore_pattern: Option<&Regex>,
) {
    if include_properties && let Some(property_name) = redundant_property_name(ctx, property_id) {
        let severity = ctx.get_effective_severity(meta, property_id);
        if !severity.is_enabled() {
            return;
        }

        let property_span = ctx.dir.get_span(property_id);
        let mut diagnostic = LintReport::new(
            OBJECT_SHORTHAND.id,
            OBJECT_SHORTHAND.code,
            OBJECT_SHORTHAND.category,
            severity,
            format!("property `{property_name}` can use shorthand form"),
            property_span,
        )
        .label("use shorthand `{ x }` instead of `{ x: x }`");

        if ctx.compute_fixes && !span_has_comment(ctx.dir.tree(), property_span) {
            let edits = ctx
                .edit_builder()
                .replace(property_span, property_name.clone())
                .into_edits();
            let fix = LintFix::safe("Use property shorthand").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        return;
    }

    if include_methods
        && let Some(method_name) = redundant_method_name(ctx, property_id, methods_ignore_pattern)
    {
        let severity = ctx.get_effective_severity(meta, property_id);
        if !severity.is_enabled() {
            return;
        }

        let property_span = ctx.dir.get_span(property_id);
        ctx.report(
            LintReport::new(
                OBJECT_SHORTHAND.id,
                OBJECT_SHORTHAND.code,
                OBJECT_SHORTHAND.category,
                severity,
                format!("method `{method_name}` can use shorthand form"),
                property_span,
            )
            .label("use shorthand method form"),
        );
    }
}

/// Report one shorthand property or method that should be longform.
fn report_shorthand_property_if_needed(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    property_id: dir::LocalNodeId<dir::Property>,
) {
    let Some(property_name) = shorthand_name(ctx, property_id) else {
        return;
    };

    let severity = ctx.get_effective_severity(meta, property_id);
    if !severity.is_enabled() {
        return;
    }

    let property_span = ctx.dir.get_span(property_id);
    let mut diagnostic = LintReport::new(
        OBJECT_SHORTHAND.id,
        OBJECT_SHORTHAND.code,
        OBJECT_SHORTHAND.category,
        severity,
        format!("property `{property_name}` should use longform form"),
        property_span,
    )
    .label("use longform property form");

    if matches!(ctx.dir.get(property_id), dir::Property::Field { .. })
        && ctx.compute_fixes
        && !span_has_comment(ctx.dir.tree(), property_span)
    {
        let replacement = format!("{property_name}: {property_name}");
        let edits = ctx
            .edit_builder()
            .replace(property_span, replacement)
            .into_edits();
        let fix = LintFix::safe("Use longform property form").with_edits(edits);
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Report one mixed shorthand object literal.
fn report_object_mix(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
) {
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.dir.get_span(expression_id);
    ctx.report(
        LintReport::new(
            OBJECT_SHORTHAND.id,
            OBJECT_SHORTHAND.code,
            OBJECT_SHORTHAND.category,
            severity,
            "unexpected mix of shorthand and non-shorthand properties",
            span,
        )
        .label("use one shorthand style consistently within this object"),
    );
}

/// Report one object literal where every eligible property should be shorthand.
fn report_object_all_shorthand(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
) {
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.dir.get_span(expression_id);
    ctx.report(
        LintReport::new(
            OBJECT_SHORTHAND.id,
            OBJECT_SHORTHAND.code,
            OBJECT_SHORTHAND.category,
            severity,
            "expected shorthand for all properties",
            span,
        )
        .label("all eligible properties in this object can use shorthand"),
    );
}

/// Return true when the object mixes shorthand and longform members.
fn object_expression_has_mixed_shorthand(
    ctx: &LintModuleContext<'_>,
    properties: &[dir::LocalNodeId<dir::Property>],
    methods_ignore_pattern: Option<&Regex>,
    require_all_redundant: bool,
) -> bool {
    let mut has_shorthand = false;
    let mut has_longform = false;

    for property_id in properties {
        match classify_property(ctx, *property_id, methods_ignore_pattern) {
            PropertyClassification::Shorthand => has_shorthand = true,
            PropertyClassification::Longform => has_longform = true,
            PropertyClassification::Neutral => {}
        }
    }

    if has_shorthand && has_longform {
        return true;
    }

    has_longform && require_all_redundant
}

/// Return true when every shorthand-capable property is longform and reducible.
fn object_expression_needs_all_shorthand(
    ctx: &LintModuleContext<'_>,
    properties: &[dir::LocalNodeId<dir::Property>],
    methods_ignore_pattern: Option<&Regex>,
) -> bool {
    let mut saw_candidate = false;

    for property_id in properties {
        match classify_property(ctx, *property_id, methods_ignore_pattern) {
            PropertyClassification::Shorthand => return false,
            PropertyClassification::Longform => saw_candidate = true,
            PropertyClassification::Neutral => return false,
        }
    }

    saw_candidate
}

/// Classify one property for object-level shorthand policy checks.
fn classify_property(
    ctx: &LintModuleContext<'_>,
    property_id: dir::LocalNodeId<dir::Property>,
    methods_ignore_pattern: Option<&Regex>,
) -> PropertyClassification {
    if shorthand_name(ctx, property_id).is_some() {
        return PropertyClassification::Shorthand;
    }

    if redundant_property_name(ctx, property_id).is_some()
        || redundant_method_name(ctx, property_id, methods_ignore_pattern).is_some()
    {
        return PropertyClassification::Longform;
    }

    PropertyClassification::Neutral
}

/// Return one shorthand property or method name.
fn shorthand_name(
    ctx: &LintModuleContext<'_>,
    property_id: dir::LocalNodeId<dir::Property>,
) -> Option<String> {
    let property = ctx.dir.get(property_id);

    match property {
        dir::Property::Field {
            key,
            value,
            is_shorthand,
        } => {
            let property_name = property_key_shorthand_name(ctx, key)?;
            let path_segments = expression_path_segments(ctx.dir.tree(), *value)?;
            if path_segments.len() != 1 || ctx.strings.get(path_segments[0]) != property_name {
                return None;
            }

            is_shorthand.then_some(property_name)
        }
        dir::Property::Method {
            key: Some(key),
            signature,
            ..
        } if !matches!(
            signature.role,
            Some(FunctionRole::Getter | FunctionRole::Setter)
        ) =>
        {
            property_key_shorthand_name(ctx, key)
        }
        _ => None,
    }
}

/// Return one longform property name when it can be reduced to shorthand.
fn redundant_property_name(
    ctx: &LintModuleContext<'_>,
    property_id: dir::LocalNodeId<dir::Property>,
) -> Option<String> {
    let property = ctx.dir.get(property_id);
    let dir::Property::Field {
        key,
        value,
        is_shorthand,
    } = property
    else {
        return None;
    };

    let property_name = property_key_redundant_name(ctx, key)?;
    let path_segments = expression_path_segments(ctx.dir.tree(), *value)?;
    if path_segments.len() != 1 {
        return None;
    }
    if *is_shorthand {
        return None;
    }

    (ctx.strings.get(path_segments[0]) == property_name).then_some(property_name)
}

/// Return one longform method name when it can be reduced to shorthand.
fn redundant_method_name(
    ctx: &LintModuleContext<'_>,
    property_id: dir::LocalNodeId<dir::Property>,
    methods_ignore_pattern: Option<&Regex>,
) -> Option<String> {
    let property = ctx.dir.get(property_id);
    let dir::Property::Field {
        key,
        value,
        is_shorthand,
    } = property
    else {
        return None;
    };

    let method_name = property_key_redundant_name(ctx, key)?;
    if *is_shorthand {
        return None;
    }
    if ctx.options.style.object_shorthand_ignore_constructors && is_constructor_name(&method_name) {
        return None;
    }
    if methods_ignore_pattern.is_some_and(|pattern| pattern.is_match(&method_name)) {
        return None;
    }

    let dir::Expression::Declaration(declaration) = ctx.dir.get(*value) else {
        return None;
    };
    let Declaration::Function(declaration) = ctx.dir.get(*declaration) else {
        return None;
    };
    if declaration.name.is_some() {
        return None;
    }
    if matches!(
        declaration.signature.role,
        Some(FunctionRole::Getter | FunctionRole::Setter)
    ) {
        return None;
    }
    if ctx
        .options
        .style
        .object_shorthand_avoid_explicit_return_arrows
        && declaration.signature.form == FunctionForm::Lambda
        && declaration
            .body
            .is_some_and(|body_id| !matches!(ctx.dir.get(body_id), dir::Expression::Block(..)))
    {
        return None;
    }

    Some(method_name)
}

/// Return one key name string for shorthand-compatible keys.
fn property_key_shorthand_name(ctx: &LintModuleContext<'_>, key: &Key) -> Option<String> {
    match key {
        Key::Name(Name::Identifier(name)) => Some(ctx.strings.get(*name).to_string()),
        _ => None,
    }
}

/// Return one key name string for reducible longform keys.
fn property_key_redundant_name(ctx: &LintModuleContext<'_>, key: &Key) -> Option<String> {
    match key {
        Key::Name(Name::Identifier(name)) => Some(ctx.strings.get(*name).to_string()),
        Key::Name(Name::String(name)) if !ctx.options.style.object_shorthand_avoid_quotes => {
            let name = ctx.strings.get(*name).to_string();
            is_identifier_like(&name).then_some(name)
        }
        _ => None,
    }
}

/// Return true when one property name looks like a constructor.
fn is_constructor_name(name: &str) -> bool {
    let Some(first_character) = name
        .chars()
        .find(|character| !matches!(character, '_' | '$') && !character.is_ascii_digit())
    else {
        return false;
    };

    first_character.is_ascii_uppercase()
}

/// Return true when one string can be written as an identifier.
fn is_identifier_like(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first_character) = characters.next() else {
        return false;
    };

    if !(first_character == '_' || first_character == '$' || first_character.is_ascii_alphabetic())
    {
        return false;
    }

    characters
        .all(|character| character == '_' || character == '$' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag redundant property shorthand candidates.
    #[test]
    fn test_detects_redundant_property() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint(
            "object_shorthand/test_detects_redundant_property.ds",
            r#"
const x = 1
const obj = { x: x }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    /// Allow shorthand properties.
    #[test]
    fn test_allows_shorthand() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint(
            "object_shorthand/test_allows_shorthand.ds",
            r#"
const x = 1
const obj = { x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Allow unrelated property values.
    #[test]
    fn test_allows_different_names() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint(
            "object_shorthand/test_allows_different_names.ds",
            r#"
const x = 1
const obj = { y: x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Fix redundant property shorthand safely.
    #[test]
    fn test_fix_shorthand() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint(
            "object_shorthand/test_fix_shorthand.ds",
            r#"
const x = 1
const obj = { x: x }
"#,
        );
        test.result(result)
            .assert_lint("object-shorthand")
            .assert_safe_fixed(
                r#"
const x = 1;
const obj = { x };
"#,
            );
    }

    /// Avoid fixes through comment trivia.
    #[test]
    fn test_no_fix_when_property_contains_comment_trivia() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint(
            "object_shorthand/test_no_fix_when_property_contains_comment_trivia.ds",
            r#"
const x = 1
const obj = {
    x /* keep */: x
}
"#,
        );
        test.result(result)
            .assert_lint("object-shorthand")
            .assert_has_no_fix("object-shorthand");
    }

    /// Respect the quoted-key exemption.
    #[test]
    fn test_allows_quoted_key_when_avoid_quotes_is_enabled() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.style.object_shorthand_avoid_quotes = true;
        });
        let result = test.lint(
            "object_shorthand/test_allows_quoted_key_when_avoid_quotes_is_enabled.ds",
            r#"
const x = 1
const obj = { "x": x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Flag longform methods when methods mode applies.
    #[test]
    fn test_flags_longform_method() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint(
            "object_shorthand/test_flags_longform_method.ds",
            r#"
const obj = {
    foo: function() {
        return 1;
    },
}
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    /// Ignore constructors when configured.
    #[test]
    fn test_allows_constructor_method_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.style.object_shorthand_ignore_constructors = true;
        });
        let result = test.lint(
            "object_shorthand/test_allows_constructor_method_when_ignored.ds",
            r#"
const obj = {
    Foo: function() {
        return 1;
    },
}
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Flag shorthand properties in never mode.
    #[test]
    fn test_flags_shorthand_in_never_mode() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.style.object_shorthand_mode = ObjectShorthandMode::Never;
        });
        let result = test.lint(
            "object_shorthand/test_flags_shorthand_in_never_mode.ds",
            r#"
const x = 1
const obj = { x }
"#,
        );
        test.result(result)
            .assert_lint("object-shorthand")
            .assert_safe_fixed(
                r#"
const x = 1;
const obj = { x: x };
"#,
            );
    }

    /// Flag mixed shorthand in consistent mode.
    #[test]
    fn test_flags_mixed_object_in_consistent_mode() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.style.object_shorthand_mode = ObjectShorthandMode::Consistent;
        });
        let result = test.lint(
            "object_shorthand/test_flags_mixed_object_in_consistent_mode.ds",
            r#"
const x = 1
const y = 2
const obj = { x, y: y }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    /// Flag all-longform reducible objects in consistent-as-needed mode.
    #[test]
    fn test_flags_all_longform_object_in_consistent_as_needed_mode() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.style.object_shorthand_mode = ObjectShorthandMode::ConsistentAsNeeded;
        });
        let result = test.lint(
            "object_shorthand/test_flags_all_longform_object_in_consistent_as_needed_mode.ds",
            r#"
const x = 1
const y = 2
const obj = { x: x, y: y }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }
}
