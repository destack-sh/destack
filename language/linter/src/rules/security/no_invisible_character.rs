use tspp_repository::ProviderError;
use tspp_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow invisible and misleading characters in source text.
    pub NO_INVISIBLE_CHARACTER {
        id: "no-invisible-character",
        summary: "Disallow invisible and misleading characters in source text",
        explanation: r#"
Invisible characters can conceal differences between identifiers or reorder displayed source.
Instead, you MUST remove them from syntax and comments and escape invisible characters inside literals.
"#,
        example: {
            reported: "const user\u{200c}Name = 1;",
            accepted: "const userName = 1;",
        },
        provenance: [
            Clippy("invisible_characters"),
            Eslint("no-irregular-whitespace"),
            Rustc("text_direction_codepoint_in_comment"),
            Rustc("text_direction_codepoint_in_literal"),
        ],
        category: Security,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// One prohibited invisible source character class.
enum InvisibleCharacter {
    /// A character that changes displayed text direction.
    Direction,
    /// An invisible source character.
    Invisible,
}

impl InvisibleCharacter {
    /// Classify one authored character for its source context.
    fn classify(character: char, is_literal: bool) -> Option<Self> {
        // reject directional controls in every source context
        if matches!(
            character,
            '\u{061C}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{206A}'..='\u{206F}'
        ) {
            return Some(Self::Direction);
        }

        // require inherently invisible format characters to be escaped
        if matches!(character, '\u{00AD}' | '\u{200B}' | '\u{2060}') {
            return Some(Self::Invisible);
        }

        // retain other arbitrary Unicode inside authored literal values
        if is_literal {
            return None;
        }

        // reject invisible and irregular whitespace outside literals
        matches!(
            character,
            '\u{000B}'
                | '\u{000C}'
                | '\u{0085}'
                | '\u{00A0}'
                | '\u{00AD}'
                | '\u{1680}'
                | '\u{180E}'
                | '\u{2000}'
                ..='\u{200D}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202F}'
                    | '\u{205F}'
                    | '\u{2060}'
                    | '\u{3000}'
                    | '\u{FEFF}'
        )
        .then_some(Self::Invisible)
    }

    /// Return the diagnostic noun for this character class.
    const fn noun(self) -> &'static str {
        match self {
            Self::Direction => "text-direction control",
            Self::Invisible => "invisible character",
        }
    }
}

/// Report invisible source characters at their exact authored positions.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // compare each contributing file with its parsed literal spans
    for file in module.files {
        let parsed = module.stages.parsed.file(file.id).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} is absent from parsed lint module {:?}",
                file.id, module.id
            ))
        })?;
        let mut literals = parsed
            .tokens
            .iter()
            .filter(|token| token.ty().is_literal())
            .peekable();

        // scan source and advance through the ordered literal spans once
        for (start, character) in file.text().char_indices() {
            while literals
                .peek()
                .is_some_and(|literal| literal.end() <= start as u32)
            {
                literals.next();
            }
            let is_literal = literals.peek().is_some_and(|literal| {
                literal.start() <= start as u32
                    && start as u32 + character.len_utf8() as u32 <= literal.end()
            });
            let Some(kind) = InvisibleCharacter::classify(character, is_literal) else {
                continue;
            };

            // report the exact UTF-8 span
            let end = start + character.len_utf8();
            let span = Span::new(file.id, start as u32, end as u32);
            let code = character as u32;
            let message = format!("{} U+{code:04X} appears in source", kind.noun());
            output.report(lint.diagnostic(message, span));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a text-direction control inside a comment.
    #[test]
    fn test_reports_direction_control_in_comment() {
        let session = TestSession::dir(
            &NO_INVISIBLE_CHARACTER,
            "// hidden \u{202e}\nconst enabled = true;",
        );
        let direction = '\u{202e}';
        let expected = format!(
            r#"
warning[no-invisible-character]: text-direction control U+202E appears in source
 ──▶ main.tspp:1:11
  │
1 │ // hidden {direction}
  │           ^
2 │ const enabled = true;
  │
"#,
        );

        session.assert_diagnostics(&expected);
    }

    /// Report a text-direction control inside a literal.
    #[test]
    fn test_reports_direction_control_in_literal() {
        let direction = '\u{202e}';
        let source = format!("const value = \"{direction}\";");
        let session = TestSession::dir(&NO_INVISIBLE_CHARACTER, &source);
        let expected = format!(
            r#"
warning[no-invisible-character]: text-direction control U+202E appears in source
 ──▶ main.tspp:1:16
  │
1 │ const value = "{direction}";
  │                ^
  │
"#,
        );

        session.assert_diagnostics(&expected);
    }

    /// Report invisible format characters inside literals.
    #[test]
    fn test_reports_invisible_characters_in_literals() {
        let soft_hyphen = '\u{00ad}';
        let zero_width_space = '\u{200b}';
        let word_joiner = '\u{2060}';
        let source = format!(
            r#"const softHyphen = "soft{soft_hyphen}hyphen";

const zeroWidthSpace = "zero{zero_width_space}width";

const wordJoiner = "word{word_joiner}joiner";"#,
        );
        let session = TestSession::dir(&NO_INVISIBLE_CHARACTER, &source);
        let expected = format!(
            r#"
warning[no-invisible-character]: invisible character U+00AD appears in source
 ──▶ main.tspp:1:25
  │
1 │ const softHyphen = "soft{soft_hyphen}hyphen";
  │                         ^
2 │
3 │ const zeroWidthSpace = "zero{zero_width_space}width";
  │

warning[no-invisible-character]: invisible character U+200B appears in source
 ──▶ main.tspp:3:29
  │
1 │ const softHyphen = "soft{soft_hyphen}hyphen";
2 │
3 │ const zeroWidthSpace = "zero{zero_width_space}width";
  │                             ^
4 │
5 │ const wordJoiner = "word{word_joiner}joiner";
  │

warning[no-invisible-character]: invisible character U+2060 appears in source
 ──▶ main.tspp:5:25
  │
3 │ const zeroWidthSpace = "zero{zero_width_space}width";
4 │
5 │ const wordJoiner = "word{word_joiner}joiner";
  │                         ^
  │
"#,
        );

        session.assert_diagnostics(&expected);
    }

    /// Accept a directional control written as an escape.
    #[test]
    fn test_accepts_escaped_direction_control() {
        let session = TestSession::dir(&NO_INVISIBLE_CHARACTER, r#"const direction = "\u{202e}";"#);

        session.assert_no_diagnostics();
    }

    /// Accept Unicode format characters inside literal text.
    #[test]
    fn test_accepts_unicode_literal_text() {
        let session = TestSession::dir(
            &NO_INVISIBLE_CHARACTER,
            "const family = \"👩\u{200d}👩\u{200d}👧\u{200d}👦\";",
        );

        session.assert_no_diagnostics();
    }

    /// Accept Unicode format characters inside template text.
    #[test]
    fn test_accepts_unicode_template_text() {
        let session = TestSession::dir(
            &NO_INVISIBLE_CHARACTER,
            r#"
const name = "Ada";
const family = `👩‍👩 ${name}`;
"#,
        );

        session.assert_no_diagnostics();
    }
}
