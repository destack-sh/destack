use smallvec::SmallVec;

/// One regular expression pattern awaiting grammar validation.
pub(crate) struct RegexPattern<'a> {
    /// The pattern source between its delimiters.
    source: &'a str,
}

/// One regular expression flag sequence awaiting validation.
pub(crate) struct RegexFlags<'a> {
    /// The flag source after the closing pattern delimiter.
    source: &'a str,
}

impl<'a> RegexPattern<'a> {
    /// Create a regular expression pattern validator.
    pub(crate) const fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Return true when content contains a regex line terminator code point.
    ///
    /// Regex literal bodies cannot contain raw line terminators.
    /// The broad lexer defers this grammar check to the parser.
    pub(crate) fn contains_line_terminator(&self) -> bool {
        self.source
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    }

    /// Return true when regex unicode escapes are valid for the provided flags.
    pub(crate) fn unicode_escapes_are_valid(&self, flags: &RegexFlags<'_>) -> bool {
        // unicode escape validation only applies in unicode regex modes
        if !flags.has_unicode_mode() {
            return true;
        }

        // validate braced unicode escapes first
        if !self.unicode_code_point_escapes_are_valid() {
            return false;
        }

        // count capturing groups so decimal escapes can be validated as backreferences
        let Some(capturing_group_count) = self.unicode_capturing_group_count() else {
            return false;
        };

        // validate unicode-mode quantifier and escape restrictions
        self.unicode_tokens_are_valid(capturing_group_count)
    }

    /// Return whether braced code point escapes such as `\u{1F600}` are valid.
    fn unicode_code_point_escapes_are_valid(&self) -> bool {
        let mut characters = self.source.chars().peekable();
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

    /// Count capturing groups in Unicode mode.
    fn unicode_capturing_group_count(&self) -> Option<usize> {
        let characters = self.source.as_bytes();
        let mut index = 0_usize;
        let mut in_character_class = false;
        let mut capturing_group_count = 0_usize;

        while index < characters.len() {
            let character = characters[index];

            // escaped tokens do not affect grouping
            if character == b'\\' {
                index += 1;
                if index >= characters.len() {
                    return None;
                }
                index += 1;
                continue;
            }

            // ignore group markers inside character classes
            if in_character_class {
                if character == b']' {
                    in_character_class = false;
                }
                index += 1;
                continue;
            }

            if character == b'[' {
                in_character_class = true;
                index += 1;
                continue;
            }

            // parse group prefixes to decide whether this is a capturing group
            if character == b'(' {
                if index + 1 < characters.len() && characters[index + 1] == b'?' {
                    if index + 2 >= characters.len() {
                        return None;
                    }

                    let marker = characters[index + 2];
                    if marker == b':' || marker == b'=' || marker == b'!' {
                        index += 3;
                        continue;
                    }

                    if marker == b'<' {
                        if index + 3 >= characters.len() {
                            return None;
                        }

                        let lookbehind_marker = characters[index + 3];
                        if lookbehind_marker == b'=' || lookbehind_marker == b'!' {
                            index += 4;
                            continue;
                        }

                        let mut name_index = index + 3;
                        let mut has_name = false;
                        while name_index < characters.len() && characters[name_index] != b'>' {
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

    /// Return whether Unicode escapes and quantifier placement are valid.
    fn unicode_tokens_are_valid(&self, capturing_group_count: usize) -> bool {
        let characters = self.source.as_bytes();
        let mut index = 0_usize;
        let mut in_character_class = false;
        let mut is_previous_quantifiable_term = false;
        let mut is_previous_quantifier = false;
        let mut group_quantifiability_stack = SmallVec::<[bool; 8]>::new();

        while index < characters.len() {
            let character = characters[index];

            // escaped tokens: validate decimal escapes and advance over escape body
            if character == b'\\' {
                let Some(next_character) = characters.get(index + 1).copied() else {
                    return false;
                };

                // consume full braced unicode code point escapes so `{...}` is not
                // interpreted as a quantifier token
                if next_character == b'u'
                    && characters
                        .get(index + 2)
                        .is_some_and(|character| *character == b'{')
                {
                    let mut escape_index = index + 3;
                    let mut has_digit = false;
                    while escape_index < characters.len() && characters[escape_index] != b'}' {
                        if !characters[escape_index].is_ascii_hexdigit() {
                            return false;
                        }
                        has_digit = true;
                        escape_index += 1;
                    }

                    if !has_digit || escape_index >= characters.len() {
                        return false;
                    }

                    is_previous_quantifiable_term = true;
                    is_previous_quantifier = false;
                    index = escape_index + 1;
                    continue;
                }

                // consume unicode property escapes like \p{Emoji} and \P{Emoji}
                if next_character == b'p' || next_character == b'P' {
                    let Some(next_index) = self.unicode_property_escape_end(index) else {
                        return false;
                    };

                    is_previous_quantifiable_term = true;
                    is_previous_quantifier = false;
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
                        let digit = *digit as usize - b'0' as usize;
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

                    is_previous_quantifiable_term = true;
                    is_previous_quantifier = false;
                    index = digit_index;
                    continue;
                }

                if next_character == b'b' || next_character == b'B' {
                    is_previous_quantifiable_term = false;
                    is_previous_quantifier = false;
                    index += 2;
                    continue;
                }

                is_previous_quantifiable_term = true;
                is_previous_quantifier = false;
                index += 2;
                continue;
            }

            // character classes are always quantifiable terms once closed
            if in_character_class {
                if character == b']' {
                    in_character_class = false;
                    is_previous_quantifiable_term = true;
                    is_previous_quantifier = false;
                }
                index += 1;
                continue;
            }

            if character == b'[' {
                in_character_class = true;
                is_previous_quantifiable_term = false;
                is_previous_quantifier = false;
                index += 1;
                continue;
            }

            // groups are quantifiable unless they are lookaround assertions
            if character == b'(' {
                let mut is_group_quantifiable = true;
                if index + 1 < characters.len() && characters[index + 1] == b'?' {
                    if index + 2 >= characters.len() {
                        return false;
                    }

                    let marker = characters[index + 2];
                    if marker == b'=' || marker == b'!' {
                        is_group_quantifiable = false;
                        index += 3;
                    } else if marker == b'<' {
                        if index + 3 >= characters.len() {
                            return false;
                        }
                        let lookbehind_marker = characters[index + 3];
                        if lookbehind_marker == b'=' || lookbehind_marker == b'!' {
                            is_group_quantifiable = false;
                            index += 4;
                        } else {
                            let mut name_index = index + 3;
                            let mut has_name = false;
                            while name_index < characters.len() && characters[name_index] != b'>' {
                                has_name = true;
                                name_index += 1;
                            }
                            if name_index >= characters.len() || !has_name {
                                return false;
                            }
                            index = name_index + 1;
                        }
                    } else if marker == b':' {
                        index += 3;
                    } else {
                        return false;
                    }
                } else {
                    index += 1;
                }

                group_quantifiability_stack.push(is_group_quantifiable);
                is_previous_quantifiable_term = false;
                is_previous_quantifier = false;
                continue;
            }

            if character == b')' {
                let Some(is_group_quantifiable) = group_quantifiability_stack.pop() else {
                    return false;
                };
                is_previous_quantifiable_term = is_group_quantifiable;
                is_previous_quantifier = false;
                index += 1;
                continue;
            }

            // assertions and alternations are not quantifiable terms
            if character == b'^' || character == b'$' || character == b'|' {
                is_previous_quantifiable_term = false;
                is_previous_quantifier = false;
                index += 1;
                continue;
            }

            // require a quantifiable pattern term before postfix quantifiers
            if character == b'*' || character == b'+' {
                if !is_previous_quantifiable_term {
                    return false;
                }
                is_previous_quantifiable_term = false;
                is_previous_quantifier = true;
                index += 1;
                continue;
            }

            if character == b'?' {
                if is_previous_quantifier {
                    is_previous_quantifier = false;
                    is_previous_quantifiable_term = false;
                    index += 1;
                    continue;
                }

                if !is_previous_quantifiable_term {
                    return false;
                }
                is_previous_quantifiable_term = false;
                is_previous_quantifier = true;
                index += 1;
                continue;
            }

            if character == b'{' {
                if !is_previous_quantifiable_term {
                    return false;
                }

                let Some(next_index) = self.unicode_braced_quantifier_end(index) else {
                    return false;
                };
                is_previous_quantifiable_term = false;
                is_previous_quantifier = true;
                index = next_index;
                continue;
            }

            // unescaped `}` is always invalid in unicode regex mode
            if character == b'}' {
                return false;
            }

            is_previous_quantifiable_term = true;
            is_previous_quantifier = false;
            index += 1;
        }

        !in_character_class && group_quantifiability_stack.is_empty()
    }

    /// Return the index after one Unicode property escape.
    fn unicode_property_escape_end(&self, start: usize) -> Option<usize> {
        let characters = self.source.as_bytes();

        // require opening brace after \p or \P
        if characters.get(start + 2).copied() != Some(b'{') {
            return None;
        }

        let mut index = start + 3;
        let mut has_content = false;
        while index < characters.len() && characters[index] != b'}' {
            let character = characters[index];
            if !(character.is_ascii_alphanumeric()
                || character == b'_'
                || character == b'='
                || character == b'-')
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

    /// Return the index after one Unicode braced quantifier.
    fn unicode_braced_quantifier_end(&self, start: usize) -> Option<usize> {
        let characters = self.source.as_bytes();
        let mut index = start + 1;

        // parse the minimum repetition count
        let minimum_start = index;
        while index < characters.len() && characters[index].is_ascii_digit() {
            index += 1;
        }
        if minimum_start == index {
            return None;
        }
        let minimum = self.decimal(minimum_start, index)?;

        // parse the optional maximum repetition count
        let mut maximum = minimum;
        if index < characters.len() && characters[index] == b',' {
            index += 1;
            let maximum_start = index;
            while index < characters.len() && characters[index].is_ascii_digit() {
                index += 1;
            }

            if maximum_start < index {
                maximum = self.decimal(maximum_start, index)?;
            } else {
                maximum = usize::MAX;
            }
        }

        if index >= characters.len() || characters[index] != b'}' {
            return None;
        }
        if maximum != usize::MAX && maximum < minimum {
            return None;
        }

        Some(index + 1)
    }

    /// Return one ASCII decimal value from the pattern range.
    fn decimal(&self, start: usize, end: usize) -> Option<usize> {
        let characters = self.source.as_bytes();
        let mut value = 0usize;

        for character in &characters[start..end] {
            if !character.is_ascii_digit() {
                return None;
            }
            let digit = (character - b'0') as usize;
            value = value.checked_mul(10)?.checked_add(digit)?;
        }

        Some(value)
    }
}

impl<'a> RegexFlags<'a> {
    /// Create a regular expression flag validator.
    pub(crate) const fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Return whether the flag sequence is empty.
    pub(crate) const fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Return whether the flag sequence is valid.
    pub(crate) fn is_valid(&self) -> bool {
        let mut seen_d = false;
        let mut seen_g = false;
        let mut seen_i = false;
        let mut seen_m = false;
        let mut seen_s = false;
        let mut seen_u = false;
        let mut seen_v = false;
        let mut seen_y = false;

        for flag in self.source.chars() {
            match flag {
                'd' if !seen_d => seen_d = true,
                'g' if !seen_g => seen_g = true,
                'i' if !seen_i => seen_i = true,
                'm' if !seen_m => seen_m = true,
                's' if !seen_s => seen_s = true,
                'u' if !seen_u => seen_u = true,
                'v' if !seen_v => seen_v = true,
                'y' if !seen_y => seen_y = true,
                _ => return false,
            }
        }

        // unicode and unicode sets are mutually exclusive
        !(seen_u && seen_v)
    }

    /// Return whether the flag sequence enables either Unicode grammar.
    fn has_unicode_mode(&self) -> bool {
        self.source.bytes().any(|flag| flag == b'u' || flag == b'v')
    }

    /// Return the flag source.
    pub(crate) const fn source(&self) -> &'a str {
        self.source
    }
}
