import { BunCrypto } from "../crypto/bun.ts";
import { mkdirSync, realpathSync } from "node:fs";
import { rm } from "node:fs/promises";
import { join } from "node:path";
import { Crypto } from "../crypto/crypto.ts";
import type { Environment, HostCapability } from "../environment/environment.ts";
import type { Host } from "../host/host.ts";
import { BunProcess } from "../process/bun.ts";
import { Workers } from "../worker/worker.ts";
import { SimulatedCompression } from "../compression/simulated.ts";
import { SimulatedFileSystem } from "../file/simulated.ts";
import { SimulatedTelemetry } from "../telemetry/simulated.ts";
import { type ProcessOptions, SimulatedProcess } from "../process/simulated.ts";
import { SimulatedClock } from "../clock/simulated.ts";
import { SimulatedNetwork } from "../network/simulated.ts";
import { SimulatedRandom } from "../random/simulated.ts";
import { Clock } from "../clock/clock.ts";

/** The rounds of answered waits a run allows without moving time, far past any real chain. */
const LIVELOCK_ROUNDS = 10_000;

/** The turns a settle allows without moving time: at about 1 µs a turn, 200 ms of busy work. */
const LIVELOCK_TURNS = 200_000;

/** The real time host work may stall in flight before a run fails, ten seconds. */
const STALL_TIMEOUT = 10_000;

/** The real time tracked work may wait beside a pending timer without progress: 100 ms. */
const DEADLOCK_MILLISECONDS = 100;

/** The turns a livelock's report records the call sites of. */
const RECORDED_TURNS = 100;

/** The call sites a livelock's report names, most frequent first. */
const RECORDED_SITES = 5;

/** The machine every node runs on unless the simulation gives another, its home aside. */
const HOST: Omit<Host, "home"> = {
    os: "linux",
    architecture: "x64",
    version: "6.8.0",
    hostname: "simulation",
};

/** The directory of simulation roots: a 48-byte root and 38-byte socket fit a 104-byte sun_path. */
const ROOT_DIRECTORY = "/tmp";

/** The random bytes of a simulation root's name: 64 bits, distinct across any run's simulations. */
const ROOT_BYTES = 8;

/** The locale every node of a simulation formats in unless it gives another. */
const LOCALE = "en-US";

/** The time zone every node of a simulation formats in unless it gives another. */
const TIME_ZONE = "UTC";

/** The runtime every node runs on unless the simulation gives another process: a fixed build. */
const RUNTIME = { revision: "0".repeat(40), isStandalone: false } as const;

/** Something that happened in a simulation, in the order it happened. */
export interface SimulationEvent {
    /** What happened. */
    readonly kind:
        | "timer"
        | "jump"
        | "request"
        | "loss"
        | "refusal"
        | "break"
        | "partition"
        | "heal"
        | "start"
        | "crash";
    /** When, in the simulation's UTC epoch milliseconds. */
    readonly at: number;
    /** What it concerned, such as the nodes and the request. */
    readonly detail: string;
}

/** Start a node's work in its environment, stopping it once the signal aborts. */
export type NodeStart<Running> = (
    environment: Environment<HostCapability>,
    signal: AbortSignal,
) => Promise<Running>;

/** A participant of a simulation, such as a machine or a tab, running work until a crash. */
export class SimulatedNode {
    /** The node's name, which partitions and the trace use. */
    readonly name: string;
    /** The node's environment: the world's clock, randomness and files, and its own process. */
    readonly environment: Environment<HostCapability>;
    /** The simulation the node runs in. */
    readonly #simulation: Simulation;
    /** Make the node a fresh process, as each start after a crash has. */
    readonly #spawn: () => SimulatedProcess;
    /** Release what the world gives the node once it crashes. */
    readonly #release: () => void;
    /** The node's current process. */
    #process: SimulatedProcess;
    /** The signal stopping the running work, absent while the node runs none. */
    #stop: AbortController | undefined;

    /** Make a node of a simulation from its effects, with a fresh process per start. */
    constructor(
        name: string,
        simulation: Simulation,
        effects: Omit<Environment<HostCapability>, "process">,
        spawn: () => SimulatedProcess,
        release: () => void,
    ) {
        // keep the node's effects, its process read through the current one
        this.name = name;
        this.#simulation = simulation;
        this.#spawn = spawn;
        this.#release = release;
        this.#process = spawn();
        const read = () => this.#process;
        this.environment = {
            ...effects,
            get process() {
                return read();
            },
        };
    }

    /** Start work on the node, or start it again after a crash, answering what the work answers. */
    async start<Running>(work: NodeStart<Running>): Promise<Running> {
        // refuse a node already running work
        if (this.#stop !== undefined) {
            throw new Error(`node ${this.name} is already running`);
        }

        // start the work under a signal a crash aborts
        const stop = new AbortController();
        this.#stop = stop;
        this.#simulation.record({
            kind: "start",
            at: this.environment.clock.now(),
            detail: this.name,
        });

        return await work(this.environment, stop.signal);
    }

    /** Stop the node at once, as a lost process, leaving only what it stored. */
    crash(): void {
        // stop the node's work, give it a fresh process, and release its sockets and listeners
        this.#simulation.record({
            kind: "crash",
            at: this.environment.clock.now(),
            detail: this.name,
        });
        this.#stop?.abort(new Error(`node ${this.name} crashed`));
        this.#stop = undefined;
        this.#process = this.#spawn();
        this.#release();
    }
}

/** A run or driven work waiting on a simulation's loop: its condition, its time, or both. */
interface SimulationWait {
    /** The condition the wait ends at, absent for a wait to a time alone. */
    readonly until: (() => boolean | Promise<boolean>) | undefined;
    /** The time the wait ends at or refuses its condition at, absent for no deadline. */
    readonly deadline: number | undefined;
    /** Answer the wait. */
    readonly resolve: () => void;
    /** Fail the wait. */
    readonly reject: (error: unknown) => void;
}

/** How a simulation's nodes run, beside its seed and start. */
export interface SimulationOptions {
    /** The process each node runs as, the host's by default. */
    readonly process?: ProcessOptions;
    /** The machine each node runs on. */
    readonly host?: Host;
    /** The locale each node formats in. */
    readonly locale?: string;
    /** The time zone each node formats in. */
    readonly timeZone?: string;
    /** The real milliseconds host work may stall in flight before a run fails. */
    readonly stallTimeout?: number;
    /** The turns a settle allows without moving time before it fails as busy. */
    readonly livelockTurns?: number;
    /** Whether the trace records what happens, off by default as it grows with every event. */
    readonly isTracing?: boolean;
}

/** A simulated world of simulated time, seeded randomness, a faulty network and nodes. */
export class Simulation {
    /** The seed, which reproduces the simulation. */
    readonly seed: number;
    /** The simulated time. */
    readonly clock: SimulatedClock;
    /** The seeded randomness. */
    readonly random: SimulatedRandom;
    /** The network between the nodes. */
    readonly network: SimulatedNetwork;
    /** What happened, in order, empty unless the simulation traces. */
    readonly trace: SimulationEvent[] = [];
    /** The host directory of the nodes' files, removed on dispose. */
    readonly root: string;
    /** The telemetry every node emits, recorded in memory. */
    readonly telemetry: SimulatedTelemetry;
    /** The process each node runs as. */
    readonly #process: ProcessOptions;
    /** The machine each node runs on. */
    readonly #host: Host;
    /** The real milliseconds host work may stall in flight before a run fails. */
    readonly #stallTimeout: number;
    /** The turns a settle allows without moving time before it fails as busy. */
    readonly #livelockTurns: number;
    /** The locale each node formats in. */
    readonly #locale: string;
    /** The time zone each node formats in. */
    readonly #timeZone: string;
    /** The nodes, by name. */
    readonly #nodes = new Map<string, SimulatedNode>();
    /** The listeners of node crashes. */
    readonly #crashListeners = new Set<(node: string) => void>();
    /** The runs and driven work waiting on the loop, each answered once met. */
    readonly #waits: SimulationWait[] = [];
    /** The loop moving time now, which later runs and drives join, absent while it stands still. */
    #moving: Promise<void> | undefined;
    /** Whether the trace records events. */
    readonly #isTracing: boolean;
    /** The events that happened, traced or not, which the loop reads as progress. */
    #events = 0;

    /** Start a simulation at a time from a seed. */
    constructor(options: SimulationOptions & { readonly seed: number; readonly start: number }) {
        // create the clock, randomness and network
        this.seed = options.seed;
        this.clock = new SimulatedClock(options.start);
        this.random = new SimulatedRandom(options.seed);
        this.telemetry = new SimulatedTelemetry({ clock: this.clock, random: this.random });
        this.#isTracing = options.isTracing ?? false;
        this.network = new SimulatedNetwork(this.clock, this.random, (event) => this.record(event));

        // give the nodes their process, machine and a root for their files
        const name = crypto.getRandomValues(new Uint8Array(ROOT_BYTES)).toHex();
        this.root = join(realpathSync(ROOT_DIRECTORY), `destack-simulation-${name}`);
        const home = join(this.root, "home");
        mkdirSync(home, { recursive: true });
        this.#process = options.process ?? Simulation.#hostProcess(this.root, home);
        this.#host = options.host ?? { ...HOST, home };
        this.#stallTimeout = options.stallTimeout ?? STALL_TIMEOUT;
        this.#livelockTurns = options.livelockTurns ?? LIVELOCK_TURNS;
        this.#locale = options.locale ?? LOCALE;
        this.#timeZone = options.timeZone ?? TIME_ZONE;
    }

    /** Fork a recorded world by a label into a world of the split seed at the recorded time. */
    static fork(
        recorded: { readonly seed: number; readonly start: number },
        label: string,
        options: SimulationOptions = {},
    ): Simulation {
        return new Simulation({
            ...options,
            seed: SimulatedRandom.split(recorded.seed, label),
            start: recorded.start,
        });
    }

    /** Fork this world by a label into a world of the split seed at this world's time. */
    fork(label: string): Simulation {
        return Simulation.fork({ seed: this.seed, start: this.clock.now() }, label, {
            process: this.#process,
            host: this.#host,
            locale: this.#locale,
            timeZone: this.#timeZone,
            stallTimeout: this.#stallTimeout,
            livelockTurns: this.#livelockTurns,
            isTracing: this.#isTracing,
        });
    }

    /** Remove the nodes' files, and stop the telemetry's context. */
    async [Symbol.asyncDispose](): Promise<void> {
        this.telemetry.context.disable();
        await rm(this.root, { recursive: true, force: true });
    }

    /** Count an event as progress, and trace it when the simulation traces. */
    record(event: SimulationEvent): void {
        this.#events += 1;
        if (this.#isTracing) {
            this.trace.push(event);
        }
    }

    /** Read a node's environment: the world's clock and randomness, and its own effects. */
    environment(node: string): Environment<HostCapability> {
        return this.node(node).environment;
    }

    /** Read a node by its name, made on its first read with its effects on the world's clock. */
    node(name: string): SimulatedNode {
        // reuse the node
        const known = this.#nodes.get(name);
        if (known !== undefined) {
            return known;
        }

        // give the node its effects, a fresh process per start, and its release on a crash
        const { clock, random } = this;
        const effects = {
            clock,
            random,
            crypto: Crypto.track(BunCrypto, clock),
            network: this.network.of(name),
            compression: SimulatedCompression,
            telemetry: this.telemetry,
            locale: this.#locale,
            timeZone: this.#timeZone,
            workers: Workers.system,
            server: this.network.server(name),
            files: new SimulatedFileSystem(this.root, clock, random),
            host: this.#host,
        };
        const spawn = () => new SimulatedProcess(this.#process, clock);
        const node = new SimulatedNode(name, this, effects, spawn, () => this.#release(name));
        this.#nodes.set(name, node);

        return node;
    }

    /** Call a listener on each node crash until the subscription disposes. */
    onCrash(listener: (node: string) => void): Disposable {
        this.#crashListeners.add(listener);

        return { [Symbol.dispose]: () => this.#crashListeners.delete(listener) };
    }

    /** Release a crashed node's sockets and listeners, and tell the crash listeners. */
    #release(node: string): void {
        this.network.release(node);
        for (const listener of this.#crashListeners) {
            listener(node);
        }
    }

    /** Run until a condition is true or to a time, moving to each next timer once idle. */
    run(
        options:
            | { readonly until: () => boolean | Promise<boolean>; readonly to?: number }
            | { readonly to: number },
    ): Promise<void> {
        return this.#wait("until" in options ? options.until : undefined, options.to);
    }

    /** Jump to a later time at once, waking each due wait a single time. */
    async jump(to: number): Promise<void> {
        // refuse a jump while a run or drive waits on the loop, else let a finishing loop stop
        if (this.#waits.length > 0) {
            throw new Error("a simulation cannot jump while it runs");
        }
        await this.#moving;

        // settle the work at the current time, then move straight to the later one
        await this.run({ to: this.clock.now() });
        this.record({ kind: "jump", at: to, detail: "" });
        this.clock.set(to);

        // settle the work the jump woke
        await this.run({ to });
    }

    /** Run until some work settles, failing loudly once the simulation goes idle before it does. */
    async drive<Value>(work: Promise<Value>): Promise<Value> {
        // move time until the work settles
        let isSettled = false;
        const until = () => isSettled;
        const settled = work.finally(() => {
            isSettled = true;
            this.#withdraw(until);
        });
        await Promise.race([settled.then(noop, noop), this.#wait(until, undefined)]);

        return await settled;
    }

    /** Wait on the loop for a condition or a time, starting the loop if it stands still. */
    #wait(
        until: (() => boolean | Promise<boolean>) | undefined,
        deadline: number | undefined,
    ): Promise<void> {
        // add the wait, answered by the loop
        const { promise, resolve, reject } = Promise.withResolvers<void>();
        this.#waits.push({ until, deadline, resolve, reject });

        // start the loop unless it moves time already
        this.#moving ??= this.#move();

        return promise;
    }

    /** Answer a wait the loop has not checked yet, as its work settled on its own. */
    #withdraw(until: () => boolean | Promise<boolean>): void {
        const index = this.#waits.findIndex((wait) => wait.until === until);
        if (index !== -1) {
            this.#waits.splice(index, 1)[0]?.resolve();
        }
    }

    /** Move time to each next timer once idle, answering each wait once met, until none waits. */
    async #move(): Promise<void> {
        let answered = 0;
        try {
            for (;;) {
                // settle the work and answer the met waits
                await this.#settle();
                const next = this.clock.next;
                if (await this.#answer(next)) {
                    // refuse a loop that answers waits without moving time
                    answered += 1;
                    if (answered >= LIVELOCK_ROUNDS) {
                        const sites = this.clock.flights().join("\n");
                        throw new Error(
                            `the simulation kept answering waits without moving time:\n${sites}`,
                        );
                    }
                    continue;
                }
                answered = 0;

                // read the earliest deadline among the waits
                const deadlines = this.#waits.flatMap((wait) =>
                    wait.deadline === undefined ? [] : [wait.deadline],
                );
                const deadline = deadlines.length === 0 ? undefined : Math.min(...deadlines);

                // end once none waits
                if (this.#waits.length === 0) {
                    this.#moving = undefined;

                    return;
                }
                // move to the earliest deadline before the next timer
                else if (deadline !== undefined && (next === undefined || next > deadline)) {
                    this.clock.set(Math.max(this.clock.now(), deadline));
                }
                // refuse the waits idle work never meets
                else if (next === undefined) {
                    throw new Error("the simulation went idle before its condition held");
                }
                // move time to the next timer
                else {
                    this.record({ kind: "timer", at: next, detail: "" });
                    this.clock.set(next);
                }
            }
        } catch (error) {
            // fail every wait with the loop
            this.#moving = undefined;
            for (const wait of this.#waits.splice(0)) {
                wait.reject(error);
            }
        }
    }

    /** Answer the waits met once the work settled, reporting whether any was. */
    async #answer(next: number | undefined): Promise<boolean> {
        // check each wait, keeping the ones still unmet
        const waits = this.#waits.splice(0);
        const now = this.clock.now();
        let isAnswered = false;
        for (const wait of waits) {
            // read whether the wait's time passed with no timer left before it
            const isAtDeadline =
                wait.deadline !== undefined &&
                now >= wait.deadline &&
                (next === undefined || next > wait.deadline);

            // read the condition, keeping what it throws for this wait alone
            const { until } = wait;
            const condition =
                until === undefined
                    ? { isMet: false }
                    : await Promise.resolve()
                          .then(until)
                          .then(
                              (isMet) => ({ isMet }),
                              (error: unknown) => ({ isMet: false, error }),
                          );

            // fail the wait whose condition threw
            if ("error" in condition) {
                wait.reject(condition.error);
                isAnswered = true;
            }
            // answer a met condition
            else if (condition.isMet) {
                wait.resolve();
                isAnswered = true;
            }
            // refuse a condition its deadline passed
            else if (wait.until !== undefined && isAtDeadline) {
                wait.reject(
                    new Error("the simulation reached its deadline before its condition held"),
                );
            }
            // answer a time once it passed
            else if (wait.until === undefined && isAtDeadline) {
                wait.resolve();
                isAnswered = true;
            }
            // wait for the condition or the deadline
            else {
                this.#waits.push(wait);
            }
        }

        return isAnswered;
    }

    /** Read the process a node runs as without another, with the host's paths and executable. */
    static #hostProcess(root: string, home: string): ProcessOptions {
        // take the host's search path, refusing a host without one
        const host = new BunProcess();
        const { PATH } = host.variables;
        if (PATH === undefined) {
            throw new TypeError("the host process has no PATH");
        }

        // put the home and the working directory in the world's root
        return {
            arguments: [],
            variables: { PATH, HOME: home },
            workingDirectory: root,
            id: 1,
            userId: host.userId,
            executable: host.executable,
            runtime: RUNTIME,
        };
    }

    /** Let pending work run until a macrotask turn passes without progress or tracked work. */
    async #settle(): Promise<void> {
        // start from the current progress, nothing counted as busy
        let progress = this.#progress();
        let busy = 0;
        let isMoving = true;
        let movedAt = Clock.system.now();
        for (;;) {
            // yield one turn while work moves, else wait for host work to settle or a host macrotask
            const { clock } = this;
            if (isMoving || clock.hostInFlight === 0) {
                await new Promise<void>((resolve) => {
                    setImmediate(resolve);
                });
            } else {
                await Promise.race([clock.nextSettle(), Simulation.#macrotask()]);
            }

            // stop once the turn made no progress and no tracked work is in flight
            const next = this.#progress();
            if (next === progress && clock.inFlight === 0) {
                return;
            }
            isMoving = next !== progress;
            progress = next;
            movedAt = isMoving ? Clock.system.now() : movedAt;

            // refuse host work stalled past its limit, listing the work in flight
            const stalled = Clock.system.now() - movedAt;
            if (clock.hostInFlight > 0 && stalled >= this.#stallTimeout) {
                throw new Error(
                    `host work stayed in flight for ${stalled} ms without settling:\n${clock.flights().join("\n")}`,
                );
            }

            // refuse tracked work waiting on a pending timer, which cannot fire while the work is in flight
            const isWaitingOnTime =
                clock.inFlight > 0 && clock.hostInFlight === 0 && clock.next !== undefined;
            if (isWaitingOnTime && stalled >= DEADLOCK_MILLISECONDS) {
                throw new Error(
                    `tracked work waits on simulated time, which cannot move while it is in flight:\n${clock.flights().join("\n")}`,
                );
            }

            // refuse busy work that neither moves time nor awaits the host, listing its call sites
            busy += clock.hostInFlight === 0 ? 1 : 0;
            if (busy >= this.#livelockTurns) {
                const sites = [...(await this.#busy()), ...clock.flights()];
                throw new Error(
                    `the simulation kept busy for ${busy} turns without moving time:\n${sites.join("\n")}`,
                );
            }
        }
    }

    /** Wait for one macrotask of the host, after its input and output poll. */
    static #macrotask(): Promise<void> {
        return new Promise<void>((resolve) => {
            setTimeout(resolve, 0);
        });
    }

    /** Record what the next turns begin, by call site, most frequent first. */
    async #busy(): Promise<string[]> {
        // record a few turns
        const stop = this.clock.record();
        for (let turn = 0; turn < RECORDED_TURNS; turn += 1) {
            await new Promise<void>((resolve) => {
                setImmediate(resolve);
            });
        }

        // count each call site
        const counts = new Map<string, number>();
        for (const site of stop()) {
            counts.set(site, (counts.get(site) ?? 0) + 1);
        }

        return [...counts]
            .toSorted(([, left], [, right]) => right - left)
            .slice(0, RECORDED_SITES)
            .map(([site, count]) => `${count}x ${site}`);
    }

    /** Read the simulation's progress: waits, settles, events and deliveries, each only growing. */
    #progress(): number {
        const { clock, network } = this;

        return clock.begun + clock.settled + this.#events + network.sent + network.delivered;
    }
}

/** Do nothing, as a settled promise's handler. */
function noop(): void {}
