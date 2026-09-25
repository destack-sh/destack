use tspp_source::{FilePatch, Patch, Span};

use crate::rules::declare_lint;
use crate::{CommentSentence, CommentSentenceEnding, DirModule, Lint, LintOutput, LintResult};

const COMMENT_LINE_WIDTH: u32 = 100;

declare_lint! {
    /// Require canonical style for comments and documentation.
    pub COMMENT_STYLE {
        id: "comment-style",
        summary: "Require canonical style for comments and documentation",
        explanation: r#"
Varying comment casing, punctuation, sentence layout, and width gives equivalent prose inconsistent source forms.
Instead, you SHOULD use the canonical comment form.

Documentation sentences begin with uppercase prose, use terminal punctuation, and separate a one-sentence summary paragraph from further prose.
Ordinary comments begin with a lowercase action or label and omit the final period when they contain one sentence.
Every sentence begins on its own physical line, ordinary sentence continuations use one additional space, and prose ends by visual column 100.
Initialisms, marked source, legal comments, Markdown, code blocks, and unbreakable tokens retain their authored form.
"#,
        example: {
            reported: r#"
/// return the active session.
function session(): int32 {
    return 1;
}
"#,
            accepted: r#"
/// Return the active session.
function session(): int32 {
    return 1;
}
"#,
        },
        provenance: [],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check comment style.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let blocks = module.comment_blocks()?;
    let mut output = LintOutput::default();

    // inspect ordinary comments and attached documentation
    for block in blocks {
        if block.is_legal() || block.is_detached_documentation() {
            continue;
        }
        let lines = block.lines()?;
        let sentences = CommentSentence::collect(&lines)?;

        // enforce canonical delimiter spacing and ordinary continuation indentation
        for line in lines.iter().copied() {
            // classify the physical comment line
            let is_continuation = !block.is_documentation()
                && line.is_line_comment()
                && sentences
                    .iter()
                    .copied()
                    .any(|sentence| sentence.continues_on(line));
            let is_structured = line.is_markdown() || line.is_indented_code();
            let Some(spacing) = line.spacing else {
                continue;
            };

            // select the spacing required by its prose role
            let expected = if line.text.is_empty() {
                ""
            } else if is_continuation {
                "  "
            } else {
                " "
            };
            let correction = if is_structured {
                (!line.has_separator()?).then(|| (Span::at(spacing.file, spacing.start, 0), " "))
            } else {
                (!line.has_spacing(expected)?).then_some((spacing, expected))
            };
            let Some((span, replacement)) = correction else {
                continue;
            };

            // report and replace the delimiter spacing
            let patch = Patch::replace(span, replacement);
            let fix = lint.fix("use canonical comment spacing", patch)?;
            let (message, help) = if is_continuation {
                (
                    "ordinary comment continuation is not indented",
                    "indent the continued sentence by one additional space",
                )
            } else {
                (
                    "comment uses non-canonical delimiter spacing",
                    "separate comment prose from its delimiter with one space",
                )
            };
            let diagnostic = lint
                .diagnostic(message, line.span)
                .help(help)
                .suggestion(fix);
            output.report(diagnostic);
        }

        // enforce casing from the block's prose role
        for (index, sentence) in sentences.iter().copied().enumerate() {
            let Some((span, character)) = sentence.initial else {
                continue;
            };

            // determine casing from documentation and annotation roles
            let expects_lowercase =
                sentence.is_annotation || (!block.is_documentation() && index == 0);
            let is_valid = if expects_lowercase {
                character.is_lowercase()
            } else {
                character.is_uppercase()
            };
            if is_valid {
                continue;
            }

            // select the diagnostic and corrected initial character
            let (message, help, replacement) = if expects_lowercase {
                (
                    "ordinary comment begins with uppercase prose",
                    "begin the comment with a lowercase action or label",
                    character.to_lowercase().collect::<String>(),
                )
            } else {
                (
                    "prose sentence begins with lowercase prose",
                    "begin the sentence with uppercase prose",
                    character.to_uppercase().collect::<String>(),
                )
            };

            // report and replace the initial character
            let patch = Patch::replace(span, replacement);
            let fix = lint.fix("use canonical comment casing", patch)?;
            let diagnostic = lint.diagnostic(message, span).help(help).suggestion(fix);
            output.report(diagnostic);
        }

        // begin every later sentence on its own physical line
        for (index, pair) in sentences.windows(2).enumerate() {
            // classify the sentence boundary
            let previous = pair[0];
            let sentence = pair[1];
            let is_summary_boundary = block.is_documentation() && index == 0;
            let is_same_line = sentence.start_line.line == previous.end_line.line;
            let has_summary_gap = sentence.start_line.line > previous.end_line.line + 1;
            if !is_same_line && (!is_summary_boundary || has_summary_gap) {
                continue;
            }

            // select the required line separation
            let (message, help) = if is_summary_boundary {
                (
                    "documentation summary is not separated from following prose",
                    "begin further documentation after an empty comment line",
                )
            } else {
                (
                    "multiple comment sentences begin on one physical line",
                    "begin each comment sentence on its own physical line",
                )
            };
            let mut diagnostic = lint
                .diagnostic(message, sentence.start_line.span)
                .help(help);

            // construct the exact line break or summary gap
            let correction = if is_same_line {
                sentence.start_line.break_before(
                    sentence.span.start,
                    usize::from(is_summary_boundary),
                    1,
                )?
            } else {
                sentence.start_line.blank_before()?
            };
            if let Some((span, replacement)) = correction {
                let patch = Patch::replace(span, replacement);
                let fix = lint.fix("use canonical comment sentence layout", patch)?;
                diagnostic = diagnostic.suggestion(fix);
            }

            output.report(diagnostic);
        }

        // enforce punctuation from the block role and sentence count
        let is_single_ordinary = !block.is_documentation() && sentences.len() == 1;
        for sentence in sentences.iter().copied() {
            // select punctuation from the sentence and block roles
            let correction = match sentence.ending {
                CommentSentenceEnding::Period(span) if is_single_ordinary => {
                    Some((span, "", "remove the final period"))
                }
                CommentSentenceEnding::Colon(_) if sentence.is_introduction => None,
                CommentSentenceEnding::Colon(span) => {
                    Some((span, ".", "end the sentence with a period"))
                }
                CommentSentenceEnding::Missing(span) if !is_single_ordinary => {
                    Some((span, ".", "end the sentence with punctuation"))
                }
                CommentSentenceEnding::Period(_)
                | CommentSentenceEnding::Question(_)
                | CommentSentenceEnding::Exclamation(_)
                | CommentSentenceEnding::Missing(_) => None,
            };
            let Some((span, replacement, help)) = correction else {
                continue;
            };

            // report and replace the sentence ending
            let patch = Patch::replace(span, replacement);
            let fix = lint.fix("use canonical comment punctuation", patch)?;
            let diagnostic = lint
                .diagnostic("comment uses non-canonical punctuation", span)
                .help(help)
                .suggestion(fix);
            output.report(diagnostic);
        }

        // enforce the standard width for authored prose
        let mut is_fenced_code = false;
        for line in lines {
            // track fenced code independently of prose width
            if line.is_code_fence() {
                is_fenced_code = !is_fenced_code;
                continue;
            }

            // exclude structured and unbreakable source lines
            if is_fenced_code
                || line.is_indented_code()
                || line.is_markdown()
                || line.width <= COMMENT_LINE_WIDTH
                || line.is_unbreakable()
            {
                continue;
            }

            // report the exact overlong physical comment line
            let mut diagnostic = lint
                .diagnostic("comment line exceeds 100 columns", line.span)
                .help("wrap comment prose before visual column 100");

            // select indentation from the comment role
            let is_continuation = !block.is_documentation()
                && sentences
                    .iter()
                    .copied()
                    .any(|sentence| sentence.continues_on(line));
            let first_spacing = if is_continuation { 2 } else { 1 };
            let continuation_spacing = if block.is_documentation() { 1 } else { 2 };

            // wrap the authored line when every word can be retained
            if let Some(replacements) =
                line.wrap(COMMENT_LINE_WIDTH, first_spacing, continuation_spacing)?
            {
                let mut file = FilePatch::new(line.span.file);
                for (span, replacement) in replacements {
                    file.replace(span, replacement);
                }
                let fix = lint.fix("wrap the comment line", file)?;
                diagnostic = diagnostic.suggestion(fix);
            }

            output.report(diagnostic);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an uppercase initialism in an ordinary comment.
    #[test]
    fn test_accepts_initialism() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// HTTP request routing
const routes = 1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept documentation beginning with an uncased numeric term.
    #[test]
    fn test_accepts_numeric_start() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// 8-bit normalized red.
const format = 1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a stylized name at the beginning of documentation.
    #[test]
    fn test_accepts_stylized_name() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// sRGB color space.
const colorSpace = 1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept protected source at the beginning of documentation.
    #[test]
    fn test_accepts_protected_source() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// `session` returns the active session.
function session(): int32 {
    return 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept canonical indentation on an ordinary sentence continuation.
    #[test]
    fn test_accepts_ordinary_sentence_continuation() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the session index from every configured source
//  before publishing the completed index
const sessions = 1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Insert the conventional separator after a line-comment delimiter.
    #[test]
    fn test_inserts_comment_separator() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
//build the session index
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment uses non-canonical delimiter spacing
 ──▶ main.tspp:1:1
  │
1 │ //build the session index
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ const sessions = 1;
  │

 = help: separate comment prose from its delimiter with one space
 = fix: use canonical comment spacing
--- a/main.tspp
+++ b/main.tspp

-   1│ //build the session index
+   1│ // build the session index
    2│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the session index
const sessions = 1;
"#,
        );
    }

    /// Insert the conventional separator before Markdown structure.
    #[test]
    fn test_inserts_documentation_separator() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
///# Sessions
function sessions(): int32 {
    return 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment uses non-canonical delimiter spacing
 ──▶ main.tspp:1:1
  │
1 │ ///# Sessions
  │ ^^^^^^^^^^^^^
2 │ function sessions(): int32 {
3 │     return 1;
  │

 = help: separate comment prose from its delimiter with one space
 = fix: use canonical comment spacing
--- a/main.tspp
+++ b/main.tspp

-   1│ ///# Sessions
+   1│ /// # Sessions
    2│ function sessions(): int32 {
"#,
        );
        session.assert_fixes(
            r#"
/// # Sessions
function sessions(): int32 {
    return 1;
}
"#,
        );
    }

    /// Indent an ordinary sentence continuation by one additional space.
    #[test]
    fn test_indents_ordinary_sentence_continuation() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the session index from every configured source
// before publishing the completed index
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: ordinary comment continuation is not indented
 ──▶ main.tspp:2:1
  │
1 │ // build the session index from every configured source
2 │ // before publishing the completed index
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ const sessions = 1;
  │

 = help: indent the continued sentence by one additional space
 = fix: use canonical comment spacing
--- a/main.tspp
+++ b/main.tspp

    1│ // build the session index from every configured source
-   2│ // before publishing the completed index
+   2│ //  before publishing the completed index
    3│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the session index from every configured source
//  before publishing the completed index
const sessions = 1;
"#,
        );
    }

    /// Begin each ordinary sentence on its own physical line.
    #[test]
    fn test_splits_ordinary_comment_sentences() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the session index. Return the completed index.
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: multiple comment sentences begin on one physical line
 ──▶ main.tspp:1:1
  │
1 │ // build the session index. Return the completed index.
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ const sessions = 1;
  │

 = help: begin each comment sentence on its own physical line
 = fix: use canonical comment sentence layout
--- a/main.tspp
+++ b/main.tspp

-   1│ // build the session index. Return the completed index.
+   1│ // build the session index.
+   2│ // Return the completed index.
    2│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the session index.
// Return the completed index.
const sessions = 1;
"#,
        );
    }

    /// Report sentence layout in a block comment without inventing a rewrite.
    #[test]
    fn test_reports_block_comment_sentences() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/* build the session index. Return the completed index. */
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: multiple comment sentences begin on one physical line
 ──▶ main.tspp:1:1
  │
1 │ /* build the session index. Return the completed index. */
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ const sessions = 1;
  │

 = help: begin each comment sentence on its own physical line
"#,
        );
    }

    /// Require uppercase prose after the first ordinary sentence.
    #[test]
    fn test_reports_lowercase_second_sentence() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the session index.
// return the completed index.
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: prose sentence begins with lowercase prose
 ──▶ main.tspp:2:4
  │
1 │ // build the session index.
2 │ // return the completed index.
  │    ^
3 │ const sessions = 1;
  │

 = help: begin the sentence with uppercase prose
 = fix: use canonical comment casing
--- a/main.tspp
+++ b/main.tspp

    1│ // build the session index.
-   2│ // return the completed index.
+   2│ // Return the completed index.
    3│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the session index.
// Return the completed index.
const sessions = 1;
"#,
        );
    }

    /// Require a lowercase annotation body.
    #[test]
    fn test_reports_uppercase_annotation_body() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// NOTE #Performance: Build this index once.
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: ordinary comment begins with uppercase prose
 ──▶ main.tspp:1:23
  │
1 │ // NOTE #Performance: Build this index once.
  │                       ^
2 │ const sessions = 1;
  │

 = help: begin the comment with a lowercase action or label
 = fix: use canonical comment casing
--- a/main.tspp
+++ b/main.tspp

-   1│ // NOTE #Performance: Build this index once.
+   1│ // NOTE #Performance: build this index once.
    2│ const sessions = 1;

warning[comment-style]: comment uses non-canonical punctuation
 ──▶ main.tspp:1:44
  │
1 │ // NOTE #Performance: Build this index once.
  │                                            ^
2 │ const sessions = 1;
  │

 = help: remove the final period
 = fix: use canonical comment punctuation
--- a/main.tspp
+++ b/main.tspp

-   1│ // NOTE #Performance: Build this index once.
+   1│ // NOTE #Performance: Build this index once
    2│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// NOTE #Performance: build this index once
const sessions = 1;
"#,
        );
    }

    /// Remove a final period from one ordinary action phrase.
    #[test]
    fn test_removes_ordinary_final_period() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the session index.
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment uses non-canonical punctuation
 ──▶ main.tspp:1:27
  │
1 │ // build the session index.
  │                           ^
2 │ const sessions = 1;
  │

 = help: remove the final period
 = fix: use canonical comment punctuation
--- a/main.tspp
+++ b/main.tspp

-   1│ // build the session index.
+   1│ // build the session index
    2│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the session index
const sessions = 1;
"#,
        );
    }

    /// Retain terminal punctuation in a multi-sentence ordinary comment.
    #[test]
    fn test_accepts_multi_sentence_comment() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the session index.
// Return the completed index.
const sessions = 1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Separate a documentation summary from following prose.
    #[test]
    fn test_separates_documentation_summary() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Return the active session.
/// The caller owns the returned value.
function session(): int32 {
    return 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: documentation summary is not separated from following prose
 ──▶ main.tspp:2:1
  │
1 │ /// Return the active session.
2 │ /// The caller owns the returned value.
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ function session(): int32 {
4 │     return 1;
  │

 = help: begin further documentation after an empty comment line
 = fix: use canonical comment sentence layout
--- a/main.tspp
+++ b/main.tspp

    1│ /// Return the active session.
+   2│ ///
    2│ /// The caller owns the returned value.
"#,
        );
        session.assert_fixes(
            r#"
/// Return the active session.
///
/// The caller owns the returned value.
function session(): int32 {
    return 1;
}
"#,
        );
    }

    /// Split and separate documentation sentences authored on one line.
    #[test]
    fn test_splits_documentation_summary() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Return the active session. The caller owns the returned value.
function session(): int32 {
    return 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: documentation summary is not separated from following prose
 ──▶ main.tspp:1:1
  │
1 │ /// Return the active session. The caller owns the returned value.
  │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function session(): int32 {
3 │     return 1;
  │

 = help: begin further documentation after an empty comment line
 = fix: use canonical comment sentence layout
--- a/main.tspp
+++ b/main.tspp

-   1│ /// Return the active session. The caller owns the returned value.
+   1│ /// Return the active session.
+   2│ ///
+   3│ /// The caller owns the returned value.
    2│ function session(): int32 {
"#,
        );
        session.assert_fixes(
            r#"
/// Return the active session.
///
/// The caller owns the returned value.
function session(): int32 {
    return 1;
}
"#,
        );
    }

    /// Accept a colon before structured documentation.
    #[test]
    fn test_accepts_introduction() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Return one of the following:
///
/// - The active session.
/// - A session error.
function session(): int32 {
    return 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an initialism period at the end of block documentation.
    #[test]
    fn test_accepts_final_initialism() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/** Accept regional date notation from the U.S. */
function parseDate(): int32 {
    return 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Require punctuation before an ordinary continuation.
    #[test]
    fn test_replaces_non_introductory_colon() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Return the active session:
///
/// The caller owns the result.
function session(): int32 {
    return 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment uses non-canonical punctuation
 ──▶ main.tspp:1:30
  │
1 │ /// Return the active session:
  │                              ^
2 │ ///
3 │ /// The caller owns the result.
  │

 = help: end the sentence with a period
 = fix: use canonical comment punctuation
--- a/main.tspp
+++ b/main.tspp

-   1│ /// Return the active session:
+   1│ /// Return the active session.
    2│ ///
"#,
        );
        session.assert_fixes(
            r#"
/// Return the active session.
///
/// The caller owns the result.
function session(): int32 {
    return 1;
}
"#,
        );
    }

    /// Report and wrap one overlong documentation line.
    #[test]
    fn test_reports_overlong_documentation() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Return the authenticated `Session<T>` after validating every configured policy for the current incoming request.
function authenticate(): int32 {
    return 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment line exceeds 100 columns
 ──▶ main.tspp:1:1
  │
1 │ ··uthenticated `Session<T>` after validating every configured policy for the current incoming request.
  │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function authenticate(): int32 {
3 │     return 1;
  │

 = help: wrap comment prose before visual column 100
 = fix: wrap the comment line
--- a/main.tspp
+++ b/main.tspp

-   1│ /// Return the authenticated `Session<T>` after validating every configured policy for the current incoming request.
+   1│ /// Return the authenticated `Session<T>` after validating every configured policy for the current
+   2│ /// incoming request.
    2│ function authenticate(): int32 {
"#,
        );
        session.assert_fixes(
            r#"
/// Return the authenticated `Session<T>` after validating every configured policy for the current
/// incoming request.
function authenticate(): int32 {
    return 1;
}
"#,
        );
    }

    /// Wrap prose around a complete Markdown link.
    #[test]
    fn test_wraps_around_markdown_link() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Return [the authenticated session](https://example.com/session) after validating every configured policy for the current request.
function authenticate(): int32 {
    return 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment line exceeds 100 columns
 ──▶ main.tspp:1:1
  │
1 │ ··sion](https://example.com/session) after validating every configured policy for the current request.
  │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ function authenticate(): int32 {
3 │     return 1;
  │

 = help: wrap comment prose before visual column 100
 = fix: wrap the comment line
--- a/main.tspp
+++ b/main.tspp

-   1│ /// Return [the authenticated session](https://example.com/session) after validating every configured policy for the current request.
+   1│ /// Return [the authenticated session](https://example.com/session) after validating every
+   2│ /// configured policy for the current request.
    2│ function authenticate(): int32 {
"#,
        );
        session.assert_fixes(
            r#"
/// Return [the authenticated session](https://example.com/session) after validating every
/// configured policy for the current request.
function authenticate(): int32 {
    return 1;
}
"#,
        );
    }

    /// Wrap an ordinary sentence with canonical continuation indentation.
    #[test]
    fn test_wraps_ordinary_comment() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the authenticated session index after validating every configured policy for the current incoming request
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: comment line exceeds 100 columns
 ──▶ main.tspp:1:1
  │
1 │ ··uthenticated session index after validating every configured policy for the current incoming request
  │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ const sessions = 1;
  │

 = help: wrap comment prose before visual column 100
 = fix: wrap the comment line
--- a/main.tspp
+++ b/main.tspp

-   1│ // build the authenticated session index after validating every configured policy for the current incoming request
+   1│ // build the authenticated session index after validating every configured policy for the current
+   2│ //  incoming request
    2│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the authenticated session index after validating every configured policy for the current
//  incoming request
const sessions = 1;
"#,
        );
    }

    /// Compose sentence splitting and line wrapping without overlapping corrections.
    #[test]
    fn test_wraps_and_splits_ordinary_comment() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// build the authenticated session index after validating every configured policy for the incoming request. Return the completed index.
const sessions = 1;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[comment-style]: multiple comment sentences begin on one physical line
 ──▶ main.tspp:1:1
  │
1 │ ··index after validating every configured policy for the incoming request. Return the completed index.
  │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ const sessions = 1;
  │

 = help: begin each comment sentence on its own physical line
 = fix: use canonical comment sentence layout
--- a/main.tspp
+++ b/main.tspp

-   1│ // build the authenticated session index after validating every configured policy for the incoming request. Return the completed index.
+   1│ // build the authenticated session index after validating every configured policy for the incoming request.
+   2│ // Return the completed index.
    2│ const sessions = 1;

warning[comment-style]: comment line exceeds 100 columns
 ──▶ main.tspp:1:1
  │
1 │ ··index after validating every configured policy for the incoming request. Return the completed index.
  │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ const sessions = 1;
  │

 = help: wrap comment prose before visual column 100
 = fix: wrap the comment line
--- a/main.tspp
+++ b/main.tspp

-   1│ // build the authenticated session index after validating every configured policy for the incoming request. Return the completed index.
+   1│ // build the authenticated session index after validating every configured policy for the incoming
+   2│ //  request. Return the completed index.
    2│ const sessions = 1;
"#,
        );
        session.assert_fixes(
            r#"
// build the authenticated session index after validating every configured policy for the incoming
//  request.
// Return the completed index.
const sessions = 1;
"#,
        );
    }

    /// Accept one unbreakable URL beyond the standard width.
    #[test]
    fn test_accepts_unbreakable_url() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
// https://example.com/one/unavoidably/long/generated/address/that/cannot/be/split/without/changing/its/value
const value = 1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a long Markdown heading whose source structure cannot wrap.
    #[test]
    fn test_accepts_markdown_heading() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// ### Authentication policy selection for incoming requests with delegated service credentials and session delegation
function authenticate(): int32 {
    return 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept long source inside a documentation code fence.
    #[test]
    fn test_accepts_fenced_code() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Run an example.
///
/// ```ts
/// const result = oneExtremelyLongIdentifierThatMustRemainExact + anotherExtremelyLongIdentifierThatMustRemainExact;
/// ```
function run(): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept long source inside an indented documentation code block.
    #[test]
    fn test_accepts_indented_code() {
        let session = TestSession::dir(
            &COMMENT_STYLE,
            r#"
/// Run an example.
///
///     const result = oneExtremelyLongIdentifierThatMustRemainExact + anotherExtremelyLongIdentifierThatMustRemainExact;
function run(): void {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
