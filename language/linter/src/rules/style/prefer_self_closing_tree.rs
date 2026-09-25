use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer self-closing tree elements without children.
    pub PREFER_SELF_CLOSING_TREE {
        id: "prefer-self-closing-tree",
        summary: "Prefer self-closing tree elements without children",
        explanation: r#"
Separate opening and closing tags imply that a tree element contains children even when its body is empty.
Instead, you SHOULD write an element without children as one self-closing tag.
"#,
        example: {
            reported: r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    id: int32 = 0;
}

extension of Panel implements TreeBuilder {
    type Tags = { hr: {} };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    return <hr></hr>;
}
"#,
            accepted: r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    id: int32 = 0;
}

extension of Panel implements TreeBuilder {
    type Tags = { hr: {} };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}

function render(): Panel {
    return <hr />;
}
"#,
        },
        provenance: [React("self-closing-comp")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report non-fragment tree elements with an empty child list.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect paired tree elements with no children
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::TreeExpression {
            left: Some(_),
            children: Some(children),
            ..
        } = node
        else {
            continue;
        };
        if !children.is_empty() {
            continue;
        }

        // replace the paired element when no comment would be discarded
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("tree element has no children", span);
        if let Some(suggestion) = suggest_self_closing(module, lint, expression)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one self-closing tree tag.
fn suggest_self_closing(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // preserve comments outside the retained opening tag
    let opening = module.source_region(expression.into_any(), NodeSpanRegion::Opening)?;
    let extent = module.source_extent(expression.into_any())?;
    if module.has_unretained_comment(extent, &[opening])? {
        return Ok(None);
    }

    // derive the self-closing tag from the exact opening source
    let source = module.source(opening)?;
    let Some(prefix) = source.strip_suffix('>') else {
        return Err(ProviderError::internal(
            "tree opening region does not end with a closing angle bracket",
        ));
    };

    // retain the complete opening tag and replace its close
    let replacement = format!("{} />", prefix.trim_end());
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use a self-closing tree tag", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// A tree builder with one empty element and one element that accepts text.
    const BUILDER: &str = r#"
import { TreeBuilder } from "tspp:tree";

class Panel {
    id: int32 = 0;
}

extension of Panel implements TreeBuilder {
    type Tags = {
        hr: {};
        span: {};
    };

    static element<const Tag: keyof this.Tags, Children: (...unknown[],)>(
        tag: Tag,
        attributes: this.Tags[Tag],
        children: Children,
    ): Panel {
        return new Panel();
    }

    static fragment<Children: (...unknown[],)>(children: Children): Panel {
        return new Panel();
    }
}
"#;

    /// Replace a paired element with no children.
    #[test]
    fn test_replaces_empty_element() {
        let source = format!(
            r#"{BUILDER}
function render(): Panel {{
    return <hr></hr>;
}}
"#,
        );
        let session = TestSession::dir(&PREFER_SELF_CLOSING_TREE, &source);
        let expected = format!(
            r#"{BUILDER}
function render(): Panel {{
    return <hr />;
}}
"#,
        );

        session.assert_fixes(&expected);
    }

    /// Accept an element with one text child.
    #[test]
    fn test_accepts_element_with_child() {
        let source = format!(
            r#"{BUILDER}
function render(): Panel {{
    return <span>name</span>;
}}
"#,
        );
        let session = TestSession::dir(&PREFER_SELF_CLOSING_TREE, &source);

        session.assert_no_diagnostics();
    }

    /// Accept an empty fragment because fragments have no self-closing form.
    #[test]
    fn test_accepts_empty_fragment() {
        let source = format!(
            r#"{BUILDER}
function render(): Panel {{
    return <></>;
}}
"#,
        );
        let session = TestSession::dir(&PREFER_SELF_CLOSING_TREE, &source);

        session.assert_no_diagnostics();
    }

    /// Accept an empty element whose body contains an explanatory comment.
    #[test]
    fn test_accepts_commented_empty_element() {
        let source = format!(
            r#"{BUILDER}
function render(): Panel {{
    return <hr>{{/* populated by the renderer */}}</hr>;
}}
"#,
        );
        let session = TestSession::dir(&PREFER_SELF_CLOSING_TREE, &source);

        session.assert_no_diagnostics();
    }
}
