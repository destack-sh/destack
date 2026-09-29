import { objectKey, type Subject } from "@destack/access";
import {
    and,
    asc,
    count,
    defineTable,
    desc,
    eq,
    gt,
    integer,
    inArray,
    isNotNull,
    isNull,
    json,
    lte,
    max,
    or,
    sql,
    text,
    type DatabaseConnection,
    type JsonValue,
    type Row,
    TABLE,
    type Table,
} from "@destack/db";
import { Condition, Expression, Order, type Scalar } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { schema } from "@destack/schema";
import { canonicalize, digest } from "@destack/schema/json";
import type { Client } from "@destack/service";
import { ServiceError } from "@destack/service/error";
import { ScopeHolder } from "../error/holder.ts";
import { Failure, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import { Observable } from "@destack/service/observable";
import { RetryPolicy } from "@destack/service/timer";
import type { LogPosition } from "@destack/db/log";
import * as sync from "@destack/sync";
import { v7 } from "uuid";
import { Call } from "../method/call.ts";
import type { Method } from "../method/method.ts";
import type { Step } from "../method/step.ts";
import type { CallableName, ObjectProcedures, ScopeField } from "../method/procedure.ts";
import {
    ObjectQuery,
    PUSH_MUTATIONS,
    type ObjectInclude,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { Duration } from "../object/duration.ts";
import { ObjectType, type ObjectStorage } from "../object/object.ts";
import { Sequence, type Run, type TextChange } from "../sequence/index.ts";
import { Chunk, type Edited } from "../text/chunk.ts";
import { chunk } from "../text/table.ts";

/** The method kinds whose predictions read only their object's and its parent's tables. */
const STANDARD_KINDS: ReadonlySet<string> = new Set(["create", "update", "delete", "updateMany"]);

/** The default retry of transient failures: a second, doubling, at most a minute. */
const RETRY = RetryPolicy.of({ maximumInterval: 60_000 });

/** The hexadecimal digest digits naming a query: 64 bits, safe among a client's queries. */
const QUERY_NAME_LENGTH = 16;

/** The default local log window, in milliseconds: a minute, far above the milliseconds live queries lag. */
const LOCAL_LOG_MILLISECONDS = 60_000;

/** A mutation a client predicted. */
export interface Submission<Result> {
    /** The result the local prediction returned, once the mutation is in the outbox. */
    readonly predicted: Promise<Result>;
    /** Settles once the replica holds the server's changes, or rejects with the server's failure. */
    readonly confirmed: Promise<void>;
}

/** One text field of an object, followed as its sequence and changed by visible offsets. */
export interface LiveText {
    /** Settles once the copy holds the text. */
    readonly ready: Promise<void>;
    /** Read the text's sequence, predictions included. */
    read(): Promise<Sequence>;
    /** Yield the sequence, then again after each change. */
    watch(signal: AbortSignal): AsyncGenerator<Sequence>;
    /** Replace the text between two offsets, after every earlier change, as one predicted edit. */
    change(change: TextChange): Promise<Submission<Edited>>;
    /** Stop following the text. */
    close(): Promise<void>;
}

/** A live query a client reads from its copy, predictions included. */
export interface LiveQuery {
    /** Settles once the copy holds the query, or once the query closes. */
    readonly ready: Promise<void>;
    /** Read the query's rows with what they include, or its groups when it aggregates. */
    read(): Promise<readonly Readonly<Record<string, unknown>>[]>;
    /** Read the query again after every local commit until the signal aborts. */
    watch(signal: AbortSignal): AsyncGenerator<readonly Readonly<Record<string, unknown>>[]>;
    /** Stop following the query. */
    close(): Promise<void>;
}

/** The mutating methods of an object type, each call its own pending mutation. */
export type Mutator<Object extends ObjectType> = {
    readonly [Name in MutatingName<Object>]: (
        input: CallInput<Object, Name>,
    ) => Submission<CallOutput<Object, Name>>;
};

/** The mutating methods of an object type within a mutation, returning predictions. */
export type MutationCalls<Object extends ObjectType> = {
    readonly [Name in MutatingName<Object>]: (
        input: CallInput<Object, Name>,
    ) => Promise<CallOutput<Object, Name>>;
};

/** The methods of an object type that change nothing, each read from the server. */
export type Reader<Object extends ObjectType> = {
    readonly [Name in ReadingName<Object>]: (
        input: CallInput<Object, Name>,
    ) => Promise<CallOutput<Object, Name>>;
};

/** Several calls a client makes as one atomic mutation. */
export interface Mutation {
    /** Call an object type's mutating methods within the mutation. */
    call<Object extends ObjectType>(object: Object): MutationCalls<Object>;
}

/** What a client holds and follows. */
export interface ClientInspection {
    /** The scope whose objects it holds. */
    readonly scope: string;
    /** The rows its copy holds, by object type name. */
    readonly rows: Readonly<Record<string, number>>;
    /** The most object rows the copy holds, absent for no limit. */
    readonly storage?: { readonly rows: number };
    /** Every party's subscriptions, open and closed. */
    readonly subscriptions: readonly {
        /** The query's name. */
        readonly name: string;
        /** The party that asked. */
        readonly origin: string;
        /** The time the query closed, in UTC epoch milliseconds, absent while open. */
        readonly releasedAt?: number;
        /** How long the closed query stays followed, in milliseconds, or always. */
        readonly keep?: number | "always";
    }[];
    /** The copy of the scope. */
    readonly replica: sync.ReplicaInspection;
    /** The outbox's mutations by outcome. */
    readonly outbox: {
        /** The mutations waiting for the server. */
        readonly pending: number;
        /** The mutations the server executed. */
        readonly executed: number;
        /** The mutations the server rejected. */
        readonly rejected: number;
    };
    /** The local feed serving live queries. */
    readonly feed: sync.FeedInspection;
}

/** One scope's objects on a client. */
export class ObjectClient {
    /** The local database holding the replica and the outbox. */
    readonly database: DatabaseConnection;
    /** The object types the client holds. */
    readonly objects: readonly ObjectType[];
    /** The scope whose objects the client holds. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The copy of the scope's durable objects. */
    readonly replica: sync.Replica;
    /** The copy of the scope's ephemeral objects, absent when the client holds none. */
    readonly ephemeral: sync.Replica | undefined;
    /** The client's own identifier, owning its ephemeral objects. */
    readonly clientId: string;
    /** The mutations waiting for the server. */
    readonly outbox: sync.Outbox;
    /** The party sharing the database, such as one browser tab. */
    readonly origin: string;
    /** The replica procedures of the scope's current holder. */
    #service: Client<ReplicaProcedures>;
    /** Connect to the replica procedures of the holder a moved scope now answers at. */
    readonly #reconnect: (holder: string) => Client<ReplicaProcedures>;
    /** Aborts the streams following a moved scope's former holder. */
    #moves = new AbortController();
    /** The input field naming the scope. */
    readonly #field: string | undefined;
    /** The local feed of watched queries. */
    readonly #local: sync.Feed;
    /** The listeners of the events sent to each object, by topic. */
    readonly #listeners = new Map<string, Set<(event: JsonValue) => void>>();
    /** How often merged changes arrive, absent for one page per change. */
    readonly #refresh: { readonly every: Duration } | undefined;
    /** The most object rows the local copy holds, absent for no limit. */
    readonly #storage: { readonly rows: number } | undefined;
    /** How long the local log keeps changes, in milliseconds. */
    readonly #logMilliseconds: number;
    /** How transient failures retry. */
    readonly #retry: RetryPolicy;
    /** Aborts outcome waits and lookups on close. */
    readonly #closing = new AbortController();
    /** The object rows the copy held at a log sequence, absent before the first count. */
    #counted: { readonly sequence: number; readonly rows: number } | undefined;
    /** The open followed queries. */
    readonly #holdings = new Set<Holding>();
    /** The queued evictions, run one at a time. */
    #evictions: Promise<void> = Promise.resolve();
    /** The running undo or redo. */
    #toggling: Promise<unknown> = Promise.resolve();
    /** The mutations waiting for their outcome, by identifier. */
    readonly #waiting = new Map<
        string,
        {
            readonly resolve: (outcome: sync.Outcome) => void;
            readonly reject: (error: Error) => void;
        }
    >();
    /** The running settle loop, absent while none wait. */
    #settling: Promise<void> | undefined;
    /** Report the follow loop's failures, absent until it follows. */
    #report: ((error: unknown) => void) | undefined;

    /** Hold one scope's objects in a local database whose tables exist. */
    private constructor(options: {
        readonly database: DatabaseConnection;
        readonly objects: readonly ObjectType[];
        readonly scope: string;
        readonly caller: Subject;
        readonly service: Client<ReplicaProcedures>;
        readonly reconnect: (holder: string) => Client<ReplicaProcedures>;
        readonly origin: string;
        readonly refresh?: { readonly every: Duration };
        readonly storage?: { readonly rows: number };
        readonly log?: { readonly keep: Duration };
        readonly retry?: Partial<RetryPolicy>;
    }) {
        // keep the options
        this.database = options.database;
        this.#refresh = options.refresh;
        this.#storage = options.storage;
        this.#logMilliseconds =
            options.log === undefined
                ? LOCAL_LOG_MILLISECONDS
                : Duration.milliseconds(options.log.keep);
        this.#retry = { ...RETRY, ...options.retry };
        this.origin = options.origin;
        this.objects = ObjectType.served(options.objects);
        this.scope = options.scope;
        this.caller = options.caller;
        this.#service = options.service;
        this.#reconnect = options.reconnect;

        // require one scope field
        const fields = new Set(this.objects.map((object) => object.route.field));
        if (fields.size !== 1) {
            throw new TypeError("a client holds object types that name their scope in one field");
        }
        this.#field = [...fields][0];
        const tables = this.objects
            .filter((object) => object.storage === "durable")
            .map((object) => object.table as Table);
        this.replica = new sync.Replica({ name: "objects", scope: options.scope, tables });
        this.outbox = new sync.Outbox(
            tables,
            (transaction, pending) => this.#replay(transaction, pending),
            (call) => this.#reach(call, tables),
        );
        // copy ephemeral objects apart
        const ephemeral = this.objects
            .filter((object) => object.storage === "ephemeral")
            .map((object) => object.table as Table);
        this.clientId = RequestId.create();
        this.ephemeral =
            ephemeral.length === 0
                ? undefined
                : new sync.Replica({ name: "ephemeral", scope: options.scope, tables: ephemeral });

        // read both copies through one local feed
        this.#local = new sync.Feed(
            options.database,
            [...tables, ...ephemeral, sync.replicaResult],
            { upstream: this.#upstream(this.objects) },
        );
    }

    /** Measure each query through the copy holding its root. */
    #upstream(objects: readonly ObjectType[]): sync.Upstream {
        // map tables to storages and copies
        const storages = new Map(objects.map((object) => [object.table as Table, object.storage]));
        const durable = this.replica.upstream(this.database, this.outbox);
        const ephemeral = this.ephemeral?.upstream(this.database, undefined);
        const rootOf = (node: sync.Node): sync.Node =>
            node.parent === undefined ? node : rootOf(node.parent);
        const sourceOf = (node: sync.Node) =>
            storages.get(rootOf(node).table) === "ephemeral" ? ephemeral! : durable;

        return {
            watches: [...durable.watches, ...(ephemeral?.watches ?? [])],
            groups: (node) => sourceOf(node).groups(node),
            groupOf: (change) => durable.groupOf(change) ?? ephemeral?.groupOf(change),
            isMeasured: (node) => {
                // measure at the source only nodes within the root's storage
                const storage = storages.get(rootOf(node).table);
                for (let at: sync.Node | undefined = node; at !== undefined; at = at.parent) {
                    if (storages.get(at.table) !== storage) {
                        return false;
                    }
                }

                return true;
            },
        };
    }

    /** Create the local tables, then hold one scope's objects in the database. */
    static async open<Object extends ObjectType>(options: {
        /** The local database. */
        readonly database: DatabaseConnection;
        /** The object types to hold. */
        readonly objects: readonly Object[];
        /** The scope whose objects to hold. */
        readonly scope: string;
        /** The calling principal. */
        readonly caller: Subject;
        /** The service's replica procedures. */
        readonly service: Client<ReplicaProcedures>;
        /** Connect to the replica procedures of the holder a moved scope now answers at. */
        readonly reconnect: (holder: string) => Client<ReplicaProcedures>;
        /** The party sharing the database, a new one by default. */
        readonly origin?: string;
        /** Whether another party created the local tables. */
        readonly isMigrated?: boolean;
        /** How often merged changes arrive, such as for a background tab. */
        readonly refresh?: { readonly every: Duration };
        /** The most object rows the local copy holds, absent for no limit. */
        readonly storage?: { readonly rows: number };
        /** How long the local log keeps changes, a minute by default. */
        readonly log?: { readonly keep: Duration };
        /** How transient failures retry, over the defaults. */
        readonly retry?: Partial<RetryPolicy>;
    }): Promise<ObjectClient> {
        // create the local tables
        if (!options.isMigrated) {
            await options.database.migrate(ObjectClient.tables(options.objects), {
                isReplica: true,
            });
        }

        return new ObjectClient({
            ...options,
            origin: options.origin ?? RequestId.create(),
        });
    }

    /** List the tables a client's local database holds for some object types. */
    static tables(objects: readonly ObjectType[]): Table[] {
        return [
            ...ObjectType.served(objects).map((object) => object.table as Table),
            ...sync.REPLICA_TABLES,
            ...sync.OUTBOX_TABLES,
            subscription,
            undoEntry,
        ];
    }

    /** Call an object type's mutating methods, each call its own mutation. */
    mutate<Object extends ObjectType>(object: Object): Mutator<Object> {
        // send an ephemeral call at once
        if (object.storage === "ephemeral") {
            return this.#methods(object, true, (name, input) => {
                const sent = this.#send(this.#recordCall(object, name, input));

                return { predicted: sent, confirmed: sent };
            }) as Mutator<Object>;
        }

        return this.#methods(object, true, (name, input) => {
            // predict the call alone
            let result: unknown;
            const pending = this.mutation(async (mutation) => {
                result = await (
                    mutation.call(object) as Record<string, (input: unknown) => Promise<unknown>>
                )[name]!(input);
            });

            return {
                predicted: pending.predicted.then(() => result),
                confirmed: pending.confirmed,
            };
        }) as Mutator<Object>;
    }

    /** Predict several calls together and add them to the outbox as one mutation. */
    mutation<Result>(run: (mutation: Mutation) => Promise<Result>): Submission<Result> {
        // predict every call
        const id = RequestId.create();
        const predicted = this.outbox.add(this.database, id, this.origin, async (database) => {
            // collect calls and steps
            const calls: sync.Call[] = [];
            const steps: StoredStep[] = [];
            const result = await run({
                call: (object) =>
                    this.#methods(object, true, async (name, input) => {
                        // reject ephemeral objects
                        if (object.storage === "ephemeral") {
                            throw new TypeError(
                                `ephemeral object ${object.name} is written one call at a time`,
                            );
                        }
                        const entry = this.#recordCall(object, name, input);
                        calls.push(entry);
                        const predicted = await this.#step(database, object, name, entry.input);
                        steps.push(predicted.step);

                        return predicted.result;
                    }) as MutationCalls<typeof object>,
            });

            // remember for undo
            await this.#remember(database, id, steps);

            return { calls, result };
        });

        // confirm with the server's outcome
        const confirmed = predicted.then(() => this.#outcome(id));
        confirmed.catch(() => {});

        return { predicted, confirmed };
    }

    /** Predict recorded calls and add them to the outbox as one mutation. */
    submit(calls: readonly sync.Call[]): Submission<void> {
        return this.mutation(async (mutation) => {
            for (const entry of calls) {
                const { object, name } = this.#method(entry.method);
                const methods = mutation.call(object) as Record<
                    string,
                    (input: unknown) => Promise<unknown>
                >;
                await methods[name]!(entry.input);
            }
        });
    }

    /** Call an object type's reading methods on the server. */
    read<Object extends ObjectType>(object: Object): Reader<Object> {
        return this.#methods(object, false, (name, input) =>
            this.#call((service) =>
                service.call({ scope: this.scope, call: this.#recordCall(object, name, input) }),
            ),
        ) as Reader<Object>;
    }

    /** Check out a branch, or the main line when undefined. */
    checkout(branch: string | undefined): Promise<void> {
        return this.outbox.checkout(this.database, branch);
    }

    /** Move a branch's mutations onto the main line. */
    merge(branch: string): Promise<void> {
        return this.outbox.merge(this.database, branch);
    }

    /** Remove a branch's mutations. */
    discard(branch: string): Promise<void> {
        return this.outbox.discard(this.database, branch);
    }

    /** List the branches holding mutations, and the one checked out. */
    async branches(): Promise<{ readonly names: readonly string[]; readonly checkedOut?: string }> {
        const checkedOut = await this.outbox.checkedOut(this.database);

        return {
            names: await this.outbox.branches(this.database),
            ...(checkedOut === undefined ? {} : { checkedOut }),
        };
    }

    /** Undo this party's latest mutation, predicting false when none is left. */
    undo(): Submission<boolean> {
        return this.#toggle("done", "undone");
    }

    /** Redo this party's latest undo, predicting false when none is left. */
    redo(): Submission<boolean> {
        return this.#toggle("undone", "done");
    }

    /** Close the client and fail unsettled mutations. */
    async close(): Promise<void> {
        this.#closing.abort();
        await Promise.all([...this.#holdings].map((holding) => holding.done));
    }

    /** Send an unstored event to an object's readers. */
    async broadcast(object: ObjectType, id: string, event: JsonValue): Promise<void> {
        await this.#call((service) =>
            service.broadcast({
                scope: this.scope,
                object: { packageId: object.policy.definition.packageId, type: object.name, id },
                event,
            }),
        );
    }

    /** Receive events sent to an object until aborted. */
    async *broadcasts(
        object: ObjectType,
        id: string,
        signal: AbortSignal,
    ): AsyncGenerator<JsonValue> {
        // collect arriving events
        const topic = objectKey(object.reference(this.scope, id));
        const arrived: JsonValue[] = [];
        let wake = () => {};
        const listener = (event: JsonValue) => {
            arrived.push(event);
            wake();
        };
        const listeners = this.#listeners.get(topic) ?? new Set();
        this.#listeners.set(topic, listeners.add(listener));
        const abort = () => wake();
        signal.addEventListener("abort", abort);
        try {
            // yield them in order
            while (!signal.aborted) {
                if (arrived.length === 0) {
                    await new Promise<void>((resolve) => (wake = resolve));
                    continue;
                }
                yield arrived.shift()!;
            }
        } finally {
            signal.removeEventListener("abort", abort);
            listeners.delete(listener);
        }
    }

    /** Push the outbox in order until aborted. */
    async push(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        let failures = 0;
        while (!signal.aborted) {
            // wait for pending mutations
            if (!(await this.outbox.wait(this.database, signal))) {
                return;
            }
            const pending = await this.outbox.pending(this.database);

            // push them
            try {
                const result = await this.#call((service) =>
                    service.push(
                        { scope: this.scope, mutations: pending.slice(0, PUSH_MUTATIONS) },
                        { signal },
                    ),
                );
                await this.#record(result.outcomes, result.watermark);
                failures = 0;
            } catch (error) {
                // stop on abort or a final failure, retrying a concurrent writer's conflict
                const isConflict = (error as { code?: unknown } | null)?.code === "CONFLICT";
                if (signal.aborted) {
                    return;
                } else if (!isConflict) {
                    report(error);
                    if (Journal.failure(error) !== undefined) {
                        return;
                    }
                }

                // back off
                failures += 1;
                await RetryPolicy.pause(this.#retry, failures, signal);
            }
        }
    }

    /** Follow every party's queries until aborted. */
    async follow(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        this.#report = report;
        await Promise.all([
            this.#follow("durable", signal, report),
            this.ephemeral === undefined ? undefined : this.#follow("ephemeral", signal, report),
        ]);
    }

    /** Follow every party's queries of one storage into its replica. */
    async #follow(
        storage: ObjectStorage,
        signal: AbortSignal,
        report: (error: unknown) => void,
    ): Promise<void> {
        // pick the storage's replica and outbox
        const isDurable = storage === "durable";
        const replica = isDurable ? this.replica : this.ephemeral!;
        const outbox = isDurable ? this.outbox : undefined;
        await replica.register(this.database);
        let failures = 0;
        while (!signal.aborted) {
            // drop expired closed queries
            await this.database
                .delete(subscription)
                .where(lte(sql`${subscription.releasedAt} + ${subscription.keep}`, Date.now()));

            // restart once the queries change
            const queries = await this.#queries(storage);
            const reshaped = new AbortController();
            const stream = AbortSignal.any([signal, reshaped.signal, this.#moves.signal]);
            void this.database.log
                .until(async () => !isSame(await this.#queries(storage), queries), stream)
                .then(
                    (isChanged) => isChanged && reshaped.abort(),
                    (error: unknown) => {
                        // report a failed watch and restart
                        if (!stream.aborted) {
                            report(error);
                            reshaped.abort();
                        }
                    },
                );
            const service = this.#service;
            let isFinal = false;
            try {
                // sync from the copy's position and queries
                const after = await replica.position(this.database);
                const held = await replica.holding(this.database);
                const isReshape = held !== undefined && !isSame(held, queries);
                const pages = await service.sync(
                    {
                        scope: this.scope,
                        queries,
                        ...(isReshape ? { previous: held as Record<string, ObjectQuery> } : {}),
                        ...(after === undefined ? {} : { after }),
                        ...(this.#refresh === undefined ? {} : { refresh: this.#refresh }),
                        ...(isDurable ? {} : { client: this.clientId }),
                    },
                    { signal: stream },
                );
                let isHeld = false;
                let isGrown = false;
                for await (const page of replica.apply(this.database, pages, outbox)) {
                    // reset failures and deliver broadcasts
                    failures = 0;
                    for (const { topic, event } of page.broadcasts ?? []) {
                        for (const listener of this.#listeners.get(topic) ?? []) {
                            listener(event);
                        }
                    }

                    // compact the local log
                    await this.database.log.compact(Date.now() - this.#logMilliseconds);

                    // record held queries and evict
                    if (page.complete && !isHeld) {
                        isHeld = true;
                        await replica.hold(this.database, queries);
                    }
                    isGrown ||=
                        page.reset || page.changes.some((change) => change.operation === "insert");
                    if (page.complete && isDurable && isGrown) {
                        isGrown = false;
                        await this.#queueEviction();
                    }
                }
            } catch (error) {
                // restart on change or move, report other failures
                if (signal.aborted) {
                    return;
                } else if (stream.aborted || this.#redirect(error, service)) {
                    continue;
                }
                report(error);
                isFinal = Journal.failure(error) !== undefined;
            } finally {
                reshaped.abort();
            }

            // wait for a change or move after a final failure
            if (isFinal) {
                await this.database.log.until(
                    async () => !isSame(await this.#queries(storage), queries),
                    AbortSignal.any([signal, this.#moves.signal]),
                );
                continue;
            }

            // back off
            failures += 1;
            await RetryPolicy.pause(this.#retry, failures, signal);
        }
    }

    /** Push the outbox and follow subscriptions until aborted. */
    async run(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        await Promise.all([this.push(signal, report), this.follow(signal, report)]);
    }

    /** Follow one text field of an object as its sequence, changing it by visible offsets. */
    text<Object extends ObjectType>(object: Object, id: string, field: string): LiveText {
        // follow the field's chunks in position order
        const chunks = this.objects.find((held) => held.table === chunk);
        if (chunks === undefined || !object.text.includes(field)) {
            throw new TypeError(`object ${object.name} holds no text field ${field}`);
        }
        const query = this.subscribe(chunks, {
            where: Condition.all(
                Condition.eq("parentType", object.name),
                Condition.eq("parentId", id),
                Condition.eq("field", field),
            ),
            order: [{ column: "position", direction: "asc" }],
        });
        const sequence = (rows: readonly Readonly<Record<string, unknown>>[]) =>
            new Sequence(rows.flatMap((row) => row.runs as readonly Run[]));

        // apply changes one after another, each against the text the previous one left
        let previous: Promise<unknown> = Promise.resolve();
        const change = (replaced: TextChange) => {
            const next = previous.then(async () => {
                // translate the offsets against the current text, then edit and wait for the prediction
                const edits = sequence(await query.read()).change(replaced, v7());
                const mutator = this.mutate(object) as unknown as Readonly<
                    Record<string, (input: object) => Submission<Edited>>
                >;
                const editing = mutator.edit!({ id, field, edits });
                await editing.predicted;

                return editing;
            });
            previous = next.catch(() => {});

            return next;
        };

        return {
            ready: query.ready,
            read: async () => sequence(await query.read()),
            watch: async function* (signal) {
                for await (const rows of query.watch(signal)) {
                    yield sequence(rows);
                }
            },
            change,
            close: () => query.close(),
        };
    }

    /** Keep a query live until closed, reading the local copy with predictions. */
    subscribe<Object extends ObjectType>(
        object: Object,
        query: ObjectInclude = {},
        options: { readonly keep?: Duration | "always" } = {},
    ): LiveQuery {
        // split the query by storage
        if (!this.objects.some((held) => held.same(object))) {
            throw new TypeError(`the client holds no object ${object.name}`);
        }
        const part = this.#part(object, query);
        const whole = ObjectQuery.parse({ object: object.name, ...query });
        const compiled = Object.values(
            ObjectType.queries(this.objects, { query: whole }, this.scope),
        )[0]!;

        // follow the part and its lookups
        const closed = new AbortController();
        const holding = this.#hold(
            part,
            undefined,
            AbortSignal.any([closed.signal, this.#closing.signal]),
        );
        this.#holdings.add(holding);
        holding.ready.catch(() => {});

        // read rows or groups
        const isDurable = object.storage === "durable";
        const replica = isDurable ? this.replica : this.ephemeral!;
        const read = async () => {
            const name = await holding.named;

            return compiled.aggregate === undefined
                ? Chunk.present(await this.#rows(name, compiled), compiled, this.objects)
                : replica.results(
                      this.database,
                      name,
                      compiled,
                      isDurable ? this.outbox : undefined,
                  );
        };

        return {
            ready: holding.ready,
            read,
            watch: (signal) => this.#watch(holding.named, compiled, signal),
            close: async () => {
                // stop following or mark released
                closed.abort();
                await holding.done;
                this.#holdings.delete(holding);
                const ids = holding.ids();
                if (options.keep === undefined) {
                    await this.database.delete(subscription).where(inArray(subscription.id, ids));
                } else {
                    await this.database
                        .update(subscription)
                        .set({
                            releasedAt: Date.now(),
                            keep:
                                options.keep === "always"
                                    ? null
                                    : Duration.milliseconds(options.keep),
                        })
                        .where(inArray(subscription.id, ids));
                }

                // evict after closing an evictable query
                if (options.keep !== undefined && options.keep !== "always") {
                    await this.#queueEviction();
                }
            },
        };
    }

    /** Describe the client for inspection. */
    async inspect(): Promise<ClientInspection> {
        // count rows per object type
        const rows: Record<string, number> = {};
        for (const object of this.objects) {
            const [counted] = await this.database
                .select({ rows: count() })
                .from(object.table as Table);
            rows[object.name] = counted!.rows;
        }

        // list subscriptions
        const subscriptions = await this.database
            .select({
                name: subscription.name,
                origin: subscription.origin,
                releasedAt: subscription.releasedAt,
                keep: subscription.keep,
            })
            .from(subscription)
            .orderBy(asc(subscription.id));

        return {
            scope: this.scope,
            rows,
            ...(this.#storage === undefined ? {} : { storage: this.#storage }),
            subscriptions: subscriptions.map((entry) => ({
                name: entry.name,
                origin: entry.origin,
                ...(entry.releasedAt === null
                    ? {}
                    : { releasedAt: entry.releasedAt, keep: entry.keep ?? ("always" as const) }),
            })),
            replica: await this.replica.inspect(this.database),
            outbox: await this.outbox.inspect(this.database),
            feed: this.#local.inspect(),
        };
    }

    /** Stop following a closed query, even one kept always. */
    async release<Object extends ObjectType>(
        object: Object,
        query: ObjectInclude = {},
    ): Promise<void> {
        // find the query's closed subscriptions
        const part = this.#part(object, query);
        const name = (await digest(part.query)).slice(0, QUERY_NAME_LENGTH);
        const closed = await this.database
            .select({ id: subscription.id })
            .from(subscription)
            .where(
                and(
                    eq(subscription.name, name),
                    isNull(subscription.root),
                    isNotNull(subscription.releasedAt),
                ),
            );

        // delete them and their lookups
        const ids = closed.map((row) => row.id);
        await this.database
            .delete(subscription)
            .where(or(inArray(subscription.id, ids), inArray(subscription.root, ids)));
    }

    /** Read the parties holding subscriptions or mutations in the shared copy. */
    async origins(): Promise<Set<string>> {
        const rows = await this.database
            .selectDistinct({ origin: subscription.origin })
            .from(subscription);
        const outbox = await this.outbox.origins(this.database);

        return new Set([...rows.map((row) => row.origin), ...outbox]);
    }

    /** Forget a gone party's subscriptions and unread rejections. */
    async forget(origin: string): Promise<void> {
        await this.database.delete(subscription).where(eq(subscription.origin, origin));
        await this.outbox.forget(this.database, origin);
    }

    /** Fill an executed mutation's missing step results from the server. */
    async #settleSteps(mutationId: string, value: unknown): Promise<void> {
        // read the mutation's entry
        const [entry] = await this.database
            .select()
            .from(undoEntry)
            .where(eq(undoEntry.mutationId, mutationId));
        if (entry === undefined || !Array.isArray(value)) {
            return;
        }

        // fill missing results in call order
        const steps = (entry.steps as StoredStep[]).map((step, position) =>
            step.result === undefined && value[position] !== undefined
                ? { ...step, result: value[position] }
                : step,
        );
        await this.database
            .update(undoEntry)
            .set({ steps: steps as never })
            .where(eq(undoEntry.id, entry.id));
    }

    /** Record pushed mutations' outcomes and revert rejected predictions. */
    async #record(outcomes: readonly PushedOutcome[], watermark: LogPosition): Promise<void> {
        // sort outcomes
        const rejected: { readonly id: string; readonly error: unknown }[] = [];
        for (const { id, outcome } of outcomes) {
            // acknowledge an executed mutation
            if ("value" in outcome) {
                await this.outbox.acknowledge(this.database, id, watermark);
                await this.#settleSteps(id, outcome.value);
            }
            // collect a rejected one
            else {
                rejected.push({ id, error: outcome.error });
            }
        }

        // revert and replay after rejections
        if (rejected.length > 0) {
            await this.database.transaction(async (transaction) => {
                // note rejections and drop their undo entries
                for (const { id, error } of rejected) {
                    await this.outbox.reject(transaction, id, error);
                }
                await transaction.delete(undoEntry).where(
                    inArray(
                        undoEntry.mutationId,
                        rejected.map((entry) => entry.id),
                    ),
                );
                await transaction.log.copying(() => this.outbox.revert(transaction));
                await this.outbox.replay(transaction, []);
            });
        }
    }

    /** Wait for a mutation's outcome, failing with the server's rejection. */
    async #outcome(id: string): Promise<void> {
        // wait for the settle loop
        const outcome = await new Promise<sync.Outcome>((resolve, reject) => {
            this.#waiting.set(id, { resolve, reject });
            if (this.#settling === undefined) {
                this.#settling = this.#settle();
            }
        });

        // throw a rejection once
        if (outcome.kind === "rejected") {
            await this.outbox.forget(this.database, this.origin, id);
            const { code, status, message, data } = Failure.parse(outcome.error);
            throw new ServiceError(code, { status, message, data });
        }
    }

    /** Settle waiting mutations as their outcomes arrive. */
    async #settle(): Promise<void> {
        // settle known outcomes
        let failure: Error | undefined;
        try {
            const isSettled = await this.database.log.until(async () => {
                const outcomes = await this.outbox.outcomes(this.database, [
                    ...this.#waiting.keys(),
                ]);
                for (const [id, outcome] of outcomes) {
                    if (outcome.kind !== "pending") {
                        this.#waiting.get(id)!.resolve(outcome);
                        this.#waiting.delete(id);
                    }
                }

                return this.#waiting.size === 0;
            }, this.#closing.signal);
            if (!isSettled) {
                failure = new Error("the client closed before its mutations settled");
            }
        } catch (error) {
            failure = error instanceof Error ? error : new Error(String(error));
        }

        // fail the rest, or restart for late waiters
        this.#settling = undefined;
        if (failure !== undefined) {
            for (const { reject } of this.#waiting.values()) {
                reject(failure);
            }
            this.#waiting.clear();
        } else if (this.#waiting.size > 0) {
            this.#settling = this.#settle();
        }
    }

    /** Queue an eviction. */
    #queueEviction(): Promise<void> {
        const eviction = this.#evictions.then(async () => {
            // report an overflow
            const overflow = await this.#evict();
            if (overflow !== undefined && this.#report !== undefined) {
                this.#report(overflow);
            } else if (overflow !== undefined) {
                throw overflow;
            }
        });
        this.#evictions = eviction.catch(() => {});

        return eviction;
    }

    /** Drop the oldest closed kept query while the copy exceeds its budget. */
    async #evict(): Promise<sync.SyncError | undefined> {
        // evict only a budgeted copy holding the current queries
        const held = await this.replica.holding(this.database);
        if (
            this.#storage === undefined ||
            held === undefined ||
            !isSame(held, await this.#queries("durable"))
        ) {
            return undefined;
        }

        // count rows
        const rows = await this.#count();
        if (rows <= this.#storage.rows) {
            return undefined;
        }

        // drop the oldest closed query, or report an overflow
        const [oldest] = await this.database
            .select({ id: subscription.id })
            .from(subscription)
            .where(
                and(
                    isNull(subscription.root),
                    isNotNull(subscription.releasedAt),
                    isNotNull(subscription.keep),
                ),
            )
            .orderBy(asc(subscription.releasedAt))
            .limit(1);
        if (oldest === undefined) {
            return new sync.SyncError(
                "OVER_CAPACITY",
                `local copy holds ${rows} rows, more than ${this.#storage.rows}, for open and always kept queries`,
            );
        }
        await this.database
            .delete(subscription)
            .where(or(eq(subscription.id, oldest.id), eq(subscription.root, oldest.id)));

        return undefined;
    }

    /** Count the copy's object rows. */
    async #count(): Promise<number> {
        // count on from the last count through the log
        const tables = this.objects.map((object) => object.table as Table);
        if (this.#counted !== undefined) {
            try {
                let { sequence, rows } = this.#counted;
                for (let reached = sequence; ; reached = sequence) {
                    const page = await this.database.log.read({ tables, after: reached });
                    for (const change of page.changes) {
                        rows +=
                            change.operation === "insert"
                                ? 1
                                : change.operation === "delete"
                                  ? -1
                                  : 0;
                    }
                    sequence = Math.max(sequence, page.sequence);
                    if (page.sequence <= reached) {
                        break;
                    }
                }
                this.#counted = { sequence, rows };

                return rows;
            } catch (error) {
                // recount after compaction
                if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }

        // count every table
        const counted = await this.database.transaction(
            async (transaction) => {
                // read sequence and counts together
                const sequence = (await transaction.log.position()).sequence;
                const counts = this.objects.map(
                    (object) => sql`(SELECT count(*) FROM ${object.table})`,
                );
                const [total] = await transaction.execute<{ rows: number }>(
                    sql`SELECT ${sql.join(counts, sql` + `)} AS rows`,
                );

                return { sequence, rows: Number(total!.rows) };
            },
            { isReadOnly: true },
        );
        this.#counted = counted;

        return counted.rows;
    }

    /** Read every party's followed queries of one storage by name. */
    async #queries(storage: ObjectStorage): Promise<Record<string, ObjectQuery>> {
        const rows = await this.database
            .select({ name: subscription.name, query: subscription.query })
            .from(subscription)
            .where(
                or(
                    isNull(subscription.releasedAt),
                    isNull(subscription.keep),
                    gt(sql`${subscription.releasedAt} + ${subscription.keep}`, Date.now()),
                ),
            )
            .orderBy(asc(subscription.id));

        // filter by storage
        const stored = new Set(
            this.objects
                .filter((object) => object.storage === storage)
                .map((object) => object.name),
        );

        return Object.fromEntries(
            rows
                .map((row) => [row.name, ObjectQuery.parse(row.query)] as const)
                .filter(([, query]) => stored.has(query.object)),
        );
    }

    /** Keep a live union of several object types in one order and limit. */
    union(
        members: Readonly<
            Record<
                string,
                Omit<ObjectInclude, "order" | "limit" | "aggregate"> & {
                    readonly object: ObjectType;
                }
            >
        >,
        options: { readonly order: Order; readonly limit: number; readonly keep?: Duration },
    ): LiveQuery {
        // follow each member
        const followed = Object.entries(members).map(([name, { object, ...query }]) => ({
            name,
            table: object.table as Table,
            live: this.subscribe(
                object,
                { ...query, order: options.order, limit: options.limit },
                options.keep === undefined ? {} : { keep: options.keep },
            ),
        }));
        const merge = (lists: readonly (readonly Row[])[]) =>
            Order.merge(
                options.order,
                followed.map(({ name, table }, index) => ({ name, table, rows: lists[index]! })),
                options.limit,
            );

        return {
            ready: Promise.all(followed.map(({ live }) => live.ready)).then(() => {}),
            read: async () => merge(await Promise.all(followed.map(({ live }) => live.read()))),
            watch: (signal) =>
                Observable.latest(
                    followed.map(({ live }) => live.watch(signal)),
                    merge,
                ),
            close: async () => {
                await Promise.all(followed.map(({ live }) => live.close()));
            },
        };
    }

    /** Watch a query's result through the local feed until aborted. */
    async *#watch(
        named: Promise<string>,
        query: Omit<sync.Query, "scopes">,
        signal: AbortSignal,
    ): AsyncGenerator<readonly Readonly<Record<string, unknown>>[]> {
        const name = await named;
        for await (const rows of this.#local.watch(
            name,
            { ...query, scopes: [this.scope] },
            signal,
        )) {
            yield query.aggregate === undefined ? Chunk.present(rows, query, this.objects) : rows;
        }
    }

    /** Read a query's rows from the local copy. */
    async #rows(name: string, query: Omit<sync.Query, "scopes">): Promise<Row[]> {
        const dataflow = new sync.Dataflow(
            { [name]: { ...query, scopes: [this.scope] } },
            {
                audience: sync.EVERYONE,
                database: this.database,
                upstream: this.#local.upstream!,
                changesThrough: sync.changesThroughLog(this.database),
                isMaterialized: true,
            },
        );
        await dataflow.fill(await sync.View.latest(this.database));

        return dataflow.read(name);
    }

    /** Split a query into its storage's part and lookups of the other storage. */
    #part(object: ObjectType, shape: ObjectInclude): Part {
        // require one storage
        const { include, via: _via, ...own } = shape;
        this.#requireOneStorage(object, own);
        const lookups: Lookup[] = [];

        // keep same-storage includes
        const kept: Record<string, ObjectInclude> = {};
        for (const [name, nested] of Object.entries(include ?? {})) {
            const joined = object.join(nested.via ?? name, this.objects);
            if (joined.object.storage === object.storage) {
                const inner = this.#part(joined.object, nested);
                const { object: _object, ...innerShape } = inner.query;
                kept[name] = {
                    ...innerShape,
                    ...(nested.via === undefined ? {} : { via: nested.via }),
                };
                lookups.push(
                    ...inner.lookups.map((lookup) => ({ ...lookup, path: [name, ...lookup.path] })),
                );
            }
            // look up other-storage objects by key
            else if (joined.on.kind === "key") {
                const { order: _order, limit: _limit, aggregate: _aggregate, ...rows } = nested;
                lookups.push({
                    path: [],
                    column: joined.on.column,
                    parent: joined.on.parent,
                    ...(joined.where === undefined ? {} : { where: joined.where }),
                    part: this.#part(joined.object, rows),
                });
            } else {
                throw new ServiceError("BAD_REQUEST", {
                    message: `object ${object.name} includes ${name} of another storage only by key`,
                });
            }
        }

        return {
            object,
            query: ObjectQuery.parse({
                object: object.name,
                ...own,
                ...(Object.keys(kept).length === 0 ? {} : { include: kept }),
            }),
            lookups,
        };
    }

    /** Refuse conditions and computed values relating objects of another storage. */
    #requireOneStorage(object: ObjectType, shape: ObjectInclude): void {
        const relations = [
            ...Condition.relations(shape.where ?? Condition.all()),
            ...Object.values(shape.compute ?? {}).flatMap((expression) => [
                ...Expression.lookups(expression),
                ...Expression.rollups(expression),
            ]),
        ];
        for (const { via } of relations) {
            if (object.join(via, this.objects).object.storage !== object.storage) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `object ${object.name} relates no ${via} of another storage`,
                });
            }
        }
    }

    /** Follow a part and the lookups its rows key. */
    #hold(part: Part, root: string | undefined, signal: AbortSignal): Holding {
        // subscribe the part's query
        const id = RequestId.create();
        const named = (async () => {
            const name = (await digest(part.query)).slice(0, QUERY_NAME_LENGTH);
            await this.database.insert(subscription).values({
                id,
                origin: this.origin,
                name,
                root: root ?? null,
                query: schema.json().parse(part.query),
            });

            return name;
        })();

        // wait until the copy holds it
        const replica = part.object.storage === "durable" ? this.replica : this.ephemeral!;
        const held = named.then(async (name) => {
            await this.database.log.until(
                async () => holds(await replica.holding(this.database), name),
                signal,
            );
        });
        const lookups = new Map<
            Lookup,
            { readonly keys: string; readonly holding: Holding; readonly stop: AbortController }
        >();
        const ids = () => [id, ...[...lookups.values()].flatMap((entry) => entry.holding.ids())];
        if (part.lookups.length === 0) {
            return { named, ready: held, done: named.then(() => {}), ids };
        }

        // key lookups from the part's rows
        const compiled = Object.values(
            ObjectType.queries(this.objects, { query: part.query }, this.scope),
        )[0]!;
        const { promise: ready, resolve: isReady } = Promise.withResolvers<void>();
        const done = (async () => {
            // watch the part's rows
            const name = await named;
            const owner = root ?? id;
            let isFirst = true;
            try {
                for await (const rows of this.#local.watch(
                    name,
                    { ...compiled, scopes: [this.scope] },
                    signal,
                )) {
                    for (const lookup of part.lookups) {
                        // skip unchanged lookups
                        const keys = keysOf(rows, lookup);
                        const key = canonicalize(keys);
                        const current = lookups.get(lookup);
                        if (current?.keys === key) {
                            continue;
                        }

                        // replace the lookup
                        if (current !== undefined) {
                            current.stop.abort();
                            await current.holding.done;
                            await this.database
                                .delete(subscription)
                                .where(inArray(subscription.id, current.holding.ids()));
                            lookups.delete(lookup);
                        }
                        if (keys.length > 0) {
                            const stop = new AbortController();
                            const holding = this.#hold(
                                lookupPart(lookup, keys),
                                owner,
                                AbortSignal.any([signal, stop.signal]),
                            );
                            lookups.set(lookup, { keys: key, holding, stop });
                        }
                    }

                    // signal ready after the first lookups
                    if (isFirst) {
                        isFirst = false;
                        await held;
                        await Promise.all(
                            [...lookups.values()].map((entry) => entry.holding.ready),
                        );
                        isReady();
                    }
                }
            } catch (error) {
                // rethrow unless stopped
                if (!signal.aborted) {
                    throw error;
                }
            } finally {
                // wait for the lookups to stop
                await Promise.all([...lookups.values()].map((entry) => entry.holding.done));
            }
        })();
        done.catch((error: unknown) => {
            // report a failed lookup
            this.#report?.(error);
        });

        return { named, ready: Promise.race([ready, done.then(() => held)]), done, ids };
    }

    /** Predict a pending mutation again after a rebase. */
    async #replay(transaction: DatabaseConnection, pending: sync.Mutation): Promise<void> {
        try {
            await transaction.transaction(async (savepoint) => {
                for (const entry of pending.calls) {
                    const { object, name } = this.#method(entry.method);
                    await this.#predict(savepoint, object, name, Call.upcast(object, name, entry));
                }
            });
        } catch (error) {
            // keep a refused mutation pending unpredicted
            if (!(error instanceof ServiceError)) {
                throw error;
            }
        }
    }

    /** Predict one call as a step with the target's row before and after. */
    async #step(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
        isInverse = false,
    ): Promise<{ readonly step: StoredStep; readonly result: unknown }> {
        // read the target around the prediction
        const id = typeof input.id === "string" ? input.id : undefined;
        const before = id === undefined ? undefined : await this.#row(database, object, id);
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
        const isHeld = !method.target || before !== undefined;
        const result =
            isInverse && !isHeld ? undefined : await this.#predict(database, object, name, input);
        const created = id ?? Call.resultId(result);
        const after =
            created === undefined ? undefined : await this.#row(database, object, created);
        const step: StoredStep = {
            object: object.name,
            version: object.version,
            name,
            input,
            ...(result === undefined ? {} : { result }),
            ...(before === undefined ? {} : { before }),
            ...(after === undefined ? {} : { after }),
        };

        return { step, result };
    }

    /** Read an object's row as the client holds it, absent when it holds none. */
    async #row(
        database: DatabaseConnection,
        object: ObjectType,
        id: string,
    ): Promise<Readonly<Record<string, unknown>> | undefined> {
        const table = object.table as Table & Record<string, never>;
        const [row] = (await database.select().from(table).where(eq(table.id, id))) as Record<
            string,
            unknown
        >[];

        return row;
    }

    /** Push an undoable mutation onto its party's undo stack and clear the redo stack. */
    async #remember(
        database: DatabaseConnection,
        id: string,
        steps: readonly StoredStep[],
    ): Promise<void> {
        // clear the redo stack
        const ownEntries = eq(undoEntry.origin, this.origin);
        await database.delete(undoEntry).where(and(ownEntries, eq(undoEntry.state, "undone")));

        // require an inverse for every call
        const isUndoable =
            steps.length > 0 &&
            steps.every((step) => this.#method(`${step.object}.${step.name}`).method.inverse);
        if (!isUndoable) {
            return;
        }
        const [last] = await database
            .select({ sequence: max(undoEntry.sequence) })
            .from(undoEntry)
            .where(ownEntries);
        await database.insert(undoEntry).values({
            id,
            origin: this.origin,
            sequence: (last?.sequence ?? 0) + 1,
            state: "done",
            steps: steps as never,
            mutationId: id,
        });
    }

    /** Invert the party's latest done or undone entry as one new mutation. */
    #toggle(from: "done" | "undone", to: "done" | "undone"): Submission<boolean> {
        // chain after the previous toggle
        const mutationId = RequestId.create();
        const predicted = this.#toggling.then(() => this.#invert(from, to, mutationId));
        this.#toggling = predicted.catch(() => {});
        const confirmed = predicted.then(async (isApplied) => {
            if (isApplied) {
                await this.#outcome(mutationId);
            }
        });
        confirmed.catch(() => {});

        return { predicted, confirmed };
    }

    /** Submit the inverse of the party's latest entry in a state, returning whether one existed. */
    async #invert(
        from: "done" | "undone",
        to: "done" | "undone",
        mutationId: string,
    ): Promise<boolean> {
        while (true) {
            // take the latest entry
            const [entry] = await this.database
                .select()
                .from(undoEntry)
                .where(and(eq(undoEntry.origin, this.origin), eq(undoEntry.state, from)))
                .orderBy(from === "done" ? desc(undoEntry.sequence) : asc(undoEntry.sequence))
                .limit(1);
            if (entry === undefined) {
                return false;
            }

            // invert its steps in reverse order
            const calls: sync.Call[] = [];
            for (const stored of [...(entry.steps as StoredStep[])].reverse()) {
                const { object, name, method } = this.#method(`${stored.object}.${stored.name}`);
                if ((stored.version ?? 1) !== object.version) {
                    calls.length = 0;
                    break;
                }
                const id = stored.after?.id ?? stored.input.id;
                const current =
                    typeof id === "string" ? await this.#row(this.database, object, id) : undefined;
                const step: Step = {
                    ...stored,
                    object,
                    name,
                    ...(current === undefined ? {} : { current }),
                };
                calls.push(...(method.inverse?.(step) ?? []));
            }
            if (calls.length === 0) {
                await this.database.delete(undoEntry).where(eq(undoEntry.id, entry.id));
                continue;
            }

            // submit the inverse as one mutation
            await this.outbox.add(this.database, mutationId, this.origin, async (database) => {
                // predict each inverse call
                const steps: StoredStep[] = [];
                for (const call of calls) {
                    const { object, name } = this.#method(call.method);
                    steps.push((await this.#step(database, object, name, call.input, true)).step);
                }
                await database
                    .update(undoEntry)
                    .set({ steps: steps as never, state: to, mutationId })
                    .where(eq(undoEntry.id, entry.id));

                return { calls, result: undefined };
            });

            return true;
        }
    }

    /** Predict one call over the local replica. */
    async #predict(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): Promise<unknown> {
        // skip unpredicted methods
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
        if (!method.isPredicted) {
            return undefined;
        }

        // split off routing fields
        const { id, revision, ...rest } = input;
        const fields = this.#field === undefined ? rest : omitField(rest, this.#field);
        const targetId = id === undefined ? undefined : schema.string().parse(id);
        const call = new Call({
            object,
            name,
            method,
            scope: this.scope,
            input: fields,
            ...(targetId === undefined ? {} : { id: targetId }),
            database,
            caller: this.caller,
            now: Date.now(),
            isPredicted: true,
            objects: this.objects,
            run: (invoked, invokedName, invokedInput) =>
                this.#predict(database, invoked, invokedName, invokedInput),
        });

        // load the target at the named revision
        const table = object.table as Table & Record<string, never>;
        const [target] =
            method.target && targetId !== undefined
                ? await database.select().from(table).where(eq(table.id, targetId))
                : [];
        if (method.target && target === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${targetId}` });
        }
        if (
            revision !== undefined &&
            (target as { revision?: unknown } | undefined)?.revision !== revision
        ) {
            throw new ServiceError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        return method.execute(target === undefined ? call : call.with({ target: target as never }));
    }

    /** Call the replica procedures, forwarding a call once to a moved scope's new holder. */
    async #call<Value>(
        send: (service: Client<ReplicaProcedures>) => Promise<Value>,
    ): Promise<Value> {
        const service = this.#service;
        try {
            return await send(service);
        } catch (error) {
            // retry once after a move
            if (!this.#redirect(error, service)) {
                throw error;
            }

            return send(this.#service);
        }
    }

    /** Reconnect to a moved scope's new holder, returning whether the failure was a move. */
    #redirect(error: unknown, service: Client<ReplicaProcedures>): boolean {
        // ignore other failures
        const moved = ScopeHolder.of(error);
        if (moved === undefined) {
            return false;
        }

        // reconnect once per move
        if (service === this.#service) {
            this.#service = this.#reconnect(moved.holder);
            this.#moves.abort();
            this.#moves = new AbortController();
        }

        return true;
    }

    /** Send one ephemeral call as this client. */
    async #send(call: sync.Call): Promise<unknown> {
        // push the call alone
        const { outcomes } = await this.#call((service) =>
            service.push({
                scope: this.scope,
                mutations: [{ id: RequestId.create(), calls: [call] }],
                client: this.clientId,
            }),
        );

        // throw a failure or return the result
        const outcome = outcomes[0]!.outcome;
        if (!("value" in outcome)) {
            const { code, status, message, data } = Failure.parse(outcome.error);
            throw new ServiceError(code, { status, message, data });
        }

        return (outcome.value as unknown[])[0];
    }

    /** Record a call with its scope and a minted identifier for creations. */
    #recordCall(
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
    ): sync.Call {
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
        const minted =
            method.kind === "create" && input.id === undefined
                ? { id: `${object.identity}-${v7()}` }
                : {};

        return Call.record(object, name, { ...input, ...minted, ...this.#scoped() });
    }

    /** Name the client's scope in its route field. */
    #scoped(): Record<string, string> {
        return this.#field === undefined ? {} : { [this.#field]: this.scope };
    }

    /** Name the tables a call's prediction may read, by SQL name. */
    #reach(call: sync.Call, tables: readonly Table[]): readonly string[] {
        const { object, method } = this.#method(call.method);
        const parent = object.parent?.object;

        return STANDARD_KINDS.has(method.kind) && parent !== "any"
            ? [object.table, ...(parent === undefined ? [] : [parent.table])].map(
                  (table) => (table as Table)[TABLE].sqlName,
              )
            : tables.map((table) => table[TABLE].sqlName);
    }

    /** Find the object type and method a recorded call names. */
    #method(named: string): {
        readonly object: ObjectType;
        readonly name: string;
        readonly method: Method;
    } {
        // split the name
        const separator = named.lastIndexOf(".");
        const object = this.objects.find((entry) => entry.name === named.slice(0, separator));
        if (!object) {
            throw new TypeError(`the client holds no object ${named.slice(0, separator)}`);
        }
        const name = named.slice(separator + 1);

        return {
            object,
            name,
            method: (object.methods as Readonly<Record<string, Method>>)[name]!,
        };
    }

    /** Expose an object type's mutating or reading methods through one call function. */
    #methods(
        object: ObjectType,
        mutates: boolean,
        call: (name: string, input: Readonly<Record<string, unknown>>) => unknown,
    ): object {
        // require a held object type
        if (!this.objects.some((held) => held.same(object))) {
            throw new TypeError(`the client holds no object ${object.name}`);
        }

        return Object.fromEntries(
            Object.entries(object.methods as Readonly<Record<string, Method>>)
                .filter(([, method]) => method.mutates === mutates && !method.isSystem)
                .map(([name]) => [
                    name,
                    (input: Readonly<Record<string, unknown>>) => call(name, input),
                ]),
        );
    }
}

/** What the server made of one pushed mutation. */
type PushedOutcome = Awaited<ReturnType<Client<ReplicaProcedures>["push"]>>["outcomes"][number];

/** The names of an object type's methods that change nothing. */
type ReadingName<Object extends ObjectType> = {
    [Name in CallableName<Object>]: Object["methods"][Name] extends { mutates: false }
        ? Name
        : never;
}[CallableName<Object>];

/** The names of an object type's mutating methods. */
type MutatingName<Object extends ObjectType> = {
    [Name in CallableName<Object>]: Object["methods"][Name] extends { mutates: true }
        ? Name
        : never;
}[CallableName<Object>];

/** The procedure one method derives. */
type ProcedureOf<
    Object extends ObjectType,
    Name extends CallableName<Object>,
> = ObjectProcedures<Object>[Name] & {
    readonly "~orpc": { readonly inputSchema: schema.Schema; readonly outputSchema: schema.Schema };
};

/** The input a client passes to a method. */
export type CallInput<Object extends ObjectType, Name extends CallableName<Object>> = Omit<
    schema.Input<ProcedureOf<Object, Name>["~orpc"]["inputSchema"]>,
    "requestId" | (Object["storage"] extends "ephemeral" ? "client" : never) | ScopeField<Object>
>;

/** The result a method returns. */
export type CallOutput<Object extends ObjectType, Name extends CallableName<Object>> = schema.Infer<
    ProcedureOf<Object, Name>["~orpc"]["outputSchema"]
>;

/** The queries each party sharing a client database follows. */
const subscription = defineTable("subscription", {
    /** The subscription's identifier, ordered by creation. */
    id: text("id").primaryKey(),
    /** The party that asked. */
    origin: text("origin").notNull(),
    /** The query's name: a prefix of its content's digest. */
    name: text("name").notNull(),
    /** The query. */
    query: json("query", schema.json()).notNull(),
    /** The subscription this lookup belongs to, absent for a party's own query. */
    root: text("root"),
    /** The time the query closed, in UTC epoch milliseconds, absent while open. */
    releasedAt: integer("released_at"),
    /** How long the closed query stays followed, in milliseconds, absent when kept until released. */
    keep: integer("keep"),
});

/** Each party's undoable mutations as steps. */
const undoEntry = defineTable("undo_entry", {
    /** The entry, named after the mutation that first applied its steps. */
    id: text("id").primaryKey(),
    /** The party the entry belongs to. */
    origin: text("origin").notNull(),
    /** The entry's place in its party's stack. */
    sequence: integer("sequence").notNull(),
    /** Whether the entry is on the undo or the redo stack. */
    state: text("state", { enum: ["done", "undone"] }).notNull(),
    /** The steps it last applied. */
    steps: json("steps", schema.json()).notNull(),
    /** The mutation that last applied them. */
    mutationId: text("mutation_id").notNull(),
});

/** A stored step, naming its object type. */
type StoredStep = Omit<Step, "object" | "current"> & {
    /** The object type's name. */
    readonly object: string;
    /** The object type's version the step was taken at, the first when absent. */
    readonly version?: number;
};

/** The part of a query one storage follows, and the lookups its rows key. */
interface Part {
    /** The object type. */
    readonly object: ObjectType;
    /** The query its storage follows, without the includes of the other storage. */
    readonly query: ObjectQuery;
    /** The lookups of the other storage below the query's rows. */
    readonly lookups: readonly Lookup[];
}

/** Other-storage objects a part's rows include by key. */
interface Lookup {
    /** The include path to the rows holding the keys. */
    readonly path: readonly string[];
    /** The looked-up objects' column holding a key. */
    readonly column: string;
    /** The holding rows' column naming the key. */
    readonly parent: string;
    /** The condition the join adds, such as an attachment's host type. */
    readonly where?: Condition;
    /** The looked-up objects' part, before keying. */
    readonly part: Part;
}

/** A part followed until closed, with the lookups its rows keyed so far. */
interface Holding {
    /** The name of the part's query, once subscribed. */
    readonly named: Promise<string>;
    /** Settles once the part's copy holds it and its first lookups are ready. */
    readonly ready: Promise<void>;
    /** Settles once the part stops, rejecting with a failed lookup. */
    readonly done: Promise<void>;
    /** List the subscriptions the part and its current lookups hold. */
    ids(): string[];
}

/** Read the distinct keys a lookup's holding rows name, in their JSON form, sorted. */
function keysOf(
    rows: readonly Readonly<Record<string, unknown>>[],
    lookup: Lookup,
): Exclude<Scalar, null>[] {
    // follow the include path
    let holding: Readonly<Record<string, unknown>>[] = [...rows];
    for (const name of lookup.path) {
        holding = holding.flatMap((row) => {
            const included = row[name];

            return Array.isArray(included)
                ? (included as Record<string, unknown>[])
                : included === null || included === undefined
                  ? []
                  : [included as Record<string, unknown>];
        });
    }

    // collect distinct keys
    const column = (lookup.part.object.table as Table)[TABLE].columns[lookup.column]!;
    const keys = new Set(
        holding
            .map((row) => row[lookup.parent])
            .filter((value) => value !== null && value !== undefined)
            .map((value) => column.definition.toJson(value) as Exclude<Scalar, null>),
    );

    return [...keys].sort((left, right) => String(left).localeCompare(String(right)));
}

/** Narrow a lookup's part to the given keys. */
function lookupPart(lookup: Lookup, keys: readonly Exclude<Scalar, null>[]): Part {
    const { where } = lookup.part.query;
    const conditions = [
        ...(where === undefined ? [] : [where]),
        Condition.oneOf(lookup.column, keys),
        ...(lookup.where === undefined ? [] : [lookup.where]),
    ];

    return {
        ...lookup.part,
        query: { ...lookup.part.query, where: Condition.all(...conditions) },
    };
}

/** Decide whether held queries include one by name. */
function holds(held: unknown, name: string): boolean {
    return typeof held === "object" && held !== null && Object.hasOwn(held, name);
}

/** Decide whether stored queries are the queries given. */
function isSame(held: unknown, queries: Readonly<Record<string, ObjectQuery>>): boolean {
    return canonicalize(held) === canonicalize(queries);
}

/** Leave out one field of an input. */
function omitField(
    input: Readonly<Record<string, unknown>>,
    field: string,
): Record<string, unknown> {
    const { [field]: _omitted, ...rest } = input;

    return rest;
}
