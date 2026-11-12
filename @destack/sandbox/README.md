# @destack/sandbox

Launch processes with operating system filesystem and network restrictions.

## Overview

A sandbox runs one executable with only the environment, paths, hosts and sockets its options list, under Seatbelt on macOS and bubblewrap on Linux.

- **Sandbox**: one restricted process; `start` launches it, with its `stdin`, `stdout`, `stderr` and `exited`.
- **SandboxOptions**: what the process may use: its environment, the paths it reads and writes, the hosts it connects to, extra Unix sockets, and whether it listens on loopback.
- **SandboxExit**: how the process ended, its code or its signal.

```ts
import { Sandbox } from "@destack/sandbox";
import { BunEnvironment } from "@destack/environment/bun";

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
const output = new Response(sandbox.stdout).text();
const exit = await sandbox.exited; // { code: 0, signal: null }
```

## Examples

### Allow hosts

`network` lists hosts as names, `*.` wildcards or IP addresses with optional ports, reached only through the proxy in `HTTP_PROXY` and `HTTPS_PROXY`.

```ts
import type { SandboxOptions } from "@destack/sandbox";

(options: SandboxOptions): SandboxOptions => ({
    ...options,
    network: ["api.example.com", "*.example.org", "203.0.113.7:8443"],
});
```

### Write to a process

`listening: "loopback"` lets the process listen on loopback ports, and `stdin` writes to its standard input.

```ts
import { Sandbox, type SandboxOptions } from "@destack/sandbox";
import { BunEnvironment } from "@destack/environment/bun";

async (options: SandboxOptions) => {
    const environment = BunEnvironment.system;
    await using runner = await Sandbox.start({ ...options, listening: "loopback" }, environment);
    const writer = runner.stdin.getWriter();
    await writer.write(new TextEncoder().encode("start\n"));
    await writer.close();
};
```

## Comparison

### Anthropic's sandbox runtime

Seatbelt and bubblewrap profiles and the filtering proxy follow `@anthropic-ai/sandbox-runtime`, which the sandbox runs; two things differ.

- Options are one schema, which the host and the sandbox launcher both parse.
- Each process gets a private `TMPDIR`, removed when it stops.
