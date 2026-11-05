use crate::host::Command;
use std::io::{self, BufRead, BufReader};
use std::process::{self, Child, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Maximum wait for graceful host shutdown.
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
/// Delay between host exit observations.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Maximum number of unread native commands.
const COMMAND_CAPACITY: usize = 32;

/// The TypeScript application host retained by the native shell.
pub struct Host {
    /// Child process retained until its resources have been released.
    pub child: Child,
}

impl Host {
    /// Start source in development or the compiled distribution host.
    pub fn start() -> io::Result<Self> {
        // locate the host bundled with this executable
        let executable = std::env::current_exe()?;
        let directory = executable
            .parent()
            .ok_or_else(|| io::Error::other("missing executable directory"))?;
        let directory = if cfg!(target_os = "macos") {
            directory.join("../Helpers")
        } else {
            directory.join("helpers")
        };
        let name = if cfg!(windows) {
            "destack-desktop-host.exe"
        } else {
            "destack-desktop-host"
        };

        // run source in development and the bundled executable in distributions
        let mut command = if cfg!(debug_assertions) {
            let mut command = process::Command::new("bun");
            command.args([
                "run",
                "--no-env-file",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../src/main.ts"),
            ]);
            command
        } else {
            process::Command::new(directory.join(name))
        };
        command.args(std::env::args_os().skip(1));

        // retain private command pipes and inherit the parent error stream
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(Self { child })
    }

    /// Read host commands independently of the native event loop.
    pub fn commands(&mut self) -> io::Result<mpsc::Receiver<io::Result<Command>>> {
        let output = self
            .child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing host output"))?;

        // bound pending messages while native windows are being created
        let (sender, receiver) = mpsc::sync_channel(COMMAND_CAPACITY);
        thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let command =
                    line.and_then(|line| serde_json::from_str(&line).map_err(io::Error::other));
                let failed = command.is_err();
                if sender.send(command).is_err() || failed {
                    break;
                }
            }
        });

        Ok(receiver)
    }

    /// Reap the host within its shutdown deadline and report forced termination.
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + STOP_TIMEOUT;
        loop {
            // finish once the host has released the daemon's connections
            if let Some(status) = self.child.try_wait()? {
                return Ok(status);
            }

            // terminate only this owned child when graceful shutdown cannot complete
            if Instant::now() >= deadline {
                self.child.kill()?;
                self.child.wait()?;

                return Err(io::Error::other("desktop host shutdown timed out"));
            }
            thread::sleep(EXIT_POLL_INTERVAL);
        }
    }
}
