use super::HtmlString;

#[cfg(test)]
const UTF_8_BOM: &[u8] = b"\xEF\xBB\xBF";
const UTF_16_BE_NAME: &str = "utf-16be";
const UTF_16_LE_NAME: &str = "utf-16le";

/// The canonical names for supported HTML encoding labels.
const HTML_ENCODING_LABELS: &[(&str, &str)] = &[
    ("866", "ibm866"),
    ("ansi_x3.4-1968", "windows-1252"),
    ("arabic", "iso-8859-6"),
    ("ascii", "windows-1252"),
    ("asmo-708", "iso-8859-6"),
    ("big5", "big5"),
    ("big5-hkscs", "big5"),
    ("chinese", "gbk"),
    ("cn-big5", "big5"),
    ("cp1250", "windows-1250"),
    ("cp1251", "windows-1251"),
    ("cp1252", "windows-1252"),
    ("cp1253", "windows-1253"),
    ("cp1254", "windows-1254"),
    ("cp1255", "windows-1255"),
    ("cp1256", "windows-1256"),
    ("cp1257", "windows-1257"),
    ("cp1258", "windows-1258"),
    ("cp819", "windows-1252"),
    ("cp866", "ibm866"),
    ("csbig5", "big5"),
    ("cseuckr", "euc-kr"),
    ("cseucpkdfmtjapanese", "euc-jp"),
    ("csgb2312", "gbk"),
    ("csibm866", "ibm866"),
    ("csiso2022jp", "iso-2022-jp"),
    ("csiso2022kr", "replacement"),
    ("csiso58gb231280", "gbk"),
    ("csiso88596e", "iso-8859-6"),
    ("csiso88596i", "iso-8859-6"),
    ("csiso88598e", "iso-8859-8"),
    ("csiso88598i", "iso-8859-8-i"),
    ("csisolatin1", "windows-1252"),
    ("csisolatin2", "iso-8859-2"),
    ("csisolatin3", "iso-8859-3"),
    ("csisolatin4", "iso-8859-4"),
    ("csisolatin5", "windows-1254"),
    ("csisolatin6", "iso-8859-10"),
    ("csisolatin9", "iso-8859-15"),
    ("csisolatinarabic", "iso-8859-6"),
    ("csisolatincyrillic", "iso-8859-5"),
    ("csisolatingreek", "iso-8859-7"),
    ("csisolatinhebrew", "iso-8859-8"),
    ("cskoi8r", "koi8-r"),
    ("csksc56011987", "euc-kr"),
    ("csmacintosh", "macintosh"),
    ("csshiftjis", "shift-jis"),
    ("csunicode", "utf-16le"),
    ("cyrillic", "iso-8859-5"),
    ("dos-874", "windows-874"),
    ("ecma-114", "iso-8859-6"),
    ("ecma-118", "iso-8859-7"),
    ("elot_928", "iso-8859-7"),
    ("euc-jp", "euc-jp"),
    ("euc-kr", "euc-kr"),
    ("gb18030", "gb18030"),
    ("gb2312", "gbk"),
    ("gb_2312", "gbk"),
    ("gb_2312-80", "gbk"),
    ("gbk", "gbk"),
    ("greek", "iso-8859-7"),
    ("greek8", "iso-8859-7"),
    ("hebrew", "iso-8859-8"),
    ("hz-gb-2312", "replacement"),
    ("ibm819", "windows-1252"),
    ("ibm866", "ibm866"),
    ("iso-10646-ucs-2", "utf-16le"),
    ("iso-2022-cn", "replacement"),
    ("iso-2022-cn-ext", "replacement"),
    ("iso-2022-jp", "iso-2022-jp"),
    ("iso-2022-kr", "replacement"),
    ("iso-8859-1", "windows-1252"),
    ("iso-8859-10", "iso-8859-10"),
    ("iso-8859-11", "windows-874"),
    ("iso-8859-13", "iso-8859-13"),
    ("iso-8859-14", "iso-8859-14"),
    ("iso-8859-15", "iso-8859-15"),
    ("iso-8859-16", "iso-8859-16"),
    ("iso-8859-2", "iso-8859-2"),
    ("iso-8859-3", "iso-8859-3"),
    ("iso-8859-4", "iso-8859-4"),
    ("iso-8859-5", "iso-8859-5"),
    ("iso-8859-6", "iso-8859-6"),
    ("iso-8859-6-e", "iso-8859-6"),
    ("iso-8859-6-i", "iso-8859-6"),
    ("iso-8859-7", "iso-8859-7"),
    ("iso-8859-8", "iso-8859-8"),
    ("iso-8859-8-e", "iso-8859-8"),
    ("iso-8859-8-i", "iso-8859-8-i"),
    ("iso-8859-9", "windows-1254"),
    ("iso-ir-100", "windows-1252"),
    ("iso-ir-101", "iso-8859-2"),
    ("iso-ir-109", "iso-8859-3"),
    ("iso-ir-110", "iso-8859-4"),
    ("iso-ir-126", "iso-8859-7"),
    ("iso-ir-127", "iso-8859-6"),
    ("iso-ir-138", "iso-8859-8"),
    ("iso-ir-144", "iso-8859-5"),
    ("iso-ir-148", "windows-1254"),
    ("iso-ir-149", "euc-kr"),
    ("iso-ir-157", "iso-8859-10"),
    ("iso-ir-58", "gbk"),
    ("iso8859-1", "windows-1252"),
    ("iso8859-10", "iso-8859-10"),
    ("iso8859-11", "windows-874"),
    ("iso8859-13", "iso-8859-13"),
    ("iso8859-14", "iso-8859-14"),
    ("iso8859-15", "iso-8859-15"),
    ("iso8859-2", "iso-8859-2"),
    ("iso8859-3", "iso-8859-3"),
    ("iso8859-4", "iso-8859-4"),
    ("iso8859-5", "iso-8859-5"),
    ("iso8859-6", "iso-8859-6"),
    ("iso8859-7", "iso-8859-7"),
    ("iso8859-8", "iso-8859-8"),
    ("iso8859-9", "windows-1254"),
    ("iso88591", "windows-1252"),
    ("iso885910", "iso-8859-10"),
    ("iso885911", "windows-874"),
    ("iso885913", "iso-8859-13"),
    ("iso885914", "iso-8859-14"),
    ("iso885915", "iso-8859-15"),
    ("iso88592", "iso-8859-2"),
    ("iso88593", "iso-8859-3"),
    ("iso88594", "iso-8859-4"),
    ("iso88595", "iso-8859-5"),
    ("iso88596", "iso-8859-6"),
    ("iso88597", "iso-8859-7"),
    ("iso88598", "iso-8859-8"),
    ("iso88599", "windows-1254"),
    ("iso_8859-1", "windows-1252"),
    ("iso_8859-15", "iso-8859-15"),
    ("iso_8859-1:1987", "windows-1252"),
    ("iso_8859-2", "iso-8859-2"),
    ("iso_8859-2:1987", "iso-8859-2"),
    ("iso_8859-3", "iso-8859-3"),
    ("iso_8859-3:1988", "iso-8859-3"),
    ("iso_8859-4", "iso-8859-4"),
    ("iso_8859-4:1988", "iso-8859-4"),
    ("iso_8859-5", "iso-8859-5"),
    ("iso_8859-5:1988", "iso-8859-5"),
    ("iso_8859-6", "iso-8859-6"),
    ("iso_8859-6:1987", "iso-8859-6"),
    ("iso_8859-7", "iso-8859-7"),
    ("iso_8859-7:1987", "iso-8859-7"),
    ("iso_8859-8", "iso-8859-8"),
    ("iso_8859-8:1988", "iso-8859-8"),
    ("iso_8859-9", "windows-1254"),
    ("iso_8859-9:1989", "windows-1254"),
    ("koi", "koi8-r"),
    ("koi8", "koi8-r"),
    ("koi8-r", "koi8-r"),
    ("koi8-ru", "koi8-u"),
    ("koi8-u", "koi8-u"),
    ("koi8_r", "koi8-r"),
    ("korean", "euc-kr"),
    ("ks_c_5601-1987", "euc-kr"),
    ("ks_c_5601-1989", "euc-kr"),
    ("ksc5601", "euc-kr"),
    ("ksc_5601", "euc-kr"),
    ("l1", "windows-1252"),
    ("l2", "iso-8859-2"),
    ("l3", "iso-8859-3"),
    ("l4", "iso-8859-4"),
    ("l5", "windows-1254"),
    ("l6", "iso-8859-10"),
    ("l9", "iso-8859-15"),
    ("latin1", "windows-1252"),
    ("latin2", "iso-8859-2"),
    ("latin3", "iso-8859-3"),
    ("latin4", "iso-8859-4"),
    ("latin5", "windows-1254"),
    ("latin6", "iso-8859-10"),
    ("logical", "iso-8859-8-i"),
    ("mac", "macintosh"),
    ("macintosh", "macintosh"),
    ("ms932", "shift-jis"),
    ("ms_kanji", "shift-jis"),
    ("replacement", "replacement"),
    ("shift-jis", "shift-jis"),
    ("shift_jis", "shift-jis"),
    ("sjis", "shift-jis"),
    ("sun_eu_greek", "iso-8859-7"),
    ("tis-620", "windows-874"),
    ("ucs-2", "utf-16le"),
    ("unicode", "utf-16le"),
    ("unicode-1-1-utf-8", "utf-8"),
    ("unicode11utf8", "utf-8"),
    ("unicode20utf8", "utf-8"),
    ("unicodefeff", "utf-16le"),
    ("unicodefffe", "utf-16be"),
    ("us-ascii", "windows-1252"),
    ("utf-16", "utf-16le"),
    ("utf-16be", "utf-16be"),
    ("utf-16le", "utf-16le"),
    ("utf-8", "utf-8"),
    ("utf8", "utf-8"),
    ("visual", "iso-8859-8"),
    ("windows-1250", "windows-1250"),
    ("windows-1251", "windows-1251"),
    ("windows-1252", "windows-1252"),
    ("windows-1253", "windows-1253"),
    ("windows-1254", "windows-1254"),
    ("windows-1255", "windows-1255"),
    ("windows-1256", "windows-1256"),
    ("windows-1257", "windows-1257"),
    ("windows-1258", "windows-1258"),
    ("windows-31j", "shift-jis"),
    ("windows-874", "windows-874"),
    ("windows-949", "euc-kr"),
    ("x-cp1250", "windows-1250"),
    ("x-cp1251", "windows-1251"),
    ("x-cp1252", "windows-1252"),
    ("x-cp1253", "windows-1253"),
    ("x-cp1254", "windows-1254"),
    ("x-cp1255", "windows-1255"),
    ("x-cp1256", "windows-1256"),
    ("x-cp1257", "windows-1257"),
    ("x-cp1258", "windows-1258"),
    ("x-euc-jp", "euc-jp"),
    ("x-gbk", "gbk"),
    ("x-mac-cyrillic", "x-mac-cyrillic"),
    ("x-mac-roman", "macintosh"),
    ("x-sjis", "shift-jis"),
    ("x-unicode20utf8", "utf-8"),
    ("x-user-defined", "x-user-defined"),
    ("x-x-big5", "big5"),
];

/// Return the canonical encoding name for one normalized HTML label.
fn find_canonical_encoding_name(normalized_label: &str) -> Option<&'static str> {
    let index = HTML_ENCODING_LABELS
        .binary_search_by_key(&normalized_label, |(label, _)| *label)
        .ok()?;

    Some(HTML_ENCODING_LABELS[index].1)
}

/// One byte-level HTML encoding prescanner.
#[cfg(test)]
pub(crate) struct EncodingScanner<'a> {
    /// The raw bytes to scan.
    bytes: &'a [u8],
    /// The current byte position.
    position: usize,
    /// Whether the scanner is inside tag markup.
    is_in_tag: bool,
    /// The current quoted attribute delimiter.
    quote: Option<u8>,
}

#[cfg(test)]
impl<'a> EncodingScanner<'a> {
    /// Create one encoding prescanner over one byte slice.
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            is_in_tag: false,
            quote: None,
        }
    }

    /// Return one encoding label found by the HTML prescan algorithm.
    pub(crate) fn scan_document_encoding(mut self) -> Option<String> {
        // utf 8 bom
        if self.bytes.starts_with(UTF_8_BOM) {
            return Some("utf-8".to_string());
        }

        // meta scan
        while let Some(tag_start) = self.find_meta_start() {
            let tag_bytes = &self.bytes[tag_start + "<meta".len()..];
            let scanner = MetaEncodingScanner::new(tag_bytes);

            if let Some(encoding) = scanner.scan()
                && let Some(encoding) = MetaEncodingScanner::canonical_label(&encoding)
            {
                return Some(encoding.to_string());
            }

            self.position = self.advance_past_tag(tag_start + "<meta".len());
        }

        None
    }

    /// Return the next `<meta` start tag position.
    fn find_meta_start(&mut self) -> Option<usize> {
        while self.position + "<meta".len() <= self.bytes.len() {
            let byte = self.bytes[self.position];

            // quoted attribute value
            if let Some(expected_quote) = self.quote {
                if byte == expected_quote {
                    self.quote = None;
                }

                self.position += 1;
                continue;
            }

            // tag body
            if self.is_in_tag {
                match byte {
                    b'"' | b'\'' => self.quote = Some(byte),
                    b'>' => self.is_in_tag = false,
                    _ => {}
                }

                self.position += 1;
                continue;
            }

            // non markup bytes
            if byte != b'<' {
                self.position += 1;
                continue;
            }

            // matching meta start
            let name_end = self.position + "<meta".len();
            let candidate = &self.bytes[self.position + 1..name_end];
            let boundary = self.bytes.get(name_end).copied();

            if candidate.eq_ignore_ascii_case(b"meta")
                && boundary
                    .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'>'))
            {
                return Some(self.position);
            }

            // non-meta tag starts
            if let Some(next) = self.bytes.get(self.position + 1).copied()
                && matches!(next, b'!' | b'/' | b'?' | b'A'..=b'Z' | b'a'..=b'z')
            {
                self.is_in_tag = true;
            }

            self.position += 1;
        }

        None
    }

    /// Return the next byte position after the current tag body.
    fn advance_past_tag(&self, mut position: usize) -> usize {
        let mut quote = None;

        while position < self.bytes.len() {
            let byte = self.bytes[position];

            if let Some(expected_quote) = quote {
                if byte == expected_quote {
                    quote = None;
                }

                position += 1;
                continue;
            }

            match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'>' => return position + 1,
                _ => {}
            }

            position += 1;
        }

        self.bytes.len()
    }
}

/// One meta-tag attribute prescanner.
pub(crate) struct MetaEncodingScanner<'a> {
    /// The raw bytes after one `<meta`.
    bytes: &'a [u8],
    #[cfg(test)]
    /// The current byte position.
    position: usize,
    #[cfg(test)]
    /// The first `charset` attribute value.
    charset: Option<String>,
    #[cfg(test)]
    /// The first `content` attribute value.
    content: Option<String>,
    #[cfg(test)]
    /// Whether `http-equiv` resolved to `content-type`.
    is_content_type: bool,
    #[cfg(test)]
    /// Whether the tag ended cleanly.
    is_closed: bool,
}

impl<'a> MetaEncodingScanner<'a> {
    /// Create one meta-tag prescanner.
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            #[cfg(test)]
            position: 0,
            #[cfg(test)]
            charset: None,
            #[cfg(test)]
            content: None,
            #[cfg(test)]
            is_content_type: false,
            #[cfg(test)]
            is_closed: false,
        }
    }

    /// Create one meta-content prescanner from one text buffer.
    pub(crate) fn new_content(content: &'a HtmlString) -> Self {
        Self::new(content.as_bytes())
    }

    /// Resolve one HTML encoding label into one canonical encoding name.
    pub(crate) fn canonical_label(label: &str) -> Option<HtmlString> {
        let normalized_label = label
            .trim_matches(|character: char| character.is_ascii_whitespace() || character == '\0')
            .to_ascii_lowercase();
        let canonical_name = find_canonical_encoding_name(&normalized_label)?;

        if canonical_name == UTF_16_BE_NAME || canonical_name == UTF_16_LE_NAME {
            return Some(HtmlString::from_slice("utf-8"));
        }

        Some(HtmlString::from_slice(canonical_name))
    }

    /// Return one encoding label found in one meta tag.
    #[cfg(test)]
    fn scan(mut self) -> Option<String> {
        // attribute scan
        while self.position < self.bytes.len() {
            self.position = self.skip_ascii_whitespace(self.position);

            if self.position >= self.bytes.len() {
                break;
            }

            // tag end
            if self.bytes[self.position] == b'>' {
                self.is_closed = true;
                break;
            }

            // self closing marker
            if self.bytes[self.position] == b'/' {
                self.position += 1;
                continue;
            }

            let Some((name, next_position)) = self.scan_attribute_name(self.position) else {
                self.position += 1;
                continue;
            };
            self.position = self.skip_ascii_whitespace(next_position);

            let Some((value, next_position, can_continue)) =
                self.scan_attribute_value(self.position)
            else {
                break;
            };
            self.position = next_position;

            // charset wins
            if name.eq_ignore_ascii_case("charset")
                && let Some(value) = value
            {
                if self.charset.is_none() {
                    self.charset = Some(value);
                }

                continue;
            }

            // content type flag
            if name.eq_ignore_ascii_case("http-equiv")
                && let Some(value) = value.as_deref()
            {
                self.is_content_type = value.eq_ignore_ascii_case("content-type");
            }

            // content payload
            if name.eq_ignore_ascii_case("content") && self.content.is_none() {
                self.content = value;
            }

            // malformed quoted attributes stop the scan
            if !can_continue {
                break;
            }
        }

        // tag must close cleanly
        if !self.is_closed {
            return None;
        }

        // direct charset
        if let Some(charset) = self.charset {
            return Some(charset);
        }

        // content type fallback
        if self.is_content_type
            && let Some(content) = self.content
        {
            let content = HtmlString::from_slice(&content);
            return MetaEncodingScanner::new_content(&content)
                .scan_content_encoding()
                .map(|value| value.to_string());
        }

        None
    }

    /// Return one encoding label from one meta content attribute payload.
    pub(crate) fn scan_content_encoding(&self) -> Option<HtmlString> {
        let bytes = self.bytes;

        // find charset
        let mut position = 0;

        loop {
            loop {
                let candidate = bytes.get(position..position + "charset".len())?;

                if candidate.eq_ignore_ascii_case(b"charset") {
                    break;
                }

                position += 1;
            }

            position += "charset".len();
            position += bytes[position..]
                .iter()
                .take_while(|byte| byte.is_ascii_whitespace())
                .count();

            if bytes.get(position).copied() == Some(b'=') {
                break;
            }
        }

        // equals and whitespace
        position += 1;
        position += bytes[position..]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();

        // quoted or unquoted value
        match bytes.get(position)? {
            quote @ (b'"' | b'\'') => {
                let quoted_start = position + 1;
                let quoted_end = bytes[quoted_start..]
                    .iter()
                    .position(|byte| byte == quote)
                    .map(|offset| quoted_start + offset);

                if let Some(quoted_end) = quoted_end {
                    let value = &bytes[quoted_start..quoted_end];
                    return Some(HtmlString::from(
                        String::from_utf8_lossy(value).into_owned(),
                    ));
                }

                None
            }
            _ => {
                let value_end = bytes[position..]
                    .iter()
                    .position(|byte| byte.is_ascii_whitespace() || *byte == b';')
                    .map(|offset| position + offset)
                    .unwrap_or(bytes.len());
                let value = &bytes[position..value_end];

                Some(HtmlString::from(
                    String::from_utf8_lossy(value).into_owned(),
                ))
            }
        }
    }

    /// Return one attribute name and the next cursor position.
    #[cfg(test)]
    fn scan_attribute_name(&self, position: usize) -> Option<(String, usize)> {
        let mut name = String::new();
        let mut position = position;

        // attribute name bytes
        while position < self.bytes.len() {
            let byte = self.bytes[position];

            if matches!(byte, b'\t' | b'\n' | b'\x0C' | b' ' | b'/' | b'=' | b'>') {
                break;
            }

            if matches!(byte, b'"' | b'\'' | b'<' | b'\0') {
                return None;
            }

            name.push((byte as char).to_ascii_lowercase());
            position += 1;
        }

        if name.is_empty() {
            return None;
        }

        Some((name, position))
    }

    /// Return one attribute value, the next cursor position, and whether the tag can keep scanning.
    #[cfg(test)]
    fn scan_attribute_value(&self, position: usize) -> Option<(Option<String>, usize, bool)> {
        let mut position = self.skip_ascii_whitespace(position);

        // missing equals
        if position >= self.bytes.len() || self.bytes[position] != b'=' {
            return Some((None, position, true));
        }

        position += 1;
        position = self.skip_ascii_whitespace(position);

        if position >= self.bytes.len() {
            return Some((Some(String::new()), position, true));
        }

        let quote = self.bytes[position];

        // quoted value
        if matches!(quote, b'"' | b'\'') {
            position += 1;

            let value_start = position;
            let value_end = self.bytes[value_start..]
                .iter()
                .position(|byte| *byte == quote)
                .map(|offset| value_start + offset)
                .unwrap_or(self.bytes.len());
            let value = self.decode_latin1_bytes(&self.bytes[value_start..value_end]);
            let next_position = value_end.saturating_add(1);
            let can_continue = next_position >= self.bytes.len()
                || matches!(
                    self.bytes[next_position],
                    b'\t' | b'\n' | b'\x0C' | b' ' | b'/' | b'>'
                );

            return Some((Some(value), next_position, can_continue));
        }

        // unquoted value
        let value_end = self.bytes[position..]
            .iter()
            .position(|byte| matches!(byte, b'\t' | b'\n' | b'\x0C' | b' ' | b'>'))
            .map(|offset| position + offset)
            .unwrap_or(self.bytes.len());
        let value = self.decode_latin1_bytes(&self.bytes[position..value_end]);

        Some((Some(value), value_end, true))
    }

    /// Return the next byte position after ASCII whitespace.
    #[cfg(test)]
    fn skip_ascii_whitespace(&self, mut position: usize) -> usize {
        while position < self.bytes.len() && self.bytes[position].is_ascii_whitespace() {
            position += 1;
        }

        position
    }

    /// Decode one raw byte slice with one byte-preserving mapping.
    #[cfg(test)]
    fn decode_latin1_bytes(&self, bytes: &[u8]) -> String {
        bytes.iter().map(|byte| char::from(*byte)).collect()
    }
}
