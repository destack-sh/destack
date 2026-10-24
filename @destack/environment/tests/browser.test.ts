import { expect, test } from "@destack/test";
import { BrowserSimulation } from "../src/browser/simulation.ts";
import { Simulation } from "../src/simulation/index.ts";

/** The simulations' start: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("store values and move through session history on a node, telling listeners each move", async () => {
    // store a value, and navigate, replace and traverse while listening
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation, { url: "https://notes.test/" });
    const { storage, navigation } = browsers.environment("tab");
    storage.setItem("theme", "dark");
    const moves: string[] = [];
    using _subscription = navigation.on((entry) => moves.push(entry.url));
    navigation.navigate("/inbox");
    navigation.navigate("/inbox?page=2", { history: "replace" });
    navigation.navigate("https://notes.test/settings");
    navigation.back();
    navigation.back();
    navigation.back();
    navigation.forward();

    expect({
        stored: [storage.getItem("theme"), storage.getItem("missing")],
        current: navigation.currentEntry.url,
        moves,
    }).toEqual({
        stored: ["dark", null],
        current: "https://notes.test/inbox?page=2",
        moves: [
            "https://notes.test/inbox",
            "https://notes.test/inbox?page=2",
            "https://notes.test/settings",
            "https://notes.test/inbox?page=2",
            "https://notes.test/",
            "https://notes.test/inbox?page=2",
        ],
    });
});

test("show frames at 60 Hz on the simulated clock, idle periods, set media and a written clipboard", async () => {
    // wait for three frames and an idle period from a moment between frames
    await using simulation = new Simulation({ seed: 1, start: START + 5 });
    const browsers = new BrowserSimulation(simulation, {
        media: { "(prefers-color-scheme: dark)": true },
    });
    const { display, clipboard } = browsers.environment("tab");
    const frames: number[] = [];
    const shown = (async () => {
        for (let count = 0; count < 3; count += 1) {
            frames.push(await display.frame());
        }
        await display.idle(50);
    })();
    await simulation.drive(shown);

    // write text to the clipboard
    await clipboard.writeText("copied");

    expect({
        frames: frames.map((time) => time - START),
        after: simulation.clock.now() - START,
        media: [
            display.media("(prefers-color-scheme: dark)").matches,
            display.media("(max-width: 640px)").matches,
        ],
        clipboard: clipboard.written,
    }).toEqual({
        frames: [17, 34, 50],
        after: 50,
        media: [true, false],
        clipboard: { text: "copied" },
    });
});

test("go offline, refuse captures, locate, share, vibrate and follow permissions on a node", async () => {
    // start a node at a position with the camera granted, and one without a position
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation, {
        position: { latitude: 48.2, longitude: 16.37, accuracy: 10 },
        permissions: { camera: "granted" },
    });
    const unlocated = new BrowserSimulation(simulation);
    const { connectivity, capture, geolocation, share, vibration, permissions } =
        browsers.environment("tab");

    // follow connectivity while going offline and back
    const changes: boolean[] = [];
    using _followed = connectivity.on((isOnline) => changes.push(isOnline));
    const online = connectivity.isOnline();
    connectivity.set(false);
    connectivity.set(false);
    connectivity.set(true);

    // refuse captures, and read positions with and without one
    const captured = await capture.getUserMedia({ video: true }).catch((error: unknown) => error);
    const position = await geolocation.getCurrentPosition();
    const missing = await unlocated
        .environment("other")
        .geolocation.getCurrentPosition()
        .catch((error: unknown) => error);

    // share, vibrate, and follow a permission's change
    await share.share({ title: "Note", url: "https://notes.test/1" });
    vibration.vibrate([100, 50, 100]);
    const camera = await permissions.query({ name: "camera" });
    const states = [camera.state, (await permissions.query({ name: "geolocation" })).state];
    const changed: string[] = [];
    camera.addEventListener("change", () => changed.push(camera.state));
    (await permissions.query({ name: "camera" })).set("denied");

    expect({
        online,
        changes,
        devices: await capture.enumerateDevices(),
        captured: captured instanceof DOMException ? [captured.name, captured.message] : captured,
        position: [position.coords.latitude, position.coords.longitude, position.timestamp],
        missing:
            missing instanceof Error && "code" in missing
                ? [missing.code, missing.message]
                : missing,
        shared: share.shared,
        patterns: vibration.patterns,
        states,
        changed,
    }).toEqual({
        online: true,
        changes: [false, true],
        devices: [],
        captured: ["NotFoundError", "the simulation has no media devices"],
        position: [48.2, 16.37, START],
        missing: [2, "the simulation has no position"],
        shared: [{ title: "Note", url: "https://notes.test/1" }],
        patterns: [[100, 50, 100]],
        states: ["granted", "prompt"],
        changed: ["denied"],
    });
});

test("grant one origin's locks across tabs in request order, shared holders together, and run without one when asked", async () => {
    // hold an exclusive lock on one tab while the others ask for it
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation);
    const first = browsers.environment("first").locks;
    const second = browsers.environment("second").locks;
    const third = browsers.environment("third").locks;
    const order: string[] = [];
    const { promise: released, resolve: release } = Promise.withResolvers<void>();
    const lifetime = new AbortController();
    const holding = first.request("leader", { signal: lifetime.signal }, async (lock) => {
        order.push(`first ${lock?.mode}`);
        await released;
    });
    const waiting = [
        second.request("leader", { mode: "shared" }, async () => void order.push("second shared")),
        third.request("leader", { mode: "shared" }, async () => void order.push("third shared")),
    ];
    const skipped = await second.request("leader", { ifAvailable: true }, async (lock) => lock);

    // give up one waiting request, abort the holder's lifetime after its grant, then release
    const controller = new AbortController();
    const aborted = third.request("leader", { signal: controller.signal }, async () => "taken");
    controller.abort(new Error("gave up"));
    lifetime.abort(new Error("tab closed"));
    release();
    await Promise.all([holding, ...waiting]);

    expect({
        order,
        skipped,
        aborted: await aborted.catch((error: Error) => error.message),
    }).toEqual({
        order: ["first exclusive", "second shared", "third shared"],
        skipped: null,
        aborted: "gave up",
    });
});

test("release a crashed tab's locks and ports, failing its waiting requests", async () => {
    // hold one lock on the crashing tab, and wait there for a lock the other tab holds
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation);
    const crashing = await simulation.add("crashing", async () => undefined);
    const crashed = browsers.environment("crashing");
    const other = browsers.environment("other");
    const order: string[] = [];
    const { promise: gateOpen, resolve: openGate } = Promise.withResolvers<void>();
    const gate = other.locks.request("gate", {}, async () => await gateOpen);
    void crashed.locks.request("leader", {}, async () => {
        order.push("crashing");
        await Promise.withResolvers<void>().promise;
    });
    const waiting = crashed.locks.request("gate", {}, async () => "granted");
    const blocked = other.locks.request("leader", {}, async () => void order.push("other"));
    const port = crashed.broadcast.open("destack:space");

    // crash the tab, take the lock it held, and send from its closed port
    crashing.crash();
    await blocked;
    openGate();
    await gate;
    const send = (() => {
        try {
            port.send("late");

            return "sent";
        } catch (error) {
            return error instanceof DOMException ? error.name : "other";
        }
    })();

    expect({
        order,
        waiting: await waiting.catch((error: Error) => error.message),
        send,
    }).toEqual({
        order: ["crashing", "other"],
        waiting: "node crashing crashed",
        send: "InvalidStateError",
    });
});

test("broadcast copies of a message to every other open port of a channel, and persist storage", async () => {
    // open two tabs' ports of one channel and one of another
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation);
    const sender = browsers.environment("sender");
    const receiver = browsers.environment("receiver");
    const heard: unknown[] = [];
    const sending = sender.broadcast.open("destack:space");
    const receiving = receiver.broadcast.open("destack:space");
    const other = receiver.broadcast.open("destack:other");
    using _sent = sending.on((message) => heard.push(["sender", message]));
    using _received = receiving.on((message) => heard.push(["receiver", message]));
    using _other = other.on((message) => heard.push(["other", message]));

    // send a message and change it after sending, then close the receiver before a delivery
    const message = { commit: 1 };
    sending.send(message);
    message.commit = 2;
    await simulation.drive(Promise.resolve());
    sending.send({ commit: 3 });
    receiving.close();
    await simulation.drive(Promise.resolve());

    expect({
        heard,
        persisted: await sender.storage.persist(),
        isPersisted: sender.storage.isPersisted,
    }).toEqual({ heard: [["receiver", { commit: 1 }]], persisted: true, isPersisted: true });
});

test("deliver a broadcast to every listener past one that throws, and report what it threw", async () => {
    // listen on two ports, the first listener throwing
    await using simulation = new Simulation({ seed: 1, start: START });
    const browsers = new BrowserSimulation(simulation);
    const sending = browsers.environment("sender").broadcast.open("destack:space");
    const failing = browsers.environment("failing").broadcast.open("destack:space");
    const listening = browsers.environment("listening").broadcast.open("destack:space");
    const heard: unknown[] = [];
    using _failing = failing.on(() => {
        throw new Error("listener broke");
    });
    using _listening = listening.on((message) => heard.push(message));

    // send once, and read what the delivery reported
    sending.send("commit");
    await simulation.drive(Promise.resolve());

    expect({
        heard,
        errors: browsers.broadcast.errors.map((error) =>
            error instanceof Error ? error.message : error,
        ),
    }).toEqual({ heard: ["commit"], errors: ["listener broke"] });
});
