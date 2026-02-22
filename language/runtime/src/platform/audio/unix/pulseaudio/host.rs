use std::process::Command;

use super::constants::PULSEAUDIO_DEFAULT_DEVICE_NAME;

/// One probed PulseAudio endpoint snapshot.
#[derive(Debug, Clone)]
pub(super) struct PulseaudioEndpointSnapshot {
    /// Enumerated playback endpoint names.
    pub(super) playback_names: Vec<String>,
    /// Enumerated capture endpoint names.
    pub(super) capture_names: Vec<String>,
    /// Enumerated loopback monitor endpoint names.
    pub(super) loopback_names: Vec<String>,
    /// The default playback endpoint name.
    pub(super) default_playback_name: String,
    /// The default capture endpoint name.
    pub(super) default_capture_name: String,
    /// The default loopback endpoint name.
    pub(super) default_loopback_name: String,
}

/// Probe PulseAudio endpoint names through pactl.
pub(super) fn probe_endpoints() -> Option<PulseaudioEndpointSnapshot> {
    // query sink and source endpoint tables
    let sinks_output = run_pactl(&["list", "short", "sinks"])?;
    let sources_output = run_pactl(&["list", "short", "sources"])?;

    // parse endpoint names from tab-separated rows
    let mut playback_names = parse_short_list_names(&sinks_output);
    let source_names = parse_short_list_names(&sources_output);
    let capture_names = source_names
        .iter()
        .filter(|name| !is_monitor_source(name))
        .cloned()
        .collect::<Vec<_>>();
    let loopback_names = source_names
        .iter()
        .filter(|name| is_monitor_source(name))
        .cloned()
        .collect::<Vec<_>>();

    // query default routes for stable default markers
    let info_output = run_pactl(&["info"]).unwrap_or_default();
    let default_playback_name = parse_info_key(&info_output, "Default Sink").unwrap_or_else(|| {
        playback_names
            .first()
            .cloned()
            .unwrap_or_else(|| PULSEAUDIO_DEFAULT_DEVICE_NAME.to_string())
    });
    let default_capture_name =
        parse_info_key(&info_output, "Default Source").unwrap_or_else(|| {
            capture_names
                .first()
                .cloned()
                .unwrap_or_else(|| PULSEAUDIO_DEFAULT_DEVICE_NAME.to_string())
        });
    let default_loopback_name = format!("{default_playback_name}.monitor");

    // ensure defaults are included in the descriptor rows
    push_unique_name(&mut playback_names, &default_playback_name);
    let mut capture_names = capture_names;
    push_unique_name(&mut capture_names, &default_capture_name);
    let mut loopback_names = loopback_names;
    if !loopback_names.is_empty() {
        push_unique_name(&mut loopback_names, &default_loopback_name);
    }

    Some(PulseaudioEndpointSnapshot {
        playback_names,
        capture_names,
        loopback_names,
        default_playback_name,
        default_capture_name,
        default_loopback_name,
    })
}

/// Run pactl with one argument vector and return utf8 output.
fn run_pactl(arguments: &[&str]) -> Option<String> {
    // run one pactl command invocation
    let output = Command::new("pactl").args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }

    // decode one stdout payload into utf8 text
    String::from_utf8(output.stdout).ok()
}

/// Parse one pulse short-list payload into endpoint names.
fn parse_short_list_names(output: &str) -> Vec<String> {
    let mut names = Vec::new();

    // parse one line at a time with stable tab-separated columns
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // ignore one malformed row missing the endpoint name column
        let mut columns = line.split('\t');
        let _index = columns.next();
        let Some(name) = columns.next() else {
            continue;
        };

        let name = name.trim();
        if name.is_empty() {
            continue;
        }

        names.push(name.to_string());
    }

    names
}

/// Parse one pactl info key from one multiline text payload.
fn parse_info_key(output: &str, key: &str) -> Option<String> {
    // parse one key line from one colon-separated info payload
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((line_key, value)) = line.split_once(':') else {
            continue;
        };
        if line_key.trim() != key {
            continue;
        }

        let value = value.trim();
        if value.is_empty() {
            return None;
        }

        return Some(value.to_string());
    }

    None
}

/// Return whether one source name is one monitor lane.
fn is_monitor_source(source_name: &str) -> bool {
    source_name.ends_with(".monitor")
}

/// Append one endpoint name when it is not already present.
fn push_unique_name(names: &mut Vec<String>, candidate: &str) {
    if candidate.is_empty() {
        return;
    }

    if names.iter().any(|name| name == candidate) {
        return;
    }

    names.push(candidate.to_string());
}
