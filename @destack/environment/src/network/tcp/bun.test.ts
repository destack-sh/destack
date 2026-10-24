import { expect, test } from "@destack/test";
import { readFile } from "node:fs/promises";
import { createServer, type Server, type Socket } from "node:net";
import { TLSSocket } from "node:tls";
import { BunNetwork } from "../bun.ts";
import { BunServer } from "../../server/bun.ts";

/** The encoder of the tests' lines. */
const ENCODER = new TextEncoder();
/** The decoder of the tests' lines. */
const DECODER = new TextDecoder();

/** Read a stream's text until it has a number of bytes. */
async function readBytes(reader: ReadableStreamDefaultReader<Uint8Array>, count: number) {
    // read chunks until the count arrived, refusing an earlier end
    const chunks: Uint8Array[] = [];
    let received = 0;
    while (received < count) {
        const { value, done } = await reader.read();
        if (done) {
            throw new Error(`the stream ended after ${received} of ${count} bytes`);
        }
        chunks.push(value);
        received += value.byteLength;
    }

    return chunks.map((chunk) => DECODER.decode(chunk)).join("");
}

/** Read the port a node server listens at, refusing one that listens at a path. */
function portOf(server: Server): number {
    const address = server.address();
    if (typeof address !== "object" || address === null) {
        throw new TypeError("the server listens at no TCP port");
    }

    return address.port;
}

test("echo bytes over a system TCP connection on the loopback, and end it from the client", async () => {
    // echo each connection's bytes back
    const listener = await BunServer.tcp({
        hostname: "127.0.0.1",
        accept: (socket) => void socket.readable.pipeTo(socket.writable).catch(() => undefined),
    });

    // connect it, write a line, read it back and close
    const socket = BunNetwork.connect(listener.address);
    await socket.opened;
    const writer = socket.writable.getWriter();
    await writer.write(ENCODER.encode("hello\r\n"));
    const echoed = await readBytes(socket.readable.getReader(), "hello\r\n".length);
    await socket.close();
    await listener.stop();

    expect(echoed).toBe("hello\r\n");
});

/** Server a STARTTLS greeting: plain until the client asks, then TLS echoing in upper case. */
function serveStartTls(pem: { readonly key: string; readonly cert: string }) {
    return createServer((plain: Socket) => {
        // greet, and switch to TLS once the client asks
        plain.write("220 ready\r\n");
        plain.once("data", () => {
            plain.write("220 go ahead\r\n", () => {
                const secure = new TLSSocket(plain, { isServer: true, ...pem });
                secure.on("data", (chunk: Buffer) => secure.write(String(chunk).toUpperCase()));
            });
        });
    });
}

test("start TLS over a plain system connection once the server agrees, trusting the given authority, and retire the plain socket", async () => {
    // serve STARTTLS on the loopback with the test's localhost certificate
    const pem = {
        key: await readFile(new URL("test/localhost.key.pem", import.meta.url), "utf8"),
        cert: await readFile(new URL("test/localhost.cert.pem", import.meta.url), "utf8"),
    };
    const server = serveStartTls(pem);
    await new Promise<void>((resolve) => {
        server.listen(0, "127.0.0.1", resolve);
    });

    // read the greeting in plain text and ask for TLS
    const plain = BunNetwork.connect(
        { hostname: "localhost", port: portOf(server) },
        { secureTransport: "starttls", certificateAuthorities: [pem.cert] },
    );
    await plain.opened;
    const reader = plain.readable.getReader();
    const greeting = await readBytes(reader, "220 ready\r\n".length);
    const writer = plain.writable.getWriter();
    await writer.write(ENCODER.encode("STARTTLS\r\n"));
    const agreed = await readBytes(reader, "220 go ahead\r\n".length);
    reader.releaseLock();

    // start TLS, refuse the plain socket's writes, and talk over the protected one
    const secure = plain.startTls();
    const stale = await writer
        .write(ENCODER.encode("NOOP\r\n"))
        .catch((error: Error) => error.message);
    await secure.opened;
    await secure.writable.getWriter().write(ENCODER.encode("hello\r\n"));
    const answered = await readBytes(secure.readable.getReader(), "HELLO\r\n".length);
    await secure.close();
    await plain.closed;
    await new Promise<void>((resolve) => {
        server.close(() => resolve());
    });

    expect({ greeting, agreed, stale, answered }).toEqual({
        greeting: "220 ready\r\n",
        agreed: "220 go ahead\r\n",
        stale: "the connection started TLS on another socket",
        answered: "HELLO\r\n",
    });
});
