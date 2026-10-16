import { Duration } from "@destack/schema";
import type { Environment } from "../environment/environment.ts";
import { VirtualClock } from "./clock.ts";
import { SimulatedNetwork } from "./network.ts";
import { SeededRandom } from "./random.ts";
import { AsyncLocalStorage } from "node:async_hooks";
import type { ClientSocket } from "../network/network.ts";
import { Random } from "../random/random.ts";
import { SYSTEM } from "./global.ts";

/** The node that work outside every simulated node sends as. */
export const OUTSIDE = "outside";

/** Something that happened in a simulation, in the order it happened. */
export interface SimulationEvent {
    /** What happened. */
    readonly kind:
        | "timer"
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
    environment: Environment,
    signal: AbortSignal,
) => Promise<Running>;

/** A process of a simulation, which it starts, crashes and restarts from what it stored. */
export class SimulatedNode<Running = unknown> {
    /** The node's name, which partitions and the trace use. */
    readonly name: string;
    /** The node's environment: the simulation's clock, and its network as the node. */
    readonly environment: Environment;
    /** Start the node's work. */
    readonly #start: NodeStart<Running>;
    /** The simulation the node runs in. */
    readonly #simulation: Simulation;
    /** The running work and the signal stopping it, absent while crashed. */
    #running: { readonly value: Running; readonly stop: AbortController } | undefined;

    /** Prepare a node of a simulation. */
    constructor(name: string, simulation: Simulation, start: NodeStart<Running>) {
        // take the node's environment from its simulation
        this.name = name;
        this.environment = simulation.environment(name);
        this.#start = start;
        this.#simulation = simulation;
    }

    /** The node's running work, refusing a crashed node. */
    get running(): Running {
        if (this.#running === undefined) {
            throw new Error(`node ${this.name} is not running`);
        }

        return this.#running.value;
    }

    /** Start the node, or start it again after a crash. */
    async start(): Promise<Running> {
        if (this.#running !== undefined) {
            throw new Error(`node ${this.name} is already running`);
        }
        const stop = new AbortController();
        this.#simulation.trace.push({
            kind: "start",
            at: this.environment.clock.now(),
            detail: this.name,
        });
        this.#running = { value: await this.#start(this.environment, stop.signal), stop };

        return this.#running.value;
    }

    /** Stop the node at once, as a lost process, leaving only what it stored. */
    crash(): void {
        // stop the node's work, listeners and connections
        this.#simulation.trace.push({
            kind: "crash",
            at: this.environment.clock.now(),
            detail: this.name,
        });
        this.#running?.stop.abort(new Error(`node ${this.name} crashed`));
        this.#running = undefined;
        this.#simulation.network.release(this.name);
    }
}

/** The installed simulation's clock and socket opening, absent while none is installed. */
let installed:
    | {
          readonly clock: VirtualClock;
          readonly connect: (url: string, protocols?: string | string[]) => ClientSocket;
      }
    | undefined;

/** A date that reads the installed simulation's time. */
class VirtualDate extends SYSTEM.Date {
    /** Create a date at a time, or at the simulation's time without one. */
    constructor(...values: [] | [string | number | Date]) {
        const [value] = values;
        super(value ?? installed?.clock.now() ?? SYSTEM.Date.now());
    }

    /** Read the simulation's time. */
    static override now(): number {
        return installed?.clock.now() ?? SYSTEM.Date.now();
    }
}

/** A WebSocket that connects through the installed simulation's network as the current node. */
class VirtualWebSocket {
    /** The state before the connection opens. */
    static readonly CONNECTING = 0;
    /** The state while the connection is open. */
    static readonly OPEN = 1;
    /** The state while the connection closes. */
    static readonly CLOSING = 2;
    /** The state once the connection closed. */
    static readonly CLOSED = 3;

    /** Connect through the installed simulation, refusing without one. */
    constructor(url: string | URL, protocols?: string | string[]) {
        if (installed === undefined) {
            throw new TypeError("no simulation is installed");
        }

        return installed.connect(String(url), protocols);
    }
}

/** A simulated world of virtual time, seeded randomness, a faulty network and nodes. */
export class Simulation {
    /** The seed, which reproduces the simulation. */
    readonly seed: number;
    /** The virtual time. */
    readonly clock: VirtualClock;
    /** The seeded randomness. */
    readonly random: SeededRandom;
    /** The network between the nodes. */
    readonly network: SimulatedNetwork;
    /** What happened, in order. */
    readonly trace: SimulationEvent[] = [];
    /** The node the current work runs as. */
    readonly #node = new AsyncLocalStorage<string>();

    /** Start a simulation at a time from a seed. */
    constructor(options: { readonly seed: number; readonly start: number }) {
        // create the clock, randomness and network
        this.seed = options.seed;
        this.clock = new VirtualClock(options.start);
        this.random = new SeededRandom(options.seed);
        this.network = new SimulatedNetwork(this.clock, this.random, (event) =>
            this.trace.push(event),
        );
    }

    /** Replace the realm's time, timers, randomness, requests and sockets with the simulation's. */
    install(): this {
        // refuse a second installation
        if (installed !== undefined) {
            throw new Error("a simulation is already installed");
        }

        // replace time and timers
        const { clock, random } = this;
        installed = { clock, connect: (url, protocols) => this.#network().connect(url, protocols) };
        Object.assign(globalThis, {
            Date: VirtualDate,
            setTimeout: clock.setTimeout.bind(clock),
            setInterval: clock.setInterval.bind(clock),
            clearTimeout: clock.clearTimeout.bind(clock),
            clearInterval: clock.clearInterval.bind(clock),
        });
        Object.assign(performance, { now: () => clock.now() });

        // replace randomness
        Object.assign(crypto, {
            getRandomValues: (values: ArrayBufferView | null) => Simulation.#fill(random, values),
            randomUUID: () => Random.uuid(random),
        });
        Object.assign(Math, { random: () => Random.fraction(random) });

        // send requests and open sockets as the current node
        Object.assign(globalThis, {
            WebSocket: VirtualWebSocket,
            fetch: (input: string | URL | Request, initialization?: RequestInit) =>
                this.#network().fetch(
                    typeof input === "string" || input instanceof URL
                        ? new Request(String(input), initialization)
                        : new Request(input, initialization),
                ),
        });

        return this;
    }

    /** Put the realm's own time, timers, randomness, requests and sockets back. */
    uninstall(): void {
        // restore time, timers, requests and sockets
        installed = undefined;
        Object.assign(globalThis, {
            WebSocket: SYSTEM.WebSocket,
            Date: SYSTEM.Date,
            setTimeout: SYSTEM.setTimeout,
            clearTimeout: SYSTEM.clearTimeout,
            setInterval: SYSTEM.setInterval,
            clearInterval: SYSTEM.clearInterval,
            fetch: SYSTEM.fetch,
        });
        Object.assign(performance, { now: SYSTEM.performanceNow });

        // restore randomness
        Object.assign(crypto, {
            getRandomValues: SYSTEM.getRandomValues,
            randomUUID: SYSTEM.randomUUID,
        });
        Object.assign(Math, { random: SYSTEM.random });
    }

    /** Read a node's environment: the simulation's clock, and its network as the node. */
    environment(node: string): Environment {
        return { clock: this.clock, network: this.network.of(node) };
    }

    /** Add a node, started at once. */
    async node<Running>(name: string, start: NodeStart<Running>): Promise<SimulatedNode<Running>> {
        const node = new SimulatedNode(name, this, (environment, signal) =>
            this.#node.run(name, () => start(environment, signal)),
        );
        await node.start();

        return node;
    }

    /** Run until a condition holds, moving time to each next timer once the work settles. */
    async run(
        options:
            | { readonly until: () => boolean | Promise<boolean>; readonly for?: Duration }
            | { readonly for: Duration },
    ): Promise<void> {
        // read the condition and the deadline
        const until = "until" in options ? options.until : undefined;
        const deadline =
            options.for === undefined
                ? undefined
                : this.clock.now() + Duration.milliseconds(options.for);
        for (;;) {
            // read whether the condition holds and the next timer, once the work settles
            await this.#settle();
            const isMet = until !== undefined && (await until());
            const next = this.clock.next;
            const isPastDeadline =
                deadline !== undefined && (next === undefined || next > deadline);

            // end once the condition holds
            if (isMet) {
                return;
            }
            // end a timed run at its deadline
            else if (until === undefined && deadline !== undefined && isPastDeadline) {
                this.clock.set(Math.max(this.clock.now(), deadline));
                await this.#settle();

                return;
            }
            // refuse a condition that idle work or the deadline never meets
            else if (next === undefined || isPastDeadline) {
                throw new Error(
                    next === undefined
                        ? "the simulation went idle before its condition held"
                        : "the simulation reached its deadline before its condition held",
                );
            }
            // move time to the next timer
            else {
                this.trace.push({ kind: "timer", at: next, detail: "" });
                this.clock.set(next);
            }
        }
    }

    /** Read the network of the node the current work runs as. */
    #network() {
        return this.network.of(this.#node.getStore() ?? OUTSIDE);
    }

    /** Fill a typed array from the seed. */
    static #fill<Values extends ArrayBufferView | null>(random: Random, values: Values): Values {
        if (values !== null) {
            const bytes = new Uint8Array(values.buffer, values.byteOffset, values.byteLength);
            bytes.set(random.bytes(bytes.length));
        }

        return values;
    }

    /** Let pending work run until a whole macrotask turn passes without progress. */
    async #settle(): Promise<void> {
        let progress = this.#progress();
        for (;;) {
            // yield one macrotask turn
            await new Promise<void>((resolve) => {
                SYSTEM.setTimeout(resolve, 0);
            });

            // stop once the turn made no progress
            const next = this.#progress();
            if (next === progress) {
                return;
            }
            progress = next;
        }
    }

    /** Read the simulation's progress: waits begun, events traced and deliveries in flight. */
    #progress(): string {
        return `${this.clock.begun} ${this.trace.length} ${this.network.inFlight}`;
    }
}
