use std::path::Path;

use crate::{PhysicalResolver, ResolveError, ResolveOptions};

/// Test behavior when builtin modules are disabled.
#[test]
fn test_resolve_builtins_disabled() {
    let f = Path::new("/");
    let resolver = PhysicalResolver::default();
    let resolved_path = resolver.resolve(f, "zlib").map(|r| r.full_path());
    assert_eq!(
        resolved_path,
        Err(ResolveError::NotFound {
            specifier: "zlib".into()
        })
    );
}

/// Test behavior when builtin modules are enabled.
#[test]
fn test_resolve_builtins_enabled() {
    let f = Path::new("/");

    let resolver = PhysicalResolver::new(ResolveOptions::default().with_builtin_modules(true));

    let pass = [
        "_http_agent",
        "_http_client",
        "_http_common",
        "_http_incoming",
        "_http_outgoing",
        "_http_server",
        "_stream_duplex",
        "_stream_passthrough",
        "_stream_readable",
        "_stream_transform",
        "_stream_wrap",
        "_stream_writable",
        "_tls_common",
        "_tls_wrap",
        "assert",
        "assert/strict",
        "async_hooks",
        "buffer",
        "child_process",
        "cluster",
        "console",
        "constants",
        "crypto",
        "dgram",
        "diagnostics_channel",
        "dns",
        "dns/promises",
        "domain",
        "events",
        "fs",
        "fs/promises",
        "http",
        "http2",
        "https",
        "inspector",
        "module",
        "net",
        "os",
        "path",
        "path/posix",
        "path/win32",
        "perf_hooks",
        "process",
        "punycode",
        "querystring",
        "readline",
        "repl",
        "stream",
        "stream/consumers",
        "stream/promises",
        "stream/web",
        "string_decoder",
        "sys",
        "timers",
        "timers/promises",
        "tls",
        "trace_events",
        "tty",
        "url",
        "util",
        "util/types",
        "v8",
        "vm",
        "worker_threads",
        "zlib",
    ];

    for request in pass {
        let prefixed_request = format!("node:{request}");
        for request in [prefixed_request.clone(), request.to_string()] {
            let starts_with_node = request.starts_with("node:");
            let resolved_path = resolver.resolve(f, &request).map(|r| r.full_path());
            let err = ResolveError::Builtin {
                resolved: prefixed_request.clone(),
                is_runtime_module: starts_with_node,
            };
            assert_eq!(resolved_path, Err(err), "{request}");
        }
    }
}

/// Test failing resolution for non-builtin modules when builtins are enabled.
#[test]
fn test_resolve_builtins_fail() {
    let f = Path::new("/");
    let resolver = PhysicalResolver::new(ResolveOptions::default().with_builtin_modules(true));
    let request = "xxx";
    let resolved_path = resolver.resolve(f, request);
    let err = ResolveError::NotFound {
        specifier: request.to_string(),
    };
    assert_eq!(resolved_path, Err(err), "{request}");
}

/// Test resolving builtins through imports field.
#[test]
fn test_resolve_builtins_imports() {
    let f = super::fixture().join("builtins");
    let resolver = PhysicalResolver::new(ResolveOptions {
        builtin_modules: true,
        condition_names: vec!["node".into()],
        ..ResolveOptions::default()
    });

    for (request, is_runtime_module) in [("#fs", false), ("#http", true)] {
        let resolved_path = resolver.resolve(f.clone(), request).map(|r| r.full_path());
        let err = ResolveError::Builtin {
            resolved: (format!("node:{}", request.trim_start_matches('#'))),
            is_runtime_module,
        };
        assert_eq!(resolved_path, Err(err));
    }
}
