#[test]
fn test_all_ascii_are_either_nonemoji_or_emojiother() {
    // All ASCII characters should have non-emoji or emoji-other status
    use crate::{EmojiStatus, UnicodeEmoji};
    for i in 0u8..=255u8 {
        let c = i as char;
        let status = c.emoji_status();
        assert!(matches!(
            status,
            EmojiStatus::NonEmoji
                | EmojiStatus::EmojiOther
                | EmojiStatus::EmojiOtherAndEmojiComponent
        ))
    }
}

#[test]
fn test_emoji_properties() {
    // Crab emoji should have proper emoji status and properties
    use crate::{EmojiStatus, UnicodeEmoji};
    assert_eq!('🦀'.emoji_status(), EmojiStatus::EmojiPresentation);
    assert!('🦀'.is_emoji_char());
    assert!(!('🦀'.is_emoji_component()));
    assert!('🦀'.is_emoji_char_or_emoji_component());
}

#[test]
fn test_general_category_properties() {
    // Various characters should have correct general category classifications
    use crate::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

    assert_eq!('A'.general_category(), GeneralCategory::UppercaseLetter);
    assert_eq!('A'.general_category_group(), GeneralCategoryGroup::Letter);
    assert!('A'.is_letter_cased());

    assert_eq!(' '.general_category(), GeneralCategory::SpaceSeparator);
    assert_eq!(
        ' '.general_category_group(),
        GeneralCategoryGroup::Separator
    );
    assert!(!(' '.is_letter_cased()));

    assert_eq!('一'.general_category(), GeneralCategory::OtherLetter);
    assert_eq!('一'.general_category_group(), GeneralCategoryGroup::Letter);
    assert!(!('一'.is_letter_cased()));

    assert_eq!('🦀'.general_category(), GeneralCategory::OtherSymbol);
    assert_eq!('🦀'.general_category_group(), GeneralCategoryGroup::Symbol);
    assert!(!('🦀'.is_letter_cased()));
}
