use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use super::constants::DUPLEX_STABLE_ID_PREFIX;
use super::core::{EndpointFlow, SelectedEndpoint};

pub(super) fn endpoint_stable_id(flow: EndpointFlow, endpoint_id: &str) -> String {
    format!("wasapi:{}:{endpoint_id}", flow.marker())
}

/// Build one normalized duplex stable id from one render and one capture endpoint id.
pub(super) fn duplex_stable_id(render_endpoint_id: &str, capture_endpoint_id: &str) -> String {
    format!("{DUPLEX_STABLE_ID_PREFIX}{render_endpoint_id}|{capture_endpoint_id}")
}

/// Parse one normalized endpoint stable id.
pub(super) fn parse_endpoint_stable_id(stable_id: &str) -> RuntimeResult<SelectedEndpoint> {
    let mut parts = stable_id.splitn(3, ':');
    let prefix = parts.next();
    let flow = parts.next();
    let endpoint_id = parts.next();

    if prefix != Some("wasapi") {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "wasapi id must start with wasapi:",
        ))
        .boxed());
    }

    let flow = match flow {
        Some("render") => EndpointFlow::Render,
        Some("capture") => EndpointFlow::Capture,
        Some("loopback") => EndpointFlow::Loopback,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "wasapi id must include render, capture, or loopback flow",
            ))
            .boxed());
        }
    };

    let endpoint_id = endpoint_id.ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "wasapi id must include endpoint id payload",
        ))
        .boxed()
    })?;

    Ok(SelectedEndpoint {
        flow,
        endpoint_id: endpoint_id.to_string(),
    })
}

/// Parse one normalized duplex stable id.
pub(super) fn parse_duplex_stable_id(stable_id: &str) -> RuntimeResult<(String, String)> {
    if !stable_id.starts_with(DUPLEX_STABLE_ID_PREFIX) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "wasapi duplex id must start with wasapi:duplex:",
        ))
        .boxed());
    }

    let payload = &stable_id[DUPLEX_STABLE_ID_PREFIX.len()..];
    let Some((render_endpoint_id, capture_endpoint_id)) = payload.split_once('|') else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "wasapi duplex id must include render and capture endpoint payloads",
        ))
        .boxed());
    };

    if render_endpoint_id.is_empty() || capture_endpoint_id.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "wasapi duplex id must include non-empty render and capture endpoint payloads",
        ))
        .boxed());
    }

    Ok((
        render_endpoint_id.to_string(),
        capture_endpoint_id.to_string(),
    ))
}
