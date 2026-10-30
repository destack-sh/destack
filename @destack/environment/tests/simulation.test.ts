import { present, Uuid } from "@destack/schema";
import { expect, test } from "@destack/test";
import { LONGEST_DELAY, Random } from "../src/index.ts";
import {
    type NodeStart,
    SimulatedClock,
    SimulatedRandom,
    Simulation,
} from "../src/simulation/index.ts";
import type { ServerSocket, Socket } from "../src/index.ts";
import { BunNetwork } from "../src/network/bun.ts";
import { BunFileSystem } from "../src/file/bun.ts";
import type { WorldStep } from "../src/world/world.ts";
import { join } from "node:path";

/** The workers racing on host work in a replayed world. */
const WORKERS = 8;

/** The rounds of host work each racing worker takes. */
const ROUNDS = 6;

/** The simulations' start: 2026-10-16 at midnight UTC. */
const START = Date.UTC(2026, 9, 16);
/** The encoder of the socket tests' lines. */
const ENCODER = new TextEncoder();
/** The decoder of the socket tests' lines. */
const DECODER = new TextDecoder();

test("release a simulated clock's waits in deadline order as it moves in whole milliseconds, and refuse moving it back", async () => {
    // pass two of three waits
    const clock = new SimulatedClock(START);
    const released: number[] = [];
    for (const delay of [300, 100, 200]) {
        void clock.sleep(delay).then(() => released.push(delay));
    }
    clock.advance(250);
    await Promise.resolve();

    // release passed waits in order
    expect([released, clock.next]).toEqual([[100, 200], START + 300]);
    expect(() => clock.set(START)).toThrow(
        new RangeError(`a simulated clock at ${START + 250} cannot go back to ${START}`),
    );

    // round a fractional wait up to the next whole millisecond, and refuse a fractional time
    void clock.sleep(0.5).then(() => released.push(0.5));
    expect(clock.next).toBe(START + 251);
    expect(() => clock.set(START + 251.5)).toThrow(
        new RangeError(`a simulated clock moves in whole milliseconds, not to ${START + 251.5}`),
    );
});

test("release the waits a flowing clock passed once it pauses, leaving none behind its reading", async () => {
    // flow past a wait's deadline on the system's time, then pause before its timer fires
    const clock = new SimulatedClock(START).resume();
    let isReleased = false;
    void clock.sleep(1).then(() => (isReleased = true));
    const flowed = Date.now() + 5;
    while (Date.now() < flowed) {
        // let the system's time pass without yielding to the timer
    }
    clock.pause();
    await Promise.resolve();

    // release it at the pause, leaving no wait before the clock's reading
    expect([isReleased, clock.next]).toEqual([true, undefined]);
});

test("abort a simulated clock's timeout signal with a timeout once the clock passes it, outside the counted waits", async () => {
    // start a timeout of a second and move the clock to just before it
    const clock = new SimulatedClock(START);
    const signal = clock.timeout(1000);
    const aborted = new Promise<void>((resolve) => {
        signal.addEventListener("abort", () => resolve());
    });
    clock.advance(999);
    const early = signal.aborted;

    // move past it and wait for the abort
    clock.advance(1);
    await aborted;
    const reason: unknown = signal.reason;
    const name = reason instanceof DOMException ? reason.name : reason;

    // abort once passed, as the web's timeout signal does, beginning no counted wait
    expect([early, signal.aborted, name, clock.begun]).toEqual([false, true, "TimeoutError", 0]);
});

/** Draw six bytes and an integer below a thousand from a seed. */
function draw(seed: number) {
    const random = new SimulatedRandom(seed);

    return [[...random.bytes(6)], Random.int(random, 1000)];
}

test("trace tracked work as spans on the world's clock, marking failed work, while tracing", async () => {
    // track host work, jump a second, then track world work that fails
    await using simulation = new Simulation({ seed: 1, start: START, isTracing: true });
    const { scheduler } = simulation;
    await scheduler.track(
        new Promise<void>((resolve) => {
            setTimeout(resolve, 1);
        }),
        "host",
    );
    await simulation.jump(START + 1000);
    const failed = await scheduler
        .track(Promise.reject(new Error("refused")), "world")
        .catch((error: Error) => error.message);
    await simulation.idle();

    // read each span's name, who finished it, its status and its times from the start
    const spans = simulation.telemetry.spans.map((span) => ({
        name: span.name,
        by: span.attributes["destack.simulation.by"],
        status: span.status.code,
        from: span.startTime[0] * 1000 - START,
        to: span.endTime[0] * 1000 - START,
    }));

    expect({ failed, spans }).toEqual({
        failed: "refused",
        spans: [
            { name: "destack.simulation.track", by: "host", status: 0, from: 0, to: 0 },
            { name: "destack.simulation.track", by: "world", status: 2, from: 1000, to: 1000 },
        ],
    });
});

test("skew each node's clock from the seed, wait its own milliseconds on the world's timers, and bump it", async () => {
    // read two skewed nodes' clocks against the world's
    await using simulation = new Simulation({
        seed: 1,
        start: START,
        skew: { offset: 500, drift: 100 },
    });
    const left = simulation.environment("left").clock;
    const right = simulation.environment("right").clock;
    const offsets = [left.now() - START, right.now() - START];

    // sleep a node's ten seconds, reading how much world time passed
    await left.sleep(10_000);
    const slept = simulation.clock.now() - START;

    // bump the right node back a second
    const before = right.now();
    simulation.bump("right", -1000);
    const bumped = right.now() - before;

    // draw offsets within half a second, the left node running slow by its drift
    expect({ offsets, slept, bumped }).toEqual({
        offsets: [-398, -329],
        slept: 10_001,
        bumped: -1000,
    });

    // refuse a bump of an unknown node, and of a node in a world without skew
    await using plain = new Simulation({ seed: 1, start: START });
    expect(() => plain.bump("node", 1)).toThrow(new TypeError("the world has no node node"));
    plain.environment("node");
    expect(() => plain.bump("node", 1)).toThrow(
        new TypeError("node node reads the world's clock: give the world a skew"),
    );
});

test("share files between nodes in memory, leaving the host's files untouched", async () => {
    // write a file from one node and read it from another in a memory world
    await using simulation = new Simulation({ seed: 1, start: START, files: "memory" });
    const writer = simulation.environment("writer").files;
    const reader = simulation.environment("reader").files;
    const directory = await writer.makeTemporaryDirectory("notes-");
    await writer.writeTextFile(join(directory, "note.txt"), "in memory");

    // read it back, and look for it on the host
    const read = await reader.readTextFile(join(directory, "note.txt"));
    const isOnHost = await new BunFileSystem().exists(join(directory, "note.txt"));

    expect({ read, isOnHost }).toEqual({ read: "in memory", isOnHost: false });
});

test("draw the same bytes and integers from the same seed, and others from another", () => {
    // match draws by seed
    const [first, again, other] = [draw(7), draw(7), draw(8)];
    expect([first === again, first]).toEqual([false, again]);
    expect(other).not.toEqual(first);
});

test("route requests between nodes through latency, partitions and loss, replayed by the seed", async () => {
    const run = async (seed: number) => {
        // request through latency, tracing what happens
        await using simulation = new Simulation({ seed, start: START, isTracing: true });
        simulation.network.server("server").http({
            hostname: "server.test",
            port: 443,
            fetch: (request) => Response.json({ path: new URL(request.url).pathname }),
        });
        simulation.network.degrade({ delay: { least: 10, most: 20 } });
        const client = simulation.environment("client").network;
        const body: unknown = await client
            .fetch(new Request("https://server.test/notes"))
            .then((response) => response.json());

        // request through a partition and full loss
        simulation.network.partition("client", "server");
        const partitioned = await client
            .fetch(new Request("https://server.test/"))
            .catch((error: Error) => error.message);
        simulation.network.heal();
        simulation.network.degrade({ drop: 1 });
        const dropped = await client
            .fetch(new Request("https://server.test/"))
            .catch((error: Error) => error.message);

        // read the world's events from its telemetry, with their times and fields
        const events = simulation.telemetry.logs.map((record) => ({
            name: record.eventName,
            at: record.hrTime,
            attributes: record.attributes,
        }));

        return { body, partitioned, dropped, events };
    };

    // replay the events by seed
    const [first, again] = [await run(3), await run(3)];
    expect({
        body: first.body,
        partitioned: first.partitioned,
        dropped: first.dropped,
        names: first.events.map((event) => event.name),
        replayed: first.events,
    }).toEqual({
        body: { path: "/notes" },
        partitioned: "fetch failed: client cannot reach server",
        dropped: "fetch failed: the request to https://server.test was lost",
        names: [
            "destack.simulation.degrade",
            "destack.simulation.timer",
            "destack.simulation.request",
            "destack.simulation.timer",
            "destack.simulation.partition",
            "destack.simulation.refusal",
            "destack.simulation.heal",
            "destack.simulation.degrade",
            "destack.simulation.loss",
        ],
        replayed: again.events,
    });
});

test("route a name to its exact host first, else to the wildcard of its nearest enclosing name, and refuse the bare domain", async () => {
    // serve a wildcard below the domain, a nearer wildcard below it, and one exact host
    await using simulation = new Simulation({ seed: 1, start: START });
    const serve = (node: string, hostname: string) =>
        simulation.network.server(node).http({
            hostname,
            port: 443,
            fetch: () => new Response(node),
        });
    serve("relay", "*.destack.computer");
    serve("nearer", "*.eu.destack.computer");
    serve("exact", "laptop--acme.destack.computer");
    const client = simulation.environment("client").network;
    const answer = (host: string) =>
        client.fetch(new Request(`https://${host}/`)).then(
            (response) => response.text(),
            (error: Error) => error.message,
        );

    expect([
        await answer("laptop--acme.destack.computer"),
        await answer("desktop--acme.destack.computer"),
        await answer("a.b.destack.computer"),
        await answer("desktop.eu.destack.computer"),
        await answer("destack.computer"),
    ]).toEqual([
        "exact",
        "relay",
        "relay",
        "nearer",
        "fetch failed: nothing serves https://destack.computer",
    ]);
});

test("route localhost and the names below it to the loopback listener of their port, as RFC 6761 reserves them", async () => {
    // serve the loopback on one port, and nothing on another
    await using simulation = new Simulation({ seed: 1, start: START });
    simulation.network.server("machine").http({
        hostname: "127.0.0.1",
        port: 4000,
        fetch: (request) => new Response(new URL(request.url).hostname),
    });
    const client = simulation.environment("client").network;
    const answer = (url: string) =>
        client.fetch(new Request(url)).then(
            (response) => response.text(),
            (error: Error) => error.message,
        );

    expect([
        await answer("http://localhost:4000/"),
        await answer("http://notes--home--acme.localhost:4000/"),
        await answer("http://notes--home--acme.localhost:4001/"),
        await answer("http://localhost.example:4000/"),
    ]).toEqual([
        "localhost",
        "notes--home--acme.localhost",
        "fetch failed: nothing serves http://notes--home--acme.localhost:4001",
        "fetch failed: nothing serves http://localhost.example:4000",
    ]);
});

test("skip simulated time to each next timer until a condition holds, and refuse a condition idle work never meets", async () => {
    // sleep a day, then wait on nothing
    await using simulation = new Simulation({ seed: 1, start: START });
    let isAwake = false;
    void simulation.clock.sleep(24 * 60 * 60_000).then(() => (isAwake = true));
    await simulation.until(() => isAwake);
    const woke = simulation.clock.now();
    const refused = await simulation.until(() => false).catch((error: Error) => error.message);

    // skip the day and refuse the idle run
    expect([woke - START, refused]).toEqual([
        24 * 60 * 60_000,
        "the world went idle with nothing left to do",
    ]);
});

test("hold time back while tracked work is in flight, move to the next timer once it settles, and run to a time", async () => {
    // start a timer a minute out, and track work that settles on the system's time
    await using simulation = new Simulation({ seed: 1, start: START });
    let isFired = false;
    void simulation.clock.sleep(60_000).then(() => (isFired = true));
    const work = Promise.withResolvers<void>();
    void simulation.scheduler.track(work.promise, "world");
    let held: [number, boolean] | undefined;
    setTimeout(() => {
        held = [simulation.clock.now() - START, isFired];
        work.resolve();
    }, 20);

    // run until the timer fires
    await simulation.until(() => isFired);
    const fired = simulation.clock.now() - START;

    // run to a time past one more timer
    let isLater = false;
    void simulation.clock.sleep(15_000).then(() => (isLater = true));
    await simulation.clock.sleep(START + 90_000 - simulation.clock.now());
    await simulation.idle();

    // stay at the start while the work runs, reach the timer only after it settled, and stop at the time
    expect({
        held,
        fired,
        reached: simulation.clock.now() - START,
        inFlight: simulation.scheduler.inFlight,
        isLater,
    }).toEqual({ held: [0, false], fired: 60_000, reached: 90_000, inFlight: 0, isLater: true });
});

/** Answer an in-process call after 20 ms of the system's time, busy before its head. */
async function answerSlowly(): Promise<Response> {
    await new Promise<void>((resolve) => {
        setTimeout(resolve, 20);
    });

    return new Response("done");
}

test("hold the clock for an in-process call until its response head, and not for the long poll streamed after it", async () => {
    // answer a long poll at once, its body ending on a timer a minute out, and a slow call after a turn of work
    await using simulation = new Simulation({ seed: 1, start: START });
    const longPoll = () =>
        new Response(
            new ReadableStream({
                start: (controller) =>
                    void simulation.clock.sleep(60_000).then(() => controller.close()),
            }),
        );
    let isAnswered = false;
    void simulation.clock.sleep(1000).then(() => undefined);

    // call both in process, tracking each until its head
    const polled = await simulation.scheduler.track(Promise.resolve(longPoll()), "world");
    const answered = simulation.scheduler.track(answerSlowly(), "world").then((response) => {
        isAnswered = true;

        return response;
    });
    await simulation.until(() => isAnswered);
    const answeredAt = simulation.clock.now() - START;
    const ended = polled.text().then(() => simulation.clock.now() - START);
    await simulation.clock.sleep(START + 60_000 - simulation.clock.now());
    await simulation.idle();

    // answer the slow call before time moves, and end the poll's body at its timer
    expect({ answeredAt, text: await (await answered).text(), ended: await ended }).toEqual({
        answeredAt: 0,
        text: "done",
        ended: 60_000,
    });
});

test("count a node's WebCrypto operations in flight on the world's clock", async () => {
    // digest on a node's crypto, reading the work in flight at once and once it settled
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("node");
    const digest = crypto.digest("SHA-256", new Uint8Array(4));
    const during = simulation.scheduler.inFlight;
    const bytes = (await digest).byteLength;
    const after = simulation.scheduler.inFlight;

    expect({ during, bytes, after }).toEqual({ during: 1, bytes: 32, after: 0 });
});

test("crash a node, losing its memory and stopping its work, and start it again from what it stored", async () => {
    // tick each second, counting starts
    await using simulation = new Simulation({ seed: 1, start: START });
    const stored = { starts: 0 };
    let ticks = 0;
    const node = simulation.node("worker");
    const work: NodeStart<{ readonly memory: string }> = async (environment, signal) => {
        stored.starts += 1;
        void (async () => {
            while (!signal.aborted) {
                await environment.clock.sleep(1000, signal).catch(() => undefined);
                ticks += signal.aborted ? 0 : 1;
            }
        })();

        return { memory: "fresh" };
    };
    await node.start(work);

    // crash, wait and restart
    await simulation.clock.sleep(3000);
    await simulation.idle();
    node.crash();
    const whileDown = ticks;
    await simulation.clock.sleep(3000);
    await simulation.idle();
    const crashed = ticks;
    const restarted = await node.start(work);

    // stop ticks on crash and start again
    expect({ whileDown, crashed, starts: stored.starts, memory: restarted }).toEqual({
        whileDown: 3,
        crashed: 3,
        starts: 2,
        memory: { memory: "fresh" },
    });
});

test("bridge the host into the world as a node, reaching an origin's requests and WebSockets at its .localhost name", async () => {
    // serve echo.test's sockets in the world, and bridge the host in as the lab
    await using simulation = new Simulation({ seed: 1, start: START });
    const { events } = serveEcho(simulation, "server");
    const bridge = simulation.network.bridge("lab");
    const { port } = bridge.url;

    // ask a plain request, which the echo refuses without an upgrade, and one for a host outside .localhost
    const plain = await fetch(`http://echo.test.localhost:${port}/`);
    const outside = await fetch(`http://127.0.0.1:${port}/`);

    // open a host WebSocket through the bridge, send a message and read the echo
    const socket = new WebSocket(`ws://echo.test.localhost:${port}/?name=lab`, ["echo"]);
    const echoed = await new Promise<string>((resolve, reject) => {
        socket.addEventListener("open", () => socket.send("hello"));
        socket.addEventListener("message", (event) => resolve(String(event.data)));
        socket.addEventListener("error", () => reject(new Error("the bridged socket failed")));
    });
    socket.close();
    await bridge.stop(true);

    expect({
        plain: [plain.status, await plain.text()],
        outside: [outside.status, await outside.text()],
        echoed,
        opened: events[0],
    }).toEqual({
        plain: [426, "no socket"],
        outside: [421, "the bridge reaches only hosts below .localhost"],
        echoed: "lab: hello",
        opened: "open lab",
    });
});

test("close a bridged WebSocket at the world's end when the host closes it, and refuse one the world refuses", async () => {
    // open a host WebSocket to the echo through the bridge, then close it from the host
    await using simulation = new Simulation({ seed: 1, start: START });
    const { events } = serveEcho(simulation, "server");
    simulation.network.server("server").http({
        hostname: "plain.test",
        port: 443,
        fetch: () => new Response("no socket", { status: 426 }),
    });
    const bridge = simulation.network.bridge("lab");
    const { port } = bridge.url;
    const socket = new WebSocket(`ws://echo.test.localhost:${port}/?name=lab`, ["echo"]);
    await new Promise<void>((resolve) => {
        socket.addEventListener("open", () => resolve(), { once: true });
    });
    const closed = new Promise<void>((resolve) => {
        socket.addEventListener("close", () => resolve(), { once: true });
    });
    socket.close(1000);
    await closed;
    await simulation.idle();

    // ask for a WebSocket the world answers with a plain response
    const refused = await fetch(`http://plain.test.localhost:${port}/`, {
        headers: {
            upgrade: "websocket",
            connection: "upgrade",
            "sec-websocket-version": "13",
            "sec-websocket-key": "dGhlIHNhbXBsZSBub25jZQ==",
        },
    });
    await bridge.stop(true);

    expect({ events, refused: [refused.status, await refused.text()] }).toEqual({
        events: ["open lab", "close lab 1000"],
        refused: [502, "the world refused the WebSocket"],
    });
});

/** Server an echo of every message and the attachment its sockets were accepted with. */
function serveEcho(simulation: Simulation, node: string) {
    const events: string[] = [];
    const listener = simulation.network.server(node).http<{ readonly name: string }>({
        hostname: "echo.test",
        port: 443,
        fetch: (request, served) =>
            served.upgrade(request, {
                attachment: { name: new URL(request.url).searchParams.get("name") ?? "" },
                headers: { "sec-websocket-protocol": "echo" },
            })
                ? undefined
                : new Response("no socket", { status: 426 }),
        websocket: {
            open: (socket: ServerSocket<{ readonly name: string }>) =>
                events.push(`open ${socket.attachment.name}`),
            message: (socket, message) =>
                socket.send(`${socket.attachment.name}: ${String(message)}`),
            close: (socket, code) => events.push(`close ${socket.attachment.name} ${code}`),
        },
    });

    return { events, listener };
}

/** Open a client's socket and read its events as text. */
function open(simulation: Simulation, node: string, name: string) {
    const events: string[] = [];
    const socket = simulation.network.of(node).websocket(`wss://echo.test/?name=${name}`, ["echo"]);
    socket.addEventListener("open", () => events.push(`open ${socket.protocol}`));
    socket.addEventListener("message", (event: MessageEvent) => events.push(String(event.data)));
    socket.addEventListener("close", (event: CloseEvent) => events.push(`close ${event.code}`));

    return { events, socket };
}

test("carry a socket's messages in order through latency, and close it from either end", async () => {
    // open a socket through 5 to 50 ms of latency and send three messages
    await using simulation = new Simulation({ seed: 9, start: START });
    simulation.network.degrade({ delay: { least: 5, most: 50 } });
    const server = serveEcho(simulation, "server");
    const client = open(simulation, "browser", "ada");
    await simulation.until(() => client.socket.readyState === WebSocket.OPEN);
    for (const message of ["one", "two", "three"]) {
        client.socket.send(message);
    }
    await simulation.until(() => client.events.length === 4);
    client.socket.close();
    await simulation.until(() => client.events.length === 5);

    // the answers arrived in order, and the close reached both ends
    expect([client.events, server.events]).toEqual([
        ["open echo", "ada: one", "ada: two", "ada: three", "close 1000"],
        ["open ada", "close ada 1000"],
    ]);
});

test("fail a socket its listener refuses with a plain answer, and discard the answer's body", async () => {
    // refuse every upgrade with a streamed body, recording its cancellation
    await using simulation = new Simulation({ seed: 9, start: START });
    const cancelled: string[] = [];
    simulation.network.server("server").http({
        hostname: "echo.test",
        port: 443,
        fetch: () =>
            new Response(new ReadableStream({ cancel: () => void cancelled.push("cancelled") }), {
                status: 503,
            }),
    });
    const refused = open(simulation, "laptop", "ada");
    await simulation.until(() => refused.events.length === 1);
    expect([refused.events, cancelled]).toEqual([["close 1006"], ["cancelled"]]);
});

test("break sockets across a partition and from a crashed node, and close them on a stopped listener", async () => {
    // serve the echo from a node, and open one socket to be cut off and one to see the crash
    await using simulation = new Simulation({ seed: 9, start: START });
    const server = simulation.node("server");
    const serve = async () => serveEcho(simulation, "server").listener;
    await server.start(serve);
    const cut = open(simulation, "browser", "ada");
    const crashed = open(simulation, "laptop", "grace");
    await simulation.until(() => cut.events.length === 1 && crashed.events.length === 1);

    // cut the browser off, then crash the server and connect it while it is down
    simulation.network.partition("browser", "server");
    server.crash();
    const down = open(simulation, "laptop", "lin");
    await simulation.until(() => down.events.length === 1);

    // restart it, and close an open socket by stopping its listener
    const listener = await server.start(serve);
    const stopped = open(simulation, "laptop", "ming");
    await simulation.until(() => stopped.events.length === 1);
    await listener.stop(true);
    await simulation.until(() => stopped.events.length === 2);

    // each socket ended the way its fault ends a connection
    expect([cut.events, crashed.events, down.events, stopped.events]).toEqual([
        ["open echo", "close 1006"],
        ["open echo", "close 1006"],
        ["close 1006"],
        ["open echo", "close 1001"],
    ]);
});

test("connect nodes sharing the host's files over a Unix socket through isolation and loss, refusing TLS toward one", async () => {
    // listen at a Unix socket on one node, then isolate both nodes and drop every network delivery
    await using simulation = new Simulation({ seed: 1, start: START });
    const listener = await simulation.network.server("database").socket({
        unix: "/run/destack.sock",
        accept: (socket) => void socket.readable.pipeTo(socket.writable).catch(() => undefined),
    });
    simulation.network.isolate("database");
    simulation.network.isolate("app");
    simulation.network.degrade({ drop: 1 });

    // echo a line from another node, refuse a path nothing listens at, and refuse TLS toward the socket
    const { network } = simulation.environment("app");
    const socket = network.connect({ unix: "/run/destack.sock" });
    const echoed = (async () => {
        await socket.opened;
        const writer = socket.writable.getWriter();
        await writer.write(ENCODER.encode("ping"));
        const { value } = await socket.readable.getReader().read();

        return DECODER.decode(value);
    })();
    const refused = network
        .connect({ unix: "/run/missing.sock" })
        .opened.catch((error: Error) => coded(error));
    const insecure = (() => {
        try {
            return network.connect({ unix: "/run/destack.sock" }, { secureTransport: "on" });
        } catch (error) {
            return error;
        }
    })();

    expect([listener.address, await echoed, await refused, insecure]).toEqual([
        { unix: "/run/destack.sock" },
        "ping",
        "ConnectionRefused: connection refused: nothing listens at /run/missing.sock",
        new TypeError("a Unix socket connection cannot use TLS"),
    ]);
    await socket.close();
    await listener.stop();
});

/** Listen for socket connections on a node, echoing each line back in upper case. */
async function listenUpper(simulation: Simulation, node: string) {
    const accepted: Socket[] = [];
    const listener = await simulation.network.server(node).socket({
        hostname: "smtp.test",
        port: 25,
        accept: (socket) => {
            accepted.push(socket);
            const upper = new TransformStream<Uint8Array, Uint8Array>({
                transform: (chunk, controller) =>
                    controller.enqueue(ENCODER.encode(DECODER.decode(chunk).toUpperCase())),
            });
            void socket.readable
                .pipeThrough(upper)
                .pipeTo(socket.writable)
                .catch(() => undefined);
        },
    });

    return { accepted, listener };
}

/** Read a failure as its error code and message. */
function coded(error: Error): string {
    return `${"code" in error ? String(error.code) : "uncoded"}: ${error.message}`;
}

/** Connect a node's TCP connection to the listener, reading what arrives as text. */
function connectUpper(simulation: Simulation, node: string) {
    const received: string[] = [];
    const socket = simulation.network
        .of(node)
        .connect({ hostname: "smtp.test", port: 25 }, { secureTransport: "starttls" });
    const read = socket.readable
        .pipeTo(new WritableStream({ write: (chunk) => void received.push(DECODER.decode(chunk)) }))
        .then(
            () => "ended",
            (error: Error) => coded(error),
        );

    return { received, read, socket };
}

test("carry TCP bytes in order through latency, start TLS on a new socket, and end the connection at both ends", async () => {
    // connect through 5 to 50 ms of latency, and ask for TLS in plain text
    await using simulation = new Simulation({ seed: 9, start: START });
    simulation.network.degrade({ delay: { least: 5, most: 50 } });
    await listenUpper(simulation, "mail");
    const client = connectUpper(simulation, "app");
    let isOpen = false;
    void client.socket.opened.then(() => (isOpen = true));
    await simulation.until(() => isOpen);
    const writer = client.socket.writable.getWriter();
    await writer.write(ENCODER.encode("ehlo\r\n"));
    await writer.write(ENCODER.encode("starttls\r\n"));
    await simulation.until(() => client.received.length === 2);

    // start TLS, refuse the old socket's writes, and talk over the new one
    const secure = client.socket.startTls();
    const stale = await writer
        .write(ENCODER.encode("noop\r\n"))
        .catch((error: Error) => error.message);
    const answered: string[] = [];
    void secure.readable.pipeTo(
        new WritableStream({ write: (chunk) => void answered.push(DECODER.decode(chunk)) }),
    );
    await secure.writable.getWriter().write(ENCODER.encode("quit\r\n"));
    await simulation.until(() => answered.length === 1);

    // close the new socket, which ends the server's end too
    let isClosed = false;
    void secure.close().then(() => (isClosed = true));
    await simulation.until(() => isClosed);

    // refuse TLS on a plain connect
    const plain = simulation.network.of("app").connect({ hostname: "smtp.test", port: 25 });
    const refused = (() => {
        try {
            plain.startTls();

            return "started";
        } catch (error) {
            return error instanceof Error ? error.message : "refused";
        }
    })();

    // answer in order, the old readable ending at the upgrade, and the new socket after it
    expect({
        received: client.received,
        read: await client.read,
        stale,
        answered,
        isNew: secure !== client.socket,
        refused,
    }).toEqual({
        received: ["EHLO\r\n", "STARTTLS\r\n"],
        read: "ended",
        stale: "the connection started TLS on another socket",
        answered: ["QUIT\r\n"],
        isNew: true,
        refused: "only a connection connected with starttls can start TLS",
    });
});

test("half-close a TCP connection, delivering the peer's bytes in flight, and ignore a second close", async () => {
    // serve a node that answers each chunk once and records what it reads
    await using simulation = new Simulation({ seed: 9, start: START });
    simulation.network.degrade({ delay: { least: 5, most: 50 } });
    const served: string[] = [];
    let serverRead = "";
    await simulation.network.server("mail").socket({
        hostname: "smtp.test",
        port: 25,
        accept: (socket) => {
            const writer = socket.writable.getWriter();
            serverRead = "reading";
            void socket.readable
                .pipeTo(
                    new WritableStream({
                        write: async (chunk) => {
                            served.push(DECODER.decode(chunk));
                            await writer.write(ENCODER.encode("250 ok\r\n"));
                        },
                    }),
                )
                .then(() => (serverRead = "ended"));
        },
    });

    // write a line and close at once, before the answer arrives
    const socket = simulation.network.of("app").connect({ hostname: "smtp.test", port: 25 });
    const received: string[] = [];
    const read = socket.readable.pipeTo(
        new WritableStream({ write: (chunk) => void received.push(DECODER.decode(chunk)) }),
    );
    await socket.opened;
    await socket.writable.getWriter().write(ENCODER.encode("quit\r\n"));
    const closes = [socket.close(), socket.close()];
    await Promise.all(closes);
    await read;

    // deliver the answer sent before the server saw the end, and end both ways
    expect({ served, received, serverRead }).toEqual({
        served: ["quit\r\n"],
        received: ["250 ok\r\n"],
        serverRead: "ended",
    });
});

test("refuse a TCP connect nothing listens at, and break connections across a partition, a released node and a stopped listener", async () => {
    // connect an unserved address, then serve and open three connections
    await using simulation = new Simulation({ seed: 9, start: START });
    const unserved = connectUpper(simulation, "app");
    const refused = await unserved.socket.opened.catch(coded);
    const server = await listenUpper(simulation, "mail");
    const [cut, crashed, stopped] = [
        connectUpper(simulation, "app"),
        connectUpper(simulation, "worker"),
        connectUpper(simulation, "cron"),
    ];
    await simulation.until(() => server.accepted.length === 3);

    // cut the app off, release the worker, and stop the listener
    simulation.network.partition("app", "mail");
    simulation.network.release("worker");
    await server.listener.stop();
    const written = await cut.socket.writable
        .getWriter()
        .write(ENCODER.encode("late\r\n"))
        .catch(coded);

    // each connection broke the way its fault breaks a transport
    const reads = [cut.read, crashed.read, stopped.read, unserved.read];
    expect([refused, written, ...(await Promise.all(reads))]).toEqual([
        "ConnectionRefused: connection refused: nothing listens at smtp.test:25",
        "ECONNRESET: the connection app mail broke",
        "ECONNRESET: the connection app mail broke",
        "ECONNRESET: the connection worker mail broke",
        "ECONNRESET: the connection cron mail broke",
        "ConnectionRefused: connection refused: nothing listens at smtp.test:25",
    ]);
});

test("await work through its timers on the clock, the world moving time once idle", async () => {
    // await work waiting on the clock in a running world
    await using simulation = new Simulation({ seed: 1, start: START });
    const timed = (async () => {
        await simulation.clock.sleep(60_000);

        return "done";
    })();

    // settle the clock's work at its sleep
    expect([await timed, simulation.clock.now() - START]).toEqual(["done", 60_000]);
});

test("refuse work that keeps busy without letting time move, naming the call site that kept it busy", async () => {
    // keep tracking work that re-enqueues itself each event loop turn
    await using simulation = new Simulation({ seed: 1, start: START, livelockTurns: 1000 });
    const stop = new AbortController();
    const spinning = (async () => {
        while (!stop.signal.aborted) {
            await simulation.scheduler.track(
                new Promise<void>((resolve) => {
                    setImmediate(resolve);
                }),
                "world",
            );
        }
    })();
    const refused = await simulation.clock.sleep(1000).then(
        () => "settled",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    stop.abort();
    await spinning;

    // refuse at the busy limit, the report leading with the spinning work's count and kind
    const [heading, site = ""] = refused.split("\n");
    expect([heading, site.split(" ").slice(0, 2), simulation.clock.now() - START]).toEqual([
        "the simulation kept busy for 1000 turns without moving time:",
        ["100x", "world"],
        0,
    ]);
});

test("settle a slow host round trip without counting its wait as busy", async () => {
    // serve a loopback reply after a delay of real time
    const server = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        fetch: async () => {
            await Bun.sleep(20);

            return new Response("late");
        },
    });

    // wait on the round trip as host work while a timer waits
    try {
        await using simulation = new Simulation({ seed: 1, start: START });
        const reply = simulation.scheduler.track(
            fetch(server.url).then((response) => response.text()),
            "host",
        );
        expect([await reply, simulation.clock.now() - START]).toEqual(["late", 0]);
    } finally {
        await server.stop(true);
    }
});

test("hold the clock while a child writes its whole output, so a timer beside it waits", async () => {
    // race a slow child's output against a one-millisecond timer
    await using simulation = new Simulation({ seed: 1, start: START });
    const { clock, process } = simulation.environment("host");
    const child = process.spawn(["sh", "-c", "sleep 0.05; echo done"]);
    const first = Promise.race([
        child.output().then(({ code, stdout }) => [code, DECODER.decode(stdout)]),
        clock.sleep(1).then(() => "timed out"),
    ]);

    expect([await first, clock.now() - START]).toEqual([[0, "done\n"], 0]);
});

test("refuse tracked work that never settles beside a timer, as time cannot move while it is in flight", async () => {
    // track work that never settles while a timer waits
    await using simulation = new Simulation({ seed: 1, start: START });
    void simulation.scheduler.track(
        new Promise<void>(() => {
            // never settle
        }),
        "world",
    );
    const refused = await simulation.clock.sleep(1000).then(
        () => "settled",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );

    // refuse the deadlock naming the one work in flight, the time unmoved
    const [header, ...sites] = refused.split("\n");
    expect([header, sites.length, simulation.clock.now() - START]).toEqual([
        "the world's work waits on simulated time, which cannot move while it is in flight:",
        1,
        0,
    ]);
});

test("fork a world by a label into a world of its own draws, starting at the parent's time", async () => {
    // draw ids in a parent, then fork it twice by one label and once by another
    await using parent = new Simulation({ seed: 1, start: START });
    const drawn = drawIds(parent);
    await parent.jump(parent.clock.now() + 1000);
    await using first = parent.fork("copy");
    await using again = Simulation.fork({ seed: 1, start: START + 1000 }, "copy");
    await using other = parent.fork("other");
    const forked = { first: drawIds(first), again: drawIds(again), other: drawIds(other) };

    // repeat a label's draws, draw others for another label, and none of the parent's
    const all = [...drawn, ...forked.first, ...forked.other];
    expect({
        starts: [first.clock.now(), again.clock.now(), other.clock.now()],
        again: forked.again,
        distinct: new Set(all).size,
    }).toEqual({
        starts: [START + 1000, START + 1000, START + 1000],
        again: forked.first,
        distinct: 9,
    });
});

/** Draw three ids in a world. */
function drawIds(world: Simulation): string[] {
    return Array.from({ length: 3 }, () => Uuid.v7(world.environment("app")));
}

test("jump a day at once, waking a repeating wait a single time where running steps through every period", async () => {
    // count the beats of a loop waiting ten seconds between beats
    const beats = async (move: (simulation: Simulation) => Promise<void>) => {
        await using simulation = new Simulation({ seed: 1, start: START });
        const { clock } = simulation.environment("node");
        let count = 0;
        const stop = new AbortController();
        const loop = (async () => {
            for (;;) {
                await clock.sleep(10_000, stop.signal);
                count += 1;
            }
        })().catch(() => undefined);
        await move(simulation);
        stop.abort();
        await loop;

        return { count, now: simulation.clock.now() };
    };

    // run an hour through every beat, and jump a day with a single beat
    expect({
        run: await beats((simulation) => simulation.clock.sleep(3_600_000)),
        jump: await beats((simulation) => simulation.jump(START + 86_400_000)),
    }).toEqual({
        run: { count: 360, now: START + 3_600_000 },
        jump: { count: 1, now: START + 86_400_000 },
    });
});

test("jump right after awaited work settles, and refuse a second jump while one jumps", async () => {
    // await a sleep, then jump at once
    await using simulation = new Simulation({ seed: 1, start: START });
    const { clock } = simulation.environment("node");
    await clock.sleep(1000);
    await simulation.jump(START + 60_000);
    const jumped = simulation.clock.now() - START;

    // refuse a jump while another jumps
    const pending = simulation.jump(START + 120_000);
    const refused = await simulation.jump(START + 180_000).then(
        () => "jumped",
        (error: unknown) => (error instanceof Error ? error.message : "refused"),
    );
    await pending;

    expect([jumped, refused]).toEqual([60_000, "a simulation cannot jump while it jumps"]);
});

test("give up an aborted request, lose a response cut on its way back, and keep a listener a node serves again", async () => {
    // serve a hung origin, and one whose answer the network cuts on its way back
    await using simulation = new Simulation({ seed: 1, start: START });
    const server = simulation.network.server("server");
    server.http({ hostname: "hung.test", port: 443, fetch: () => new Promise<Response>(() => {}) });
    server.http({
        hostname: "cut.test",
        port: 443,
        fetch: () => {
            simulation.network.partition("client", "server");

            return new Response("late");
        },
    });
    const { clock, network } = simulation.environment("client");
    const failure = (url: string, signal?: AbortSignal) =>
        network.fetch(new Request(url, signal === undefined ? {} : { signal })).then(
            (response) => response.text(),
            (error: Error) => [error.name, error.message],
        );
    const hung = await failure("https://hung.test/", clock.timeout(1000));
    const cut = await failure("https://cut.test/");
    simulation.network.heal();

    // stop an old listener after the address serves again, keeping the new one
    const old = server.http({
        hostname: "again.test",
        port: 443,
        fetch: () => new Response("old"),
    });
    simulation.network.release("server");
    server.http({ hostname: "again.test", port: 443, fetch: () => new Response("new") });
    await old.stop();
    const again = await failure("https://again.test/");

    expect({ hung, cut, again }).toEqual({
        hung: ["TimeoutError", "signal timed out"],
        cut: ["TypeError", "fetch failed: the response from https://cut.test was lost"],
        again: "new",
    });
});

test("fail a streamed response's body once its request aborts, cancelling the listener's stream, as fetch does", async () => {
    // serve a stream that sends one chunk and never ends, noting its cancellation
    await using simulation = new Simulation({ seed: 1, start: START });
    let cancelled: Error | undefined;
    simulation.network.server("server").http({
        hostname: "stream.test",
        port: 443,
        fetch: () =>
            new Response(
                new ReadableStream({
                    start: (controller) => {
                        controller.enqueue(new TextEncoder().encode("first"));
                    },
                    cancel: (reason) => {
                        cancelled = reason instanceof Error ? reason : undefined;
                    },
                }),
            ),
    });

    // read the first chunk, then abort the request and read on
    const { network } = simulation.environment("client");
    const controller = new AbortController();
    const response = await network.fetch(
        new Request("https://stream.test/", { signal: controller.signal }),
    );
    const reader = present(response.body, "the streamed body").getReader();
    const first = new TextDecoder().decode((await reader.read()).value);
    controller.abort(new DOMException("stopped", "AbortError"));
    const rest = await reader.read().then(
        () => "read",
        (error: Error) => [error.name, error.message],
    );

    expect({ first, rest, cancelled: cancelled?.name }).toEqual({
        first: "first",
        rest: ["AbortError", "stopped"],
        cancelled: "AbortError",
    });
});

test("send an upload whole and a response body's chunks across the link, cutting the rest once the link is cut", async () => {
    // echo the uploaded body, then keep the response open for a chunk after it
    await using simulation = new Simulation({ seed: 1, start: START });
    const streams: ReadableStreamDefaultController<Uint8Array>[] = [];
    simulation.network.server("server").http({
        hostname: "stream.test",
        port: 443,
        fetch: async (request) => {
            const uploaded = await request.text();

            return new Response(
                new ReadableStream<Uint8Array>({
                    start: (controller) => {
                        controller.enqueue(new TextEncoder().encode(uploaded));
                        streams.push(controller);
                    },
                }),
            );
        },
    });

    // read the echo, then cut the link and send a second chunk
    const { network } = simulation.environment("client");
    const response = await network.fetch(
        new Request("https://stream.test/", { method: "POST", body: "uploaded" }),
    );
    const reader = present(response.body, "the streamed body").getReader();
    const echo = new TextDecoder().decode((await reader.read()).value);
    simulation.network.partition("client", "server");
    present(streams[0], "the response stream").enqueue(new TextEncoder().encode("second"));
    const rest = await reader.read().then(
        () => "read",
        (error: Error) => [error.name, error.message],
    );

    expect({ echo, rest }).toEqual({
        echo: "uploaded",
        rest: ["TypeError", "the body from server to client was cut off"],
    });
});

test("publish a node's listener at a world address for nodes and a host address for host processes", async () => {
    // publish a node's listener, answering each request with its path
    await using simulation = new Simulation({ seed: 1, start: START });
    const published = simulation.network.publish("server", {
        hostname: "127.0.0.1",
        port: 0,
        fetch: (request) => new Response(new URL(request.url).pathname),
    });

    // call it from the host at its host address, and from another node at its world address
    const fromHost = await simulation.scheduler.track(
        BunNetwork.fetch(new Request(new URL("/host", published.hostUrl))).then((response) =>
            response.text(),
        ),
        "peer",
    );
    const { network } = simulation.environment("client");
    const fromNode = await network
        .fetch(new Request(new URL("/node", published.url)))
        .then((response) => response.text());
    await published.stop(true);

    expect({ fromHost, fromNode, world: published.url.port }).toEqual({
        fromHost: "/host",
        fromNode: "/node",
        world: "49152",
    });
});

test("refuse leaf host work stalled past the stall timeout, and peer work past the peer one, naming each", async () => {
    // read a stall's message, its milliseconds and call sites left out
    const stallOf = async (by: "host" | "peer") => {
        await using simulation = new Simulation({
            seed: 1,
            start: START,
            stallTimeout: 50,
            peerStallTimeout: 80,
        });
        void simulation.scheduler.track(new Promise<void>(() => {}), by);
        const message = await simulation.clock.sleep(1000).then(
            () => "settled",
            (error: Error) => error.message,
        );
        const [heading = "", ...items] = message.split("\n");

        return [
            heading.replace(/\d+ ms/u, "N ms"),
            items.map((item) => item.split(" ").slice(0, 2)),
        ];
    };

    expect([await stallOf("host"), await stallOf("peer")]).toEqual([
        [
            "work stayed in flight for N ms without settling, 1 of 1 host and peer work running and 0 settled:",
            [["host", "running"]],
        ],
        [
            "work stayed in flight for N ms without settling, 0 of 1 host and peer work running and 0 settled:",
            [["peer", "running"]],
        ],
    ]);
});

test("answer settled host work in the order it began, recording each release with its kind, and list what runs", async () => {
    // track two host works settling in reverse, and a peer that stays running
    await using simulation = new Simulation({ seed: 1, start: START, isTracing: true });
    const { scheduler } = simulation;
    const order: string[] = [];
    const settle = (name: string, after: number) =>
        scheduler.track(
            new Promise<void>((resolve) => {
                setTimeout(resolve, after);
            }).then(() => order.push(name)),
            "host",
        );
    const peer = Promise.withResolvers<void>();
    void scheduler.track(peer.promise, "peer");
    const [first, second] = [settle("first", 20), settle("second", 1)];
    const answered: string[] = [];
    void first.then(() => answered.push("first"));
    void second.then(() => answered.push("second"));
    while (answered.length < 2) {
        await new Promise<void>((resolve) => {
            setTimeout(resolve, 5);
        });
    }
    const pending = scheduler.pending().map((line) => line.split(" ").slice(0, 2));
    peer.resolve();
    await simulation.idle();

    // settle on the host in reverse, answer in start order, and record each release by kind
    const releases = simulation.telemetry.logs
        .filter((record) => record.eventName === "destack.simulation.release")
        .map((record) => [
            record.attributes["destack.simulation.order"],
            record.attributes["destack.simulation.by"],
        ]);
    expect({ order, answered, pending, releases }).toEqual({
        order: ["second", "first"],
        answered: ["first", "second"],
        pending: [["peer", "running"]],
        releases: [
            [1, "host"],
            [2, "host"],
            [0, "peer"],
        ],
    });
});

test("leave a service child untracked, so time moves while it runs", async () => {
    // spawn a sleeping child as a service, then sleep a simulated minute
    await using simulation = new Simulation({ seed: 1, start: START });
    const { process, clock } = simulation.environment("node");
    const child = process.spawn(["/bin/sleep", "5"], { tracking: "none" });
    const during = simulation.scheduler.inFlight;
    await clock.sleep(60_000);
    const moved = clock.now() - START;
    child.kill("SIGKILL");
    await child.exited;

    expect({ during, moved }).toEqual({ during: 0, moved: 60_000 });
});

test("fork a world with its skew and its memory files", async () => {
    // fork a skewed memory world, and write a file in the fork
    await using simulation = new Simulation({
        seed: 1,
        start: START,
        files: "memory",
        skew: { offset: 100, drift: 0 },
    });
    await using forked = simulation.fork("branch");
    const { files } = forked.environment("node");
    await files.writeTextFile("note.txt", "in the fork");

    // read it in the fork, find it nowhere on the host, and bump the fork's skewed node
    const read = await files.readTextFile("note.txt");
    const isOnHost = await new BunFileSystem().exists(join(forked.root, "note.txt"));
    forked.bump("node", 10);

    expect({ read, isOnHost }).toEqual({ read: "in the fork", isOnHost: false });
});

test("take every shared world step, recording each as the world's event", async () => {
    // take each step on a skewed world with two nodes
    await using simulation = new Simulation({
        seed: 1,
        start: START,
        isTracing: true,
        skew: { offset: 0, drift: 0 },
    });
    simulation.environment("left");
    simulation.environment("right");
    const steps: WorldStep[] = [
        { action: "partition", nodes: ["left", "right"] },
        { action: "heal", nodes: ["left", "right"] },
        { action: "isolate", node: "left" },
        { action: "rejoin", node: "left" },
        { action: "degrade", drop: 0.5, delay: { least: 1, most: 2 } },
        { action: "bump", node: "right", milliseconds: -5 },
        { action: "crash", node: "right" },
        { action: "run", milliseconds: 1000 },
        { action: "jump", milliseconds: 60_000 },
    ];
    for (const step of steps) {
        await simulation.take(step);
    }

    // record one event per step at the time it was taken
    const events = simulation.telemetry.logs.map((record) => [
        record.eventName?.replace("destack.simulation.", ""),
        record.hrTime[0] * 1000 + record.hrTime[1] / 1_000_000 - START,
    ]);
    expect(events).toEqual([
        ["partition", 0],
        ["heal", 0],
        ["partition", 0],
        ["heal", 0],
        ["degrade", 0],
        ["bump", 0],
        ["crash", 0],
        ["timer", 1000],
        ["jump", 61_000],
    ]);
});

test("read a drifted node's time in whole milliseconds", async () => {
    // drift a node's clock, then read its time after an odd stretch of world time
    await using simulation = new Simulation({
        seed: 3,
        start: START,
        skew: { offset: 0, drift: 100 },
    });
    const { clock } = simulation.environment("node");
    await simulation.take({ action: "jump", milliseconds: 1_234_567 });

    expect(Number.isInteger(clock.now())).toBe(true);
});

test("release a crashed node's children, workers and file locks", async () => {
    // start a child, a worker and a lock on a node, then crash it
    await using simulation = new Simulation({ seed: 1, start: START, files: "memory" });
    simulation.workers.define(new URL("https://app.test/idle.worker.js"), () => {});
    const node = simulation.environment("node");
    const child = node.process.spawn(["/bin/sleep", "5"], { tracking: "none" });
    const worker = node.workers.start(new URL("https://app.test/idle.worker.js"));
    const answers: unknown[] = [];
    worker.addEventListener("message", (event) => answers.push(event.data));
    await node.files.lock("/lock", "exclusive");
    simulation.node("node").crash();

    // end the child, take the lock from another node, and deliver nothing from the worker
    const code = await child.exited;
    const other = await simulation.environment("other").files.tryLock("/lock", "exclusive");
    worker.postMessage("after the crash");
    await simulation.idle();

    expect({ code, isLocked: other !== undefined, answers }).toEqual({
        code: 137,
        isLocked: true,
        answers: [],
    });
});

test("run timers by deadline and, within one, in start order, a zero-delay timer first and an overflowing one a millisecond later, across many waits", async () => {
    // start a zero-delay timer and an overflowing one after two later ones, and many sleeps of seeded delays
    await using simulation = new Simulation({ seed: 3, start: START });
    const { clock, random } = simulation;
    const order: string[] = [];
    clock.setTimeout(() => order.push("later first"), 10);
    clock.setTimeout(() => order.push("zero"), 0);
    clock.setTimeout(() => order.push("later second"), 10);
    clock.setTimeout(() => order.push("overflow"), LONGEST_DELAY + 1);
    const released: [number, number][] = [];
    const delays = Array.from({ length: 2000 }, () => Random.int(random, 50));
    for (const [index, delay] of delays.entries()) {
        void clock.sleep(delay + 1).then(() => released.push([delay + 1, index]));
    }
    await simulation.clock.sleep(START + 100 - simulation.clock.now());
    await simulation.idle();

    // release every wait by deadline, ties in start order
    const expected = delays
        .map((delay, index): [number, number] => [delay + 1, index])
        .toSorted(
            ([left, leftIndex], [right, rightIndex]) => left - right || leftIndex - rightIndex,
        );
    expect({ order, isOrdered: JSON.stringify(released) === JSON.stringify(expected) }).toEqual({
        order: ["zero", "overflow", "later first", "later second"],
        isOrdered: true,
    });
});

test("decode a gzip response as Bun's fetch does, and keep its encoding when asked", async () => {
    // serve one gzip-encoded body
    await using simulation = new Simulation({ seed: 1, start: START });
    const encoded = await new Response(
        new Blob(["stored"]).stream().pipeThrough(new CompressionStream("gzip")),
    ).bytes();
    simulation.network.server("server").http({
        hostname: "files.test",
        port: 443,
        fetch: () => new Response(encoded, { headers: { "content-encoding": "gzip" } }),
    });
    const { network } = simulation.environment("client");

    // fetch it decoded, then as stored
    const decoded = await network.fetch(new Request("https://files.test/"));
    const stored = await network.fetch(new Request("https://files.test/"), { decompress: false });

    expect({
        decoded: [await decoded.text(), decoded.headers.get("content-encoding")],
        stored: [(await stored.bytes()).length, stored.headers.get("content-encoding")],
    }).toEqual({ decoded: ["stored", null], stored: [encoded.length, "gzip"] });
});

test("answer idle once the work due now settled, and wait for a condition through the next timer", async () => {
    // chain zero-delay timers around host work at the start, beside a timer a second out
    await using simulation = new Simulation({ seed: 1, start: START });
    const { clock } = simulation;
    let done: number | undefined;
    void (async () => {
        await clock.sleep(0);
        await simulation.scheduler.track(
            new Promise<void>((resolve) => {
                setTimeout(resolve, 5);
            }),
            "host",
        );
        await clock.sleep(0);
        done = clock.now() - START;
    })();
    let isFired = false;
    void clock.sleep(1000).then(() => (isFired = true));

    // answer at the start once the chain settled, then at the timer
    await simulation.idle();
    const first = { done, now: clock.now() - START };
    await simulation.until(() => isFired);

    expect([first, clock.now() - START]).toEqual([{ done: 0, now: 0 }, 1000]);
});

/** Race workers on hashing, files and timers, and return the order their rounds finished in. */
async function race(simulation: Simulation): Promise<string[]> {
    // give each worker its own file under the world's root
    const { crypto, files, clock } = simulation.environment("node");
    const finished: string[] = [];
    const worker = async (index: number) => {
        const path = join(simulation.root, `worker-${index}`);
        for (let round = 0; round < ROUNDS; round += 1) {
            // hash a buffer whose size differs by worker, so the host finishes them out of order
            const data = new Uint8Array((index + 1) * 64 * 1024).fill(round);
            const digest = new Uint8Array(await crypto.digest("SHA-256", data));

            // write and read the digest back, then wait a timer
            await files.writeFile(path, digest);
            await files.readFile(path);
            await clock.sleep(1 + ((index * round) % 3));
            finished.push(`${index}.${round}`);
        }
    };

    // run every worker to the end on the world's loop
    await Promise.all(Array.from({ length: WORKERS }, (_, index) => worker(index)));

    return finished;
}

test("replay a seed's racing host work in one order with one fingerprint", async () => {
    // run the same seed and work twice
    await using first = new Simulation({ seed: 7, start: START });
    await using second = new Simulation({ seed: 7, start: START });
    const runs = [await race(first), await race(second)];

    // expect the same interleaving, and the same events behind it
    expect(runs[1]).toEqual(runs[0]);
    expect(second.fingerprint).toBe(first.fingerprint);
});

test("move a world's time to each next timer once idle, as plain awaits wait on them", async () => {
    // await the world's timers plainly
    await using simulation = new Simulation({ seed: 1, start: START });
    const { clock } = simulation.environment("node");
    await clock.sleep(30_000);
    await Promise.all([clock.sleep(1000), clock.sleep(2000)]);

    // expect the clock at the last timer's deadline
    expect(clock.now() - START).toBe(32_000);
});

test("jump a world at once, waking each due timer a single time", async () => {
    // keep a timer every minute, then jump an hour
    await using simulation = new Simulation({ seed: 1, start: START });
    const { clock } = simulation.environment("node");
    const fired: number[] = [];
    void (async () => {
        for (;;) {
            await clock.sleep(60_000);
            fired.push(clock.now() - START);
        }
    })();
    await simulation.jump(START + 3_600_000);

    // expect one firing at the jump's time, and none in between
    expect(fired).toEqual([3_600_000]);
});
