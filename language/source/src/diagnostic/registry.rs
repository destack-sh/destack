use tspp_core::StringId;

/// Diagnostics available to one compilation session.
#[derive(Debug)]
pub struct DiagnosticRegistry {
    /// Sorted diagnostic ids and their controllability.
    diagnostics: Box<[(StringId, Box<str>, bool)]>,
}

impl DiagnosticRegistry {
    /// Index globally unique diagnostic ids.
    pub fn new<'a>(diagnostics: impl IntoIterator<Item = (&'a str, bool)>) -> Self {
        let mut diagnostics = diagnostics
            .into_iter()
            .map(|(id, is_controllable)| {
                // require nonempty kebab-case segments
                let is_valid = id.split('-').all(|segment| {
                    !segment.is_empty()
                        && segment
                            .bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                });
                assert!(is_valid, "diagnostic id '{id}' is not kebab-case");

                (id.to_string(), is_controllable)
            })
            .collect::<Vec<_>>();

        // reject duplicate ids
        diagnostics.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        if let Some(pair) = diagnostics.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            panic!("diagnostic id '{}' is registered more than once", pair[0].0);
        }

        // reject StringId collisions
        let mut diagnostics = diagnostics
            .into_iter()
            .map(|(id, is_controllable)| (StringId::for_text(&id), id, is_controllable))
            .collect::<Vec<_>>();
        diagnostics.sort_unstable_by_key(|(diagnostic, _, _)| *diagnostic);
        if let Some(pair) = diagnostics.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            panic!(
                "diagnostic ids '{}' and '{}' have the same StringId",
                pair[0].1, pair[1].1,
            );
        }

        let diagnostics = diagnostics
            .into_iter()
            .map(|(diagnostic, id, is_controllable)| {
                (diagnostic, id.into_boxed_str(), is_controllable)
            })
            .collect::<Vec<_>>();

        Self {
            diagnostics: diagnostics.into_boxed_slice(),
        }
    }

    /// Return whether one registered diagnostic may be controlled.
    pub fn is_controllable(&self, id: &str) -> Option<bool> {
        let diagnostic = StringId::for_text(id);
        let index = self
            .diagnostics
            .binary_search_by_key(&diagnostic, |(registered, _, _)| *registered)
            .ok()?;
        let (_, registered, is_controllable) = &self.diagnostics[index];
        if registered.as_ref() != id {
            return None;
        }

        Some(*is_controllable)
    }
}
