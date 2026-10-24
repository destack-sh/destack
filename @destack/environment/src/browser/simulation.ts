import { present } from "@destack/schema";
import type { Capture } from "./capture/capture.ts";
import type { Clipboard } from "./clipboard/clipboard.ts";
import type { Connectivity } from "./connectivity/connectivity.ts";
import type { Display } from "./display/display.ts";
import type { Geolocation } from "./geolocation/geolocation.ts";
import type { NavigateOptions, Navigation, NavigationEntry } from "./navigation/navigation.ts";
import type { Permissions } from "./permission/permission.ts";
import type { Share } from "./share/share.ts";
import type { Broadcast, BroadcastPort } from "./broadcast/broadcast.ts";
import type { HeldLock, LockMode, LockOptions, LockManager } from "./lock/lock.ts";
import type { Storage } from "./storage/storage.ts";
import type { Vibration } from "./vibration/vibration.ts";
import type { Environment } from "../environment/environment.ts";
import type { SimulatedClock } from "../clock/simulated.ts";
import type { SimulatedNetwork } from "../network/simulated.ts";
import type { Simulation } from "../simulation/simulation.ts";
import type { BrowserCapability } from "./capability.ts";

/** The URL every node's session history starts at unless the browser gives another. */
const URL_START = "https://app.test/";

/** The frames a simulated display shows each second. */
const FRAMES_PER_SECOND = 60;

/** The Geolocation API's code for a position the device cannot report. */
const POSITION_UNAVAILABLE = 2;

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

/** A node's clipboard, keeping what was last written. */
export class SimulatedClipboard implements Clipboard {
    /** What was last written: items, text, or nothing yet. */
    written: { readonly items: ClipboardItem[] } | { readonly text: string } | undefined;

    /** Read the clipboard's items, refusing written text, which a simulation holds as no item. */
    async read(): Promise<ClipboardItem[]> {
        // answer the written items, none before a write
        const { written } = this;
        if (written !== undefined && "text" in written) {
            throw new DOMException(
                "a simulated clipboard reads written text as no item",
                "NotSupportedError",
            );
        }

        return written?.items ?? [];
    }

    /** Write items to the clipboard. */
    async write(items: ClipboardItem[]): Promise<void> {
        this.written = { items };
    }

    /** Write text to the clipboard. */
    async writeText(text: string): Promise<void> {
        this.written = { text };
    }
}

/** A node's display: frames at 60 Hz and idle periods on the simulated clock, and fixed media. */
export class SimulatedDisplay implements Display {
    /** The simulated time. */
    readonly #clock: SimulatedClock;
    /** Whether each media query matches, false for any other. */
    readonly #media: Readonly<Record<string, boolean>>;

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

    /** Follow a media query, matching as the options set it. */
    media(query: string): MediaQueryList {
        return new SimulatedMediaQueryList(query, this.#media[query] ?? false);
    }
}

/** A media query of fixed result. */
class SimulatedMediaQueryList extends EventTarget implements MediaQueryList {
    /** The query. */
    readonly media: string;
    /** Whether the query matches. */
    readonly matches: boolean;
    /** The change handler, never called as the result is fixed. */
    onchange: ((this: MediaQueryList, event: MediaQueryListEvent) => unknown) | null = null;

    /** Fix a query's result. */
    constructor(media: string, matches: boolean) {
        super();
        this.media = media;
        this.matches = matches;
    }

    /** Take changes the deprecated way, which never come. */
    addListener(): void {}

    /** Stop taking changes the deprecated way. */
    removeListener(): void {}
}

/** A node's connectivity, online until a test sets it otherwise. */
export class SimulatedConnectivity implements Connectivity {
    /** The listeners of changes. */
    readonly #listeners = new Set<(isOnline: boolean) => void>();
    /** The network the node is cut off from while offline, which owns whether it is. */
    readonly #network: SimulatedNetwork;
    /** The node. */
    readonly #node: string;

    /** Start a node online or offline on a network, following its isolation there. */
    constructor(isOnline: boolean, network: SimulatedNetwork, node: string) {
        // follow the node's isolation on the network, however it changes
        this.#network = network;
        this.#node = node;
        network.onIsolation((isolated, isIsolated) => {
            if (isolated === node) {
                for (const listener of this.#listeners) {
                    listener(!isIsolated);
                }
            }
        });

        // cut a node starting offline off the network
        if (!isOnline) {
            network.isolate(node);
        }
    }

    /** Report whether the node is online. */
    isOnline(): boolean {
        return !this.#network.isIsolated(this.#node);
    }

    /** Follow each change of whether the node is online, until disposed. */
    on(listener: (isOnline: boolean) => void): Disposable {
        this.#listeners.add(listener);

        return { [Symbol.dispose]: () => this.#listeners.delete(listener) };
    }

    /** Go online or offline, joining or leaving the network. */
    set(isOnline: boolean): void {
        if (isOnline) {
            this.#network.rejoin(this.#node);
        } else {
            this.#network.isolate(this.#node);
        }
    }
}

/** A node's media devices: none, so every capture finds no device. */
export class SimulatedCapture implements Capture {
    /** List no devices. */
    async enumerateDevices(): Promise<MediaDeviceInfo[]> {
        return [];
    }

    /** Refuse a camera or microphone capture, as a browser without devices does. */
    async getUserMedia(_constraints?: MediaStreamConstraints): Promise<MediaStream> {
        throw new DOMException("the simulation has no media devices", "NotFoundError");
    }

    /** Refuse a screen capture, as a browser without a display to share does. */
    async getDisplayMedia(_options?: DisplayMediaStreamOptions): Promise<MediaStream> {
        throw new DOMException("the simulation has no display to capture", "NotFoundError");
    }

    /** Follow device changes, which never come. */
    on(): Disposable {
        return { [Symbol.dispose]: () => undefined };
    }
}

/** A position a simulated node reports, in degrees and meters. */
export interface SimulatedPosition {
    /** The latitude in degrees. */
    readonly latitude: number;
    /** The longitude in degrees. */
    readonly longitude: number;
    /** The accuracy radius in meters. */
    readonly accuracy: number;
}

/** A node's geolocation: a fixed position stamped on the simulated clock, or none. */
export class SimulatedGeolocation implements Geolocation {
    /** The simulated time. */
    readonly #clock: SimulatedClock;
    /** The node's position, absent for a node that cannot locate itself. */
    readonly #position: SimulatedPosition | undefined;

    /** Report a position, or none, at the clock's time. */
    constructor(clock: SimulatedClock, position: SimulatedPosition | undefined) {
        this.#clock = clock;
        this.#position = position;
    }

    /** Read the position, refusing as unavailable without one. */
    async getCurrentPosition(): Promise<GeolocationPosition> {
        const position = this.#read();
        if (position === undefined) {
            throw new SimulatedPositionError();
        }

        return position;
    }

    /** Report the position or its error once, until disposed. */
    watchPosition(
        listener: (position: GeolocationPosition) => void,
        error: (error: GeolocationPositionError) => void,
    ): Disposable {
        // report on the next turn, as a browser's first watch callback does
        let isWatching = true;
        void this.#clock.sleep(0).then(() => {
            const position = this.#read();
            if (!isWatching) {
                return;
            } else if (position === undefined) {
                error(new SimulatedPositionError());
            } else {
                listener(position);
            }
        });

        return { [Symbol.dispose]: () => (isWatching = false) };
    }

    /** Read the position at the clock's time, absent for a node without one. */
    #read(): GeolocationPosition | undefined {
        // read no position for a node without one
        const position = this.#position;
        if (position === undefined) {
            return undefined;
        }

        // stamp the fixed coordinates with the clock's time
        const coords: GeolocationCoordinates = {
            ...position,
            altitude: null,
            altitudeAccuracy: null,
            heading: null,
            speed: null,
            toJSON: () => ({ ...position }),
        };
        const timestamp = this.#clock.now();

        return { coords, timestamp, toJSON: () => ({ coords: { ...position }, timestamp }) };
    }
}

/** A node's share sheet, keeping what was shared. */
export class SimulatedShare implements Share {
    /** The data shared so far, oldest first. */
    readonly shared: ShareData[] = [];

    /** Report that any data can be shared. */
    canShare(): boolean {
        return true;
    }

    /** Keep the shared data. */
    async share(data: ShareData): Promise<void> {
        this.shared.push(data);
    }
}

/** A node's vibration motor, keeping each pattern. */
export class SimulatedVibration implements Vibration {
    /** The patterns vibrated so far, oldest first. */
    readonly patterns: VibratePattern[] = [];

    /** Keep the pattern and report that it started. */
    vibrate(pattern: VibratePattern): boolean {
        this.patterns.push(pattern);

        return true;
    }
}

/** A permission's status in a simulation, which a test changes. */
export class SimulatedPermissionStatus extends EventTarget implements PermissionStatus {
    /** The permission's name. */
    readonly name: string;
    /** The permission's state. */
    state: PermissionState;
    /** The listener of changes set as a property. */
    onchange: ((this: PermissionStatus, event: Event) => unknown) | null = null;

    /** Start a permission in a state. */
    constructor(name: string, state: PermissionState) {
        super();
        this.name = name;
        this.state = state;
    }

    /** Change the state, telling the listeners. */
    set(state: PermissionState): void {
        // change the state, then tell the property listener and the added ones
        this.state = state;
        const event = new Event("change");
        this.onchange?.(event);
        this.dispatchEvent(event);
    }
}

/** A node's permissions: each in its given state, otherwise prompting. */
export class SimulatedPermissions implements Permissions {
    /** Each permission's status, created on first query. */
    readonly #statuses = new Map<string, SimulatedPermissionStatus>();
    /** Each permission's starting state, prompting for any other. */
    readonly #states: Readonly<Record<string, PermissionState>>;

    /** Start each permission in its given state. */
    constructor(states: Readonly<Record<string, PermissionState>>) {
        this.#states = states;
    }

    /** Read a permission's status, the same one on each query. */
    async query(descriptor: PermissionDescriptor): Promise<SimulatedPermissionStatus> {
        // answer the status of an earlier query
        const { name } = descriptor;
        const known = this.#statuses.get(name);
        if (known !== undefined) {
            return known;
        }

        // start the status in its given state
        const status = new SimulatedPermissionStatus(name, this.#states[name] ?? "prompt");
        this.#statuses.set(name, status);

        return status;
    }
}

/** The error of a position a simulated node cannot report. */
export class SimulatedPositionError extends Error implements GeolocationPositionError {
    /** The Geolocation API's code for an unavailable position. */
    readonly code = POSITION_UNAVAILABLE;
    /** The code of a denied permission. */
    readonly PERMISSION_DENIED = 1;
    /** The code of an unavailable position. */
    readonly POSITION_UNAVAILABLE = POSITION_UNAVAILABLE;
    /** The code of a timeout. */
    readonly TIMEOUT = 3;

    /** Describe the missing position. */
    constructor() {
        super("the simulation has no position");
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
    /** The node's clipboard. */
    readonly clipboard: SimulatedClipboard;
    /** The node's display. */
    readonly display: SimulatedDisplay;
    /** The node's connectivity. */
    readonly connectivity: SimulatedConnectivity;
    /** The node's media devices. */
    readonly capture: SimulatedCapture;
    /** The node's position. */
    readonly geolocation: SimulatedGeolocation;
    /** The node's share sheet. */
    readonly share: SimulatedShare;
    /** The node's vibration motor. */
    readonly vibration: SimulatedVibration;
    /** The node's permissions. */
    readonly permissions: SimulatedPermissions;
};

/** What a browser simulation starts each node with. */
export interface BrowserSimulationOptions {
    /** The URL each node's session history starts at. */
    readonly url?: string;
    /** Whether each media query matches on each node's display, false for any other. */
    readonly media?: Readonly<Record<string, boolean>>;
    /** Whether each node starts online, true by default. */
    readonly isOnline?: boolean;
    /** The position each node reports, none by default. */
    readonly position?: SimulatedPosition;
    /** Each permission's starting state on each node, prompting for any other. */
    readonly permissions?: Readonly<Record<string, PermissionState>>;
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
        const { clock, network } = this.#simulation;
        const options = this.#options;
        const environment = {
            ...base,
            storage: new SimulatedStorage(),
            locks: this.locks.of(node),
            broadcast: this.broadcast.of(node),
            navigation: new SimulatedNavigation(options.url ?? URL_START),
            clipboard: new SimulatedClipboard(),
            display: new SimulatedDisplay(clock, options.media ?? {}),
            connectivity: new SimulatedConnectivity(options.isOnline ?? true, network, node),
            capture: new SimulatedCapture(),
            geolocation: new SimulatedGeolocation(clock, options.position),
            share: new SimulatedShare(),
            vibration: new SimulatedVibration(),
            permissions: new SimulatedPermissions(options.permissions ?? {}),
        };
        this.#environments.set(node, environment);

        return environment;
    }
}
