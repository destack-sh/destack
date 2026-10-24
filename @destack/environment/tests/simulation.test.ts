import { Uuid } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Random } from "../src/index.ts";
import { SimulatedRandom, Simulation, SimulatedClock } from "../src/simulation/index.ts";
import type { ServerSocket, TcpSocket } from "../src/index.ts";

/** The simulations' start: 2026-10-16 at midnight UTC. */
const START = Date.UTC(2026, 9, 16);
/** The encoder of the TCP tests' lines. */
const ENCODER = new TextEncoder();
/** The decoder of the TCP tests' lines. */
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
        const body: unknown = await simulation.drive(
            client
                .fetch(new Request("https://server.test/notes"))
                .then((response) => response.json()),
        );

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

        return { body, partitioned, dropped, trace: simulation.trace };
    };

    // replay the trace by seed
    const [first, again] = [await run(3), await run(3)];
    expect({
        body: first.body,
        partitioned: first.partitioned,
        dropped: first.dropped,
        replayed: first.trace,
    }).toEqual({
        body: { path: "/notes" },
        partitioned: "fetch failed: client cannot reach server",
        dropped: "fetch failed: the request to https://server.test was lost",
        replayed: again.trace,
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
        simulation.drive(
            client.fetch(new Request(`https://${host}/`)).then(
                (response) => response.text(),
                (error: Error) => error.message,
            ),
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
        simulation.drive(
            client.fetch(new Request(url)).then(
                (response) => response.text(),
                (error: Error) => error.message,
            ),
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
    await simulation.run({ until: () => isAwake });
    const woke = simulation.clock.now();
    const refused = await simulation
        .run({ until: () => false })
        .catch((error: Error) => error.message);

    // skip the day and refuse the idle run
    expect([woke - START, refused]).toEqual([
        24 * 60 * 60_000,
        "the simulation went idle before its condition held",
    ]);
});

test("hold time back while tracked work is in flight, move to the next timer once it settles, and run to a time", async () => {
    // start a timer a minute out, and track work that settles on the system's time
    await using simulation = new Simulation({ seed: 1, start: START });
    let isFired = false;
    void simulation.clock.sleep(60_000).then(() => (isFired = true));
    const work = Promise.withResolvers<void>();
    void simulation.clock.track(work.promise);
    let held: [number, boolean] | undefined;
    setTimeout(() => {
        held = [simulation.clock.now() - START, isFired];
        work.resolve();
    }, 20);

    // run until the timer fires
    await simulation.run({ until: () => isFired });
    const fired = simulation.clock.now() - START;

    // run to a time past one more timer
    let isLater = false;
    void simulation.clock.sleep(15_000).then(() => (isLater = true));
    await simulation.run({ to: START + 90_000 });

    // stay at the start while the work runs, reach the timer only after it settled, and stop at the time
    expect({
        held,
        fired,
        reached: simulation.clock.now() - START,
        inFlight: simulation.clock.inFlight,
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
    const polled = await simulation.clock.track(Promise.resolve(longPoll()));
    const answered = simulation.clock.track(answerSlowly()).then((response) => {
        isAnswered = true;

        return response;
    });
    await simulation.run({ until: () => isAnswered });
    const answeredAt = simulation.clock.now() - START;
    const ended = polled.text().then(() => simulation.clock.now() - START);
    await simulation.run({ to: START + 60_000 });

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
    const during = simulation.clock.inFlight;
    const bytes = (await digest).byteLength;
    const after = simulation.clock.inFlight;

    expect({ during, bytes, after }).toEqual({ during: 1, bytes: 32, after: 0 });
});

test("crash a node, losing its memory and stopping its work, and start it again from what it stored", async () => {
    // tick each second, counting starts
    await using simulation = new Simulation({ seed: 1, start: START });
    const stored = { starts: 0 };
    let ticks = 0;
    const node = await simulation.add("worker", async (environment, signal) => {
        stored.starts += 1;
        void (async () => {
            while (!signal.aborted) {
                await environment.clock.sleep(1000, signal).catch(() => undefined);
                ticks += signal.aborted ? 0 : 1;
            }
        })();

        return { memory: "fresh" };
    });

    // crash, wait and restart
    await simulation.run({ to: simulation.clock.now() + 3000 });
    node.crash();
    const whileDown = ticks;
    await simulation.run({ to: simulation.clock.now() + 3000 });
    const crashed = ticks;
    await node.start();

    // stop ticks on crash and start again
    expect({ whileDown, crashed, starts: stored.starts, memory: node.running }).toEqual({
        whileDown: 3,
        crashed: 3,
        starts: 2,
        memory: { memory: "fresh" },
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
    await simulation.run({ until: () => client.socket.readyState === WebSocket.OPEN });
    for (const message of ["one", "two", "three"]) {
        client.socket.send(message);
    }
    await simulation.run({ until: () => client.events.length === 4 });
    client.socket.close();
    await simulation.run({ until: () => client.events.length === 5 });

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
    await simulation.run({ until: () => refused.events.length === 1 });
    expect([refused.events, cancelled]).toEqual([["close 1006"], ["cancelled"]]);
});

test("break sockets across a partition and from a crashed node, and close them on a stopped listener", async () => {
    // serve the echo from a node, and open one socket to be cut off and one to see the crash
    await using simulation = new Simulation({ seed: 9, start: START });
    const server = await simulation.add(
        "server",
        async () => serveEcho(simulation, "server").listener,
    );
    const cut = open(simulation, "browser", "ada");
    const crashed = open(simulation, "laptop", "grace");
    await simulation.run({ until: () => cut.events.length === 1 && crashed.events.length === 1 });

    // cut the browser off, then crash the server and connect it while it is down
    simulation.network.partition("browser", "server");
    server.crash();
    const down = open(simulation, "laptop", "lin");
    await simulation.run({ until: () => down.events.length === 1 });

    // restart it, and close an open socket by stopping its listener
    const listener = await server.start();
    const stopped = open(simulation, "laptop", "ming");
    await simulation.run({ until: () => stopped.events.length === 1 });
    await listener.stop(true);
    await simulation.run({ until: () => stopped.events.length === 2 });

    // each socket ended the way its fault ends a connection
    expect([cut.events, crashed.events, down.events, stopped.events]).toEqual([
        ["open echo", "close 1006"],
        ["open echo", "close 1006"],
        ["close 1006"],
        ["open echo", "close 1001"],
    ]);
});

/** Listen for TCP connections on a node, echoing each line back in upper case. */
async function listenUpper(simulation: Simulation, node: string) {
    const accepted: TcpSocket[] = [];
    const listener = await simulation.network.server(node).tcp({
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
    await simulation.run({ until: () => isOpen });
    const writer = client.socket.writable.getWriter();
    await writer.write(ENCODER.encode("ehlo\r\n"));
    await writer.write(ENCODER.encode("starttls\r\n"));
    await simulation.run({ until: () => client.received.length === 2 });

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
    await simulation.run({ until: () => answered.length === 1 });

    // close the new socket, which ends the server's end too
    let isClosed = false;
    void secure.close().then(() => (isClosed = true));
    await simulation.run({ until: () => isClosed });

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
    await simulation.network.server("mail").tcp({
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
    await simulation.drive(socket.opened);
    await socket.writable.getWriter().write(ENCODER.encode("quit\r\n"));
    const closes = [socket.close(), socket.close()];
    await simulation.drive(Promise.all(closes));
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
    await simulation.run({ until: () => server.accepted.length === 3 });

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

test("drive work through its timers on the clock, and stall loudly on work waiting outside the simulation", async () => {
    // drive work waiting on the clock, then work waiting on the system's time alone
    await using simulation = new Simulation({ seed: 1, start: START });
    const timed = (async () => {
        await simulation.clock.sleep(60_000);

        return "done";
    })();
    const driven = await simulation.drive(timed);
    const outside = new Promise<void>((resolve) => {
        setTimeout(resolve, 5);
    });
    const stalled = await simulation.drive(outside).then(
        () => "settled",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    await outside;

    // settle the clock's work at its sleep, and refuse the untracked wait instead of jumping time
    expect([driven, stalled, simulation.clock.now() - START]).toEqual([
        "done",
        "the simulation went idle before its condition held",
        60_000,
    ]);
});

test("wait inside a run in progress for a run or drive started meanwhile, without moving time twice", async () => {
    // run to a timer whose callback drives work and runs until a later timer
    await using simulation = new Simulation({ seed: 1, start: START });
    const seen: number[] = [];
    const inner = (async () => {
        await simulation.clock.sleep(1000);
        await simulation.drive(simulation.clock.sleep(1000));
        seen.push(simulation.clock.now() - START);
        await simulation.run({ to: START + 5000 });
        seen.push(simulation.clock.now() - START);
    })();
    await simulation.run({ to: START + 10_000 });
    await inner;

    // reach each inner time once, as the outer run alone moves the clock
    expect([seen, simulation.clock.now() - START]).toEqual([[2000, 5000], 10_000]);
});

test("answer a drive started inside a run's condition once its work settles, between the run's timers", async () => {
    // tick a counter each second, and run until a condition that drives settled work reads it
    await using simulation = new Simulation({ seed: 1, start: START });
    let ticks = 0;
    simulation.clock.setInterval(() => {
        ticks += 1;
    }, 1000);
    const checks: number[] = [];
    await simulation.run({
        until: async () => {
            const read = await simulation.drive(Promise.resolve(ticks));
            checks.push(read);

            return read === 3;
        },
    });

    // check once after each timer the run moved to
    expect(checks).toEqual([0, 1, 2, 3]);
});

test("move time for driven work past the end of a drive started inside it, until every driven work settles", async () => {
    // drive work whose first step drives a short wait before the outer drive begins, then waits longer
    await using simulation = new Simulation({ seed: 1, start: START });
    const outer = (async () => {
        await simulation.drive(simulation.clock.sleep(1000));
        await simulation.clock.sleep(4000);

        return simulation.clock.now() - START;
    })();

    // reach the outer wait, which the inner drive's run alone would leave behind
    expect(await simulation.drive(outer)).toBe(5000);
});

test("settle the work a timer at a run's deadline starts before answering a run inside another", async () => {
    // keep a drive moving time, and finish tracked work over several turns once a timer at the deadline fires
    await using simulation = new Simulation({ seed: 1, start: START });
    const outer = simulation.drive(simulation.clock.sleep(10_000));
    let isFinished = false;
    void simulation.clock.sleep(2000).then(async () => {
        for (let step = 0; step < 3; step += 1) {
            await simulation.clock.track(
                new Promise<void>((resolve) => {
                    setImmediate(resolve);
                }),
            );
        }
        isFinished = true;
    });

    // run to the deadline inside the moving drive, which answers once the timer's work settled
    await simulation.run({ to: START + 2000 });
    const isSettled = isFinished;
    await outer;

    expect(isSettled).toBe(true);
});

test("check a run's condition inside another only once the work settles, never between its steps", async () => {
    // keep a drive moving time, and write two steps of tracked work once a timer fires
    await using simulation = new Simulation({ seed: 1, start: START });
    const outer = simulation.drive(simulation.clock.sleep(10_000));
    let steps = 0;
    void simulation.clock.sleep(1000).then(async () => {
        for (let step = 0; step < 2; step += 1) {
            await simulation.clock.track(
                new Promise<void>((resolve) => {
                    setImmediate(resolve);
                }),
            );
            steps += 1;
        }
    });

    // run until the first step inside the moving drive, which answers once both settled
    await simulation.run({ until: () => steps > 0 });
    const seen = steps;
    await outer;

    expect(seen).toBe(2);
});

test("refuse work that keeps busy without letting time move, naming the call site that kept it busy", async () => {
    // keep tracking work that re-enqueues itself each event loop turn
    await using simulation = new Simulation({ seed: 1, start: START, livelockTurns: 1000 });
    const stop = new AbortController();
    const spinning = (async () => {
        while (!stop.signal.aborted) {
            await simulation.clock.track(
                new Promise<void>((resolve) => {
                    setImmediate(resolve);
                }),
            );
        }
    })();
    const refused = await simulation.drive(simulation.clock.sleep(1000)).then(
        () => "settled",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    stop.abort();
    await spinning;

    // refuse at the busy limit, the report leading with the spinning work's count and kind
    const [heading, site = ""] = refused.split("\n");
    expect([heading, site.split(" ").slice(0, 2), simulation.clock.now() - START]).toEqual([
        "the simulation kept busy for 1000 turns without moving time:",
        ["100x", "track"],
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
        const reply = simulation.clock.trackHost(
            fetch(server.url).then((response) => response.text()),
        );
        expect([await simulation.drive(reply), simulation.clock.now() - START]).toEqual([
            "late",
            0,
        ]);
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

    expect([await simulation.drive(first), clock.now() - START]).toEqual([[0, "done\n"], 0]);
});

test("refuse tracked work that never settles beside a timer, as time cannot move while it is in flight", async () => {
    // track work that never settles while a timer waits
    await using simulation = new Simulation({ seed: 1, start: START });
    void simulation.clock.track(
        new Promise<void>(() => {
            // never settle
        }),
    );
    const refused = await simulation.drive(simulation.clock.sleep(1000)).then(
        () => "settled",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );

    // refuse the deadlock naming the one work in flight, the time unmoved
    const [header, ...sites] = refused.split("\n");
    expect([header, sites.length, simulation.clock.now() - START]).toEqual([
        "tracked work waits on simulated time, which cannot move while it is in flight:",
        1,
        0,
    ]);
});

test("fork a world by a label into a world of its own draws, starting at the parent's time", async () => {
    // draw ids in a parent, then fork it twice by one label and once by another
    await using parent = new Simulation({ seed: 1, start: START });
    const drawn = drawIds(parent);
    parent.clock.advance(1000);
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
        run: await beats((simulation) => simulation.run({ to: START + 3_600_000 })),
        jump: await beats((simulation) => simulation.jump(START + 86_400_000)),
    }).toEqual({
        run: { count: 360, now: START + 3_600_000 },
        jump: { count: 1, now: START + 86_400_000 },
    });
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
        simulation.drive(
            network.fetch(new Request(url, signal === undefined ? {} : { signal })).then(
                (response) => response.text(),
                (error: Error) => [error.name, error.message],
            ),
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

test("refuse host work stalled past the stall timeout, and hand work driven during a timed run a run of its own", async () => {
    // track host work that never settles under a short host timeout
    await using stalled = new Simulation({ seed: 1, start: START, stallTimeout: 50 });
    void stalled.clock.trackHost(new Promise<void>(() => {}));
    const refused = await stalled.drive(stalled.clock.sleep(1000)).then(
        () => "settled",
        (error: Error) => error.message.split("\n").length,
    );

    // drive a wait past the end of a timed run started before it
    await using simulation = new Simulation({ seed: 1, start: START });
    const timed = simulation.run({ to: START + 1000 });
    const driven = simulation.drive(simulation.clock.sleep(3000));
    await timed;
    await driven;

    expect({ refused, driven: simulation.clock.now() - START }).toEqual({
        refused: 2,
        driven: 3000,
    });
});

test("run timers by deadline and, within one, in start order, a zero-delay timer first, across many waits", async () => {
    // start a zero-delay timer after two later ones, and many sleeps of seeded delays
    await using simulation = new Simulation({ seed: 3, start: START });
    const { clock, random } = simulation;
    const order: string[] = [];
    clock.setTimeout(() => order.push("later first"), 10);
    clock.setTimeout(() => order.push("zero"), 0);
    clock.setTimeout(() => order.push("later second"), 10);
    const released: [number, number][] = [];
    const delays = Array.from({ length: 2000 }, () => Random.int(random, 50));
    for (const [index, delay] of delays.entries()) {
        void clock.sleep(delay + 1).then(() => released.push([delay + 1, index]));
    }
    await simulation.run({ to: START + 100 });

    // release every wait by deadline, ties in start order
    const expected = delays
        .map((delay, index): [number, number] => [delay + 1, index])
        .toSorted(
            ([left, leftIndex], [right, rightIndex]) => left - right || leftIndex - rightIndex,
        );
    expect({ order, isOrdered: JSON.stringify(released) === JSON.stringify(expected) }).toEqual({
        order: ["zero", "later first", "later second"],
        isOrdered: true,
    });
});
