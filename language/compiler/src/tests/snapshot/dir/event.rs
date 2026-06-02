use super::DirSnapshotBuilder;
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl DirSnapshotBuilder<'_> {
    /// Add event snapshot rows for selected prefixes.
    pub(crate) fn add_events(&mut self, prefixes: &[&'static str], content: &str) {
        let events = parse_events(content);
        let mut matched = false;

        // add events for each selected prefix
        for prefix in prefixes {
            matched |= self.add_event_rows(prefix, &events);
        }

        assert!(
            matched,
            "event prefixes {prefixes:?} did not match any rows"
        );
    }

    /// Add event rows matching one prefix.
    fn add_event_rows(&mut self, prefix: &'static str, events: &[EventEntry<'_>]) -> bool {
        let (phase, event_prefix) = event_row_prefix(prefix);
        let mut matched = false;

        // add matching event lines in trace order
        for (index, event) in events.iter().enumerate() {
            if !event.name.starts_with(event_prefix) {
                continue;
            }

            let row = SnapshotRow::new(SnapshotAnchor::End, phase, "events")
                .field("index", index.to_string())
                .field("event", event.name)
                .field("fields", event.fields);

            self.push(row);
            matched = true;
        }

        matched
    }
}

/// One parsed event log line.
struct EventEntry<'a> {
    /// The event name.
    name: &'a str,
    /// The raw field payload.
    fields: &'a str,
}

/// Parse event log lines.
fn parse_events(content: &str) -> Vec<EventEntry<'_>> {
    let mut events = Vec::new();

    // parse one event per line
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (name, fields) = line.split_once(' ').unwrap_or((line, ""));
        events.push(EventEntry { name, fields });
    }

    events
}

/// Return the phase and event name prefix selected by one row prefix.
fn event_row_prefix(prefix: &'static str) -> (&'static str, &'static str) {
    let Some((phase, rest)) = prefix.split_once(".events") else {
        panic!("event row prefix `{prefix}` must include an events segment")
    };
    let event = match rest {
        "" => "",
        rest => rest
            .strip_prefix('.')
            .unwrap_or_else(|| panic!("event row prefix `{prefix}` must use dotted event names")),
    };

    (phase, event)
}
