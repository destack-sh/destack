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

/** The turns the world's own work may spin between host answers: at about 1 µs a turn, 200 ms. */
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

/** The FNV-1a 32-bit offset basis and prime, hashing a simulation's events. */
const FNV = { basis: 0x811c9dc5, prime: 0x01000193 } as const;

/** The locale every node of a simulation formats in unless it gives another. */
const LOCALE = "en-US";

/** The time zone every node of a simulation formats in unless it gives another. */
const TIME_ZONE = "UTC";

/** The runtime every node runs on unless the simulation gives another process: a fixed build. */
const RUNTIME = { revision: "0".repeat(40), isStandalone: false } as const;

/** Something that happened in a simulation, in the order it happened. */
export type SimulationEvent =
    | {
          /** The clock moving to the next timer, or jumping. */
          readonly kind: "timer" | "jump";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
      }
    | {
          /** A request or connection arriving, or lost on its way. */
          readonly kind: "request" | "loss";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
          /** The sending node. */
          readonly from: string;
          /** What the node reached for: a request's method and URL, or a socket's address. */
          readonly target: string;
      }
    | {
          /** A request or connection refused. */
          readonly kind: "refusal";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
          /** The sending node. */
          readonly from: string;
          /** What the node reached for: a request's method and URL, or a socket's address. */
          readonly target: string;
          /** Why: nothing serves the target, or a partition cuts the node off from it. */
          readonly reason: "unserved" | "partitioned";
      }
    | {
          /** A connection breaking at both ends. */
          readonly kind: "break";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
          /** The connecting node. */
          readonly client: string;
          /** The accepting node. */
          readonly server: string;
      }
    | {
          /** The network cut or joined between nodes. */
          readonly kind: "partition" | "heal";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
          /** A pair, one node against every other, or none for every pair. */
          readonly nodes: readonly string[];
      }
    | {
          /** A node starting its work, or crashing. */
          readonly kind: "start" | "crash";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
          /** The node. */
          readonly node: string;
      }
    | {
          /** Settled host work answered to the simulation, in the order it began. */
          readonly kind: "release";
          /** When, in the simulation's UTC epoch milliseconds. */
          readonly at: number;
          /** The order the host work began in. */
          readonly order: number;
      };

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
            node: this.name,
        });

        return await work(this.environment, stop.signal);
    }

    /** Stop the node at once, as a lost process, leaving only what it stored. */
    crash(): void {
        // stop the node's work, give it a fresh process, and release its sockets and listeners
        this.#simulation.record({
            kind: "crash",
            at: this.environment.clock.now(),
            node: this.name,
        });
        this.#stop?.abort(new Error(`node ${this.name} crashed`));
        this.#stop = undefined;
        this.#process = this.#spawn();
        this.#release();
    }
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
    /** The turns the world's own work may spin between host answers before it fails as busy. */
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
    /** The turns the world's own work may spin between host answers before it fails as busy. */
    readonly #livelockTurns: number;
    /** The locale each node formats in. */
    readonly #locale: string;
    /** The time zone each node formats in. */
    readonly #timeZone: string;
    /** The nodes, by name. */
    readonly #nodes = new Map<string, SimulatedNode>();
    /** The listeners of node crashes. */
    readonly #crashListeners = new Set<(node: string) => void>();
    /** Move the world's clock, which its loop alone does. */
    readonly #setTime: (time: number) => void;
    /** The loop moving the world's time, running from construction until disposed. */
    readonly #moving: Promise<void>;
    /** Whether the world runs, false once disposed. */
    #isRunning = true;
    /** The waits for the world to go idle, answered at the loop's next idle point. */
    readonly #idlers: PromiseWithResolvers<void>[] = [];
    /** The jump the loop takes at its next idle point, absent with none asked. */
    #jump: { readonly to: number; readonly answer: PromiseWithResolvers<void> } | undefined;
    /** The wake of the loop waiting for new work, absent while it works. */
    #wake: (() => void) | undefined;
    /** The waits for the next idle point after time moves, each with the moves at its start. */
    readonly #steps: { readonly answer: PromiseWithResolvers<void>; readonly moves: number }[] = [];
    /** The times the loop moved the world's time, to a timer or by a jump. */
    #moves = 0;
    /** The failure of the loop, refusing later waits, thrown at dispose unless a wait took it. */
    #failure: { readonly error: unknown; readonly isTaken: boolean } | undefined;
    /** The world's disposal, absent until it began. */
    #disposed: Promise<void> | undefined;
    /** Whether the trace records events. */
    readonly #isTracing: boolean;
    /** The events that happened, traced or not, which the loop reads as progress. */
    #events = 0;
    /** The FNV-1a hash of every event so far, equal across runs of one seed and work. */
    #fingerprint = FNV.basis;

    /** Start a simulation at a time from a seed. */
    constructor(options: SimulationOptions & { readonly seed: number; readonly start: number }) {
        // create the clock, randomness and network
        this.seed = options.seed;
        const owned = SimulatedClock.owned(options.start);
        this.clock = owned.clock;
        this.#setTime = owned.move;
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

        // run the world
        this.#moving = this.#move();
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

    /** Stop the world's loop, remove the nodes' files, and stop the telemetry's context, once. */
    [Symbol.asyncDispose](): Promise<void> {
        this.#disposed ??= this.#dispose();

        return this.#disposed;
    }

    /** Stop the world's loop, remove the nodes' files, and stop the telemetry's context. */
    async #dispose(): Promise<void> {
        // stop the loop, answering host work at once from now on, then refuse a failure no wait took
        this.#isRunning = false;
        this.#wake?.();
        await this.#moving;
        this.clock.detach();
        this.telemetry.context.disable();
        await rm(this.root, { recursive: true, force: true });
        if (this.#failure !== undefined && !this.#failure.isTaken) {
            throw this.#failure.error;
        }
    }

    /** Count an event as progress, fold it into the fingerprint, and trace it when tracing. */
    record(event: SimulationEvent): void {
        // count the event
        this.#events += 1;

        // fold its encoding into the fingerprint with FNV-1a
        const encoded = JSON.stringify(event);
        let hash = this.#fingerprint;
        for (let index = 0; index < encoded.length; index += 1) {
            hash = Math.imul(hash ^ encoded.charCodeAt(index), FNV.prime) >>> 0;
        }
        this.#fingerprint = hash;

        // trace it
        if (this.#isTracing) {
            this.trace.push(event);
        }
    }

    /** The hash of every event so far, equal across runs of one seed and the same work. */
    get fingerprint(): number {
        return this.#fingerprint;
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

    /** Wait for the world's next idle point: nothing left to do before its next timer. */
    idle(): Promise<void> {
        // refuse a wait on a failed world
        if (this.#failure !== undefined) {
            return Promise.reject(this.#failure.error);
        }

        // wait for the loop's next idle point
        const answer = Promise.withResolvers<void>();
        this.#idlers.push(answer);
        this.#wake?.();

        return answer.promise;
    }

    /** Wait for the world's idle points until a condition is met, refusing it past a deadline. */
    async until(condition: () => boolean | Promise<boolean>, deadline?: number): Promise<void> {
        // stop the world's time at the deadline, withdrawing the deadline once the wait ends
        const withdrawn = new AbortController();
        let isLate = false;
        if (deadline !== undefined) {
            void this.clock.sleep(deadline - this.clock.now(), withdrawn.signal).then(
                () => (isLate = true),
                () => undefined,
            );
        }
        try {
            // check after the work due now, then after each move of time
            let isSettled = false;
            while (!(await condition())) {
                // refuse a condition the deadline passed
                if (isLate) {
                    throw new Error("the world reached its deadline before its condition was met");
                }
                await (isSettled ? this.#step() : this.idle());
                isSettled = true;
            }
        } finally {
            withdrawn.abort();
        }
    }

    /** Jump to a later time at once, waking each due timer once, until the world is idle. */
    jump(to: number): Promise<void> {
        // refuse a jump on a failed world, and a second one at once
        if (this.#failure !== undefined) {
            return Promise.reject(this.#failure.error);
        }
        if (this.#jump !== undefined) {
            return Promise.reject(new Error("a simulation cannot jump while it jumps"));
        }

        // let the loop jump at its next idle point
        const answer = Promise.withResolvers<void>();
        this.#jump = { to, answer };
        this.#wake?.();

        return answer.promise;
    }

    /** Wait for the world's next idle point after its time moves, refused once nothing is left. */
    #step(): Promise<void> {
        // refuse a wait on a failed world
        if (this.#failure !== undefined) {
            return Promise.reject(this.#failure.error);
        }

        // wait for the loop's next idle point after a move
        const answer = Promise.withResolvers<void>();
        this.#steps.push({ answer, moves: this.#moves });
        this.#wake?.();

        return answer.promise;
    }

    /** Move the world's time to each next timer once idle, until disposed. */
    async #move(): Promise<void> {
        try {
            while (this.#isRunning) {
                // settle the work at the current time
                await this.#settle();
                const { next } = this.clock;
                const now = this.clock.now();
                const jump = this.#jump;

                // fire the timers due now
                if (next !== undefined && next <= now) {
                    this.#setTime(now);
                }
                // answer the waits for an idle world, then settle the work they start
                else if (this.#idlers.length > 0) {
                    for (const idler of this.#idlers.splice(0)) {
                        idler.resolve();
                    }
                }
                // answer the steps begun before the last move, then settle the work they start
                else if (this.#steps.some((step) => step.moves < this.#moves)) {
                    const moved = this.#steps.filter((step) => step.moves < this.#moves);
                    this.#steps.splice(
                        0,
                        this.#steps.length,
                        ...this.#steps.filter((step) => !moved.includes(step)),
                    );
                    for (const step of moved) {
                        step.answer.resolve();
                    }
                }
                // jump when asked, answering the jump once the woken work is idle
                else if (jump !== undefined) {
                    this.#jump = undefined;
                    this.record({ kind: "jump", at: jump.to });
                    this.#setTime(Math.max(now, jump.to));
                    this.#moves += 1;
                    this.#idlers.push(jump.answer);
                }
                // move time to the next timer
                else if (next !== undefined) {
                    this.record({ kind: "timer", at: next });
                    this.#setTime(next);
                    this.#moves += 1;
                }
                // refuse the steps of a world with nothing left to do
                else if (this.#steps.length > 0) {
                    const error = new Error("the world went idle with nothing left to do");
                    for (const step of this.#steps.splice(0)) {
                        step.answer.reject(error);
                    }
                }
                // wait for new work with no timer pending, unless disposed meanwhile
                else if (this.#isRunning) {
                    const woken = new Promise<void>((resolve) => {
                        this.#wake = resolve;
                    });
                    await Promise.race([woken, this.clock.nextActivity()]);
                    this.#wake = undefined;
                }
            }
        } catch (error) {
            // fail the clock's waits, the idle waits and the jump with the loop, keeping the failure
            const failed = this.clock.fail(error);
            const idlers = [
                ...this.#idlers.splice(0),
                ...this.#steps.splice(0).map((step) => step.answer),
            ];
            for (const idler of idlers) {
                idler.reject(error);
            }
            this.#jump?.answer.reject(error);
            const isTaken = failed > 0 || idlers.length > 0 || this.#jump !== undefined;
            this.#jump = undefined;
            this.#failure = { error, isTaken };
        }
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
    /** Let pending work run until a macrotask turn passes without progress or tracked work. */
    async #settle(): Promise<void> {
        // start from the current progress, nothing counted as busy
        let progress = this.#progress();
        let busy = 0;
        let isMoving = true;
        let movedAt = Clock.system.now();
        for (;;) {
            // yield one turn while work moves or no host work is in flight
            const { clock } = this;
            if (isMoving || clock.hostInFlight === 0) {
                await new Promise<void>((resolve) => {
                    setImmediate(resolve);
                });
            }
            // answer the settled host work in the order it began, once no leaf of it is pending
            else if (clock.hostPending === 0 && clock.hostReady > 0) {
                const at = clock.now();
                for (const order of clock.answer()) {
                    this.record({ kind: "release", at, order });
                }
                busy = 0;
                await new Promise<void>((resolve) => {
                    setImmediate(resolve);
                });
            }
            // wait for host work to begin or settle, checking for a stall at the host's stall limit
            else {
                const watchdog = new AbortController();
                const stall = Clock.system
                    .sleep(this.#stallTimeout, watchdog.signal)
                    .catch(() => undefined);
                await Promise.race([clock.nextActivity(), stall]);
                watchdog.abort();
            }

            // stop once the turn made no progress and no tracked work is in flight
            const next = this.#progress();
            if (next === progress && clock.inFlight === 0) {
                return;
            }
            isMoving = next !== progress;
            progress = next;
            movedAt = isMoving ? Clock.system.now() : movedAt;

            // refuse leaf host work stalled past its limit, listing the work in flight
            const stalled = Clock.system.now() - movedAt;
            if (clock.hostPending > 0 && stalled >= this.#stallTimeout) {
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

            // refuse the world's own work spinning between host answers without moving time, listing its call sites
            busy += clock.hostInFlight === 0 ? 1 : 0;
            if (busy >= this.#livelockTurns) {
                const sites = [...(await this.#busy()), ...clock.flights()];
                throw new Error(
                    `the simulation kept busy for ${busy} turns without moving time:\n${sites.join("\n")}`,
                );
            }
        }
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
