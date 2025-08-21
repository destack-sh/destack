use crate::xid::UnicodeXID;

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

/// Generates all valid Unicode scalar values (excluding surrogates).
fn all_valid_chars() -> impl Iterator<Item = char> {
    (0u32..=0xD7FF).chain(0xE000u32..=0x10FFFF).map(|u| {
        core::convert::TryFrom::try_from(u)
            .expect("The selected range should be infallible if the docs match impl")
    })
}

#[test]
fn test_all_valid_chars_do_not_panic_for_is_xid_start() {
    // Verify XID start check doesn't panic on any valid Unicode character.
    for c in all_valid_chars() {
        let _ = UnicodeXID::is_xid_start(c);
    }
}

#[test]
fn test_all_valid_chars_do_not_panic_for_is_xid_continue() {
    // Verify XID continue check doesn't panic on any valid Unicode character.
    for c in all_valid_chars() {
        let _ = UnicodeXID::is_xid_continue(c);
    }
}
