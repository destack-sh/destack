import { expect, onTestFinished, refusal, test } from "@destack/test";
import { nth } from "@destack/schema";
import { testSimulation } from "@destack/test/world";
import { Frame, FrameFlag, FrameType } from "./frame.ts";
import { Head, type RequestHead } from "./head.ts";
import { MAX_STREAMS, Session, SESSION_WINDOW_BYTES } from "./session.ts";
import { MAX_WINDOW_BYTES, WINDOW_BYTES, type Stream } from "./stream.ts";

/** The head of a request for a note. */
const head: RequestHead = {
    method: "POST",
    url: "https://notes--personal--acme.destack.space/notes",
    headers: [["content-type", "text/plain"]],
};

/** A relay's and a machine's session joined by an in-memory channel, delivering after a delay. */
function pair(delay?: number) {
    // deliver after the delay on a world's simulated clock, which the test's driven work moves, removing the world after the test
    const simulation = testSimulation();
    onTestFinished(() => simulation[Symbol.asyncDispose]());
    const { clock } = simulation;
    const drive = <Value>(work: Promise<Value>) => simulation.drive(work);

    // signal each frame sent and received, and each stream accepted
    const accepted: Stream[] = [];
    const sent: Frame[] = [];
    let changing = Promise.withResolvers<void>();
    const change = () => {
        changing.resolve();
        changing = Promise.withResolvers<void>();
    };

    // wait on the signal until a condition holds
    const until = async (condition: () => boolean): Promise<void> => {
        while (!condition()) {
            await changing.promise;
        }
    };

    // deliver each frame to the other session
    const channel = (deliver: () => Session) => ({
        send: (message: Uint8Array) => {
            sent.push(Frame.decode(message.slice()));
            change();
            const receive = () => {
                deliver().receive(message);
                change();
            };
            void clock.sleep(delay ?? 0).then(receive);
        },
        close: () => void clock.sleep(0).then(() => deliver().terminate()),
    });
    const relay: Session = new Session(
        channel(() => machine),
        "server",
        { accept: () => {} },
    );
    const machine: Session = new Session(
        channel(() => relay),
        "client",
        {
            accept: (stream) => {
                accepted.push(stream);
                change();
            },
        },
    );

    return { relay, machine, accepted, sent, until, drive };
}

/** Read a stream of bytes to its end, each chunk as it arrives on the world's time. */
async function drain(readable: ReadableStream<Uint8Array>): Promise<Uint8Array> {
    // take each chunk in turn
    const chunks: Uint8Array[] = [];
    for await (const chunk of readable) {
        chunks.push(chunk);
    }

    // join them in order
    const joined = new Uint8Array(chunks.reduce((total, chunk) => total + chunk.length, 0));
    let offset = 0;
    for (const chunk of chunks) {
        joined.set(chunk, offset);
        offset += chunk.length;
    }

    return joined;
}

test("forward a request with its head and body, and answer with the response's head and body", async () => {
    const { relay, machine, accepted, until } = pair();

    // answer each forwarded request with its method, path and body
    const answering = until(() => accepted.length === 1).then(async () => {
        const request = nth(accepted, 0).request();
        const body = await request.text();
        const path = new URL(request.url).pathname;
        await nth(accepted, 0).respond(
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
        nth(accepted, 0).id,
        nth(accepted, 0).head,
        response.status,
        response.headers.getSetCookie(),
        await response.text(),
    ]).toEqual([2, head, 201, ["a=1", "b=2"], "POST /notes hello"]);
    await until(() => relay.size === 0 && machine.size === 0);
});

test("abort the machine's request once the relay's caller cancels it while the machine streams its answer", async () => {
    const { relay, machine, accepted, until } = pair();

    // answer with a stream the machine leaves open until its request aborts
    const aborted = Promise.withResolvers<unknown>();
    void until(() => accepted.length === 1).then(async () => {
        const stream = nth(accepted, 0);
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
    const { relay, accepted, sent, until } = pair();
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
    const reader = nth(accepted, 0).readable.getReader();
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
    const { relay, accepted, sent, until } = pair();
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

test("grow a window the writer outpaces while the reader catches up, up to its limit", async () => {
    const { relay, accepted, until, drive } = pair(5);
    const opened = relay.open(head);
    const payload = new Uint8Array(4 * MAX_WINDOW_BYTES).fill(7);
    const writer = opened.writable.getWriter();
    void writer.write(payload).then(() => writer.close());
    await drive(until(() => accepted.length === 1));

    // read everything on arrival and find the window grown to its limit
    const received = await drive(drain(nth(accepted, 0).readable));
    expect([received.length, nth(accepted, 0).window]).toEqual([payload.length, MAX_WINDOW_BYTES]);
});

test("reset a stream cancelled early, and fail the peer's side and its answer", async () => {
    const { relay, accepted, until } = pair();
    const opened = relay.open(head);
    await until(() => accepted.length === 1);

    await nth(accepted, 0).readable.cancel();

    const failed = await Promise.allSettled([opened.answer, drain(opened.readable)]);
    expect(failed.map((result) => result.status === "rejected" && String(result.reason))).toEqual([
        "Error: stream 2 was reset by the peer",
        "Error: stream 2 was reset by the peer",
    ]);
});

test("end both sides of a stream whose machine responds without reading the request body", async () => {
    const { relay, machine, accepted, until } = pair();

    // answer a large request at once and stop its unread body
    const answering = until(() => accepted.length === 1).then(async () => {
        const request = nth(accepted, 0).request();
        await nth(accepted, 0).respond(new Response("early"));
        if (request.body === null) {
            throw new TypeError("the forwarded request lacks a body");
        }
        await request.body.cancel();
    });
    const body = new Uint8Array(4 * WINDOW_BYTES).fill(7);
    const response = await relay.fetch(new Request(head.url, { method: "POST", body }));
    await answering;

    expect(await response.text()).toBe("early");
    await until(() => relay.size === 0 && machine.size === 0);
});

test("reset the streams a peer opens beyond the open stream limit, leaving the session open", () => {
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
    const request = new Request("https://notes--personal--acme.destack.space/notes", {
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

test("refuse a request on a closed session, and frames shorter than their header or of an unknown type", async () => {
    // close the relay's session, then decode a cut frame and one of an unknown type
    const { relay } = pair();
    relay.close();
    const closed = await refusal(relay.ping());
    const encoded = new Frame(FrameType.data, 0, 1, 0, new Uint8Array(4)).encode();
    const cut = await refusal(async () => Frame.decode(encoded.slice(0, 3)));
    const unknown = encoded.slice();
    unknown[0] = 0xff;
    const typed = await refusal(async () => Frame.decode(unknown));

    expect([closed, cut, typed]).toEqual([
        ["SERVICE_UNAVAILABLE", "SESSION_CLOSED", "session is closed"],
        ["BAD_REQUEST", "PROTOCOL_VIOLATION", "frame is shorter than its header"],
        ["BAD_REQUEST", "PROTOCOL_VIOLATION", "frame type 255 is unknown"],
    ]);
});
