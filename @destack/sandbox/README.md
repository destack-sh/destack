# @destack/sandbox

Run processes with explicit filesystem and network permissions on macOS (Seatbelt) and Linux (bubblewrap with `socat` and `ripgrep`).

## Processes

`Sandbox.start` runs an executable with only the environment, paths and hosts its options list.

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

## Sockets

`sockets` lists further Unix sockets the workload connects to, in addition to its private `TMPDIR` that the sandbox removes when the workload stops.

```ts
await using sandbox = await Sandbox.start({ ...options, sockets: ["/run/destack/services.sock"] });
```

## Messages

`SandboxOptions` and `SandboxExit` are schemas that parse the messages between the host and the sandbox launcher.

```ts
const options = SandboxOptions.parse(JSON.parse(line));
```

## Runners

`allowsListening` lets the workload listen on loopback ports, and `stdin` writes to its standard input.

```ts
await using runner = await Sandbox.start({ ...options, allowsListening: true });
runner.stdin.end(`${JSON.stringify(start)}\n`);
```

## Network

`network` lists the hosts a workload connects to as names, `*.` wildcards or IP addresses with optional ports, and the workload connects to them only through the proxy in `HTTP_PROXY` and `HTTPS_PROXY`.

```ts
const options = { ...base, network: ["api.example.com", "*.example.org", "203.0.113.7:8443"] };
```

## Refusals

The proxy answers HTTP requests and HTTPS tunnels to a host outside `network` with status 403, and the operating system refuses direct connections.

```http
CONNECT other.example.com:443 HTTP/1.1

HTTP/1.1 403 Forbidden
```

## Errors

A failed start or stop throws a `SandboxError` with the code `UNSUPPORTED`, `START_FAILED` or `STOP_FAILED`.

```ts
import { SandboxError } from "@destack/sandbox/error";

if (error instanceof SandboxError && error.code === "UNSUPPORTED") {
    report("this platform has no sandbox");
}
```
