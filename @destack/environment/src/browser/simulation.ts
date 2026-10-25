import { present } from "@destack/schema";
import type { Display } from "./display/display.ts";
import type { NavigateOptions, Navigation, NavigationEntry } from "./navigation/navigation.ts";
import type { Broadcast, BroadcastPort } from "./broadcast/broadcast.ts";
import type { HeldLock, LockMode, LockOptions, LockManager } from "./lock/lock.ts";
import type { Storage } from "./storage/storage.ts";
import type { Environment } from "../environment/environment.ts";
import type { SimulatedClock } from "../clock/simulated.ts";
import type { Simulation } from "../simulation/simulation.ts";
import type { BrowserCapability } from "./capability.ts";

/** The URL every node's session history starts at unless the browser gives another. */
const URL_START = "https://app.test/";

/** The frames a simulated display shows each second. */
const FRAMES_PER_SECOND = 60;

/** A node's storage, kept in memory. */
export class SimulatedStorage implements Storage {
    /** The stored values by key. */
    readonly #values = new Map<string, string>();
    /** Whether the browser keeps the storage under pressure. */
    isPersisted = false;

    /** Read a key's value, null without one. */
    getItem(key: string): string | null {
        return this.#values.get(key) ?? null;
    }

    /** Write a key's value. */
    setItem(key: string, value: string): void {
        this.#values.set(key, value);
    }

    /** Persist the storage, as a browser granting it does. */
    async persist(): Promise<boolean> {
        this.isPersisted = true;

        return true;
    }
}

/** A node's hold of a lock, or its request waiting for one. */
interface LockHold {
    /** The node that asked. */
    readonly node: string;
    /** How it holds the lock. */
    readonly mode: LockMode;
    /** Grant the waiting request the lock. */
    readonly grant: () => void;
    /** Fail the waiting request. */
    readonly reject: (reason: unknown) => void;
}

/** One origin's locks, shared by every node of a browser simulation, granted in request order. */
export class SimulatedLocks {
    /** The holders of each lock by name. */
    readonly #held = new Map<string, LockHold[]>();
    /** The waiting requests of each lock by name, oldest first. */
    readonly #waiting = new Map<string, LockHold[]>();

    /** Read a node's view of the locks, its holds released once it crashes. */
    of(node: string): LockManager {
        return {
            request: (name, options, callback) => this.#request(node, name, options, callback),
        };
    }

    /** Release a crashed node's holds, and fail its waiting requests. */
    release(node: string): void {
        for (const name of new Set([...this.#held.keys(), ...this.#waiting.keys()])) {
            // drop the node's holds and requests
            const held = this.#held.get(name) ?? [];
            const waiting = this.#waiting.get(name) ?? [];
            this.#held.set(
                name,
                held.filter((hold) => hold.node !== node),
            );
            this.#waiting.set(
                name,
                waiting.filter((hold) => hold.node !== node),
            );

            // fail the node's requests, and grant the requests its holds blocked
            for (const hold of waiting.filter((each) => each.node === node)) {
                hold.reject(new Error(`node ${node} crashed`));
            }
            this.#grant(name);
        }
    }

    /** Take a named lock while a callback runs, answering the callback's result. */
    async #request<Value>(
        node: string,
        name: string,
        options: LockOptions,
        callback: (lock: HeldLock | null) => Promise<Value>,
    ): Promise<Value> {
        // refuse an aborted request, and run without the lock if asked and taken
        options.signal?.throwIfAborted();
        const mode = options.mode ?? "exclusive";
        const held = this.#held.get(name) ?? [];
        const waiting = this.#waiting.get(name) ?? [];
        const isFree = waiting.length === 0 && fits(held, mode);
        if (!isFree && options.ifAvailable === true) {
            return callback(null);
        }

        // join the line, leaving it once the signal aborts before the grant
        const { promise: granted, resolve, reject } = Promise.withResolvers<void>();
        const hold = { node, mode, grant: resolve, reject };
        const abort = () => {
            this.#waiting.set(
                name,
                (this.#waiting.get(name) ?? []).filter((each) => each !== hold),
            );
            reject(options.signal?.reason);
            this.#grant(name);
        };
        options.signal?.addEventListener("abort", abort, { once: true });
        this.#held.set(name, held);
        this.#waiting.set(name, [...waiting, hold]);
        this.#grant(name);
        await granted.finally(() => options.signal?.removeEventListener("abort", abort));

        // run the callback, then release the hold if a crash did not
        try {
            return await callback({ name, mode });
        } finally {
            this.#held.set(
                name,
                (this.#held.get(name) ?? []).filter((each) => each !== hold),
            );
            this.#grant(name);
        }
    }

    /** Grant the waiting requests that fit beside the holders, in request order. */
    #grant(name: string): void {
        const held = this.#held.get(name) ?? [];
        const waiting = this.#waiting.get(name) ?? [];
        while (waiting[0] !== undefined && fits(held, waiting[0].mode)) {
            const hold = waiting[0];
            waiting.shift();
            held.push(hold);
            hold.grant();
        }
    }
}

/** An open port of a simulated broadcast channel. */
interface SimulatedPort {
    /** The node that opened it. */
    readonly node: string;
    /** Its listeners. */
    readonly listeners: Set<(message: unknown) => void>;
    /** Whether it closed. */
    isClosed: boolean;
}

/** One origin's broadcast channels, shared by every node of a browser simulation. */
export class SimulatedBroadcast {
    /** The clock deliveries count on. */
    readonly #clock: SimulatedClock;
    /** The open ports of each channel by name. */
    readonly #ports = new Map<string, Set<SimulatedPort>>();
    /** What listeners threw, as a browser's reportError reports it, in order. */
    readonly errors: unknown[] = [];

    /** Deliver messages on a clock. */
    constructor(clock: SimulatedClock) {
        this.#clock = clock;
    }

    /** Read a node's view of the channels, its ports closed once it crashes. */
    of(node: string): Broadcast {
        return { open: (name) => this.#open(node, name) };
    }

    /** Close a crashed node's ports. */
    release(node: string): void {
        for (const ports of this.#ports.values()) {
            for (const port of ports) {
                if (port.node === node) {
                    port.isClosed = true;
                    ports.delete(port);
                }
            }
        }
    }

    /** Open a node's port of a named channel. */
    #open(node: string, name: string): BroadcastPort {
        // add the port to the channel's ports
        const ports = this.#ports.get(name) ?? new Set();
        this.#ports.set(name, ports);
        const port: SimulatedPort = { node, listeners: new Set(), isClosed: false };
        ports.add(port);

        return {
            send: (message) => {
                // refuse a closed port, as BroadcastChannel does
                if (port.isClosed) {
                    throw new DOMException("the broadcast port is closed", "InvalidStateError");
                }

                // deliver a copy after this turn to the other ports still open then
                const copy: unknown = structuredClone(message);
                const others = [...ports].filter((each) => each !== port);
                void this.#clock.track(
                    Promise.resolve().then(() => this.errors.push(...deliver(others, copy))),
                );
            },
            on: (listener) => {
                port.listeners.add(listener);

                return { [Symbol.dispose]: () => port.listeners.delete(listener) };
            },
            close: () => {
                port.isClosed = true;
                ports.delete(port);
            },
        };
    }
}

/** Report whether a request of a mode fits beside a lock's holders. */
function fits(held: readonly LockHold[], mode: LockMode): boolean {
    return held.length === 0 || (mode === "shared" && held.every((hold) => hold.mode === "shared"));
}

/** Deliver a message to each open port's listeners, answering what they threw. */
function deliver(ports: readonly SimulatedPort[], message: unknown): unknown[] {
    // call every listener, collecting what they throw
    const errors: unknown[] = [];
    for (const port of ports.filter((each) => !each.isClosed)) {
        for (const listener of port.listeners) {
            try {
                listener(message);
            } catch (error) {
                errors.push(error);
            }
        }
    }

    return errors;
}

/** A node's session history, kept in memory from a first URL. */
export class SimulatedNavigation implements Navigation {
    /** The entries' URLs, oldest first. */
    readonly #entries: string[];
    /** The position of the current entry. */
    #index = 0;
    /** The listeners of entry changes. */
    readonly #listeners = new Set<(entry: NavigationEntry) => void>();

    /** Start a history at a URL. */
    constructor(url: string) {
        this.#entries = [url];
    }

    /** The current entry. */
    get currentEntry(): NavigationEntry {
        return { url: present(this.#entries[this.#index], "the current history entry") };
    }

    /** Go to a URL, adding or replacing an entry. */
    navigate(url: string, options: NavigateOptions = {}): void {
        // resolve the URL against the current entry, then add or replace an entry
        const resolved = new URL(url, this.currentEntry.url).href;
        if (options.history === "replace") {
            this.#entries[this.#index] = resolved;
        } else {
            this.#index += 1;
            this.#entries.splice(this.#index, Infinity, resolved);
        }
        this.#emit();
    }

    /** Go back one entry, staying at the first. */
    back(): void {
        this.#traverse(-1);
    }

    /** Go forward one entry, staying at the last. */
    forward(): void {
        this.#traverse(1);
    }

    /** Call a listener on each change of the current entry until the subscription disposes. */
    on(listener: (entry: NavigationEntry) => void): Disposable {
        this.#listeners.add(listener);

        return { [Symbol.dispose]: () => this.#listeners.delete(listener) };
    }

    /** Move through the entries by a step, telling the listeners of a move. */
    #traverse(step: number): void {
        const index = this.#index + step;
        if (index >= 0 && index < this.#entries.length) {
            this.#index = index;
            this.#emit();
        }
    }

    /** Tell each listener the current entry. */
    #emit(): void {
        const entry = this.currentEntry;
        for (const listener of this.#listeners) {
            listener(entry);
        }
    }
}

/** A node's display: 60 Hz frames and idle periods on the simulated clock, and emulated media. */
export class SimulatedDisplay implements Display {
    /** The simulated time. */
    readonly #clock: SimulatedClock;
    /** Whether each media query matches, false for any other. */
    #media: Readonly<Record<string, boolean>>;
    /** The list of each query asked, which hears of its changes. */
    readonly #lists = new Map<string, SimulatedMediaQueryList>();

    /** Show frames on a clock, matching the given media queries. */
    constructor(clock: SimulatedClock, media: Readonly<Record<string, boolean>>) {
        this.#clock = clock;
        this.#media = media;
    }

    /** Wait for the next frame of the 60 Hz display, answering its time. */
    async frame(signal?: AbortSignal): Promise<number> {
        // wait until the next frame's millisecond
        const now = this.#clock.now();
        const next = Math.floor((now * FRAMES_PER_SECOND) / 1000) + 1;
        const at = Math.ceil((next * 1000) / FRAMES_PER_SECOND);
        await this.#clock.sleep(at - now, signal);

        return this.#clock.now();
    }

    /** Wait for an idle period, which comes once other work at this time settles. */
    async idle(_timeout?: number, signal?: AbortSignal): Promise<void> {
        await this.#clock.sleep(0, signal);
    }

    /** Follow a media query, matching as the emulated media set it. */
    media(query: string): MediaQueryList {
        // answer the query's one list, which hears of its changes
        const known = this.#lists.get(query);
        if (known !== undefined) {
            return known;
        }
        const list = new SimulatedMediaQueryList(query, this.#media[query] ?? false);
        this.#lists.set(query, list);

        return list;
    }

    /** Change whether media queries match, as Playwright's `emulateMedia` does. */
    emulate(media: Readonly<Record<string, boolean>>): void {
        this.#media = { ...this.#media, ...media };
        for (const [query, matches] of Object.entries(media)) {
            this.#lists.get(query)?.change(matches);
        }
    }
}

/** A media query whose result the simulated display sets. */
class SimulatedMediaQueryList extends EventTarget implements MediaQueryList {
    /** The query. */
    readonly media: string;
    /** The change handler. */
    onchange: ((this: MediaQueryList, event: MediaQueryListEvent) => unknown) | null = null;
    /** Whether the query matches now. */
    #matches: boolean;
    /** The event listener each deprecated listener added through. */
    readonly #listeners = new WeakMap<(event: MediaQueryListEvent) => void, EventListener>();

    /** Answer a query with its current result. */
    constructor(media: string, matches: boolean) {
        super();
        this.media = media;
        this.#matches = matches;
    }

    /** Whether the query matches. */
    get matches(): boolean {
        return this.#matches;
    }

    /** Take changes the deprecated way. */
    addListener(listener: ((event: MediaQueryListEvent) => void) | null): void {
        // add the listener once, through a listener of change events
        if (listener !== null && !this.#listeners.has(listener)) {
            const added = (event: Event) => {
                if (event instanceof SimulatedMediaQueryListEvent) {
                    listener(event);
                }
            };
            this.#listeners.set(listener, added);
            this.addEventListener("change", added);
        }
    }

    /** Stop taking changes the deprecated way. */
    removeListener(listener: ((event: MediaQueryListEvent) => void) | null): void {
        const added = listener === null ? undefined : this.#listeners.get(listener);
        if (listener !== null && added !== undefined) {
            this.#listeners.delete(listener);
            this.removeEventListener("change", added);
        }
    }

    /** Set whether the query matches, telling the handler and the listeners when it changes. */
    change(matches: boolean): void {
        // tell of a result that differs
        if (matches !== this.#matches) {
            this.#matches = matches;
            const event = new SimulatedMediaQueryListEvent(this.media, matches);
            this.onchange?.(event);
            this.dispatchEvent(event);
        }
    }
}

/** The event a simulated media query fires once its result changes. */
class SimulatedMediaQueryListEvent extends Event implements MediaQueryListEvent {
    /** The query. */
    readonly media: string;
    /** Whether the query matches now. */
    readonly matches: boolean;

    /** Tell of a query's new result. */
    constructor(media: string, matches: boolean) {
        super("change");
        this.media = media;
        this.matches = matches;
    }
}

/** A node's browser environment: a browser's effects, each the simulated one a test controls. */
export type SimulatedBrowserEnvironment = Omit<Environment<"workers">, BrowserCapability> & {
    /** The node's storage. */
    readonly storage: SimulatedStorage;
    /** The origin's locks, shared by every node. */
    readonly locks: LockManager;
    /** The origin's broadcast channels, shared by every node. */
    readonly broadcast: Broadcast;
    /** The node's session history. */
    readonly navigation: SimulatedNavigation;
    /** The node's display. */
    readonly display: SimulatedDisplay;
};

/** What a browser simulation starts each node with. */
export interface BrowserSimulationOptions {
    /** The URL each node's session history starts at. */
    readonly url?: string;
    /** Whether each media query matches on each node's display, false for any other. */
    readonly media?: Readonly<Record<string, boolean>>;
}

/** The browser half of a simulation: each node's environment with a browser's capabilities. */
export class BrowserSimulation {
    /** The simulation the nodes run in. */
    readonly #simulation: Simulation;
    /** What each node's browser starts with. */
    readonly #options: BrowserSimulationOptions;
    /** Each node's browser environment, made once. */
    readonly #environments = new Map<string, SimulatedBrowserEnvironment>();
    /** The origin's locks every node shares. */
    readonly locks = new SimulatedLocks();
    /** The origin's broadcast channels every node shares, with what their listeners threw. */
    readonly broadcast: SimulatedBroadcast;

    /** Give a simulation's nodes browsers that start as the options say. */
    constructor(simulation: Simulation, options: BrowserSimulationOptions = {}) {
        // store the simulation and its options, and open the shared channels
        this.#simulation = simulation;
        this.#options = options;
        this.broadcast = new SimulatedBroadcast(simulation.clock);

        // release a crashed node's locks and ports
        simulation.onCrash((node) => {
            this.locks.release(node);
            this.broadcast.release(node);
        });
    }

    /** Read a node's environment with its browser, the same one on each call. */
    environment(node: string): SimulatedBrowserEnvironment {
        // answer the node's browser made before
        const known = this.#environments.get(node);
        if (known !== undefined) {
            return known;
        }

        // give the node a browser's base and threads, and a browser on the node's clock
        const { files, process, server, host, ...base } = this.#simulation.environment(node);
        const { clock } = this.#simulation;
        const options = this.#options;
        const environment = {
            ...base,
            storage: new SimulatedStorage(),
            locks: this.locks.of(node),
            broadcast: this.broadcast.of(node),
            navigation: new SimulatedNavigation(options.url ?? URL_START),
            display: new SimulatedDisplay(clock, options.media ?? {}),
        };
        this.#environments.set(node, environment);

        return environment;
    }
}
