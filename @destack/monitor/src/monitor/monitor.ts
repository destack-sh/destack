import { until } from "@destack/service/timer";
import {
    and,
    asc,
    desc,
    eq,
    gte,
    inArray,
    isNull,
    lt,
    lte,
    sql,
    type DatabaseConnection,
    type Select,
    type SQL,
    type Channel,
    typedChannel,
} from "@destack/db";
import type { Bucket } from "@destack/bucket";
import { schema, type Identifier } from "@destack/schema";
import { v7 } from "uuid";
import type { AsyncBuffer } from "hyparquet";
import type { OtlpSignal } from "@destack/telemetry/otlp";
import {
    Entry,
    EntryFilter,
    type EntryPage,
    EntrySearch,
    Otlp,
    OtlpLogsRequest,
    OtlpMetricsRequest,
    OtlpTracesRequest,
    type PointSeries,
    type Series,
} from "../entry/index.ts";
import { aggregate } from "./series.ts";
import { SettingValue } from "@destack/setting/object";
import { POINT_BIT, Segment, UNSPECIFIED_BIT } from "../segment/segment.ts";
import { telemetryRetention, traceSampling } from "../setting/setting.ts";
import type { Setting } from "@destack/setting";

import { monitorSegment } from "../segment/table.ts";

/** How long a segment stays open: a minute, the delay before sealed entries reach the bucket. */
const SEAL_MILLISECONDS = 60_000;

/** The most entries an open segment keeps: ~75 MB in memory at ~300 bytes per entry. */
const SEGMENT_ROWS = 250_000;

/** An hour, the window compaction merges minute segments over, in microseconds. */
const HOUR_MICROSECONDS = 60 * 60 * 1_000_000;

/** A day, the unit of retention, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60 * 1000;

/** How long an unnamed file stays before a sweep deletes it: ten minutes, longer than any search reads. */
const SWEEP_GRACE_MILLISECONDS = 10 * 60 * 1000;

/** The most segment files one query reads at once: ~8 streams of R2's 50-100 MB/s each. */
const READ_CONCURRENCY = 8;

/** The longest trace span, beyond which the lookup cuts traces. */
const TRACE_MICROSECONDS = 60 * 60 * 1_000_000;

/** How often an instance announces itself and its tails to the others: ten seconds. */
const HEARTBEAT_MILLISECONDS = 10_000;

/** How long an instance counts as alive after its last announcement: three missed heartbeats. */
const PEER_MILLISECONDS = 3 * HEARTBEAT_MILLISECONDS;

/** How long a query waits for the other instances' open entries: a second, far above a channel round trip. */
const ANSWER_MILLISECONDS = 1_000;

/** How long entries for another instance's tail gather before they are sent together: a tenth of a second. */
const FORWARD_MILLISECONDS = 100;

/** A query every instance answers from its open segments: a search, a trace or a series. */
const EntryQuery = schema.discriminatedUnion("kind", [
    schema.object({ kind: schema.literal("search"), search: EntrySearch }),
    schema.object({ kind: schema.literal("trace"), trace: schema.string() }),
    schema.object({ kind: schema.literal("series"), name: schema.string() }),
]);
/** A query over entries every instance answers from its open segments. */
type EntryQuery = schema.Infer<typeof EntryQuery>;

/** A live tail of entries, which an instance announces so the others forward the entries it follows. */
const Tail = schema.object({
    /** The tail's identifier. */
    id: schema.string(),
    /** The entries followed, in one scope. */
    filter: EntryFilter,
});
/** A live tail of entries. */
type Tail = schema.Infer<typeof Tail>;

/** A message between the monitors of one database's instances. */
const MonitorMessage = schema.discriminatedUnion("kind", [
    schema.object({
        /** An instance announcing itself and the tails it serves, on start and every heartbeat. */
        kind: schema.literal("alive"),
        instance: schema.string(),
        tails: schema.array(Tail),
    }),
    schema.object({
        /** An instance stopping. */
        kind: schema.literal("gone"),
        instance: schema.string(),
    }),
    schema.object({
        /** Entries for a tail another instance serves. */
        kind: schema.literal("entries"),
        tail: schema.string(),
        entries: schema.array(Entry),
    }),
    schema.object({
        /** A query for every instance's open entries. */
        kind: schema.literal("ask"),
        request: schema.string(),
        instance: schema.string(),
        scope: schema.string(),
        installation: schema.identifier("installation").exactOptional(),
        window: schema.object({ from: schema.number(), to: schema.number() }),
        query: EntryQuery,
    }),
    schema.object({
        /** One instance's open entries for a query. */
        kind: schema.literal("answer"),
        request: schema.string(),
        instance: schema.string(),
        entries: schema.array(Entry),
    }),
]);
/** A message between the monitors of one database's instances. */
type MonitorMessage = schema.Infer<typeof MonitorMessage>;

/** A tail this instance serves, with where its entries go. */
interface Subscriber extends Tail {
    /** Receive a matching entry. */
    readonly push: (entry: Entry) => void;
}

/** A stream of entries its reader can end at once. */
export type EntryStream = AsyncIteratorObject<Entry, undefined, void> & {
    return(): Promise<IteratorResult<Entry, undefined>>;
};

/** One open segment and the time it opened. */
interface Open {
    /** The scope whose entries it keeps. */
    readonly scope: string;
    /** The segment. */
    readonly segment: Segment;
    /** The opening time, in Unix milliseconds. */
    readonly openedAt: number;
}

/** Stores the entries of scopes' installations: open in memory, sealed as Parquet segments in a bucket. */
export class Monitor {
    /** The database with the segment catalog. */
    readonly database: DatabaseConnection;
    /** The bucket with the sealed segments. */
    readonly bucket: Bucket;
    /** The open segments, by scope and installation. */
    readonly #open = new Map<string, Open>();
    /** The live tails. */
    readonly #subscribers = new Set<Subscriber>();
    /** The seals in flight. */
    readonly #sealing = new Set<Promise<void>>();
    /** Report a segment that failed to seal and lost its entries. */
    readonly #report: (error: unknown) => void;
    /** This instance among the database's monitors. */
    readonly #instance = crypto.randomUUID();
    /** The channel to the monitors of the database's other instances, absent for a sole instance. */
    readonly #channel: Channel<MonitorMessage> | undefined;
    /** The other instances alive, with the tails they serve. */
    readonly #peers = new Map<string, { seenAt: number; tails: readonly Tail[] }>();
    /** The entries gathered for other instances' tails, by tail. */
    readonly #forwarding = new Map<string, Entry[]>();
    /** The queries waiting for other instances' answers, by request. */
    readonly #asking = new Map<
        string,
        { waiting: Set<string>; entries: Entry[]; done: () => void }
    >();

    /** Store the catalog in a database and the segments in a bucket, reaching other instances over a channel, reporting failed seals. */
    constructor(
        database: DatabaseConnection,
        bucket: Bucket,
        report: (error: unknown) => void,
        channel?: Channel<unknown>,
    ) {
        // keep the catalog, the bucket, the report and the channel
        this.database = database;
        this.bucket = bucket;
        this.#report = report;
        this.#channel = channel === undefined ? undefined : typedChannel(channel, MonitorMessage);
    }

    /** Take a scope's entries: append them to their open segments and pass them to matching tails. */
    ingest(scope: string, entries: readonly Entry[], now = Date.now()): void {
        for (const entry of entries) {
            // append to the installation's open segment, sealing a full one
            const key = Monitor.key(scope, entry.installation);
            let open = this.#open.get(key);
            if (open === undefined) {
                open = { scope, segment: new Segment(entry.installation), openedAt: now };
                this.#open.set(key, open);
            }
            open.segment.append(entry);
            if (open.segment.entries.length >= SEGMENT_ROWS) {
                this.#seal(key, open, now);
            }

            // pass it to the tails following it here and on other instances
            for (const subscriber of this.#subscribers) {
                if (subscriber.filter.scope === scope && matches(entry, subscriber.filter)) {
                    subscriber.push(entry);
                }
            }
            for (const peer of this.#peers.values()) {
                for (const tail of peer.tails) {
                    if (tail.filter.scope === scope && matches(entry, tail.filter)) {
                        this.#forward(tail.id, entry);
                    }
                }
            }
        }
    }

    /** Take an OTLP/JSON export an installation or the scope's host sent. */
    receive(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        signal: OtlpSignal,
        body: unknown,
    ): void {
        // read the export's entries by signal
        if (signal === "logs") {
            this.ingest(scope, Otlp.logs(OtlpLogsRequest.parse(body), installation));
        } else if (signal === "traces") {
            this.ingest(scope, Otlp.traces(OtlpTracesRequest.parse(body), installation));
        } else {
            this.ingest(scope, Otlp.metrics(OtlpMetricsRequest.parse(body), installation));
        }
    }

    /** Key an installation's entries in a scope, or the scope host's. */
    static key(scope: string, installation: Identifier<"installation"> | undefined): string {
        return `${scope}\0${installation ?? ""}`;
    }

    /** Read the scope and installation, absent for the scope's host, a key names. */
    static emitter(key: string): {
        readonly scope: string;
        readonly installation: Identifier<"installation"> | undefined;
    } {
        // split the key into its scope and installation
        const [scope, installation, ...rest] = key.split("\0");
        if (scope === undefined || installation === undefined || rest.length > 0) {
            throw new TypeError(`emitter key ${JSON.stringify(key)} is no scope and installation`);
        }

        return {
            scope,
            installation:
                installation === ""
                    ? undefined
                    : schema.identifier("installation").parse(installation),
        };
    }

    /** List the keys of every installation and scope host the catalog has segments of. */
    async emitters(): Promise<string[]> {
        const rows = await this.database
            .selectDistinct({
                scope: monitorSegment.scope,
                installation: monitorSegment.installationId,
            })
            .from(monitorSegment);

        return rows.map((row) => Monitor.key(row.scope, row.installation ?? undefined));
    }

    /** Search a scope's log records in a window, newest first, across sealed and open segments. */
    async search(scope: string, search: EntrySearch): Promise<EntryPage> {
        // read the sealed segments overlapping the window with a wanted severity, newest first
        const to = search.before - 1;
        const window = { from: search.from, to };
        const severities =
            search.severity === undefined ? logBits(1) | UNSPECIFIED_BIT : logBits(search.severity);
        const segments = await this.database
            .select()
            .from(monitorSegment)
            .where(
                and(
                    eq(monitorSegment.scope, scope),
                    emittedBy(search.installation),
                    gte(monitorSegment.to, search.from),
                    lte(monitorSegment.from, to),
                    sql`(${monitorSegment.contents} & ${severities}) != 0`,
                ),
            )
            .orderBy(desc(monitorSegment.to));

        // collect matching records from the open segment, then from each sealed one, newest first
        const query: EntryQuery = { kind: "search", search };
        const isWanted = Monitor.#wanted(query, search.installation);
        const collected = await this.#recent(scope, search.installation, window, query);
        for (let start = 0; start < segments.length; start += READ_CONCURRENCY) {
            // stop once more than a page is newer than every remaining segment
            const batch = segments.slice(start, start + READ_CONCURRENCY);
            const newest = Math.max(...batch.map((segment) => segment.to));
            if (collected.filter((entry) => entry.time > newest).length > search.limit) {
                break;
            }

            // read the next batch of segments at once
            const read = await this.#read(batch, search.installation, window, isWanted);
            collected.push(...read.flat());
        }

        // page newest first and keep entries of one time on one page
        const found = newestFirst(collected);
        const last = found.at(search.limit - 1);
        if (last === undefined || found.length === search.limit) {
            return { entries: found };
        }
        const entries = found.filter(
            (entry, index) => index < search.limit || entry.time === last.time,
        );

        return entries.length < found.length ? { entries, before: last.time } : { entries };
    }

    /** Follow a scope's entries matching a filter from now on, until the signal aborts or the reader stops. */
    tail(filter: EntryFilter, signal: AbortSignal): EntryStream {
        // queue entries between reads, or hand one to the waiting read
        const queue: Entry[] = [];
        let waiting: ((result: IteratorResult<Entry>) => void) | undefined;
        let isClosed = false;
        const subscriber: Subscriber = {
            id: crypto.randomUUID(),
            filter,
            push: (entry) => {
                const read = waiting;
                waiting = undefined;
                if (read === undefined) {
                    queue.push(entry);
                } else {
                    read({ done: false, value: entry });
                }
            },
        };

        // unsubscribe once, ending the waiting read
        const close = () => {
            // stop receiving, then release a read waiting for more
            isClosed = true;
            this.#subscribers.delete(subscriber);
            this.#announce();
            signal.removeEventListener("abort", close);
            waiting?.({ done: true, value: undefined });
            waiting = undefined;
        };

        // subscribe at once, unless already stopped
        if (signal.aborted) {
            isClosed = true;
        } else {
            this.#subscribers.add(subscriber);
            this.#announce();
            signal.addEventListener("abort", close, { once: true });
        }

        return {
            next: () => {
                const entry = queue.shift();
                if (entry !== undefined) {
                    return Promise.resolve({ done: false, value: entry });
                } else if (isClosed) {
                    return Promise.resolve({ done: true, value: undefined });
                } else {
                    return new Promise((resolve) => {
                        waiting = resolve;
                    });
                }
            },
            return: () => {
                close();

                return Promise.resolve({ done: true, value: undefined });
            },
            [Symbol.asyncIterator]() {
                return this;
            },
            [Symbol.asyncDispose]: () => {
                close();

                return Promise.resolve();
            },
        };
    }

    /** Read every span and log record of a trace, oldest first, from the time its identifier carries. */
    async trace(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        trace: string,
    ): Promise<Entry[]> {
        // read the window starting at the trace's millisecond prefix
        const start = Number.parseInt(trace.slice(0, 12), 16) * 1000;
        const window = { from: start, to: start + TRACE_MICROSECONDS };
        const segments = await this.database
            .select()
            .from(monitorSegment)
            .where(
                and(
                    eq(monitorSegment.scope, scope),
                    emittedBy(installation),
                    gte(monitorSegment.to, window.from),
                    lte(monitorSegment.from, window.to),
                ),
            )
            .orderBy(asc(monitorSegment.from));

        // collect the trace's entries from the sealed segments and the open one
        const query: EntryQuery = { kind: "trace", trace };
        const isWanted = Monitor.#wanted(query, installation);
        const found = (await this.#read(segments, installation, window, isWanted, trace)).flat();
        found.push(...(await this.#recent(scope, installation, window, query)));

        return found.toSorted((first, second) => first.time - second.time);
    }

    /** Resolve the share of traces an installation keeps. */
    sampling(
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ): Promise<number> {
        return this.#resolve(traceSampling, scope, installation);
    }

    /** Merge each finished hour's minute segments of an installation, or of the scope's host, into one. */
    async compact(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        now = Date.now(),
    ): Promise<void> {
        // group the minute segments of finished hours by hour
        const current = Math.floor((now * 1000) / HOUR_MICROSECONDS);
        const minutes = await this.database
            .select()
            .from(monitorSegment)
            .where(
                and(
                    eq(monitorSegment.scope, scope),
                    emittedBy(installation),
                    eq(monitorSegment.level, 0),
                ),
            );
        const hours = Map.groupBy(minutes, (row) => Math.floor(row.from / HOUR_MICROSECONDS));

        // merge each finished hour's segments in groups that fit one segment, oldest first
        for (const [hour, segments] of hours) {
            if (hour >= current) {
                continue;
            }
            const ordered = segments.toSorted((first, second) => first.from - second.from);
            for (const rows of fitting(ordered)) {
                if (rows.length >= 2) {
                    await this.#merge(scope, installation, rows, now);
                }
            }
        }
    }

    /** Merge a group of minute segments into one, storing its file before replacing their rows. */
    async #merge(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        rows: readonly Select<typeof monitorSegment>[],
        now: number,
    ): Promise<void> {
        // read the group's entries, a bounded number of files at once, oldest first
        const window = {
            from: Math.min(...rows.map((row) => row.from)),
            to: Math.max(...rows.map((row) => row.to)),
        };
        const entries = (await this.#read(rows, installation, window, () => true)).flat();
        const merged = new Segment(installation);
        for (const entry of entries.toSorted((first, second) => first.time - second.time)) {
            merged.append(entry);
        }

        // store the merged file, then replace the minute rows with its row
        const id = schema.identifier("segment").parse(`segment-${v7()}`);
        const key = `${scope}/${installation ?? "host"}/${merged.from}-${id}.parquet`;
        const body = await merged.encode();
        await this.bucket.put(key, body);
        await this.database.transaction(async (transaction) => {
            await transaction.insert(monitorSegment).values({
                id,
                scope,
                installationId: installation ?? null,
                from: merged.from,
                to: merged.to,
                level: 1,
                rows: merged.entries.length,
                bytes: body.byteLength,
                contents: merged.contents,
                key,
                createdAt: now,
            });
            await transaction.delete(monitorSegment).where(
                inArray(
                    monitorSegment.id,
                    rows.map((row) => row.id),
                ),
            );
        });
    }

    /** Drop the segments of an installation, or of the scope's host, older than its retention. */
    async prune(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        now = Date.now(),
    ): Promise<void> {
        // keep the entries of the retained days
        const days = await this.#resolve(telemetryRetention, scope, installation);
        const before = (now - days * DAY_MILLISECONDS) * 1000;
        await this.database
            .delete(monitorSegment)
            .where(
                and(
                    eq(monitorSegment.scope, scope),
                    emittedBy(installation),
                    lt(monitorSegment.to, before),
                ),
            );
    }

    /** Delete the files of an installation, or of the scope's host, that no segment names any more. */
    async sweep(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        now = Date.now(),
    ): Promise<void> {
        // list the files the catalog keeps
        const kept = new Set(
            (
                await this.database
                    .select({ key: monitorSegment.key })
                    .from(monitorSegment)
                    .where(and(eq(monitorSegment.scope, scope), emittedBy(installation)))
            ).map((row) => row.key),
        );

        // delete each page's uncatalogued files older than the grace
        const prefix = `${scope}/${installation ?? "host"}/`;
        let cursor: string | undefined;
        do {
            const page = await this.bucket.list({
                prefix,
                ...(cursor === undefined ? {} : { cursor }),
            });
            const orphans = page.files
                .filter((file) => !kept.has(file.key))
                .filter((file) => now - file.uploaded.getTime() > SWEEP_GRACE_MILLISECONDS)
                .map((file) => file.key);
            if (orphans.length > 0) {
                await this.bucket.delete(orphans);
            }
            cursor = page.cursor;
        } while (cursor !== undefined);
    }

    /** Resolve a space setting for an installation from the values placed along its space's chain, or its default for a host. */
    async #resolve<Value extends schema.Schema>(
        declared: Setting<Value>,
        scope: string,
        installation: Identifier<"installation"> | undefined,
    ): Promise<schema.Infer<Value>> {
        // take the default for the host's entries
        if (installation === undefined) {
            return declared.definition.default;
        }

        return SettingValue.resolve(this.database, declared, { scope, installation });
    }

    /** Aggregate a metric's points in a window into steps per attribute group, across sealed and open segments. */
    async series(scope: string, request: PointSeries): Promise<Series> {
        // read the sealed segments with points in the window
        const window = { from: request.from, to: request.before - 1 };
        const segments = await this.database
            .select()
            .from(monitorSegment)
            .where(
                and(
                    eq(monitorSegment.scope, scope),
                    emittedBy(request.installation),
                    gte(monitorSegment.to, window.from),
                    lte(monitorSegment.from, window.to),
                    sql`(${monitorSegment.contents} & ${POINT_BIT}) != 0`,
                ),
            );

        // collect the metric's points from each sealed segment and the open one
        const query: EntryQuery = { kind: "series", name: request.name };
        const isWanted = Monitor.#wanted(query, request.installation);
        const points = await this.#recent(scope, request.installation, window, query);
        points.push(...(await this.#read(segments, request.installation, window, isWanted)).flat());

        return aggregate(points, request);
    }

    /** Seal the segments open longer than the seal interval, or every one when forced. */
    async seal(now = Date.now(), isForced = false): Promise<void> {
        // seal each due segment
        for (const [key, open] of this.#open) {
            if (isForced || now - open.openedAt >= SEAL_MILLISECONDS) {
                this.#seal(key, open, now);
            }
        }

        // wait for every seal in flight
        await Promise.all(this.#sealing);
    }

    /** Seal due segments every few seconds until the signal aborts, then seal the rest. */
    async run(signal: AbortSignal): Promise<void> {
        // seal on an interval, and hear and greet the other instances
        const interval = setInterval(() => void this.seal(), SEAL_MILLISECONDS / 12);
        const channel = this.#channel;
        const stop = channel?.listen(
            (message) => this.#receive(message, channel),
            () => this.#announce(),
            this.#report,
        );
        const heartbeat = setInterval(() => this.#announce(), HEARTBEAT_MILLISECONDS);
        try {
            await until(signal);
        } finally {
            clearInterval(interval);
            clearInterval(heartbeat);
            this.#channel?.notify({ kind: "gone", instance: this.#instance });
            stop?.();
        }

        // seal what remains open
        await this.seal(Date.now(), true);
    }

    /** Close and store an open segment and keep its entries searchable until the catalog lists it. */
    #seal(key: string, open: Open, now: number): void {
        // close the segment
        this.#open.delete(key);
        const { segment, scope } = open;

        // write the file, then name it in the catalog
        const id = schema.identifier("segment").parse(`segment-${v7()}`);
        const file = `${scope}/${segment.installation ?? "host"}/${segment.from}-${id}.parquet`;
        const sealing = (async () => {
            const body = await segment.encode();
            await this.bucket.put(file, body);
            await this.database.insert(monitorSegment).values({
                id,
                scope,
                installationId: segment.installation ?? null,
                from: segment.from,
                to: segment.to,
                level: 0,
                rows: segment.entries.length,
                bytes: body.byteLength,
                contents: segment.contents,
                key: file,
                createdAt: now,
            });
        })().then(
            () => undefined,
            (error: unknown) => this.#report(error),
        );
        this.#sealing.add(sealing);
        void sealing.finally(() => this.#sealing.delete(sealing));
    }

    /** Read the open entries of an installation in a window that a predicate selects. */
    #openEntries(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        window: { readonly from: number; readonly to: number },
        isWanted: (entry: Entry) => boolean,
    ): Entry[] {
        const open = this.#open.get(Monitor.key(scope, installation));
        const entries = open?.segment.entries ?? [];

        return entries.filter(
            (entry) => entry.time >= window.from && entry.time <= window.to && isWanted(entry),
        );
    }

    /** Read the open entries a query wants in a window, on this instance and every other alive one. */
    async #recent(
        scope: string,
        installation: Identifier<"installation"> | undefined,
        window: { readonly from: number; readonly to: number },
        query: EntryQuery,
    ): Promise<Entry[]> {
        // read this instance's open entries
        const local = this.#openEntries(
            scope,
            installation,
            window,
            Monitor.#wanted(query, installation),
        );
        const peers = [...this.#alive()];
        if (this.#channel === undefined || peers.length === 0) {
            return local;
        }

        // ask the other instances, taking the answers that arrive in time
        const asked = await this.#ask(this.#channel, peers, scope, installation, window, query);

        return [...local, ...asked];
    }

    /** Ask other instances for their open entries a query wants, collecting the answers that arrive in time. */
    async #ask(
        channel: Channel<MonitorMessage>,
        peers: readonly string[],
        scope: string,
        installation: Identifier<"installation"> | undefined,
        window: { readonly from: number; readonly to: number },
        query: EntryQuery,
    ): Promise<Entry[]> {
        // wait for every answer, or the timeout
        const request = crypto.randomUUID();
        const answered = Promise.withResolvers<void>();
        const entries: Entry[] = [];
        const asking = { waiting: new Set(peers), entries, done: answered.resolve };
        this.#asking.set(request, asking);
        channel.notify({
            kind: "ask",
            request,
            instance: this.#instance,
            scope,
            ...(installation === undefined ? {} : { installation }),
            window,
            query,
        });
        const timeout = setTimeout(answered.resolve, ANSWER_MILLISECONDS);
        await answered.promise;
        clearTimeout(timeout);
        this.#asking.delete(request);

        return entries;
    }

    /** Take a message from another instance's monitor. */
    #receive(message: MonitorMessage, channel: Channel<MonitorMessage>): void {
        // keep an alive instance's tails, and forget a stopped one
        if (message.kind === "alive") {
            this.#peers.set(message.instance, { seenAt: Date.now(), tails: message.tails });
        } else if (message.kind === "gone") {
            this.#peers.delete(message.instance);
        }
        // pass forwarded entries to the tail they are for, when it is this instance's
        else if (message.kind === "entries") {
            const subscriber = [...this.#subscribers].find((each) => each.id === message.tail);
            for (const entry of message.entries) {
                subscriber?.push(entry);
            }
        }
        // answer another instance's query with this instance's open entries
        else if (message.kind === "ask") {
            const entries = this.#openEntries(
                message.scope,
                message.installation,
                message.window,
                Monitor.#wanted(message.query, message.installation),
            );
            channel.notify({
                kind: "answer",
                request: message.request,
                instance: this.#instance,
                entries,
            });
        }
        // collect an answer to this instance's query, finishing once every instance answered
        else {
            const asking = this.#asking.get(message.request);
            if (asking !== undefined) {
                asking.entries.push(...message.entries);
                asking.waiting.delete(message.instance);
                if (asking.waiting.size === 0) {
                    asking.done();
                }
            }
        }
    }

    /** Announce this instance and the tails it serves to the others. */
    #announce(): void {
        const tails = [...this.#subscribers].map(({ id, filter }) => ({ id, filter }));
        this.#channel?.notify({ kind: "alive", instance: this.#instance, tails });
    }

    /** List the other instances heard from within the last few heartbeats. */
    *#alive(): Generator<string> {
        const now = Date.now();
        for (const [instance, peer] of this.#peers) {
            if (now - peer.seenAt <= PEER_MILLISECONDS) {
                yield instance;
            } else {
                this.#peers.delete(instance);
            }
        }
    }

    /** Gather an entry for another instance's tail, sending the gathered ones together shortly. */
    #forward(tail: string, entry: Entry): void {
        // start gathering for the tail, and send what gathered after a short wait
        const gathering = this.#forwarding.get(tail);
        if (gathering === undefined) {
            const gathered = [entry];
            this.#forwarding.set(tail, gathered);
            setTimeout(() => {
                this.#forwarding.delete(tail);
                this.#channel?.notify({ kind: "entries", tail, entries: gathered });
            }, FORWARD_MILLISECONDS);
        }
        // add to the entries gathering for the tail
        else {
            gathering.push(entry);
        }
    }

    /** Decide which entries a query wants, of an installation or of the scope's host. */
    static #wanted(
        query: EntryQuery,
        installation: Identifier<"installation"> | undefined,
    ): (entry: Entry) => boolean {
        // want a search's log records, a trace's entries, or a metric's points
        if (query.kind === "search") {
            return (entry) => entry.kind === "log" && matches(entry, query.search);
        } else if (query.kind === "trace") {
            const filter = {
                ...(installation === undefined ? {} : { installation }),
                trace: query.trace,
            };

            return (entry) => matches(entry, filter);
        } else {
            return (entry) =>
                entry.kind === "point" &&
                entry.name === query.name &&
                entry.installation === installation;
        }
    }

    /** Read the wanted entries of segments in a window, a bounded number of segments at once, in segment order. */
    async #read(
        segments: readonly { readonly key: string; readonly bytes: number }[],
        installation: Identifier<"installation"> | undefined,
        window: { readonly from: number; readonly to: number },
        isWanted: (entry: Entry) => boolean,
        trace?: string,
    ): Promise<Entry[][]> {
        // start the next segment when a read finishes, up to the concurrency limit
        const read: Entry[][] = [];
        const pending = segments.entries();
        const worker = async () => {
            for (const [index, segment] of pending) {
                const file = this.#file(segment.key, segment.bytes);
                const entries = await Segment.decode(file, installation, window, trace);
                read[index] = entries.filter(isWanted);
            }
        };
        await Promise.all(Array.from({ length: READ_CONCURRENCY }, worker));

        return read;
    }

    /** Read a segment file through ranged bucket reads. */
    #file(key: string, byteLength: number): AsyncBuffer {
        return {
            byteLength,
            slice: async (start, end = byteLength) => {
                const body = await this.bucket.get(key, {
                    range: { offset: start, length: end - start },
                });
                if (body === null) {
                    throw new Error(`segment file ${key} is missing`);
                }

                return body.arrayBuffer();
            },
        };
    }
}

/** Pack segments into consecutive groups of at most one segment's rows, so each merge fits in memory. */
function fitting<Stored extends { readonly rows: number }>(
    segments: readonly Stored[],
): Stored[][] {
    // fill each group up to the rows one segment keeps
    const groups: Stored[][] = [];
    let group: Stored[] = [];
    let rows = 0;
    for (const segment of segments) {
        // close the group once the segment would not fit
        if (group.length > 0 && rows + segment.rows > SEGMENT_ROWS) {
            groups.push(group);
            group = [];
            rows = 0;
        }
        group.push(segment);
        rows += segment.rows;
    }

    // close the last group
    if (group.length > 0) {
        groups.push(group);
    }

    return groups;
}

/** Build the contents bits of log severities from a lowest one up to 24. */
function logBits(lowest: number): number {
    let bits = 0;
    for (let severity = lowest; severity <= 24; severity++) {
        bits |= 1 << severity;
    }

    return bits;
}

/** Select the catalog rows of an installation, or of the scope's host. */
function emittedBy(installation: Identifier<"installation"> | undefined): SQL {
    return installation === undefined
        ? isNull(monitorSegment.installationId)
        : eq(monitorSegment.installationId, installation);
}

/** Order entries collected oldest first as newest first, the later arrival first among equal times. */
function newestFirst(entries: Entry[]): Entry[] {
    return entries.toSorted((first, second) => first.time - second.time).toReversed();
}

/** Decide whether an entry matches a filter. */
function matches(entry: Entry, filter: Omit<EntryFilter, "scope">): boolean {
    return (
        entry.installation === filter.installation &&
        (filter.severity === undefined || (entry.severity ?? 0) >= filter.severity) &&
        (filter.names === undefined || filter.names.includes(entry.name)) &&
        (filter.trace === undefined || entry.trace === filter.trace) &&
        (filter.text === undefined || contains(entry, filter.text)) &&
        Object.entries(filter.attributes ?? {}).every(
            ([key, value]) => JSON.stringify(entry.attributes[key]) === JSON.stringify(value),
        )
    );
}

/** Decide whether an entry's name or body contains a text, ignoring case. */
function contains(entry: Entry, text: string): boolean {
    const wanted = text.toLowerCase();

    return (
        entry.name.toLowerCase().includes(wanted) ||
        (entry.body?.toLowerCase().includes(wanted) ?? false)
    );
}
