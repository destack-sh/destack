import { expect, test } from "@destack/test";
import { createHash } from "node:crypto";
import { createServer, type Socket } from "node:net";
import { Frame, FrameFlag, FrameType, TUNNEL_PROTOCOL } from "../session/index.ts";
import { freePort, until } from "../test/index.ts";
import { TunnelClient } from "./client.ts";

/** The GUID a WebSocket server appends to the client's key to accept it, from RFC 6455 section 1.3. */
const WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/** The opcode of a binary WebSocket message, from RFC 6455 section 5.2. */
const BINARY_OPCODE = 2;

/** The opcode of a WebSocket close, from RFC 6455 section 5.2. */
const CLOSE_OPCODE = 8;

/** A connection to a silent relay: the offered subprotocols and the bytes sent after the upgrade. */
interface SilentConnection {
    /** The offered subprotocols, as the client listed them. */
    readonly protocols: string | undefined;
    /** The chunks the client sent after the upgrade. */
    readonly chunks: Buffer[];
}

/** One WebSocket frame a client sent, unmasked. */
interface ClientFrame {
    /** The frame's opcode. */
    readonly opcode: number;
    /** The unmasked payload. */
    readonly payload: Uint8Array;
}

/** Read the masked frames a client sent, with payloads under 64 KiB, per RFC 6455 section 5.2. */
function clientFrames(chunks: readonly Buffer[]): ClientFrame[] {
    const bytes = new Uint8Array(Buffer.concat(chunks));
    const view = new DataView(bytes.buffer);
    const frames: ClientFrame[] = [];
    let offset = 0;
    while (offset < bytes.length) {
        // read the opcode and the 7-bit or 16-bit length
        const opcode = view.getUint8(offset) & 0x0f;
        const short = view.getUint8(offset + 1) & 0x7f;
        const isExtended = short === 126;
        const length = isExtended ? view.getUint16(offset + 2) : short;

        // unmask the payload behind the mask key
        const maskAt = offset + (isExtended ? 4 : 2);
        const masked = bytes.subarray(maskAt + 4, maskAt + 4 + length);
        const payload = masked.map((byte, index) => byte ^ view.getUint8(maskAt + (index % 4)));
        frames.push({ opcode, payload });
        offset = maskAt + 4 + length;
    }

    return frames;
}

/** Accept WebSocket upgrades and leave every later frame unanswered, recording each connection. */
async function silentRelay() {
    // answer each upgrade with the tunnel protocol and count the bytes it sends
    const connections: SilentConnection[] = [];
    const sockets = new Set<Socket>();
    const server = createServer((socket) => {
        sockets.add(socket);
        let head = "";
        const read = (chunk: Buffer) => {
            head += chunk.toString("latin1");
            if (!head.includes("\r\n\r\n")) {
                return;
            }
            socket.off("data", read);
            const field = (name: string) =>
                new RegExp(`^${name}: (.*)$`, "imu").exec(head)?.[1]?.trim();
            const connection: SilentConnection = {
                protocols: field("sec-websocket-protocol"),
                chunks: [],
            };
            connections.push(connection);
            socket.on("data", (frames: Buffer) => connection.chunks.push(frames));
            const accept = createHash("sha1")
                .update(`${field("sec-websocket-key")}${WEBSOCKET_GUID}`)
                .digest("base64");
            socket.write(
                [
                    "HTTP/1.1 101 Switching Protocols",
                    "upgrade: websocket",
                    "connection: Upgrade",
                    `sec-websocket-accept: ${accept}`,
                    `sec-websocket-protocol: ${TUNNEL_PROTOCOL}`,
                    "",
                    "",
                ].join("\r\n"),
            );
        };
        socket.on("data", read);
    });
    await new Promise<void>((resolve) => {
        server.listen(0, "127.0.0.1", resolve);
    });
    const address = server.address();
    if (address === null || typeof address === "string") {
        throw new TypeError("the silent relay bound no TCP port");
    }

    return {
        url: `http://127.0.0.1:${address.port}/tunnel`,
        connections,
        [Symbol.asyncDispose]: async () => {
            for (const socket of sockets) {
                socket.destroy();
            }
            await new Promise<void>((resolve) => {
                server.close(() => resolve());
            });
        },
    };
}

test("refuse callers waiting for a tunnel once the client closes", async () => {
    // dial a relay that never answers, as a host with a token
    const client = TunnelClient.open({
        url: `http://127.0.0.1:${await freePort()}/tunnel`,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        name: () => {},
        retry: { initialInterval: 10, maximumInterval: 10 },
        report: () => {},
    });
    const waiting = Promise.allSettled([client.opened()]);

    // refuse the caller waiting before the close, and one asking after it
    await client.close();
    expect([await waiting, await Promise.allSettled([client.opened()])]).toEqual([
        [{ status: "rejected", reason: new Error("tunnel client is closed") }],
        [{ status: "rejected", reason: new Error("tunnel client is closed") }],
    ]);
});

test("keep dialing after the issuer grants no token, reporting each failed dial", async () => {
    // dial with a token the issuer refuses
    const reports: unknown[] = [];
    const url = `http://127.0.0.1:${await freePort()}/tunnel`;
    const client = TunnelClient.open({
        url,
        token: async () => {
            throw new TypeError("the issuer is unreachable");
        },
        fetch: async () => new Response(null, { status: 204 }),
        name: () => {},
        retry: { initialInterval: 10, maximumInterval: 10 },
        report: (error) => reports.push(error),
    });

    // report every failed dial and stop cleanly
    await until(() => reports.length >= 4);
    await client.close();
    expect(reports.slice(0, 2).map(String)).toEqual([
        "TypeError: the issuer is unreachable",
        `Error: tunnel to ${url} did not open`,
    ]);
});

test("drop a tunnel whose relay stops answering, dialing again and closing without its close answered", async () => {
    // dial a relay that accepts the upgrade and answers nothing after
    await using relay = await silentRelay();
    const reports: unknown[] = [];
    const client = TunnelClient.open({
        url: relay.url,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        name: () => {},
        heartbeat: 20,
        retry: { initialInterval: 10, maximumInterval: 10 },
        report: (error) => reports.push(error),
    });

    // dial again after the unanswered heartbeat drops the socket and close while connected to a peer that never answers the close
    await until(() => relay.connections.length >= 2);
    await client.close();
    const [first] = relay.connections;
    const head = { method: "PUT", url: relay.url, headers: [["authorization", "Bearer token"]] };

    // offer the token, ping and send the unanswered renewal before closing normally, reporting the tunnel that never opened
    expect({
        protocols: first?.protocols,
        frames: clientFrames(first?.chunks ?? []),
        report: reports.at(0),
    }).toEqual({
        protocols: `${TUNNEL_PROTOCOL}, destack.bearer.dG9rZW4`,
        frames: [
            {
                opcode: BINARY_OPCODE,
                payload: new Frame(FrameType.ping, FrameFlag.syn, 0).encode(),
            },
            {
                opcode: BINARY_OPCODE,
                payload: new Frame(
                    FrameType.data,
                    FrameFlag.syn,
                    1,
                    0,
                    new TextEncoder().encode(JSON.stringify(head)),
                ).encode(),
            },
            {
                opcode: BINARY_OPCODE,
                payload: new Frame(FrameType.data, FrameFlag.fin, 1).encode(),
            },
            { opcode: CLOSE_OPCODE, payload: Uint8Array.of(0x03, 0xe8) },
        ],
        report: new Error(`tunnel to ${relay.url} did not open`),
    });
});
