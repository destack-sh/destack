# @destack/sandbox

Run processes with explicit filesystem and network permissions on macOS (Seatbelt) and Linux (bubblewrap with `socat` and `ripgrep`) through a separate sandbox manager.

```ts
import { Sandbox } from "@destack/sandbox";

await using sandbox = await Sandbox.start({
    executable: process.execPath,
    arguments: ["run", "--no-env-file", "/application/main.ts"],
    directory: "/application",
    environment: {},
    read: ["/application"],
    write: ["/storage"],
    network: ["api.example.com"],
});

// drain both streams while the workload runs
sandbox.stdout.pipe(process.stdout);
sandbox.stderr.pipe(process.stderr);
const exit = await sandbox.exited;
console.log(exit);
```

A workload reads its standard input through `sandbox.stdin`, and listens on loopback ports only when `allowsListening` is set, as a runner serving its host does.

```ts
await using runner = await Sandbox.start({ ...options, allowsListening: true });
runner.stdin.end(`${JSON.stringify(start)}\n`);
```

Messages between the sandbox and its launcher are parsed by schema on both sides, so `SandboxOptions` and `SandboxExit` are schemas as well as types.
