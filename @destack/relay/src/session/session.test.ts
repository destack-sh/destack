import { expect, test } from "@destack/test";
import { until } from "../server/test/fixture.ts";
import { Frame, FrameFlag, FrameType, MAX_FRAME_BYTES } from "./frame.ts";
import { Head, type RequestHead } from "./head.ts";
import { MAX_STREAMS, Session } from "./session.ts";
import { MAX_WINDOW_BYTES, WINDOW_BYTES, type Stream } from "./stream.ts";

/** The head of a request for a note. */
const head: RequestHead = {
    method: "POST",
    url: "https://notes.personal.acme.destack.space/notes",
    headers: [["content-type", "text/plain"]],
};

/** A relay's and a host's session joined by an in-memory channel, delivering after a delay. */
function pair(delay?: number) {
    const accepted: Stream[] = [];
    const sent: Frame[] = [];
    const channel = (deliver: () => Session) => ({
        send: (message: Uint8Array) => {
            sent.push(Frame.decode(message.slice()));
            const receive = () => deliver().receive(message);
            if (delay === undefined) {
                setImmediate(receive);
            } else {
                setTimeout(receive, delay);
            }
        },
        close: () => setImmediate(() => deliver().terminate()),
    });
    const relay: Session = new Session(
        channel(() => host),
        "server",
        { accept: () => {} },
    );
    const host: Session = new Session(
        channel(() => relay),
        "client",
        { accept: (stream) => accepted.push(stream) },
    );

    return { relay, host, accepted, sent };
}

/** Read a stream of bytes to its end. */
async function drain(readable: ReadableStream<Uint8Array>): Promise<Uint8Array> {
    return new Uint8Array(await new Response(readable).arrayBuffer());
}

test("forward a request with its head and body, and answer with the response's head and body", async () => {
    const { relay, host, accepted } = pair();

    // answer each forwarded request with its method, path and body
    const answering = until(() => accepted.length === 1).then(async () => {
        const request = accepted[0]!.request();
        const body = await request.text();
        const path = new URL(request.url).pathname;
        await accepted[0]!.respond(
            new Response(`${request.method} ${path} ${body}`, {
                status: 201,
                headers: [
                    ["set-cookie", "a=1"],
                    ["set-cookie", "b=2"],
                ],
            }),
        );
    });
    const response = await relay.fetch(
        new Request(head.url, {
            method: "POST",
            headers: { "content-type": "text/plain" },
            body: "hello",
        }),
    );
    await answering;

    expect([
        accepted[0]!.id,
        accepted[0]!.head,
        response.status,
        response.headers.getSetCookie(),
        await response.text(),
    ]).toEqual([2, head, 201, ["a=1", "b=2"], "POST /notes hello"]);
    await until(() => relay.size === 0 && host.size === 0);
});

test("stop a writer at the window until a slow reader credits it", async () => {
    const { relay, accepted, sent } = pair();
    const opened = relay.open(head);
    const payload = new Uint8Array(4 * WINDOW_BYTES).fill(7);
    const writer = opened.writable.getWriter();
    const written = writer.write(payload).then(() => writer.close());

    // send no more than one window before the reader reads
    const sentBytes = () =>
        sent
            .filter((frame) => frame.type === FrameType.data && !frame.has(FrameFlag.syn))
            .reduce((total, frame) => total + frame.payload.length, 0);
    await until(() => accepted.length === 1 && sentBytes() >= WINDOW_BYTES);
    expect(sentBytes()).toBe(WINDOW_BYTES);

    // read everything slowly, crediting the writer as the reader takes bytes
    let read = 0;
    const reader = accepted[0]!.readable.getReader();
    for (let chunk = await reader.read(); !chunk.done; chunk = await reader.read()) {
        read += chunk.value.length;
        await new Promise((resolve) => setImmediate(resolve));
    }
    await written;
    expect(read).toBe(payload.length);
});

test("grow a window the writer outpaces while the reader keeps up, up to its limit", async () => {
    const { relay, accepted } = pair(5);
    const opened = relay.open(head);
    const payload = new Uint8Array(4 * MAX_WINDOW_BYTES).fill(7);
    const writer = opened.writable.getWriter();
    void writer.write(payload).then(() => writer.close());
    await until(() => accepted.length === 1);

    // read everything on arrival and find the window grown to its limit
    const received = await drain(accepted[0]!.readable);
    expect([received.length, accepted[0]!.window]).toEqual([payload.length, MAX_WINDOW_BYTES]);
});

test("reset a stream cancelled early, and fail the peer's side and its answer", async () => {
    const { relay, accepted } = pair();
    const opened = relay.open(head);
    await until(() => accepted.length === 1);

    await accepted[0]!.readable.cancel();

    const failed = await Promise.allSettled([opened.answer, drain(opened.readable)]);
    expect(failed.map((result) => result.status === "rejected" && String(result.reason))).toEqual([
        "Error: stream 2 was reset by the peer",
        "Error: stream 2 was reset by the peer",
    ]);
});

test("answer pings", async () => {
    const { host } = pair();

    await expect(Promise.all([host.ping(), host.ping()])).resolves.toEqual([undefined, undefined]);
});

test("fail the pings waiting for an answer once the session ends", async () => {
    const host = new Session({ send: () => {}, close: () => {} }, "client", { accept: () => {} });
    const waiting = host.ping();

    host.terminate();

    await expect(Promise.allSettled([waiting, host.ping()])).resolves.toEqual([
        { status: "rejected", reason: new Error("session is closed") },
        { status: "rejected", reason: new Error("session is closed") },
    ]);
});

test("end a session whose peer opens a stream of this side's parity", () => {
    const sent: Frame[] = [];
    let isClosed = false;
    const relay = new Session(
        {
            send: (message) => sent.push(Frame.decode(message)),
            close: () => (isClosed = true),
        },
        "server",
        { accept: () => {} },
    );
    const payload = new TextEncoder().encode(JSON.stringify(head));

    relay.receive(new Frame(FrameType.data, FrameFlag.syn, 2, 0, payload).encode());
    expect([sent.map((frame) => [frame.type, frame.value]), isClosed, relay.isClosed]).toEqual([
        [[FrameType.goAway, 1]],
        true,
        true,
    ]);
});

test("reset a stream answered with no response head, keeping the session", async () => {
    const { relay, host, accepted } = pair();

    // answer the forwarded request with a head that is no response head
    const answering = until(() => accepted.length === 1).then(() =>
        host.send(
            new Frame(
                FrameType.data,
                FrameFlag.ack,
                accepted[0]!.id,
                0,
                new TextEncoder().encode("{}"),
            ),
        ),
    );
    const failed = await relay.fetch(new Request(head.url)).then(
        () => "answered",
        (error: Error) => error.message,
    );
    await answering;

    await until(() => relay.size === 0);
    expect([failed, relay.isClosed, host.isClosed]).toEqual([
        "stream 2 was answered with no response head",
        false,
        false,
    ]);
});

test("end both sides of a stream whose request body the host answered without reading", async () => {
    const { relay, host, accepted } = pair();

    // answer a large request at once, then stop its unread body
    const answering = until(() => accepted.length === 1).then(async () => {
        const request = accepted[0]!.request();
        await accepted[0]!.respond(new Response("early"));
        await request.body!.cancel();
    });
    const body = new Uint8Array(4 * WINDOW_BYTES).fill(7);
    const response = await relay.fetch(new Request(head.url, { method: "POST", body }));
    await answering;

    expect(await response.text()).toBe("early");
    await until(() => relay.size === 0 && host.size === 0);
});

test("end a session whose peer sends a frame longer than a frame may be", () => {
    const sent: Frame[] = [];
    const relay = new Session(
        { send: (message) => sent.push(Frame.decode(message)), close: () => {} },
        "server",
        { accept: () => {} },
    );

    relay.receive(new Uint8Array(MAX_FRAME_BYTES + 1));
    expect([sent.map((frame) => [frame.type, frame.value]), relay.isClosed]).toEqual([
        [[FrameType.goAway, 1]],
        true,
    ]);
});

test("reset the streams a peer opens beyond the open stream limit, keeping the session", () => {
    const sent: Frame[] = [];
    const relay = new Session(
        { send: (message) => sent.push(Frame.decode(message)), close: () => {} },
        "server",
        { accept: () => {} },
    );
    const payload = new TextEncoder().encode(JSON.stringify(head));

    // open one stream more than the limit from the peer's side
    for (let index = 0; index <= MAX_STREAMS; index += 1) {
        relay.receive(new Frame(FrameType.data, FrameFlag.syn, 2 * index + 1, 0, payload).encode());
    }
    expect([
        sent.map((frame) => [frame.type, frame.flags, frame.stream]),
        relay.size,
        relay.isClosed,
    ]).toEqual([[[FrameType.window, FrameFlag.rst, 2 * MAX_STREAMS + 1]], MAX_STREAMS, false]);
});

test("drop the connection's own headers and those it lists from a forwarded head", () => {
    const request = new Request("https://notes.personal.acme.destack.space/notes", {
        headers: {
            connection: "keep-alive, x-hop",
            "keep-alive": "timeout=5",
            te: "trailers",
            upgrade: "h2c",
            "x-hop": "1",
            accept: "text/html",
        },
    });

    expect(Head.request(request).headers).toEqual([["accept", "text/html"]]);
});
