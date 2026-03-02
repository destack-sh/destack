use destack_runtime::platform::PlatformContext;
use destack_runtime::runtime::Runtime;
use destack_workspace::RuntimeOptions;

/// Execute one runtime smoke pass and return one summary string.
fn run_runtime_smoke() -> Result<String, String> {
    // build a runtime with default options
    let platform = PlatformContext::new(Vec::new());
    let options = RuntimeOptions::default();
    let runtime = Runtime::from_options(platform, &options).map_err(|error| format!("{error}"))?;
    let primary_agent_id = runtime.primary_agent_id();
    let primary_agent = runtime
        .agent(primary_agent_id)
        .ok_or_else(|| format!("runtime agent {} does not exist", primary_agent_id.0))?;

    // poll host events in nonblocking mode
    let host_events = primary_agent
        .host()
        .poll_events(Some(0))
        .map_err(|error| format!("{error}"))?;
    let host_event_count = host_events.events.len();

    // verify core counters are initialized
    let dropped_dispatch_events = primary_agent.dropped_dispatch_events();
    let dropped_host_queue_events = primary_agent.dropped_host_queue_events();
    let dropped_unwatched_dispatch_events = primary_agent.dropped_unwatched_dispatch_events();
    if dropped_dispatch_events != 0
        || dropped_host_queue_events != 0
        || dropped_unwatched_dispatch_events != 0
    {
        return Err(format!(
            "unexpected dropped-event counters: dispatch={dropped_dispatch_events}, host_queue={dropped_host_queue_events}, unwatched_dispatch={dropped_unwatched_dispatch_events}",
        ));
    }

    // return smoke summary
    Ok(format!(
        "runtime-smoke-ok host_events={host_event_count} callback_agent_id={:?}",
        primary_agent.host_callback_agent_id(),
    ))
}

/// Run the runtime smoke executable.
fn main() {
    // execute one smoke pass and report status
    match run_runtime_smoke() {
        Ok(summary) => {
            println!("{summary}");
        }
        Err(error) => {
            eprintln!("runtime-smoke-failed: {error}");
            std::process::exit(1);
        }
    }
}
