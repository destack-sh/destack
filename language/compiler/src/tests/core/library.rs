use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, ProfileId, TargetId};

use crate::tests::TestSession;

/// Report every builtin module's outcome for one per-module artifact, in declaration order.
pub(crate) fn library_report(
    make_key: impl Fn(ModuleId, ProfileId, TargetId) -> ArtifactKey,
) -> String {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let package_id = package.package_id();
    let target = TargetId::new(package_id, "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    // build every module concurrently before the ordered per-module readout
    let keys: Vec<_> = package
        .files()
        .iter()
        .map(|file| make_key(file.module_id(), profile, target))
        .collect();
    let _ = session.require_all(keys.iter().copied());

    // report each module's outcome in declaration order
    let mut report = String::new();
    for file in package.files() {
        let key = make_key(file.module_id(), profile, target);
        let Err(error) = session.require_all([key]) else {
            report.push_str(&format!("ok   {}\n", file.path));

            continue;
        };

        // keep the rendered diagnostics, or the trailing error text when none render
        let rendered = session.render_terminal_diagnostics_for(&[key]);
        let rendered = strip_terminal_styles(&rendered);
        let mut lines = error_rows(&rendered).into_iter().peekable();
        if lines.peek().is_none() {
            let message = error.to_string();
            let message = match message.rsplit_once("}: ") {
                Some((_, tail)) => tail.to_string(),
                None => message,
            };
            let message = message.lines().next().unwrap_or_default();
            report.push_str(&format!("FAIL {} :: {message}\n", file.path));
        }
        let mut reported = Vec::new();
        for line in lines {
            if reported.contains(&line) {
                continue;
            }

            reported.push(line.clone());
            report.push_str(&format!("FAIL {} :: {line}\n", file.path));
        }
    }

    report
}

/// Remove the terminal color codes from one rendered report.
fn strip_terminal_styles(rendered: &str) -> String {
    let mut stripped = String::with_capacity(rendered.len());
    let mut in_escape = false;
    for character in rendered.chars() {
        match (in_escape, character) {
            (false, '\u{1b}') => in_escape = true,
            (false, character) => stripped.push(character),
            (true, 'm') => in_escape = false,
            (true, _) => {}
        }
    }

    stripped
}

/// Return each rendered error header joined with the location line that follows it.
fn error_rows(rendered: &str) -> Vec<String> {
    let lines: Vec<&str> = rendered.lines().collect();
    let mut rows = Vec::new();

    // append the source location the renderer prints beneath each header
    for (index, line) in lines.iter().enumerate() {
        if !line.contains("error[") {
            continue;
        }
        let location = lines
            .get(index + 1)
            .and_then(|next| next.trim().strip_prefix("──▶ "))
            .map(|location| format!(" @ {location}"))
            .unwrap_or_default();
        rows.push(format!("{}{location}", line.trim()));
    }

    rows
}
