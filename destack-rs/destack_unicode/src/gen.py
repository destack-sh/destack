import fileinput
import operator
import os
import re
import sys
from typing import Any, Callable, Optional, TextIO

PREAMBLE = """\
// NOTE: DO NOT EDIT. This code was generated in gen.py
#![allow(missing_docs, non_upper_case_globals, non_snake_case)]
#![cfg_attr(rustfmt, rustfmt_skip)]
"""

UNICODE_VERSION = (16, 0, 0)
UNICODE_VERSION_NUMBER = "{}.{}.{}".format(*UNICODE_VERSION)


def _fetch_unidata(filename: str) -> None:
    """Download the UCD table file if it doesn't exist."""
    if not os.path.exists(os.path.basename(filename)):
        os.system(f"curl -O https://www.unicode.org/Public/{UNICODE_VERSION_NUMBER}/ucd/{filename}")
    if not os.path.exists(os.path.basename(filename)):
        sys.stderr.write(f"cannot load {filename}")
        sys.exit(1)


def _load_emoji_properties(filename: str) -> dict[str | None, list[tuple[int, int]]]:
    """
    Load code point data from emoji-data.txt.
    """
    _fetch_unidata(filename)
    emoji_kinds: dict[str | None, list[tuple[int, int]]] = {}

    # compile regex patterns for parsing
    single_codepoint_regex = re.compile(r"^ *([0-9A-F]+) *; *(\w+)")
    range_regex = re.compile(r"^ *([0-9A-F]+)\.\.([0-9A-F]+) *; *(\w+) *#")

    with fileinput.input(
        os.path.basename(filename), openhook=fileinput.hook_encoded("utf-8")
    ) as file_input:
        for line in file_input:
            kind: Optional[str] = None
            data_lo = 0
            data_hi = 0

            # try to match single codepoint format
            match = single_codepoint_regex.match(line)
            if match:
                data_lo = match.group(1)
                data_hi = match.group(1)
                kind = match.group(2).strip()
            else:
                # try to match range format
                match = range_regex.match(line)
                if match:
                    data_lo = match.group(1)
                    data_hi = match.group(2)
                    kind = match.group(3).strip()
                else:
                    continue

            # convert hex strings to integers
            data_lo_int = int(data_lo, 16)
            data_hi_int = int(data_hi, 16)

            # add to emoji kinds dictionary
            if kind not in emoji_kinds:
                emoji_kinds[kind] = []
            emoji_kinds[kind].append((data_lo_int, data_hi_int))
    return emoji_kinds


def load_general_category_properties(filename: str) -> list[tuple[int, int, str]]:
    """Load general category properties from UnicodeData.txt."""
    _fetch_unidata(filename)
    general_category_list: list[tuple[int, int, str]] = []

    # compile regex patterns for different line formats
    main_regex = re.compile(r"^([0-9A-F]+);([^;]+);([A-Za-z]+);.*$")
    first_regex = re.compile(r"^<(.*), First>$")
    last_regex = re.compile(r"^<(.*), Last>$")
    single_regex = re.compile(r"^<(.*)>$")

    # state for handling special group ranges
    special_group_lo = 0
    special_group_text = ""
    special_group_gc = ""

    with fileinput.input(
        os.path.basename(filename), openhook=fileinput.hook_encoded("utf-8")
    ) as file_input:
        for line in file_input:
            data_ch: str = ""
            data_name: str = ""
            data_gc: str = ""
            data_lo = 0
            data_hi = 0

            # parse main line format
            match = main_regex.match(line)
            if not match:
                continue

            data_ch = match.group(1)
            data_name = match.group(2).strip()
            data_gc = match.group(3).strip()

            # handle regular character entries
            if not data_name.startswith("<"):
                data_lo = int(data_ch, 16)
                data_hi = data_lo
                general_category_list.append((data_lo, data_hi, data_gc))
                continue

            # handle special group start markers
            first_match = first_regex.match(data_name)
            if first_match:
                special_group_lo = int(data_ch, 16)
                special_group_text = first_match.group(1)
                special_group_gc = data_gc
                continue

            # handle special group end markers
            last_match = last_regex.match(data_name)
            if last_match:
                assert special_group_text == last_match.group(1)
                assert special_group_gc == data_gc
                data_lo = special_group_lo
                data_hi = int(data_ch, 16)
                general_category_list.append((data_lo, data_hi, data_gc))
                continue

            # handle single special entries
            single_match = single_regex.match(data_name)
            if single_match:
                data_lo = int(data_ch, 16)
                data_hi = data_lo
                general_category_list.append((data_lo, data_hi, data_gc))
                continue

            raise ValueError("unreachable")
    return general_category_list


def _merge_adjacent_and_overlapping(ranges: list[tuple[int, int]]) -> list[tuple[int, int]]:
    """Merge overlapping or directly adjacent ranges.

    This keeps the output small while preserving the same set membership.
    """
    if not ranges:
        return []
    ranges_sorted = sorted(ranges, key=operator.itemgetter(0, 1))
    merged: list[tuple[int, int]] = []
    cur_lo, cur_hi = ranges_sorted[0]
    for lo, hi in ranges_sorted[1:]:
        if lo <= cur_hi + 1:
            if hi > cur_hi:
                cur_hi = hi
        else:
            merged.append((cur_lo, cur_hi))
            cur_lo, cur_hi = lo, hi
    merged.append((cur_lo, cur_hi))
    return merged


def _subtract_range_from_list(
    ranges: list[tuple[int, int]], sub_lo: int, sub_hi: int
) -> list[tuple[int, int]]:
    """Subtract a single [sub_lo, sub_hi] range from a list of ranges.

    Returns a new list with the subtraction applied.
    """
    if not ranges:
        return []
    result: list[tuple[int, int]] = []
    for lo, hi in ranges:
        if hi < sub_lo or lo > sub_hi:
            result.append((lo, hi))
            continue
        if lo < sub_lo:
            result.append((lo, sub_lo - 1))
        if hi > sub_hi:
            result.append((sub_hi + 1, hi))
    return result


def load_derived_core_properties(
    filename: str, interesting_props: list[str]
) -> dict[str | None, list[tuple[int, int]]]:
    """
    Load selected properties from DerivedCoreProperties.txt.

    Only properties listed in interesting_props are returned.
    """
    _fetch_unidata(filename)
    props: dict[str | None, list[tuple[int, int]]] = {}

    single_codepoint_regex = re.compile(r"^ *([0-9A-F]+) *; *([A-Za-z_]+)")
    range_regex = re.compile(r"^ *([0-9A-F]+)\.\.([0-9A-F]+) *; *([A-Za-z_]+)")

    with fileinput.input(
        os.path.basename(filename), openhook=fileinput.hook_encoded("utf-8")
    ) as file_input:
        for line in file_input:
            prop: Optional[str] = None
            data_lo = 0
            data_hi = 0

            match = single_codepoint_regex.match(line)
            if match:
                data_lo = int(match.group(1), 16)
                data_hi = data_lo
                prop = match.group(2).strip()
            else:
                match = range_regex.match(line)
                if match:
                    data_lo = int(match.group(1), 16)
                    data_hi = int(match.group(2), 16)
                    prop = match.group(3).strip()
                else:
                    continue

            if interesting_props and (prop not in interesting_props):
                continue

            if prop not in props:
                props[prop] = []
            props[prop].append((data_lo, data_hi))

    # normalize tables: merge adjacent/overlapping and remove surrogate range
    for prop in list(props.keys()):
        props[prop] = _merge_adjacent_and_overlapping(props[prop])
        # skip surrogates because they're not representable by Rust `char`
        props[prop] = _subtract_range_from_list(props[prop], 0xD800, 0xDFFF)

    return props


def _format_table_content(file_handle: TextIO, content: str, indent: int) -> None:
    """Format table content with proper line wrapping."""
    line = " " * indent
    is_first = True

    # split content and format with line wrapping
    for chunk in content.split(","):
        if len(line) + len(chunk) < 98:
            if is_first:
                line += chunk
            else:
                line += ", " + chunk
            is_first = False
        else:
            file_handle.write(line + ",\n")
            line = " " * indent + chunk
    file_handle.write(line)


def _escape_char(char_value: Any) -> str:
    """Escape a character value for Rust code generation."""
    if char_value == "multi":
        return '"<multiple code points>"'
    return f"'\\u{{{char_value:x}}}'"


def _generate_table(
    file_handle: TextIO,
    name: str,
    table_data: list[Any],
    table_type: str = "&'static [(char, char)]",
    is_pub: bool = True,
    print_function: Callable[[Any], str] = lambda x: f"({_escape_char(x[0])},{_escape_char(x[1])})",
    is_const: bool = True,
) -> None:
    """Emit a Rust table definition."""
    # determine visibility and mutability keywords
    pub_string = "const"
    if not is_const:
        pub_string = "let"
    if is_pub:
        pub_string = "pub(crate) " + pub_string

    # write table header
    file_handle.write(f"    {pub_string} {name}: {table_type} = &[\n")

    # format table data
    data = ""
    is_first = True
    for data_item in table_data:
        if not is_first:
            data += ","
        is_first = False
        data += print_function(data_item)

    # write formatted content and close table
    _format_table_content(file_handle, data, 8)
    file_handle.write("\n    ];\n\n")


def _generate_general_category_module(file_handle: TextIO) -> None:
    """Emit the general category module."""
    file_handle.write("""\
pub(crate) mod general_category {""")
    file_handle.write("""

    #[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
    /// The most general classification of a character.
    pub enum GeneralCategory {
        /// `Lu`, an uppercase letter
        UppercaseLetter,
        /// `Ll`, a lowercase letter
        LowercaseLetter,
        /// `Lt`, a digraphic character, with first part uppercase
        TitlecaseLetter,
        /// `Lm`, a modifier letter
        ModifierLetter,
        /// `Lo`, other letters, including syllables and ideographs
        OtherLetter,
        /// `Mn`, a nonspacing combining mark (zero advance width)
        NonspacingMark,
        /// `Mc`, a spacing combining mark (positive advance width)
        SpacingMark,
        /// `Me`, an enclosing combining mark
        EnclosingMark,
        /// `Nd`, a decimal digit
        DecimalNumber,
        /// `Nl`, a letterlike numeric character
        LetterNumber,
        /// `No`, a numeric character of other type
        OtherNumber,
        /// `Pc`, a connecting punctuation mark, like a tie
        ConnectorPunctuation,
        /// `Pd`, a dash or hyphen punctuation mark
        DashPunctuation,
        /// `Ps`, an opening punctuation mark (of a pair)
        OpenPunctuation,
        /// `Pe`, a closing punctuation mark (of a pair)
        ClosePunctuation,
        /// `Pi`, an initial quotation mark
        InitialPunctuation,
        /// `Pf`, a final quotation mark
        FinalPunctuation,
        /// `Po`, a punctuation mark of other type
        OtherPunctuation,
        /// `Sm`, a symbol of mathematical use
        MathSymbol,
        /// `Sc`, a currency sign
        CurrencySymbol,
        /// `Sk`, a non-letterlike modifier symbol
        ModifierSymbol,
        /// `So`, a symbol of other type
        OtherSymbol,
        /// `Zs`, a space character (of various non-zero widths)
        SpaceSeparator,
        /// `Zl`, U+2028 LINE SEPARATOR only
        LineSeparator,
        /// `Zp`, U+2029 PARAGRAPH SEPARATOR only
        ParagraphSeparator,
        /// `Cc`, a C0 or C1 control code
        Control,
        /// `Cf`, a format control character
        Format,
        /// `Cs`, a surrogate code point
        Surrogate,
        /// `Co`, a private-use character
        PrivateUse,
        /// `Cn`, a reserved unassigned code point or a noncharacter
        Unassigned,
    }

    #[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
    /// Groupings of the most general classification of a character.
    pub enum GeneralCategoryGroup {
        /// Lu | Ll | Lt | Lm | Lo
        Letter,
        /// Mn | Mc | Me
        Mark,
        /// Nd | Nl | No
        Number,
        /// Pc | Pd | Ps | Pe | Pi | Pf | Po
        Punctuation,
        /// Sm | Sc | Sk | So
        Symbol,        
        /// Zs | Zl | Zp
        Separator,
        /// Cc | Cf | Cs | Co | Cn
        Other,
    }

    #[inline]
    pub(crate) fn general_category_of_char(c: char) -> GeneralCategory {
        super::util::bsearch_range_value_table(c, GENERAL_CATEGORY).unwrap_or(GeneralCategory::Unassigned)
    }

    #[inline]
    pub(crate) fn general_category_is_letter_cased(gc: GeneralCategory) -> bool {
        matches!(gc, GeneralCategory::UppercaseLetter | GeneralCategory::LowercaseLetter | GeneralCategory::TitlecaseLetter)
    }

    #[inline]
    pub(crate) fn general_category_group(gc: GeneralCategory) -> GeneralCategoryGroup {
        match gc {
            GeneralCategory::UppercaseLetter |
            GeneralCategory::LowercaseLetter |
            GeneralCategory::TitlecaseLetter |
            GeneralCategory::ModifierLetter |
            GeneralCategory::OtherLetter => GeneralCategoryGroup::Letter,
            GeneralCategory::NonspacingMark |
            GeneralCategory::SpacingMark |
            GeneralCategory::EnclosingMark => GeneralCategoryGroup::Mark,
            GeneralCategory::DecimalNumber |
            GeneralCategory::LetterNumber |
            GeneralCategory::OtherNumber => GeneralCategoryGroup::Number,
            GeneralCategory::ConnectorPunctuation |
            GeneralCategory::DashPunctuation |
            GeneralCategory::OpenPunctuation |
            GeneralCategory::ClosePunctuation |
            GeneralCategory::InitialPunctuation |
            GeneralCategory::FinalPunctuation |
            GeneralCategory::OtherPunctuation => GeneralCategoryGroup::Punctuation,
            GeneralCategory::MathSymbol |
            GeneralCategory::CurrencySymbol |
            GeneralCategory::ModifierSymbol |
            GeneralCategory::OtherSymbol => GeneralCategoryGroup::Symbol,
            GeneralCategory::SpaceSeparator |
            GeneralCategory::LineSeparator |
            GeneralCategory::ParagraphSeparator => GeneralCategoryGroup::Separator,
            GeneralCategory::Control |
            GeneralCategory::Format |
            GeneralCategory::Surrogate |
            GeneralCategory::PrivateUse |
            GeneralCategory::Unassigned => GeneralCategoryGroup::Other,
        }
    }
""")
    gc_variants: dict[str, str] = {
        "Lu": "GeneralCategory::UppercaseLetter",
        "Ll": "GeneralCategory::LowercaseLetter",
        "Lt": "GeneralCategory::TitlecaseLetter",
        "Lm": "GeneralCategory::ModifierLetter",
        "Lo": "GeneralCategory::OtherLetter",
        "Mn": "GeneralCategory::NonspacingMark",
        "Mc": "GeneralCategory::SpacingMark",
        "Me": "GeneralCategory::EnclosingMark",
        "Nd": "GeneralCategory::DecimalNumber",
        "Nl": "GeneralCategory::LetterNumber",
        "No": "GeneralCategory::OtherNumber",
        "Pc": "GeneralCategory::ConnectorPunctuation",
        "Pd": "GeneralCategory::DashPunctuation",
        "Ps": "GeneralCategory::OpenPunctuation",
        "Pe": "GeneralCategory::ClosePunctuation",
        "Pi": "GeneralCategory::InitialPunctuation",
        "Pf": "GeneralCategory::FinalPunctuation",
        "Po": "GeneralCategory::OtherPunctuation",
        "Sm": "GeneralCategory::MathSymbol",
        "Sc": "GeneralCategory::CurrencySymbol",
        "Sk": "GeneralCategory::ModifierSymbol",
        "So": "GeneralCategory::OtherSymbol",
        "Zs": "GeneralCategory::SpaceSeparator",
        "Zl": "GeneralCategory::LineSeparator",
        "Zp": "GeneralCategory::ParagraphSeparator",
        "Cc": "GeneralCategory::Control",
        "Cf": "GeneralCategory::Format",
        "Cs": "GeneralCategory::Surrogate",
        "Co": "GeneralCategory::PrivateUse",
        "Cn": "GeneralCategory::Unassigned",
    }

    file_handle.write("    // General category table:\n")
    general_category_char_table = load_general_category_properties("UnicodeData.txt")
    general_category_group_table: list[tuple[int, int, str]] = []

    for input_idx in range(len(general_category_char_table)):
        if general_category_char_table[input_idx][2] == "Cs":
            continue
        existing_group_count = len(general_category_group_table)
        if existing_group_count == 0:
            general_category_group_table.append(general_category_char_table[input_idx])
        elif (
            general_category_group_table[existing_group_count - 1][1] + 1
            == general_category_char_table[input_idx][0]
            and general_category_group_table[existing_group_count - 1][2]
            == general_category_char_table[input_idx][2]
        ):
            general_category_group_table[existing_group_count - 1] = (
                general_category_group_table[existing_group_count - 1][0],
                general_category_char_table[input_idx][1],
                general_category_group_table[existing_group_count - 1][2],
            )
        else:
            general_category_group_table.append(general_category_char_table[input_idx])

    _generate_table(
        file_handle,
        "GENERAL_CATEGORY",
        general_category_group_table,
        "&[(char, char, GeneralCategory)]",
        is_pub=False,
        print_function=lambda x: f"({_escape_char(x[0])},{_escape_char(x[1])},{gc_variants[x[2]]})",
    )
    file_handle.write("}\n\n")


def _generate_emoji_module(file_handle: TextIO) -> None:
    """Generate the emoji module."""
    file_handle.write("""\
pub(crate) mod emoji {""")
    file_handle.write("""\

    #[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
    #[non_exhaustive]
    /// The emoji character properties of a character.
    pub enum EmojiStatus {
        /// `Emoji=NO`, `Emoji_Component=NO`
        NonEmoji,
        /// `Emoji=NO`, `Emoji_Component=YES`
        NonEmojiButEmojiComponent,
        /// `Emoji=YES`, `Emoji_Component=NO`;`Emoji_Presentation=YES`
        EmojiPresentation,
        /// `Emoji=YES`, `Emoji_Component=NO`;`Emoji_Modifier_Base=YES`
        EmojiModifierBase,
        /// `Emoji=YES`, `Emoji_Component=NO`;`Emoji_Presentation=YES`, `Emoji_Modifier_Base=YES`
        EmojiPresentationAndModifierBase,
        /// `Emoji=YES`, `Emoji_Component=NO`
        EmojiOther,
        /// `Emoji=YES`, `Emoji_Component=YES`;`Emoji_Presentation=YES`
        EmojiPresentationAndEmojiComponent,
        /// `Emoji=YES`, `Emoji_Component=YES`;`Emoji_Presentation=YES`, `Emoji_Modifier=YES`
        EmojiPresentationAndModifierAndEmojiComponent,
        /// `Emoji=YES`, `Emoji_Component=YES`
        EmojiOtherAndEmojiComponent,
    }
    #[inline]
    pub(crate) fn emoji_status(c: char) -> EmojiStatus {
        super::util::bsearch_range_value_table(c, EMOJI_STATUS).unwrap()
    }
    #[inline]
    pub(crate) fn is_emoji_status_for_emoji_char_or_emoji_component(s: EmojiStatus) -> bool {
        !matches!(s, EmojiStatus::NonEmoji)
    }
    #[inline]
    pub(crate) fn is_emoji_status_for_emoji_char(s: EmojiStatus) -> bool {
        !matches!(s, EmojiStatus::NonEmoji | EmojiStatus::NonEmojiButEmojiComponent)
    }
    #[inline]
    pub(crate) fn is_emoji_status_for_emoji_component(s: EmojiStatus) -> bool {
        matches!(s, EmojiStatus::EmojiPresentationAndEmojiComponent |
            EmojiStatus::EmojiPresentationAndModifierAndEmojiComponent |
            EmojiStatus::EmojiOtherAndEmojiComponent)
    }
""")

    file_handle.write("    // Emoji status table:\n")
    emoji_status_table = _load_emoji_properties("emoji/emoji-data.txt")

    # we combine things together here
    # `Extended_Pictographic` is only for future proof usages, we ignore it here
    emoji_prop_list = [
        "Emoji",
        "Emoji_Presentation",
        "Emoji_Modifier",
        "Emoji_Modifier_Base",
        "Emoji_Component",
    ]

    # need to skip surrogates because they're not representable by rust `char`s
    emoji_status_table["Surrogate"] = [(0xD800, 0xDFFF)]
    emoji_prop_list.append("Surrogate")

    emoji_prop_list_len = [len(emoji_status_table[prop]) for prop in emoji_prop_list]
    emoji_prop_count = len(emoji_prop_list)
    code_point_first = 0
    code_point_last = 0x10FFFF
    emoji_prop_list_pos = [0 for _ in emoji_prop_list]
    current_group_first = code_point_first
    emoji_table: list[tuple[int, int, str]] = []

    def _group_text(status_string: str) -> str:
        """Convert emoji status string to Rust enum variant."""
        if status_string == "Surrogate":
            return "<Surrogate>"
        elif status_string == "":
            return "EmojiStatus::NonEmoji"
        elif status_string == "Emoji_Component":
            return "EmojiStatus::NonEmojiButEmojiComponent"
        elif status_string == "Emoji;Emoji_Presentation":
            return "EmojiStatus::EmojiPresentation"
        elif status_string == "Emoji;Emoji_Presentation;Emoji_Modifier_Base":
            return "EmojiStatus::EmojiPresentationAndModifierBase"
        elif status_string == "Emoji;Emoji_Modifier_Base":
            return "EmojiStatus::EmojiModifierBase"
        elif status_string == "Emoji":
            return "EmojiStatus::EmojiOther"
        elif status_string == "Emoji;Emoji_Presentation;Emoji_Component":
            return "EmojiStatus::EmojiPresentationAndEmojiComponent"
        elif status_string == "Emoji;Emoji_Presentation;Emoji_Modifier;Emoji_Component":
            return "EmojiStatus::EmojiPresentationAndModifierAndEmojiComponent"
        elif status_string == "Emoji;Emoji_Component":
            return "EmojiStatus::EmojiOtherAndEmojiComponent"
        else:
            return 'EmojiStatus::NewCombination("' + status_string + '")'

    while current_group_first <= code_point_last:
        current_group_props: list[str] = []
        current_group_last = code_point_last

        for prop_list_idx in range(emoji_prop_count):
            if emoji_prop_list_pos[prop_list_idx] >= emoji_prop_list_len[prop_list_idx]:
                continue
            elif (
                emoji_status_table[emoji_prop_list[prop_list_idx]][
                    emoji_prop_list_pos[prop_list_idx]
                ][0]
                > current_group_first
            ):
                current_group_last = min(
                    current_group_last,
                    emoji_status_table[emoji_prop_list[prop_list_idx]][
                        emoji_prop_list_pos[prop_list_idx]
                    ][0]
                    - 1,
                )
            else:
                current_group_props.append(emoji_prop_list[prop_list_idx])
                current_group_last = min(
                    current_group_last,
                    emoji_status_table[emoji_prop_list[prop_list_idx]][
                        emoji_prop_list_pos[prop_list_idx]
                    ][1],
                )

        current_group_text = _group_text(";".join(current_group_props))
        if current_group_text != "<Surrogate>":
            emoji_table.append((current_group_first, current_group_last, current_group_text))

        for prop_list_idx in range(emoji_prop_count):
            if emoji_prop_list_pos[prop_list_idx] >= emoji_prop_list_len[prop_list_idx]:
                continue
            elif (
                emoji_status_table[emoji_prop_list[prop_list_idx]][
                    emoji_prop_list_pos[prop_list_idx]
                ][0]
                > current_group_first
            ):
                continue
            else:
                if (
                    current_group_last
                    == emoji_status_table[emoji_prop_list[prop_list_idx]][
                        emoji_prop_list_pos[prop_list_idx]
                    ][1]
                ):
                    emoji_prop_list_pos[prop_list_idx] += 1
        current_group_first = current_group_last + 1

    _generate_table(
        file_handle,
        "EMOJI_STATUS",
        emoji_table,
        "&[(char, char, EmojiStatus)]",
        is_pub=False,
        print_function=lambda x: f"({_escape_char(x[0])},{_escape_char(x[1])},{x[2]})",
    )
    file_handle.write("}\n\n")


def _generate_util_mod(file_handle: TextIO) -> None:
    """Generate the utility module."""
    file_handle.write("""
#[allow(dead_code)]
pub(crate) mod util {
    use core::result::Result::{Ok, Err};

    pub(crate) fn bsearch_range_value_table<T: Copy>(c: char, r: &'static [(char, char, T)]) -> Option<T> {
        use core::cmp::Ordering::{Equal, Less, Greater};
        match r.binary_search_by(|&(lo, hi, _)| {
            if lo <= c && c <= hi { Equal }
            else if hi < c { Less }
            else { Greater }
        }) {
            Ok(idx) => {
                let (_, _, cat) = r[idx];
                Some(cat)
            }
            Err(_) => None
        }
    }

    pub(crate) fn bsearch_range_table(c: char, r: &'static [(char, char)]) -> bool {
        use core::cmp::Ordering::{Equal, Less, Greater};
        r.binary_search_by(|&(lo, hi)| {
            // favor ASCII by testing Greater before Less
            if lo > c { Greater }
            else if hi < c { Less }
            else { Equal }
        }).is_ok()
    }

}

""")


def _generate_derived_core_properties_module(file_handle: TextIO) -> None:
    """Generate the derived core properties module (XID_Start and XID_Continue)."""
    file_handle.write("""\
pub(crate) mod derived_property {""")

    file_handle.write("    // Derived core property tables (subset):\n")
    wanted = ["XID_Start", "XID_Continue"]
    derived = load_derived_core_properties("DerivedCoreProperties.txt", wanted)

    # emit XID_Start and XID_Continue tables
    for prop in wanted:
        table_name = prop.upper().replace("_", "_")
        _generate_table(
            file_handle,
            table_name,
            derived.get(prop, []),
            "&[(char, char)]",
            is_pub=True,
        )

    file_handle.write("}\n\n")


if __name__ == "__main__":
    result_filename = "src/table_gen.rs"
    if os.path.exists(result_filename):
        os.remove(result_filename)
    with open(result_filename, "w") as rust_file:
        # write the file's preamble
        rust_file.write(PREAMBLE)

        rust_file.write(
            """
pub const UNICODE_VERSION: (u64, u64, u64) = ({}, {}, {});

""".format(*UNICODE_VERSION)
        )

        _generate_util_mod(rust_file)
        _generate_general_category_module(rust_file)
        _generate_emoji_module(rust_file)
        _generate_derived_core_properties_module(rust_file)

    # cargo fmt
    os.system("cargo fmt")
