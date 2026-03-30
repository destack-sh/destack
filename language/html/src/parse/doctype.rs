use crate::lex::{Doctype, HtmlString};

use super::parser::{LimitedQuirks, NoQuirks, Quirks, QuirksMode};

// these should all be lowercase for ASCII-insensitive matching
static QUIRKY_PUBLIC_PREFIXES: &[&str] = &[
    "-//advasoft ltd//dtd html 3.0 aswedit + extensions//",
    "-//as//dtd html 3.0 aswedit + extensions//",
    "-//ietf//dtd html 2.0 level 1//",
    "-//ietf//dtd html 2.0 level 2//",
    "-//ietf//dtd html 2.0 strict level 1//",
    "-//ietf//dtd html 2.0 strict level 2//",
    "-//ietf//dtd html 2.0 strict//",
    "-//ietf//dtd html 2.0//",
    "-//ietf//dtd html 2.1e//",
    "-//ietf//dtd html 3.0//",
    "-//ietf//dtd html 3.2 final//",
    "-//ietf//dtd html 3.2//",
    "-//ietf//dtd html 3//",
    "-//ietf//dtd html level 0//",
    "-//ietf//dtd html level 1//",
    "-//ietf//dtd html level 2//",
    "-//ietf//dtd html level 3//",
    "-//ietf//dtd html strict level 0//",
    "-//ietf//dtd html strict level 1//",
    "-//ietf//dtd html strict level 2//",
    "-//ietf//dtd html strict level 3//",
    "-//ietf//dtd html strict//",
    "-//ietf//dtd html//",
    "-//metrius//dtd metrius presentational//",
    "-//microsoft//dtd internet explorer 2.0 html strict//",
    "-//microsoft//dtd internet explorer 2.0 html//",
    "-//microsoft//dtd internet explorer 2.0 tables//",
    "-//microsoft//dtd internet explorer 3.0 html strict//",
    "-//microsoft//dtd internet explorer 3.0 html//",
    "-//microsoft//dtd internet explorer 3.0 tables//",
    "-//netscape comm. corp.//dtd html//",
    "-//netscape comm. corp.//dtd strict html//",
    "-//o'reilly and associates//dtd html 2.0//",
    "-//o'reilly and associates//dtd html extended 1.0//",
    "-//o'reilly and associates//dtd html extended relaxed 1.0//",
    "-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//",
    "-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//",
    "-//spyglass//dtd html 2.0 extended//",
    "-//sq//dtd html 2.0 hotmetal + extensions//",
    "-//sun microsystems corp.//dtd hotjava html//",
    "-//sun microsystems corp.//dtd hotjava strict html//",
    "-//w3c//dtd html 3 1995-03-24//",
    "-//w3c//dtd html 3.2 draft//",
    "-//w3c//dtd html 3.2 final//",
    "-//w3c//dtd html 3.2//",
    "-//w3c//dtd html 3.2s draft//",
    "-//w3c//dtd html 4.0 frameset//",
    "-//w3c//dtd html 4.0 transitional//",
    "-//w3c//dtd html experimental 19960712//",
    "-//w3c//dtd html experimental 970421//",
    "-//w3c//dtd w3 html//",
    "-//w3o//dtd w3 html 3.0//",
    "-//webtechs//dtd mozilla html 2.0//",
    "-//webtechs//dtd mozilla html//",
];

static QUIRKY_PUBLIC_MATCHES: &[&str] = &[
    "-//w3o//dtd w3 html strict 3.0//en//",
    "-/w3c/dtd html 4.0 transitional/en",
    "html",
];

static QUIRKY_SYSTEM_MATCHES: &[&str] =
    &["http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd"];

static LIMITED_QUIRKY_PUBLIC_PREFIXES: &[&str] = &[
    "-//w3c//dtd xhtml 1.0 frameset//",
    "-//w3c//dtd xhtml 1.0 transitional//",
];

static HTML4_PUBLIC_PREFIXES: &[&str] = &[
    "-//w3c//dtd html 4.01 frameset//",
    "-//w3c//dtd html 4.01 transitional//",
];

/// Return one optional HTML string as one borrowed string slice.
fn optional_html_string_as_slice(value: &Option<HtmlString>) -> Option<&str> {
    value.as_deref()
}

/// Lower one optional string to lowercase for ASCII matching.
fn optional_ascii_lower(value: Option<&str>) -> Option<String> {
    value.map(str::to_ascii_lowercase)
}

/// Return whether one string starts with any listed prefix.
fn contains_prefix(candidates: &[&str], value: &str) -> bool {
    candidates
        .iter()
        .any(|&candidate| value.starts_with(candidate))
}

/// Return whether one doctype is erroneous and which quirks mode it implies.
pub(crate) fn doctype_error_and_quirks(
    doctype: &Doctype,
    iframe_srcdoc: bool,
) -> (bool, QuirksMode) {
    let name = optional_html_string_as_slice(&doctype.name);
    let public = optional_html_string_as_slice(&doctype.public_id);
    let system = optional_html_string_as_slice(&doctype.system_id);

    // doctype conformance error
    let is_error = !matches!(
        (name, public, system),
        (Some("html"), None, None)
            | (Some("html"), None, Some("about:legacy-compat"))
            | (Some("html"), Some("-//W3C//DTD HTML 4.0//EN"), None)
            | (
                Some("html"),
                Some("-//W3C//DTD HTML 4.0//EN"),
                Some("http://www.w3.org/TR/REC-html40/strict.dtd"),
            )
            | (Some("html"), Some("-//W3C//DTD HTML 4.01//EN"), None)
            | (
                Some("html"),
                Some("-//W3C//DTD HTML 4.01//EN"),
                Some("http://www.w3.org/TR/html4/strict.dtd"),
            )
            | (
                Some("html"),
                Some("-//W3C//DTD XHTML 1.0 Strict//EN"),
                Some("http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd"),
            )
            | (
                Some("html"),
                Some("-//W3C//DTD XHTML 1.1//EN"),
                Some("http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd"),
            )
    );

    // quirks-mode matches are case-insensitive
    let public = optional_ascii_lower(public);
    let system = optional_ascii_lower(system);

    // quirks-mode classification
    let quirks_mode = match (public.as_deref(), system.as_deref()) {
        _ if doctype.force_quirks => Quirks,
        _ if name != Some("html") => Quirks,

        _ if iframe_srcdoc => NoQuirks,

        (Some(ref p), _) if QUIRKY_PUBLIC_MATCHES.contains(p) => Quirks,
        (_, Some(ref s)) if QUIRKY_SYSTEM_MATCHES.contains(s) => Quirks,

        (Some(public_id), _) if contains_prefix(QUIRKY_PUBLIC_PREFIXES, public_id) => Quirks,
        (Some(public_id), _) if contains_prefix(LIMITED_QUIRKY_PUBLIC_PREFIXES, public_id) => {
            LimitedQuirks
        }

        (Some(public_id), system_id) if contains_prefix(HTML4_PUBLIC_PREFIXES, public_id) => {
            match system_id {
                None => Quirks,
                Some(_) => LimitedQuirks,
            }
        }

        _ => NoQuirks,
    };

    (is_error, quirks_mode)
}
