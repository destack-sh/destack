# @destack/sandbox

Run processes with explicit filesystem and network permissions on macOS (Seatbelt) and Linux (bubblewrap with `socat` and `ripgrep`).

## Processes

`Sandbox.start` runs an executable with only the environment, paths and hosts its options list.

```ts
import { Sandbox } from "@destack/sandbox";

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

// drain both streams while the workload runs
const output = new Response(sandbox.stdout).text();
const errors = new Response(sandbox.stderr).text();
const exit = await sandbox.exited; // { code, signal }
```

## Sockets

`sockets` lists further Unix sockets the workload connects to, in addition to its private `TMPDIR` that the sandbox removes when the workload stops.

```ts
await using sandbox = await Sandbox.start(
    { ...options, sockets: ["/run/destack/services.sock"] },
    environment,
);
```

## Messages

`SandboxOptions` and `SandboxExit` are schemas that parse the messages between the host and the sandbox launcher.

```ts
const options = SandboxOptions.parse(JSON.parse(line));
```

## Runners

`listening: "loopback"` lets the workload listen on loopback ports, and `stdin` is a writable stream to its standard input.

```ts
await using runner = await Sandbox.start({ ...options, listening: "loopback" }, environment);
const writer = runner.stdin.getWriter();
await writer.write(new TextEncoder().encode(`${JSON.stringify(start)}\n`));
await writer.close();
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

A refused or failed sandbox throws a `SandboxError` failure: `INVALID_GRACE_PERIOD` for a stop timeout out of range, `UNSUPPORTED` where the platform has no sandbox, and `START_FAILED` or `STOP_FAILED` for a launcher that fails.

```ts
import { SandboxError } from "@destack/sandbox/error";

if (SandboxError.is(error, "UNSUPPORTED")) {
    console.warn(`running unsandboxed: ${error.message}`);
}
// { code: "NOT_IMPLEMENTED", reason: "UNSUPPORTED", domain: "@destack/sandbox", … }
SandboxError.of("UNSUPPORTED", "sandboxing is unsupported on win32").toCallError();
```
