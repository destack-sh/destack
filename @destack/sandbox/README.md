# @destack/sandbox

Launch processes with operating system filesystem and network restrictions.

## Overview

`@destack/sandbox` is process isolation over `@anthropic-ai/sandbox-runtime`: Seatbelt on macOS, bubblewrap on Linux, and explicit grants.

Hosts launch restricted processes:

- `Sandbox`: one restricted process.
- `SandboxOptions`: what the process may use: variables, paths, hosts and sockets.
- `SandboxExit`: how the process ended.
- `SandboxError`: the reasons a sandbox fails to start or stop, checked with `SandboxError.is`.

Start an application that reads its code and writes its storage, then read its output:

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { Sandbox } from "@destack/sandbox";

// run the application with read, write and network grants
const environment = BunEnvironment.system;
await using sandbox = await Sandbox.start(
    {
        executable: environment.process.executable,
        arguments: ["run", "--no-env-file", "/application/main.ts"],
        directory: "/application",
        environment: {},
        read: ["/application"],
        write: ["/storage"],
        network: ["api.example.com"],
        sockets: [],
        listening: "denied",
    },
    environment,
);

// read what it wrote, and how it ended
const output = await new Response(sandbox.stdout).text();
const exit = await sandbox.exited;
```

## Reference

### SandboxOptions

Paths are absolute without globs, and no host variable is inherited.

| Option        | What it is                                                                                                          |
| ------------- | ------------------------------------------------------------------------------------------------------------------- |
| `executable`  | the executable's absolute path                                                                                      |
| `arguments`   | the arguments, passed unchanged                                                                                     |
| `directory`   | the working directory                                                                                               |
| `environment` | the process's whole environment                                                                                     |
| `read`        | the paths it reads, beside the executable and the system's runtime files                                            |
| `write`       | the files and directories it writes, and reads                                                                      |
| `network`     | the hosts it reaches through the filtering proxy: names, `*.` wildcards or IP addresses, each with an optional port |
| `sockets`     | the extra Unix sockets it may connect to                                                                            |
| `listening`   | `"denied"`, or `"loopback"` to listen on loopback ports                                                             |

### Sandbox

`Sandbox.start(options, environment)` launches the process with a private `TMPDIR`.
On Linux it needs `bubblewrap`, `ripgrep` and `socat`.
The process serves its host on the socket in `DESTACK_SERVER_SOCKET`.

| Member              | What it is                                                                                          |
| ------------------- | --------------------------------------------------------------------------------------------------- |
| `stdin`             | the process's standard input                                                                        |
| `stdout`, `stderr`  | its standard output and error                                                                       |
| `exited`            | resolves with its `SandboxExit` once cleaned up, `{ code, signal }`                                 |
| `socket`            | the socket it serves its host on                                                                    |
| `stop(gracePeriod)` | stops its process group after a grace period of at most five minutes, as disposing the sandbox does |

## Examples

### Feed a process its input

`stdin` writes to the process's standard input, and `listening: "loopback"` lets it serve on loopback ports.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { Sandbox, type SandboxOptions } from "@destack/sandbox";

/** Start a process that may listen on loopback, and tell it to start. */
export async function startServer(options: SandboxOptions) {
    // launch the process with loopback listening
    const environment = BunEnvironment.system;
    await using runner = await Sandbox.start({ ...options, listening: "loopback" }, environment);

    // write one line to its input, then close it
    const writer = runner.stdin.getWriter();
    await writer.write(new TextEncoder().encode("start\n"));
    await writer.close();

    return await runner.exited;
}
```
