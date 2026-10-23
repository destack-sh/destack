import { expect, test } from "@destack/test";
import { Random } from "../src/index.ts";
import { SeededRandom, Simulation, VirtualClock } from "../src/simulation/index.ts";
import type { ServerSocket } from "../src/index.ts";

/** The simulations' start: 2026-10-16 at midnight UTC. */
const START = Date.UTC(2026, 9, 16);

test("release a virtual clock's waits in deadline order as it moves in whole milliseconds, and refuse moving it back", async () => {
    // pass two of three waits
    const clock = new VirtualClock(START);
    const released: number[] = [];
    for (const delay of [300, 100, 200]) {
        void clock.sleep(delay).then(() => released.push(delay));
    }
    clock.advance(250);
    await Promise.resolve();

    // release passed waits in order
    expect([released, clock.next]).toEqual([[100, 200], START + 300]);
    expect(() => clock.set(START)).toThrow(
        new RangeError(`a virtual clock at ${START + 250} cannot go back to ${START}`),
    );

    // round a fractional wait up to the next whole millisecond, and refuse a fractional time
    void clock.sleep(0.5).then(() => released.push(0.5));
    expect(clock.next).toBe(START + 251);
    expect(() => clock.set(START + 251.5)).toThrow(
        new RangeError(`a virtual clock moves in whole milliseconds, not to ${START + 251.5}`),
    );
});

test("abort a virtual clock's timeout signal with a timeout once the clock passes it, outside the counted waits", async () => {
    // start a timeout of a second and move the clock to just before it
    const clock = new VirtualClock(START);
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
    const random = new SeededRandom(seed);

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
        // request through latency
        const simulation = new Simulation({ seed, start: START });
        simulation.network.of("server").serve({
            hostname: "server.test",
            port: 443,
            fetch: (request) => Response.json({ path: new URL(request.url).pathname }),
        });
        simulation.network.degrade({ delay: { least: 10, most: 20 } });
        const client = simulation.environment("client").network;
        const answered = client.fetch(new Request("https://server.test/notes"));
        await simulation.run({
            until: () => simulation.trace.some((event) => event.kind === "request"),
        });
        const body: unknown = await (await answered).json();

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

test("skip virtual time to each next timer until a condition holds, and refuse a condition idle work never meets", async () => {
    // sleep a day, then wait on nothing
    const simulation = new Simulation({ seed: 1, start: START });
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

test("crash a node, losing its memory and stopping its work, and start it again from what it stored", async () => {
    // tick each second, counting starts
    const simulation = new Simulation({ seed: 1, start: START });
    const stored = { starts: 0 };
    let ticks = 0;
    const node = await simulation.node("worker", async (environment, signal) => {
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
    await simulation.run({ for: { seconds: 3 } });
    node.crash();
    const whileDown = ticks;
    await simulation.run({ for: { seconds: 3 } });
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

test("run the realm's time, timers, randomness and requests on an installed simulation, and put them back", async () => {
    // read every effect through the globals
    const simulation = new Simulation({ seed: 5, start: START }).install();
    const effects = await (async () => {
        try {
            simulation.network.of("server").serve({
                hostname: "server.test",
                port: 443,
                fetch: () => Response.json({ at: Date.now() }),
            });
            let isFired = false;
            setTimeout(() => (isFired = true), 60_000);
            await simulation.run({ until: () => isFired });
            const node = await simulation.node("browser", async (): Promise<unknown> =>
                (await fetch("https://server.test/")).json(),
            );

            return {
                now: Date.now() - START,
                date: new Date().getTime() - START,
                answered: node.running,
                uuid: crypto.randomUUID(),
                request: simulation.trace.findLast((event) => event.kind === "request")?.detail,
            };
        } finally {
            simulation.uninstall();
        }
    })();

    // replace each effect and restore the system's
    const again = new Simulation({ seed: 5, start: START }).install();
    const uuid = crypto.randomUUID();
    again.uninstall();
    expect({ ...effects, isSystem: Date.now() > START + 60_000 }).toEqual({
        now: 60_000,
        date: 60_000,
        answered: { at: START + 60_000 },
        uuid: effects.uuid,
        request: "browser GET https://server.test/",
        isSystem: true,
    });
    expect([uuid === effects.uuid, uuid.slice(14, 15)]).toEqual([true, "4"]);
});

/** Serve an echo of every message and the attachment its sockets were accepted with. */
function serveEcho(simulation: Simulation, node: string) {
    const events: string[] = [];
    const listener = simulation.network.of(node).serve<{ readonly name: string }>({
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
function dial(simulation: Simulation, node: string, name: string) {
    const events: string[] = [];
    const socket = simulation.network.of(node).connect(`wss://echo.test/?name=${name}`, ["echo"]);
    socket.addEventListener("open", () => events.push(`open ${socket.protocol}`));
    socket.addEventListener("message", (event: MessageEvent) => events.push(String(event.data)));
    socket.addEventListener("close", (event: CloseEvent) => events.push(`close ${event.code}`));

    return { events, socket };
}

test("carry a socket's messages in order through latency, and close it from either end", async () => {
    // open a socket through 5 to 50 ms of latency and send three messages
    const simulation = new Simulation({ seed: 9, start: START });
    simulation.network.degrade({ delay: { least: 5, most: 50 } });
    const server = serveEcho(simulation, "server");
    const client = dial(simulation, "browser", "ada");
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
    const simulation = new Simulation({ seed: 9, start: START });
    const cancelled: string[] = [];
    simulation.network.of("server").serve({
        hostname: "echo.test",
        port: 443,
        fetch: () =>
            new Response(new ReadableStream({ cancel: () => void cancelled.push("cancelled") }), {
                status: 503,
            }),
    });
    const refused = dial(simulation, "laptop", "ada");
    await simulation.run({ until: () => refused.events.length === 1 });
    expect([refused.events, cancelled]).toEqual([["close 1006"], ["cancelled"]]);
});

test("break sockets across a partition and from a crashed node, and close them on a stopped listener", async () => {
    // serve the echo from a node, and open one socket to be cut off and one to see the crash
    const simulation = new Simulation({ seed: 9, start: START });
    const server = await simulation.node(
        "server",
        async () => serveEcho(simulation, "server").listener,
    );
    const cut = dial(simulation, "browser", "ada");
    const crashed = dial(simulation, "laptop", "grace");
    await simulation.run({ until: () => cut.events.length === 1 && crashed.events.length === 1 });

    // cut the browser off, then crash the server and dial it while it is down
    simulation.network.partition("browser", "server");
    server.crash();
    const down = dial(simulation, "laptop", "lin");
    await simulation.run({ until: () => down.events.length === 1 });

    // restart it, and close an open socket by stopping its listener
    const listener = await server.start();
    const stopped = dial(simulation, "laptop", "ming");
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

test("connect the realm's WebSocket as the current node on an installed simulation", async () => {
    // open a socket from inside a node through the global WebSocket
    const simulation = new Simulation({ seed: 9, start: START }).install();
    try {
        serveEcho(simulation, "server");
        const node = await simulation.node("browser", async () => {
            const socket = new WebSocket("wss://echo.test/?name=tab", "echo");
            const answered = Promise.withResolvers<string>();
            socket.addEventListener("open", () => socket.send("hello"));
            socket.addEventListener("message", (event: MessageEvent) =>
                answered.resolve(String(event.data)),
            );

            return answered.promise;
        });

        // the socket opened as the browser node and answered through the echo
        expect([node.running, simulation.trace.at(-1)?.detail]).toEqual([
            "tab: hello",
            "browser GET https://echo.test/?name=tab",
        ]);
    } finally {
        simulation.uninstall();
    }
});
