use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::os::abi_generated::{
    BackgroundTaskOptionsValue, BackgroundTaskScheduleKindValue,
};
use crate::platform::os::background::storage::background_interval_seconds;

/// Render one launchd plist for one wrapper script.
pub(super) fn render_launchd_plist(
    label: &str,
    script_path: &Path,
    options: &BackgroundTaskOptionsValue,
    scheduled_unix_ns: Option<u64>,
    is_run_at_load: bool,
) -> RuntimeResult<String> {
    let label = plist_text(label);
    let script_path = plist_text(&script_path.to_string_lossy());
    let run_at_load = if is_run_at_load { "true" } else { "false" };
    let schedule_fragment = match options.schedule.kind {
        // one-shot launchd timers still use a single calendar firing
        BackgroundTaskScheduleKindValue::Once => {
            if let Some(scheduled_unix_ns) = scheduled_unix_ns {
                let scheduled = launchd_calendar_components(scheduled_unix_ns)?;

                format!(
                    "  <key>StartCalendarInterval</key>\n\
  <dict>\n\
    <key>Month</key>\n\
    <integer>{}</integer>\n\
    <key>Day</key>\n\
    <integer>{}</integer>\n\
    <key>Hour</key>\n\
    <integer>{}</integer>\n\
    <key>Minute</key>\n\
    <integer>{}</integer>\n\
  </dict>\n",
                    scheduled.month, scheduled.day, scheduled.hour, scheduled.minute,
                )
            } else {
                String::new()
            }
        }

        // recurring launchd timers use one scheduler-owned interval
        BackgroundTaskScheduleKindValue::Recurring => {
            let interval_seconds = background_interval_seconds(options);

            format!(
                "  <key>StartInterval</key>\n\
  <integer>{interval_seconds}</integer>\n"
            )
        }
    };
    let launch_only_once_fragment =
        if options.schedule.kind == BackgroundTaskScheduleKindValue::Once {
            "  <key>LaunchOnlyOnce</key>\n  <true/>\n"
        } else {
            ""
        };

    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{label}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
    <string>{script_path}</string>\n\
  </array>\n\
  <key>RunAtLoad</key>\n\
  <{run_at_load}/>\n\
{schedule_fragment}{launch_only_once_fragment}\
</dict>\n\
</plist>\n"
    ))
}

/// One minute-granularity launchd calendar payload.
struct LaunchdCalendarComponents {
    /// The local month component.
    month: i8,
    /// The local day component.
    day: i8,
    /// The local hour component.
    hour: i8,
    /// The local minute component.
    minute: i8,
}

/// Return one minute-granularity launchd calendar payload for one Unix timestamp.
fn launchd_calendar_components(unix_ns: u64) -> RuntimeResult<LaunchdCalendarComponents> {
    let rounded_unix_ns =
        unix_ns.saturating_add(60_000_000_000 - 1) / 60_000_000_000 * 60_000_000_000;
    let timestamp = Timestamp::from_nanosecond(i128::from(rounded_unix_ns)).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "schedule.earliestBeginUnixNs",
            format!("desktop background timestamp is invalid: {error}"),
        ))
        .boxed()
    })?;
    let zoned = timestamp.to_zoned(TimeZone::system());

    Ok(LaunchdCalendarComponents {
        month: zoned.month(),
        day: zoned.day(),
        hour: zoned.hour(),
        minute: zoned.minute(),
    })
}

/// Escape one launchd plist text payload.
fn plist_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::render_launchd_plist;
    use crate::platform::os::abi_generated::{
        BackgroundConflictPolicyValue, BackgroundNetworkRequirementValue,
        BackgroundTaskOptionsValue, BackgroundTaskScheduleKindValue, BackgroundTaskScheduleValue,
        BackgroundTriggerKindValue,
    };

    /// Render one launchd plist with escaped text values.
    #[test]
    fn test_render_launchd_plist_escapes_text_payloads() {
        let plist = render_launchd_plist(
            "destack<&>",
            Path::new("/tmp/a&b<script>"),
            &background_options(BackgroundTaskScheduleKindValue::Once, None),
            Some(1_700_000_000_000_000_000),
            false,
        )
        .expect("launchd plist should render");

        assert!(plist.contains("<string>destack&lt;&amp;&gt;</string>"));
        assert!(plist.contains("<string>/tmp/a&amp;b&lt;script&gt;</string>"));
        assert!(plist.contains("<key>StartCalendarInterval</key>"));
        assert!(plist.contains("<key>LaunchOnlyOnce</key>"));
    }

    /// Render one immediate plist with one run-at-load launch.
    #[test]
    fn test_render_launchd_plist_renders_immediate_launch() {
        let plist = render_launchd_plist(
            "destack",
            Path::new("/tmp/task"),
            &background_options(BackgroundTaskScheduleKindValue::Once, None),
            None,
            true,
        )
        .expect("launchd plist should render");

        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        assert!(!plist.contains("<key>StartCalendarInterval</key>"));
    }

    /// Render one recurring plist with one scheduler-owned interval.
    #[test]
    fn test_render_launchd_plist_renders_recurring_interval() {
        let plist = render_launchd_plist(
            "destack",
            Path::new("/tmp/task"),
            &background_options(
                BackgroundTaskScheduleKindValue::Recurring,
                Some(300_000_000_000),
            ),
            None,
            true,
        )
        .expect("launchd plist should render");

        assert!(plist.contains("<key>StartInterval</key>"));
        assert!(plist.contains("<integer>300</integer>"));
        assert!(!plist.contains("<key>LaunchOnlyOnce</key>"));
    }

    /// Return one background task options payload for plist rendering tests.
    fn background_options(
        kind: BackgroundTaskScheduleKindValue,
        repeat_interval_ns: Option<u64>,
    ) -> BackgroundTaskOptionsValue {
        BackgroundTaskOptionsValue {
            identifier: "sync".to_string(),
            trigger: BackgroundTriggerKindValue::AppRefresh,
            schedule: BackgroundTaskScheduleValue {
                kind,
                earliest_begin_unix_ns: None,
                repeat_interval_ns,
            },
            network: BackgroundNetworkRequirementValue::None,
            requires_charging: false,
            requires_idle: false,
            conflict_policy: BackgroundConflictPolicyValue::Replace,
        }
    }
}
