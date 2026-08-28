/// One ECMAScript lexical token class relevant to token separation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TokenClass {
    /// An identifier or keyword.
    Word,
    /// A number literal.
    Number,
    /// A bigint literal.
    Bigint,
    /// A regular expression literal.
    Regex,
    /// A token ending in `+`.
    Plus,
    /// A token ending in `-`.
    Minus,
    /// A division token.
    Slash,
    /// Any token without an adjacent-token hazard.
    Other,
}

impl TokenClass {
    /// Classify one fixed ECMAScript token.
    pub(super) fn classify(token: &str) -> Self {
        match token {
            "+" | "++" => Self::Plus,
            "-" | "--" => Self::Minus,
            "/" => Self::Slash,
            _ => Self::Other,
        }
    }

    /// Return whether the next token requires one separating space.
    pub(super) fn needs_separator(self, next: Self, next_text: &str) -> bool {
        let previous_is_word = matches!(self, Self::Word | Self::Number | Self::Bigint);
        let next_is_word = matches!(next, Self::Word | Self::Number | Self::Bigint);

        if previous_is_word && next_is_word {
            return true;
        }

        if self == Self::Regex && next == Self::Word {
            return true;
        }

        if self == Self::Plus && next_text.starts_with('+') {
            return true;
        }

        if self == Self::Minus && next_text.starts_with('-') {
            return true;
        }

        self == Self::Slash && matches!(next_text.as_bytes().first(), Some(b'/' | b'*'))
    }
}
