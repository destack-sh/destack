//! Query character Unicode properties according to:
//!  - [Unicode Standard Annex #44](https://www.unicode.org/reports/tr44/)
//!  - [Unicode Technical Standard #51](https://www.unicode.org/reports/tr51/)
//!
//! Currently we support:
//!  - the `General_Category` property
//!  - the `Emoji` and `Emoji_Component` properties

#[rustfmt::skip]
mod table_gen;

pub use emoji::{EmojiStatus, UnicodeEmoji};
pub use general_category::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};
pub use table_gen::UNICODE_VERSION;

#[cfg(test)]
mod tests;

/// Query the emoji character properties of a character.
pub mod emoji {
    pub use crate::table_gen::emoji::EmojiStatus;

    /// Query the emoji character properties of a character.
    pub trait UnicodeEmoji: Sized {
        /// Gets the emoji character properties in a status enum.
        fn emoji_status(self) -> EmojiStatus;

        /// Checks whether this character is recommended for use as emoji, i.e. `Emoji=YES`.
        #[allow(clippy::wrong_self_convention)]
        fn is_emoji_char(self) -> bool {
            crate::table_gen::emoji::is_emoji_status_for_emoji_char(self.emoji_status())
        }

        /// Checks whether this character are used in emoji sequences where they're not
        /// intended for independent, direct input, i.e. `Emoji_Component=YES`.
        #[allow(clippy::wrong_self_convention)]
        fn is_emoji_component(self) -> bool {
            crate::table_gen::emoji::is_emoji_status_for_emoji_component(self.emoji_status())
        }

        /// Checks whether this character occurs in emoji sequences, i.e. `Emoji=YES | Emoji_Component=YES`
        #[allow(clippy::wrong_self_convention)]
        fn is_emoji_char_or_emoji_component(self) -> bool {
            crate::table_gen::emoji::is_emoji_status_for_emoji_char_or_emoji_component(
                self.emoji_status(),
            )
        }
    }

    impl UnicodeEmoji for char {
        fn emoji_status(self) -> EmojiStatus {
            crate::table_gen::emoji::emoji_status(self)
        }
    }

    /// Checks whether this character is the U+200D ZERO WIDTH JOINER (ZWJ) character.
    ///
    /// It can be used between the elements of a sequence of characters to indicate that
    /// a single glyph should be presented if available.
    #[inline]
    pub fn is_zwj(c: char) -> bool {
        c == '\u{200D}'
    }

    /// Checks whether this character is the U+FE0F VARIATION SELECTOR-16 (VS16) character, used to
    /// request an emoji presentation for an emoji character.
    #[inline]
    pub fn is_emoji_presentation_selector(c: char) -> bool {
        c == '\u{FE0F}'
    }

    /// Checks whether this character is the U+FE0E VARIATION SELECTOR-15 (VS15) character, used to
    /// request a text presentation for an emoji character.
    #[inline]
    pub fn is_text_presentation_selector(c: char) -> bool {
        c == '\u{FE0E}'
    }

    /// Checks whether this character is one of the Regional Indicator characters.
    ///
    /// A pair of REGIONAL INDICATOR symbols is referred to as an emoji_flag_sequence.
    #[inline]
    pub fn is_regional_indicator(c: char) -> bool {
        matches!(c, '\u{1F1E6}'..='\u{1F1FF}')
    }

    /// Checks whether this character is one of the Tag Characters.
    ///
    /// These can be used in indicating variants or extensions of emoji characters.
    #[inline]
    pub fn is_tag_character(c: char) -> bool {
        matches!(c, '\u{E0020}'..='\u{E007F}')
    }
}

/// Query the general category property of a character.
pub mod general_category {
    pub use crate::table_gen::general_category::{GeneralCategory, GeneralCategoryGroup};

    /// Query the general category property of a character.
    ///
    /// See [General Category Values](https://www.unicode.org/reports/tr44/#General_Category_Values) for more info.
    pub trait UnicodeGeneralCategory: Sized {
        /// Gets the most general classification of a character.
        fn general_category(self) -> GeneralCategory;

        /// Gets the grouping of the most general classification of a character.
        fn general_category_group(self) -> GeneralCategoryGroup {
            crate::table_gen::general_category::general_category_group(self.general_category())
        }

        /// Checks whether the most general classification of a character belongs to the `LetterCased` group.
        ///
        /// The `LetterCased` group includes `LetterUppercase`, `LetterLowercase`, and `LetterTitlecase`
        /// categories, and is a subset of the `Letter` group.
        #[allow(clippy::wrong_self_convention)]
        fn is_letter_cased(self) -> bool {
            crate::table_gen::general_category::general_category_is_letter_cased(
                self.general_category(),
            )
        }
    }

    impl UnicodeGeneralCategory for char {
        fn general_category(self) -> GeneralCategory {
            crate::table_gen::general_category::general_category_of_char(self)
        }
    }
}
