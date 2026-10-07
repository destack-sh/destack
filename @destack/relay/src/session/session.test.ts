import { expect, test } from "@destack/test";
import { aligned } from "@destack/schema";
import { until } from "../test/index.ts";
import { Frame, FrameFlag, FrameType } from "./frame.ts";
import { Head, type RequestHead } from "./head.ts";
import { MAX_STREAMS, Session, SESSION_WINDOW_BYTES } from "./session.ts";
import { MAX_WINDOW_BYTES, WINDOW_BYTES, type Stream } from "./stream.ts";

/** The head of a request for a note. */
const head: RequestHead = {
    method: "POST",
    url: "https://notes.personal.acme.destack.space/notes",
    headers: [["content-type", "text/plain"]],
};

/** A relay's and a machine's session joined by an in-memory channel, delivering after a delay. */
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
        channel(() => machine),
        "server",
        { accept: () => {} },
    );
    const machine: Session = new Session(
        channel(() => relay),
        "client",
        { accept: (stream) => accepted.push(stream) },
    );

    return { relay, machine, accepted, sent };
}

/** Read a stream of bytes to its end. */
async function drain(readable: ReadableStream<Uint8Array>): Promise<Uint8Array> {
    return new Uint8Array(await new Response(readable).arrayBuffer());
}

test("forward a request with its head and body, and answer with the response's head and body", async () => {
    const { relay, machine, accepted } = pair();

    // answer each forwarded request with its method, path and body
    const answering = until(() => accepted.length === 1).then(async () => {
        const request = aligned(accepted, 0).request();
        const body = await request.text();
        const path = new URL(request.url).pathname;
        await aligned(accepted, 0).respond(
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
        aligned(accepted, 0).id,
        aligned(accepted, 0).head,
        response.status,
        response.headers.getSetCookie(),
        await response.text(),
    ]).toEqual([2, head, 201, ["a=1", "b=2"], "POST /notes hello"]);
    await until(() => relay.size === 0 && machine.size === 0);
});

test("abort the machine's request once the relay's caller cancels it while the machine streams its answer", async () => {
    const { relay, machine, accepted } = pair();

    // answer with a stream the machine keeps open until its request aborts
    const aborted = Promise.withResolvers<unknown>();
    void until(() => accepted.length === 1).then(async () => {
        const stream = aligned(accepted, 0);
        const request = stream.request();
        request.signal.addEventListener("abort", () => aborted.resolve(request.signal.reason));
        await stream.respond(new Response(new ReadableStream({ start: () => {} })));
    });

    // cancel the forwarded request once its answer's head arrives
    const cancelling = new AbortController();
    const response = await relay.fetch(
        new Request(head.url, { method: "GET", signal: cancelling.signal }),
    );
    cancelling.abort();
    const reason = await aborted.promise;
    expect([response.status, reason, machine.size]).toEqual([
        200,
        new Error("stream 2 was reset by the peer"),
        0,
    ]);
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
    const reader = aligned(accepted, 0).readable.getReader();
    for (let chunk = await reader.read(); !chunk.done; chunk = await reader.read()) {
        read += chunk.value.length;
        await new Promise((resolve) => {
            setImmediate(resolve);
        });
    }
    await written;
    expect(read).toBe(payload.length);
});

test("stop the writers of every stream at the session's window until their readers take bytes", async () => {
    // fill a stream window on four streams more than the session's window holds
    const { relay, accepted, sent } = pair();
    const count = SESSION_WINDOW_BYTES / WINDOW_BYTES + 4;
    const written = Array.from({ length: count }, async () => {
        const writer = relay.open(head).writable.getWriter();
        await writer.write(new Uint8Array(WINDOW_BYTES).fill(7));
        await writer.close();
    });

    // send no more than the session's window before any reader reads
    const sentBytes = () =>
        sent
            .filter((frame) => frame.type === FrameType.data && !frame.has(FrameFlag.syn))
            .reduce((total, frame) => total + frame.payload.length, 0);
    await until(() => accepted.length === count && sentBytes() >= SESSION_WINDOW_BYTES);
    const held = sentBytes();

    // read every stream, letting the held writes finish
    const received = await Promise.all(accepted.map((stream) => drain(stream.readable)));
    await Promise.all(written);
    expect([held, received.reduce((total, bytes) => total + bytes.length, 0)]).toEqual([
        SESSION_WINDOW_BYTES,
        count * WINDOW_BYTES,
    ]);
});

test("grow a window the writer outpaces while the reader keeps up, up to its limit", async () => {
    const { relay, accepted } = pair(5);
    const opened = relay.open(head);
    const payload = new Uint8Array(4 * MAX_WINDOW_BYTES).fill(7);
    const writer = opened.writable.getWriter();
    void writer.write(payload).then(() => writer.close());
    await until(() => accepted.length === 1);

    // read everything on arrival and find the window grown to its limit
    const received = await drain(aligned(accepted, 0).readable);
    expect([received.length, aligned(accepted, 0).window]).toEqual([
        payload.length,
        MAX_WINDOW_BYTES,
    ]);
});

test("reset a stream cancelled early, and fail the peer's side and its answer", async () => {
    const { relay, accepted } = pair();
    const opened = relay.open(head);
    await until(() => accepted.length === 1);

    await aligned(accepted, 0).readable.cancel();

    const failed = await Promise.allSettled([opened.answer, drain(opened.readable)]);
    expect(failed.map((result) => result.status === "rejected" && String(result.reason))).toEqual([
        "Error: stream 2 was reset by the peer",
        "Error: stream 2 was reset by the peer",
    ]);
});

test("end both sides of a stream whose request body the machine answered without reading", async () => {
    const { relay, machine, accepted } = pair();

    // answer a large request at once and stop its unread body
    const answering = until(() => accepted.length === 1).then(async () => {
        const request = aligned(accepted, 0).request();
        await aligned(accepted, 0).respond(new Response("early"));
        if (request.body === null) {
            throw new TypeError("the forwarded request has no body");
        }
        await request.body.cancel();
    });
    const body = new Uint8Array(4 * WINDOW_BYTES).fill(7);
    const response = await relay.fetch(new Request(head.url, { method: "POST", body }));
    await answering;

    expect(await response.text()).toBe("early");
    await until(() => relay.size === 0 && machine.size === 0);
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
