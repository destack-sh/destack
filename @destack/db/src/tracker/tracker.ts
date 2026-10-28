import { type JsonValue, present, schema, Uuid } from "@destack/schema";
import type { Environment } from "@destack/environment";
import { type Channel, typedChannel } from "../channel/index.ts";

/** The heartbeat interval in milliseconds: three missed beats bound a crash to 45 s. */
const HEARTBEAT_MILLISECONDS = 15_000;

/** The missed heartbeats before the others count an instance stopped. */
const MISSED_HEARTBEATS = 3;

/** A message between the instances sharing a database. */
export const TrackerMessage = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("track"),
        instance: schema.string(),
        owner: schema.string(),
    }),
    schema.object({
        kind: schema.literal("release"),
        instance: schema.string(),
        owner: schema.string(),
    }),
    schema.object({
        kind: schema.literal("broadcast"),
        instance: schema.string(),
        topic: schema.string(),
        event: schema.json(),
    }),
    schema.object({
        kind: schema.literal("mark"),
        instance: schema.string(),
        marks: schema.record(schema.string(), schema.json()),
    }),
    schema.object({
        kind: schema.literal("unmark"),
        instance: schema.string(),
        owner: schema.string(),
    }),
    schema.object({ kind: schema.literal("hello"), instance: schema.string() }),
    schema.object({
        kind: schema.literal("state"),
        instance: schema.string(),
        tracked: schema.array(schema.string()),
        marks: schema.record(schema.string(), schema.json()),
    }),
    schema.object({
        kind: schema.literal("heartbeat"),
        instance: schema.string(),
        marks: schema.record(schema.string(), schema.json()),
    }),
    schema.object({ kind: schema.literal("stop"), instance: schema.string() }),
]);
/** A message between the instances sharing a database. */
export type TrackerMessage = schema.Infer<typeof TrackerMessage>;

/** An event on a topic, with the instance that sent it. */
export interface TrackerEvent {
    /** The sending instance. */
    readonly instance: string;
    /** The event. */
    readonly event: JsonValue;
}

/** How a tracker announces itself. */
export interface TrackerOptions {
    /** This instance's identifier. */
    readonly instance?: string;
    /** The heartbeat interval, in milliseconds. */
    readonly heartbeat?: number;
}

/** The live instances sharing a database, the owners and marks of each, after Phoenix.Tracker. */
export class Tracker {
    /** This instance's identifier. */
    readonly instance: string;
    /** The environment whose clock times the heartbeats. */
    readonly #environment: Pick<Environment, "clock" | "random">;
    /** The channel to the other instances, absent for a sole writer. */
    readonly #channel: Channel<TrackerMessage> | undefined;
    /** The last heartbeat of each other instance, in UTC epoch milliseconds. */
    readonly #instances = new Map<string, number>();
    /** The instances tracking each owner. */
    readonly #tracked = new Map<string, Set<string>>();
    /** The times this instance tracks each owner it tracks, released on the last release. */
    readonly #holds = new Map<string, number>();
    /** The listeners of owner tracking changes. */
    readonly #watchers = new Set<(owner: string, isTracked: boolean) => void>();
    /** The value each instance marked each owner with, by owner and instance. */
    readonly #marks = new Map<string, Map<string, JsonValue>>();
    /** This instance's marks the next heartbeat announces, by owner. */
    readonly #pending = new Map<string, JsonValue>();
    /** The listeners of mark changes. */
    readonly #marked = new Set<(owner: string) => void>();
    /** The listeners of instances found stopped. */
    readonly #stopped = new Set<(instance: string) => void>();
    /** The listeners of each topic. */
    readonly #listeners = new Map<string, Set<(event: TrackerEvent) => void>>();
    /** The heartbeat interval, in milliseconds. */
    readonly #heartbeat: number;
    /** When this instance started listening, in UTC epoch milliseconds. */
    readonly #startedAt: number;
    /** Stops the heartbeat loop. */
    readonly #closing = new AbortController();
    /** Stop listening to the channel. */
    readonly #stop: () => void;

    /** Track the instances on a channel, or this one alone without one. */
    constructor(
        environment: Pick<Environment, "clock" | "random">,
        channel: Channel<unknown> | undefined,
        options: TrackerOptions = {},
    ) {
        // announce this instance on the channel
        this.instance = options.instance ?? Uuid.v4(environment.random);
        this.#environment = environment;
        this.#heartbeat = options.heartbeat ?? HEARTBEAT_MILLISECONDS;
        this.#startedAt = environment.clock.now();
        this.#channel = channel === undefined ? undefined : typedChannel(channel, TrackerMessage);

        // receive messages, asking for the tracked owners on each start or resume
        this.#stop =
            this.#channel?.listen({
                receive: (message) => this.#receive(message),
                resume: () => this.#notify({ kind: "hello", instance: this.instance }),
            }) ?? (() => {});

        // announce this instance and count silent ones stopped each heartbeat
        if (this.#channel !== undefined) {
            void this.#beat();
        }
    }

    /** Track an owner on this instance once more, announcing only its first hold. */
    track(owner: string): void {
        // count the hold, announcing the first
        const holds = this.#holds.get(owner) ?? 0;
        this.#holds.set(owner, holds + 1);
        if (holds === 0) {
            this.#track(owner, this.instance);
            this.#notify({ kind: "track", instance: this.instance, owner });
        }
    }

    /** Release one hold of an owner this instance tracked, announcing only its last. */
    release(owner: string): void {
        // count the release down
        const holds = present(this.#holds.get(owner), `owner ${owner} is not tracked here`);
        if (holds > 1) {
            this.#holds.set(owner, holds - 1);
        }
        // release the owner once its last hold ends
        else {
            this.#holds.delete(owner);
            this.#release(owner, this.instance);
            this.#notify({ kind: "release", instance: this.instance, owner });
        }
    }

    /** Mark an owner with this instance's new value, announced to the others at once. */
    mark(owner: string, value: JsonValue): void {
        this.#setMark(owner, this.instance, value);
        this.#pending.delete(owner);
        this.#notify({ kind: "mark", instance: this.instance, marks: { [owner]: value } });
    }

    /** Refresh this instance's mark of an owner, announced with the next heartbeat. */
    refresh(owner: string, value: JsonValue): void {
        this.#setMark(owner, this.instance, value);
        this.#pending.set(owner, value);
    }

    /** Drop this instance's mark of an owner, announced to the others at once. */
    unmark(owner: string): void {
        this.#unmark(owner, this.instance);
        this.#pending.delete(owner);
        this.#notify({ kind: "unmark", instance: this.instance, owner });
    }

    /** Read the values the live instances marked an owner with. */
    marks(owner: string): JsonValue[] {
        return [...(this.#marks.get(owner)?.values() ?? [])];
    }

    /** Tell a listener each owner whose marks change, until stopped. */
    watchMarked(listener: (owner: string) => void): () => void {
        this.#marked.add(listener);

        return () => this.#marked.delete(listener);
    }

    /** Decide whether an instance is live: this one, a recent sender, or any while starting. */
    isLive(instance: string): boolean {
        const isListening =
            this.#environment.clock.now() - this.#startedAt < this.#heartbeat * MISSED_HEARTBEATS;

        return instance === this.instance || this.#instances.has(instance) || isListening;
    }

    /** Decide whether any instance tracks an owner. */
    isTracked(owner: string): boolean {
        return this.#tracked.has(owner);
    }

    /** Tell a listener when an owner becomes tracked or released, until stopped. */
    watchTracked(listener: (owner: string, isTracked: boolean) => void): () => void {
        this.#watchers.add(listener);

        return () => this.#watchers.delete(listener);
    }

    /** Tell a listener each instance found stopped, until stopped. */
    watchStopped(listener: (instance: string) => void): () => void {
        this.#stopped.add(listener);

        return () => this.#stopped.delete(listener);
    }

    /** Send an event to a topic on every instance, this one included. */
    broadcast(topic: string, event: JsonValue): void {
        this.#deliver(topic, { instance: this.instance, event });
        this.#notify({ kind: "broadcast", instance: this.instance, topic, event });
    }

    /** Listen to a topic until stopped. */
    listen(topic: string, receive: (event: TrackerEvent) => void): () => void {
        const listeners = this.#listeners.get(topic) ?? new Set();
        this.#listeners.set(topic, listeners.add(receive));

        return () => {
            listeners.delete(receive);
            if (listeners.size === 0) {
                this.#listeners.delete(topic);
            }
        };
    }

    /** Stop tracking and tell the others this instance stopped. */
    close(): void {
        this.#closing.abort();
        this.#notify({ kind: "stop", instance: this.instance });
        this.#stop();
    }

    /** Announce this instance and stop silent ones at each heartbeat until closed. */
    async #beat(): Promise<void> {
        const { clock } = this.#environment;
        const { signal } = this.#closing;
        while (
            await clock.sleep(this.#heartbeat, signal).then(
                () => true,
                () => false,
            )
        ) {
            // announce this instance and its pending marks
            const marks = Object.fromEntries(this.#pending);
            this.#pending.clear();
            this.#notify({ kind: "heartbeat", instance: this.instance, marks });

            // stop the instances silent for too many heartbeats
            const deadline = clock.now() - this.#heartbeat * MISSED_HEARTBEATS;
            for (const [instance, seen] of this.#instances) {
                if (seen < deadline) {
                    this.#drop(instance);
                }
            }
        }
    }

    /** Apply another instance's message. */
    #receive(message: TrackerMessage): void {
        // skip the messages of this instance a channel echoes back
        if (message.instance === this.instance) {
            return;
        }

        // note the sender as live
        if (message.kind !== "stop") {
            this.#instances.set(message.instance, this.#environment.clock.now());
        }

        // apply the tracking changes, marks and broadcasts
        if (message.kind === "mark" || message.kind === "heartbeat") {
            this.#setMarks(message.instance, message.marks);
        } else if (message.kind === "unmark") {
            this.#unmark(message.owner, message.instance);
        } else if (message.kind === "track") {
            this.#track(message.owner, message.instance);
        } else if (message.kind === "release") {
            this.#release(message.owner, message.instance);
        } else if (message.kind === "broadcast") {
            this.#deliver(message.topic, { instance: message.instance, event: message.event });
        } else if (message.kind === "state") {
            for (const owner of message.tracked) {
                this.#track(owner, message.instance);
            }
            this.#setMarks(message.instance, message.marks);
        }
        // answer a hello with this instance's tracked owners and marks
        else if (message.kind === "hello") {
            const marks = [...this.#marks].flatMap(([owner, values]) => {
                const value = values.get(this.instance);

                return value === undefined ? [] : [[owner, value] as const];
            });
            this.#notify({
                kind: "state",
                instance: this.instance,
                tracked: [...this.#holds.keys()],
                marks: Object.fromEntries(marks),
            });
        }
        // count a stopping instance stopped
        else if (message.kind === "stop") {
            this.#drop(message.instance);
        }
    }

    /** Count an instance stopped: forget it, its marks, and every owner it tracked. */
    #drop(instance: string): void {
        // forget the instance and release the owners it tracked
        this.#instances.delete(instance);
        for (const owner of this.#tracked.keys()) {
            this.#release(owner, instance);
        }

        // forget its marks, telling the listeners
        for (const owner of this.#marks.keys()) {
            this.#unmark(owner, instance);
        }

        // tell the listeners the instance stopped
        for (const listener of this.#stopped) {
            listener(instance);
        }
    }

    /** Note an instance tracking an owner. */
    #track(owner: string, instance: string): void {
        // tell the listeners on the first tracking instance
        const instances = this.#tracked.get(owner) ?? new Set<string>();
        const isFirst = instances.size === 0;
        this.#tracked.set(owner, instances.add(instance));
        if (isFirst) {
            for (const listener of this.#watchers) {
                listener(owner, true);
            }
        }
    }

    /** Note an instance releasing an owner. */
    #release(owner: string, instance: string): void {
        // tell the listeners on the last releasing instance
        const instances = this.#tracked.get(owner);
        if (instances === undefined || !instances.delete(instance) || instances.size > 0) {
            return;
        }
        this.#tracked.delete(owner);
        for (const listener of this.#watchers) {
            listener(owner, false);
        }
    }

    /** Store an instance's marks, telling the listeners. */
    #setMarks(instance: string, marks: Readonly<Record<string, JsonValue>>): void {
        for (const [owner, value] of Object.entries(marks)) {
            this.#setMark(owner, instance, value);
        }
    }

    /** Store an instance's mark of an owner, telling the listeners. */
    #setMark(owner: string, instance: string, value: JsonValue): void {
        const values = this.#marks.get(owner) ?? new Map<string, JsonValue>();
        this.#marks.set(owner, values.set(instance, value));
        this.#announceMarked(owner);
    }

    /** Forget an instance's mark of an owner, telling the listeners. */
    #unmark(owner: string, instance: string): void {
        const values = this.#marks.get(owner);
        if (values?.delete(instance) === true) {
            if (values.size === 0) {
                this.#marks.delete(owner);
            }
            this.#announceMarked(owner);
        }
    }

    /** Tell the listeners an owner's marks changed. */
    #announceMarked(owner: string): void {
        for (const listener of this.#marked) {
            listener(owner);
        }
    }

    /** Hand an event to a topic's listeners on this instance. */
    #deliver(topic: string, event: TrackerEvent): void {
        for (const listener of this.#listeners.get(topic) ?? []) {
            listener(event);
        }
    }

    /** Send a message to the other instances, when any share the database. */
    #notify(message: TrackerMessage): void {
        this.#channel?.notify(message);
    }
}
