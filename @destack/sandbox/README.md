Run processes with explicit filesystem and network permissions on macOS (Seatbelt) and Linux (bubblewrap with `socat` and `ripgrep`) through a separate sandbox manager.

## Processes

`Sandbox.start` runs an executable with only the environment, paths and hosts its options grant.

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
const exit = await sandbox.exited; // { code, signal }
```

A workload writes its private temporary folder (`TMPDIR`) and serves and reaches Unix sockets in it, removed when it stops.
`sockets` names further Unix sockets the host mediates service connections through.
`SandboxOptions` and `SandboxExit` are schemas as well as types, since both sides of the sandbox parse their messages.

## Runners

A runner serving its host reads its standard input and listens on loopback ports with `allowsListening`.

```ts
await using runner = await Sandbox.start({ ...options, allowsListening: true });
runner.stdin.end(`${JSON.stringify(start)}\n`);
```

## Network

A workload reaches the network only through the sandbox's proxy, which `HTTP_PROXY` and `HTTPS_PROXY` name, with `NO_PROXY` cleared so loopback hosts go through it too.

```ts
const options = { ...base, network: ["api.example.com", "*.example.org", "203.0.113.7:8443"] };
```

`network` lists names, `*.` wildcards and IP addresses, each optionally qualified by a port.
The proxy answers a request to any other host with status 403, for plain HTTP and an HTTPS tunnel alike, and the operating system refuses direct connections.
With `allowsListening`, the operating system also lets the workload connect to loopback ports directly.

## Errors

A failed start or stop throws a `SandboxError` from `@destack/sandbox/error` with the code `UNSUPPORTED`, `START_FAILED` or `STOP_FAILED`.

```ts
import { SandboxError } from "@destack/sandbox/error";

if (error instanceof SandboxError && error.code === "UNSUPPORTED") {
    report("this platform has no sandbox");
}
```
