# @destack/tunnel

Carry HTTP requests over one WebSocket session between a machine and a relay, as a tunnel.

## Overview

`@destack/tunnel` is yamux over a WebSocket: a client that dials a relay, sessions, and one HTTP exchange per stream.

Machines keep a tunnel open:

- `TunnelClient`: dials the relay and serves the requests it forwards.
- `Renewal`: the machine's routed name, and when its token lapses.
- `TunnelStatus`: `"open"` or `"dialing"`.

Both ends run a session:

- `Session`: streams multiplexed over one channel.
- `Stream`: one HTTP exchange.
- `Frame`: one binary message of a session.

Open a tunnel that serves the machine's requests, and wait until it is open:

```ts
import type { Environment } from "@destack/environment";
import type { PackageId } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { TunnelClient } from "@destack/tunnel";

/** Serve a machine's requests through its relay until the signal aborts. */
export async function serveThroughRelay(
    machineId: Identifier<"machine">,
    token: (audience: PackageId) => Promise<string>,
    serve: (request: Request) => Promise<Response>,
    environment: Environment,
    signal: AbortSignal,
) {
    // dial the relay, and log each change
    const tunnel = TunnelClient.open({
        url: "https://relay.eu.destack.space/tunnel",
        machineId,
        token,
        fetch: serve,
        renewed: ({ name, lapsesAt }) => console.log(name, lapsesAt),
        changed: (status) => console.log(status),
        environment,
    });

    // wait until it is open, and close it on abort
    await tunnel.opened();
    signal.addEventListener("abort", () => void tunnel.close(), { once: true });
}
```

## Reference

### TunnelClient

`TunnelClient.open(options)` dials the relay, and redials with jittered backoff from 0.5 to 30 seconds.
`opened()` waits until the tunnel is open, and `close()` ends it and stops dialing.

| Option        | What it is                                                              |
| ------------- | ----------------------------------------------------------------------- |
| `url`         | the relay's tunnel URL, ending in `/tunnel`                             |
| `machineId`   | the machine, which seeds the backoff's jitter                           |
| `token`       | a function answering the machine's access token for the relay's package |
| `fetch`       | answers a request the relay forwards, a failure answering 502           |
| `renewed`     | learns the `Renewal` each time the token renews                         |
| `changed`     | learns each `TunnelStatus`                                              |
| `heartbeat`   | how often to probe the tunnel, 15 seconds by default                    |
| `retry`       | the backoff, as a `RetryPolicy`                                         |
| `environment` | the clock, network and telemetry the tunnel runs on                     |

### Protocol

The token renews through the session at 80% of its lifetime.
The relay asks for a renewal with a `PUT` to the tunnel URL.

| Step                                                     | What it carries                                                             |
| -------------------------------------------------------- | --------------------------------------------------------------------------- |
| `GET /.well-known/oauth-protected-resource/tunnel`       | the relay's `resource`, as RFC 9728 has it, whose package the token is for  |
| WebSocket to the tunnel URL                              | the subprotocols `destack.tunnel` and `destack.bearer.<token in base64url>` |
| the text messages `ping` and `pong`                      | the machine's liveness probe each heartbeat, and the relay's answer         |
| `PUT` to the tunnel URL with a bearer token, as a stream | a renewal, answered with `{ name, lapsesAt }`                               |
| binary messages                                          | the session's frames                                                        |

### Session

`new Session(transport, role, { accept })` runs a session over any channel with `send(message)` and `close()`.
The `"client"` opens odd streams and the `"server"` even ones, at most 1000 each way.
`accept` takes each stream the peer opens.

| Verb                                           | What it does                                                             |
| ---------------------------------------------- | ------------------------------------------------------------------------ |
| `receive(message)`                             | takes one message from the channel, ending the session on a broken frame |
| `fetch(request)`                               | sends a request down a new stream, and answers the peer's response       |
| `open(head)`                                   | opens a stream with a request's head                                     |
| `ping()`                                       | probes the peer, resolving on its answer                                 |
| `close(code)`                                  | tells the peer the session ends, and ends every stream                   |
| `terminate()`                                  | ends every stream once the channel closed                                |
| `stream.request()`, `stream.respond(response)` | read a peer's stream's request, and send its response                    |

### Frame

A frame is one binary message: a 10-byte header, then at most 64 KiB of payload.
The header has the type in byte 0, the flags in byte 1, the stream in bytes 2 to 5 and a value in bytes 6 to 9, big-endian.

| Type        | Value                         | Payload                                                                                    |
| ----------- | ----------------------------- | ------------------------------------------------------------------------------------------ |
| `data`, 0   | none                          | the stream's bytes, or the request's head as JSON with `syn` and the response's with `ack` |
| `window`, 1 | credit in bytes               | none                                                                                       |
| `ping`, 2   | an opaque value               | none, answered with `ack`                                                                  |
| `goAway`, 3 | `normal`, 0, or `protocol`, 1 | none                                                                                       |

| Flag     | What it does                        |
| -------- | ----------------------------------- |
| `syn`, 1 | opens a stream                      |
| `ack`, 2 | answers a ping, or opens a response |
| `fin`, 4 | ends the sender's half of a stream  |
| `rst`, 8 | aborts a stream                     |

## Examples

### Accept a tunnel on the relay's side

A relay runs a server session over the machine's WebSocket.

```ts
import { Session } from "@destack/tunnel";

/** Run a session over a machine's WebSocket, and forward a request through it. */
export async function forward(socket: WebSocket, request: Request): Promise<Response> {
    // answer the machine's own streams with a status
    const session = new Session(socket, "server", {
        accept: (stream) => void stream.respond(new Response(null, { status: 204 })),
    });

    // pass each message to the session
    socket.binaryType = "arraybuffer";
    socket.addEventListener("message", (event) => {
        if (event.data instanceof ArrayBuffer) {
            session.receive(new Uint8Array(event.data));
        }
    });
    socket.addEventListener("close", () => session.terminate());

    return await session.fetch(request);
}
```

## Comparison

### yamux

Sessions, frames, flags, windows and go-away codes basically follow yamux, as WebSocket messages.
Each stream is one HTTP exchange.

| yamux                                    | `@destack/tunnel`                   | Difference                                                    |
| ---------------------------------------- | ----------------------------------- | ------------------------------------------------------------- |
| a 12-byte header with a version          | a 10-byte header                    | One frame per WebSocket message, so no length and no version. |
| `Data`, `WindowUpdate`, `Ping`, `GoAway` | `data`, `window`, `ping`, `goAway`  | The same.                                                     |
| `SYN`, `ACK`, `FIN`, `RST`               | `syn`, `ack`, `fin`, `rst`          | The same, with the HTTP heads as JSON on `syn` and `ack`.     |
| a 256 KiB initial window                 | `WINDOW_BYTES`                      | The same, grown up to 16 MiB as round trips allow.            |
| odd client and even server streams       | the `"client"` and `"server"` roles | The same.                                                     |
| `session.Open()`, `Accept()`             | `fetch(request)`, `accept`          | Streams carry HTTP exchanges instead of bytes.                |

### Cloudflare Tunnel

Machines dial out to a relay, as `cloudflared` dials Cloudflare's edge.

| Cloudflare Tunnel       | `@destack/tunnel`          | Difference                                                               |
| ----------------------- | -------------------------- | ------------------------------------------------------------------------ |
| `cloudflared`           | `TunnelClient`             | One WebSocket to the residency's relay.                                  |
| a tunnel token          | the machine's access token | Renewed through the session at 80% of its lifetime.                      |
| hostnames routed in DNS | `Renewal.name`             | The relay names the machine, such as `laptop--florian.destack.computer`. |
