import { expect, onTestFinished, test } from "@destack/test";
import { type Channel, channelHub } from "../channel/index.ts";
import { Tracker, type TrackerEvent, type TrackerMessage } from "./tracker.ts";
import { Simulation } from "@destack/environment/simulation";

/** The heartbeat interval of the test trackers, in milliseconds of their clocks. */
const HEARTBEAT = 5;

test("replicate which instance tracks an owner, released everywhere at its last release or once its instance stops", async () => {
    const simulation = new Simulation({ seed: 1, start: 0 });
    const join = channelHub<TrackerMessage>();
    const east = open(join(), simulation);
    const west = open(join(), simulation);
    const changes: [string, boolean][] = [];
    west.watchTracked((owner, isTracked) => changes.push([owner, isTracked]));

    // see the owner tracked on the other instance until its last hold is released
    east.track("alice");
    east.track("alice");
    await tracked(west, "alice", true);
    east.release("alice");
    const isHeld = east.isTracked("alice");
    east.release("alice");
    await tracked(west, "alice", false);
    expect(isHeld).toBe(true);

    // release an instance's tracked owners when it stops
    east.track("bob");
    await tracked(west, "bob", true);
    east.close();
    await tracked(west, "bob", false);
    expect(changes).toEqual([
        ["alice", true],
        ["alice", false],
        ["bob", true],
        ["bob", false],
    ]);
});

test("hand a late instance the tracked owners, and count a silent instance stopped after three missed heartbeats", async () => {
    const simulation = new Simulation({ seed: 1, start: 0 });
    const join = channelHub<TrackerMessage>();
    const east = open(join(), simulation);
    east.track("alice");

    // answer a late instance with the owners tracked before it joined
    const west = open(join(), simulation);
    const stopped: string[] = [];
    west.watchStopped((instance) => stopped.push(instance));
    await tracked(west, "alice", true);

    // count a crashed instance stopped once the clock passes three missed heartbeats
    let isCrashed = false;
    const channel = join();
    const silenced: Channel<TrackerMessage> = {
        notify: (message) => {
            if (!isCrashed) {
                channel.notify(message);
            }
        },
        listen: (listener) => channel.listen(listener),
        hear: (token) => channel.hear(token),
        close: () => channel.close(),
    };
    const crashing = open(silenced, simulation);
    crashing.track("bob");
    await tracked(west, "bob", true);
    isCrashed = true;
    await simulation.run({ to: simulation.clock.now() + HEARTBEAT * 4 });
    await tracked(west, "bob", false);
    expect({ alice: west.isTracked("alice"), stopped }).toEqual({
        alice: true,
        stopped: [crashing.instance],
    });
});

test("count an unknown instance live only until this one could hear it, and report each instance that stops", async () => {
    const simulation = new Simulation({ seed: 1, start: 0 });
    const join = channelHub<TrackerMessage>();
    const east = open(join(), simulation);
    const west = open(join(), simulation);
    const stopped: string[] = [];
    west.watchStopped((instance) => stopped.push(instance));

    // count an unknown instance live while west listens, and east once heard
    const isListening = west.isLive("unknown");
    east.track("alice");
    await tracked(west, "alice", true);
    simulation.clock.advance(HEARTBEAT * 4);
    const isForgotten = !west.isLive("unknown");
    const isHeard = west.isLive(east.instance);

    // report east once it stops
    east.close();
    await tracked(west, "alice", false);
    expect({ isListening, isForgotten, isHeard, stopped }).toEqual({
        isListening: true,
        isForgotten: true,
        isHeard: true,
        stopped: [east.instance],
    });
});

test("deliver a broadcast on every instance with its sender, and track alone without a channel", async () => {
    const simulation = new Simulation({ seed: 1, start: 0 });
    const join = channelHub<TrackerMessage>();
    const east = open(join(), simulation);
    const west = open(join(), simulation);
    const alone = open(undefined, simulation);

    // hear a broadcast on the sending and the other instance
    const heard: TrackerEvent[] = [];
    const received = Promise.withResolvers<void>();
    east.listen("board", (event) => heard.push(event));
    west.listen("board", (event) => {
        heard.push(event);
        received.resolve();
    });
    east.broadcast("board", { reaction: "👍" });
    await received.promise;

    // track an owner and broadcast on an instance without others
    const lone: TrackerEvent[] = [];
    alone.listen("board", (event) => lone.push(event));
    alone.track("alice");
    alone.broadcast("board", { reaction: "👋" });
    expect({ heard, lone, isTracked: alone.isTracked("alice") }).toEqual({
        heard: [
            { instance: east.instance, event: { reaction: "👍" } },
            { instance: east.instance, event: { reaction: "👍" } },
        ],
        lone: [{ instance: alone.instance, event: { reaction: "👋" } }],
        isTracked: true,
    });
});

test("skip the messages a channel echoes back to their sender, as PostgreSQL's does", async () => {
    const simulation = new Simulation({ seed: 1, start: 0 });
    const join = channelHub<TrackerMessage>();
    const east = open(join(), simulation);
    const echo = join();

    // echo the instance's own release and stop back to it
    east.track("alice");
    echo.notify({ kind: "release", instance: east.instance, owner: "alice" });
    echo.notify({ kind: "stop", instance: east.instance });
    await new Promise<void>((resolve) => {
        queueMicrotask(resolve);
    });
    expect({ isTracked: east.isTracked("alice"), isLive: east.isLive(east.instance) }).toEqual({
        isTracked: true,
        isLive: true,
    });
});

/** Track the instances on a channel, or alone, in the test's world. */
function open(channel: Channel<unknown> | undefined, simulation: Simulation): Tracker {
    const tracker = new Tracker(simulation.environment("database"), channel, {
        heartbeat: HEARTBEAT,
    });
    onTestFinished(() => tracker.close());

    return tracker;
}

/** Wait until an instance sees an owner tracked or released. */
async function tracked(tracker: Tracker, owner: string, isTracked: boolean): Promise<void> {
    // wait for the change unless the instance already sees it
    if (tracker.isTracked(owner) !== isTracked) {
        await new Promise<void>((resolve) => {
            const stop = tracker.watchTracked((changed, now) => {
                if (changed === owner && now === isTracked) {
                    stop();
                    resolve();
                }
            });
        });
    }
}
