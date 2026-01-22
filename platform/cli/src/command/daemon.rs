use clap::Args;

use destack_daemon::protocol::ProtocolServerOptions;
use destack_daemon::{DaemonService, DaemonServiceOptions};
use destack_source::DiagnosticOptions;

use crate::common::diagnostic::DiagnosticArgs;
use crate::common::program::ProgramArgs;
use crate::console;
use crate::pipeline::watch::build_daemon_options;

/// Arguments for the daemon command.
#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(flatten)]
    pub program: ProgramArgs,

    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,
}

/// Run the daemon service over stdio.
pub fn run(args: &DaemonArgs) -> i32 {
    // build the session and compiler options
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let session = args.program.setup();
    let compiler_options = build_daemon_options(&args.program, diagnostic_options, None);

    // start the daemon service
    let service = DaemonService::with_options(
        session,
        DaemonServiceOptions {
            compiler_options,
            protocol: ProtocolServerOptions::default(),
        },
    );

    if let Err(error) = service.serve_stdio() {
        console::error(&format!("daemon error: {error}"));
        return 1;
    }

    0
}
