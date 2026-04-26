use crate::Parser;

impl Parser {
    /// Return true when content contains a regex line terminator code point.
    ///
    /// Regex literal bodies cannot contain raw line terminators.
    /// This stays in parser validation because the lexer intentionally tokenizes
    /// regex literals broadly and defers syntax-specific checks to parse.
    pub(super) fn contains_regex_line_terminator(&self, content: &str) -> bool {
        content
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    }

    /// Return true when regex flags are valid in typed and untyped regex literals.
    pub(super) fn regex_flags_are_valid(&self, flags: &str) -> bool {
        let mut seen_d = false;
        let mut seen_g = false;
        let mut seen_i = false;
        let mut seen_m = false;
        let mut seen_s = false;
        let mut seen_u = false;
        let mut seen_v = false;
        let mut seen_y = false;

        for flag in flags.chars() {
            match flag {
                'd' => {
                    if seen_d {
                        return false;
                    }
                    seen_d = true;
                }
                'g' => {
                    if seen_g {
                        return false;
                    }
                    seen_g = true;
                }
                'i' => {
                    if seen_i {
                        return false;
                    }
                    seen_i = true;
                }
                'm' => {
                    if seen_m {
                        return false;
                    }
                    seen_m = true;
                }
                's' => {
                    if seen_s {
                        return false;
                    }
                    seen_s = true;
                }
                'u' => {
                    if seen_u {
                        return false;
                    }
                    seen_u = true;
                }
                'v' => {
                    if seen_v {
                        return false;
                    }
                    seen_v = true;
                }
                'y' => {
                    if seen_y {
                        return false;
                    }
                    seen_y = true;
                }
                _ => return false,
            }
        }

        // unicode and unicode-sets are mutually exclusive
        if seen_u && seen_v {
            return false;
        }

        true
    }

    /// Return true when regex unicode escapes are valid for the provided flags.
    pub(super) fn regex_unicode_escapes_are_valid(&self, pattern: &str, flags: &str) -> bool {
        // unicode escape validation only applies in unicode regex modes
        let has_unicode_mode = flags.chars().any(|flag| flag == 'u' || flag == 'v');
        if !has_unicode_mode {
            return true;
        }

        // validate braced unicode escapes first
        if !self.regex_unicode_code_point_escapes_are_valid(pattern) {
            return false;
        }

        // count capturing groups so decimal escapes can be validated as backreferences
        let Some(capturing_group_count) = self.regex_unicode_capturing_group_count(pattern) else {
            return false;
        };

        // validate unicode-mode quantifier and escape restrictions
        self.regex_unicode_tokens_are_valid(pattern, capturing_group_count)
    }

    // validate braced code point escapes like \u{1F600}
    fn regex_unicode_code_point_escapes_are_valid(&self, pattern: &str) -> bool {
        let mut characters = pattern.chars().peekable();
        while let Some(character) = characters.next() {
            if character != '\\' {
                continue;
            }

            let Some(next) = characters.next() else {
                return false;
            };
            if next != 'u' {
                continue;
            }
            if !matches!(characters.peek(), Some('{')) {
                continue;
            }
            characters.next();

            let mut digits = 0_usize;
            let mut significant_digits = 0_usize;
            let mut value: u32 = 0;
            while let Some(next_character) = characters.peek().copied() {
                if next_character == '}' {
                    break;
                }

                let Some(digit) = next_character.to_digit(16) else {
                    return false;
                };
                digits += 1;

                if digit != 0 || significant_digits > 0 {
                    significant_digits += 1;
                    if significant_digits > 6 {
                        return false;
                    }

                    value = match value
                        .checked_mul(16)
                        .and_then(|value| value.checked_add(digit))
                    {
                        Some(value) => value,
                        None => return false,
                    };
                }
                characters.next();
            }

            if digits == 0 {
                return false;
            }
            if characters.next() != Some('}') {
                return false;
            }
            if value > 0x10FFFF {
                return false;
            }
        }

        true
    }

    // count capturing groups in unicode regex mode
    fn regex_unicode_capturing_group_count(&self, pattern: &str) -> Option<usize> {
        let characters: Vec<char> = pattern.chars().collect();
        let mut index = 0_usize;
        let mut in_character_class = false;
        let mut capturing_group_count = 0_usize;

        while index < characters.len() {
            let character = characters[index];

            // escaped tokens do not affect grouping
            if character == '\\' {
                index += 1;
                if index >= characters.len() {
                    return None;
                }
                index += 1;
                continue;
            }

            // ignore group markers inside character classes
            if in_character_class {
                if character == ']' {
                    in_character_class = false;
                }
                index += 1;
                continue;
            }

            if character == '[' {
                in_character_class = true;
                index += 1;
                continue;
            }

            // parse group prefixes to decide whether this is a capturing group
            if character == '(' {
                if index + 1 < characters.len() && characters[index + 1] == '?' {
                    if index + 2 >= characters.len() {
                        return None;
                    }

                    let marker = characters[index + 2];
                    if marker == ':' || marker == '=' || marker == '!' {
                        index += 3;
                        continue;
                    }

                    if marker == '<' {
                        if index + 3 >= characters.len() {
                            return None;
                        }

                        let lookbehind_marker = characters[index + 3];
                        if lookbehind_marker == '=' || lookbehind_marker == '!' {
                            index += 4;
                            continue;
                        }

                        let mut name_index = index + 3;
                        let mut has_name = false;
                        while name_index < characters.len() && characters[name_index] != '>' {
                            has_name = true;
                            name_index += 1;
                        }
                        if name_index >= characters.len() || !has_name {
                            return None;
                        }

                        capturing_group_count += 1;
                        index = name_index + 1;
                        continue;
                    }

                    return None;
                }

                capturing_group_count += 1;
                index += 1;
                continue;
            }

            index += 1;
        }

        if in_character_class {
            return None;
        }

        Some(capturing_group_count)
    }

    // validate unicode-mode escapes and quantifier placement
    fn regex_unicode_tokens_are_valid(&self, pattern: &str, capturing_group_count: usize) -> bool {
        let characters: Vec<char> = pattern.chars().collect();
        let mut index = 0_usize;
        let mut in_character_class = false;
        let mut previous_is_quantifiable_atom = false;
        let mut previous_is_quantifier = false;
        let mut group_quantifiability_stack: Vec<bool> = Vec::new();

        while index < characters.len() {
            let character = characters[index];

            // escaped tokens: validate decimal escapes and advance over escape body
            if character == '\\' {
                let Some(next_character) = characters.get(index + 1).copied() else {
                    return false;
                };

                // consume full braced unicode code point escapes so `{...}` is not
                // interpreted as a quantifier token
                if next_character == 'u'
                    && characters
                        .get(index + 2)
                        .is_some_and(|character| *character == '{')
                {
                    let mut escape_index = index + 3;
                    let mut has_digit = false;
                    while escape_index < characters.len() && characters[escape_index] != '}' {
                        if !characters[escape_index].is_ascii_hexdigit() {
                            return false;
                        }
                        has_digit = true;
                        escape_index += 1;
                    }

                    if !has_digit || escape_index >= characters.len() {
                        return false;
                    }

                    previous_is_quantifiable_atom = true;
                    previous_is_quantifier = false;
                    index = escape_index + 1;
                    continue;
                }

                // consume unicode property escapes like \p{Emoji} and \P{Emoji}
                if next_character == 'p' || next_character == 'P' {
                    let Some(next_index) =
                        self.regex_unicode_parse_property_escape(&characters, index)
                    else {
                        return false;
                    };

                    previous_is_quantifiable_atom = true;
                    previous_is_quantifier = false;
                    index = next_index;
                    continue;
                }

                if next_character.is_ascii_digit() {
                    let mut digit_index = index + 1;
                    while digit_index < characters.len() && characters[digit_index].is_ascii_digit()
                    {
                        digit_index += 1;
                    }

                    let mut number = 0usize;
                    for digit in &characters[index + 1..digit_index] {
                        let digit = match digit {
                            '0'..='9' => *digit as usize - '0' as usize,
                            _ => unreachable!("checked ascii digit"),
                        };
                        let Some(next) = number
                            .checked_mul(10)
                            .and_then(|number| number.checked_add(digit))
                        else {
                            return false;
                        };
                        number = next;
                    }

                    if number > 0 && number > capturing_group_count {
                        return false;
                    }

                    previous_is_quantifiable_atom = true;
                    previous_is_quantifier = false;
                    index = digit_index;
                    continue;
                }

                if next_character == 'b' || next_character == 'B' {
                    previous_is_quantifiable_atom = false;
                    previous_is_quantifier = false;
                    index += 2;
                    continue;
                }

                previous_is_quantifiable_atom = true;
                previous_is_quantifier = false;
                index += 2;
                continue;
            }

            // character classes are always quantifiable atoms once closed
            if in_character_class {
                if character == ']' {
                    in_character_class = false;
                    previous_is_quantifiable_atom = true;
                    previous_is_quantifier = false;
                }
                index += 1;
                continue;
            }

            if character == '[' {
                in_character_class = true;
                previous_is_quantifiable_atom = false;
                previous_is_quantifier = false;
                index += 1;
                continue;
            }

            // groups are quantifiable unless they are lookaround assertions
            if character == '(' {
                let mut group_is_quantifiable = true;
                if index + 1 < characters.len() && characters[index + 1] == '?' {
                    if index + 2 >= characters.len() {
                        return false;
                    }

                    let marker = characters[index + 2];
                    if marker == '=' || marker == '!' {
                        group_is_quantifiable = false;
                        index += 3;
                    } else if marker == '<' {
                        if index + 3 >= characters.len() {
                            return false;
                        }
                        let lookbehind_marker = characters[index + 3];
                        if lookbehind_marker == '=' || lookbehind_marker == '!' {
                            group_is_quantifiable = false;
                            index += 4;
                        } else {
                            let mut name_index = index + 3;
                            let mut has_name = false;
                            while name_index < characters.len() && characters[name_index] != '>' {
                                has_name = true;
                                name_index += 1;
                            }
                            if name_index >= characters.len() || !has_name {
                                return false;
                            }
                            index = name_index + 1;
                        }
                    } else if marker == ':' {
                        index += 3;
                    } else {
                        return false;
                    }
                } else {
                    index += 1;
                }

                group_quantifiability_stack.push(group_is_quantifiable);
                previous_is_quantifiable_atom = false;
                previous_is_quantifier = false;
                continue;
            }

            if character == ')' {
                let Some(group_is_quantifiable) = group_quantifiability_stack.pop() else {
                    return false;
                };
                previous_is_quantifiable_atom = group_is_quantifiable;
                previous_is_quantifier = false;
                index += 1;
                continue;
            }

            // assertions and alternations are not quantifiable atoms
            if character == '^' || character == '$' || character == '|' {
                previous_is_quantifiable_atom = false;
                previous_is_quantifier = false;
                index += 1;
                continue;
            }

            // postfix quantifiers require a quantifiable atom
            if character == '*' || character == '+' {
                if !previous_is_quantifiable_atom {
                    return false;
                }
                previous_is_quantifiable_atom = false;
                previous_is_quantifier = true;
                index += 1;
                continue;
            }

            if character == '?' {
                if previous_is_quantifier {
                    previous_is_quantifier = false;
                    previous_is_quantifiable_atom = false;
                    index += 1;
                    continue;
                }

                if !previous_is_quantifiable_atom {
                    return false;
                }
                previous_is_quantifiable_atom = false;
                previous_is_quantifier = true;
                index += 1;
                continue;
            }

            if character == '{' {
                if !previous_is_quantifiable_atom {
                    return false;
                }

                let Some(next_index) =
                    self.regex_unicode_parse_braced_quantifier(&characters, index)
                else {
                    return false;
                };
                previous_is_quantifiable_atom = false;
                previous_is_quantifier = true;
                index = next_index;
                continue;
            }

            // unescaped `}` is always invalid in unicode regex mode
            if character == '}' {
                return false;
            }

            previous_is_quantifiable_atom = true;
            previous_is_quantifier = false;
            index += 1;
        }

        !in_character_class && group_quantifiability_stack.is_empty()
    }

    // parse a unicode property escape at `start` and return the next index
    fn regex_unicode_parse_property_escape(
        &self,
        characters: &[char],
        start: usize,
    ) -> Option<usize> {
        // require opening brace after \p or \P
        if characters.get(start + 2).copied() != Some('{') {
            return None;
        }

        let mut index = start + 3;
        let mut has_content = false;
        while index < characters.len() && characters[index] != '}' {
            let character = characters[index];
            if !(character.is_ascii_alphanumeric()
                || character == '_'
                || character == '='
                || character == '-')
            {
                return None;
            }
            has_content = true;
            index += 1;
        }

        if !has_content || index >= characters.len() {
            return None;
        }

        Some(index + 1)
    }

    // parse a unicode-mode braced quantifier at `start` and return the next index
    fn regex_unicode_parse_braced_quantifier(
        &self,
        characters: &[char],
        start: usize,
    ) -> Option<usize> {
        let mut index = start + 1;

        // parse the minimum repetition count
        let minimum_start = index;
        while index < characters.len() && characters[index].is_ascii_digit() {
            index += 1;
        }
        if minimum_start == index {
            return None;
        }
        let minimum: usize = characters[minimum_start..index]
            .iter()
            .collect::<String>()
            .parse()
            .ok()?;

        // parse the optional maximum repetition count
        let mut maximum = minimum;
        if index < characters.len() && characters[index] == ',' {
            index += 1;
            let maximum_start = index;
            while index < characters.len() && characters[index].is_ascii_digit() {
                index += 1;
            }

            if maximum_start < index {
                maximum = characters[maximum_start..index]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()?;
            } else {
                maximum = usize::MAX;
            }
        }

        if index >= characters.len() || characters[index] != '}' {
            return None;
        }
        if maximum != usize::MAX && maximum < minimum {
            return None;
        }

        Some(index + 1)
    }
}
