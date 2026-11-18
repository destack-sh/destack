import type { DatabaseConnection, Table } from "@destack/db";
import { Snapshot, TABLE } from "@destack/db";
import { Failure } from "@destack/error";
import { RetryPolicy } from "@destack/environment";
import type { Subject } from "@destack/sync";
import { canonicalize, found, type JsonValue, nth, present, schema, Uuid } from "@destack/schema";
import * as sync from "@destack/sync";
import { Branch, BranchType } from "../branch/index.ts";
import { CallError } from "../error/call.ts";
import { Replay } from "../audit/replay.ts";
import { ObjectType } from "../object/object.ts";
import { QUERIES_PUBLICATION, PUSH_MUTATIONS, type PushResult } from "../replica/replica.ts";
import { Shape } from "../replica/shape.ts";
import type { ObjectStore } from "./store.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** Milliseconds per second: the clock reads milliseconds, and durations record seconds. */
const MILLISECONDS = 1000;

/** The default retry of transient failures: a second, doubling, at most a minute. */
const RETRY = RetryPolicy.of({ maximumInterval: 60_000 });

/** The method kinds with predictions that read only their object's and parent's tables. */
const STANDARD_KINDS: ReadonlySet<string> = new Set([
    "create",
    "createMany",
    "update",
    "updateMany",
    "delete",
    "deleteMany",
]);

/** The results of a queued mutation's calls, in call order. */
const MUTATION_RESULTS = schema.array(schema.json());

/** A waiter for a queued mutation's outcome. */
interface OutcomeWaiter {
    /** Settle with the outcome. */
    readonly resolve: (outcome: sync.MutationState) => void;
    /** Fail once the outbox closes first. */
    readonly reject: (error: Error) => void;
}

/** A party queueing mutations here, such as a tab: its listeners and the outcomes none took yet. */
interface Party {
    /** The listeners of the party's outcomes, one of which takes each. */
    readonly listeners: Set<(outcomes: readonly sync.MutationOutcome[]) => void>;
    /** The outcomes no listener took yet, handed to the next one. */
    held: sync.MutationOutcome[];
}

/** How a store's owner tunes its outbox and reacts to its outcomes. */
export interface OutboxOptions {
    /** The branch types among the followed ones, absent for a node without branches. */
    readonly branch?: BranchType;
    /** The most mutations one push sends, the home's most by default. */
    readonly mutations?: number;
    /** How transient push failures retry, over the defaults. */
    readonly retry?: Partial<RetryPolicy>;
    /** Take an executed mutation's results, such as for undo. */
    readonly executed?: (
        transaction: DatabaseConnection,
        id: string,
        value: JsonValue,
    ) => Promise<void>;
    /** Drop what rejected mutations left in a transaction, such as their undo entries. */
    readonly rejected?: (transaction: DatabaseConnection, ids: readonly string[]) => Promise<void>;
    /** Wait until a party listening for outcomes closes, such as a tab, keeping it until then. */
    readonly closed?: (origin: string, signal: AbortSignal) => Promise<void>;
}

/** A store's writes of rows homed elsewhere: predicted, queued and pushed until settled. */
export class Outbox {
    /** What the replicas show over their homes' rows: the queued mutations and the branch. */
    readonly prediction: sync.Prediction;
    /** The store predicting and recording the mutations. */
    readonly #store: ObjectStore;
    /** How the store's owner tunes the outbox and reacts to outcomes. */
    readonly #options: OutboxOptions;
    /** The most mutations one push sends. */
    readonly #mutations: number;
    /** How transient push failures retry. */
    readonly #retry: RetryPolicy;
    /** The outbox's captures and spans, on the store's telemetry. */
    readonly #instruments: TelemetryScope;
    /** The seconds from queueing a waited mutation to its settlement or rejection. */
    readonly #confirms;
    /** Aborts the outcome waits once the outbox closes. */
    readonly #closing = new AbortController();
    /** The waiters for each queued mutation's outcome, by its identifier. */
    readonly #waiting = new Map<string, OutcomeWaiter[]>();
    /** The mutations a queueing receiver took, whose outcomes arrive on its pages. */
    readonly #queuedAt = new Set<string>();
    /** The parties that listened here and did not close yet, by origin. */
    readonly #parties = new Map<string, Party>();
    /** The running settle loop, absent while none wait. */
    #settling: Promise<void> | undefined;
    /** The results the home answered each executed mutation, until its waiters read them. */
    readonly #results = new Map<string, JsonValue>();

    /** Queue a store's writes of rows homed elsewhere, pushing them to their homes. */
    constructor(store: ObjectStore, options: OutboxOptions = {}) {
        // refuse a push larger than a home takes
        const mutations = options.mutations ?? PUSH_MUTATIONS;
        if (!Number.isInteger(mutations) || mutations < 1 || mutations > PUSH_MUTATIONS) {
            throw new TypeError(`a push needs 1 to ${PUSH_MUTATIONS} mutations`);
        }
        this.#store = store;
        this.#options = options;
        this.#mutations = mutations;
        this.#retry = { ...RETRY, ...options.retry };
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            store.environment.telemetry,
        );
        this.#confirms = this.#instruments.metric.histogram("destack.mutation.confirm.duration", {
            unit: "s",
            attributes: { "destack.mutation.outcome": ["settled", "rejected"] },
        });

        // predict over the followed durable types' tables, with the branch checked out over them
        const tables = store.replicated
            .filter((object) => object.storage === "durable")
            .map((object) => object.table);
        const { branch } = options;
        this.prediction = new sync.Prediction({
            tables,
            predict: (transaction, pending) => this.#predict(transaction, pending),
            reads: (call) => this.#reads(call, tables),
            ...(branch === undefined ? {} : { branches: this.#branches(branch) }),
        });
    }

    /** Predict and queue a mutation once for a party, which a push then sends toward its home. */
    async queue(
        mutation: { readonly id: string; readonly calls: readonly sync.Call[] },
        origin: string,
    ): Promise<void> {
        // predict and queue a mutation not queued already, as the principal with its groups at the home
        const { database } = this.#store;
        const states = await this.prediction.outcomes(database, [mutation.id]);
        if (found(states, mutation.id).kind === "settled") {
            await this.#instruments.span(
                "destack.mutation.predict",
                { "destack.mutation.calls": mutation.calls.length },
                async () => {
                    await this.prediction.add(
                        database,
                        mutation.id,
                        origin,
                        async (transaction) => {
                            // predict it as the principal with its groups at the mutation's home
                            const home = this.#home(nth(mutation.calls, 0));
                            const caller = await this.#store.callerContext(home, transaction);
                            await this.#store.predict(transaction, mutation, caller);

                            return { kind: "add", calls: mutation.calls, result: undefined };
                        },
                        { isMainLine: true },
                    );
                },
            );
        }
    }

    /** Wait for a party's queued mutation to end at its home, answering its calls' results. */
    async results(id: string, origin: string): Promise<readonly unknown[]> {
        const value = await this.outcome(id, origin);

        return MUTATION_RESULTS.parse(present(value, `the results of mutation '${id}'`));
    }

    /** Take how the homes ended the mutations this node queued at its device. */
    async receive(outcomes: readonly sync.MutationOutcome[]): Promise<void> {
        // record the results and run the owner's reactions in one transaction
        const { database } = this.#store;
        const arrived = outcomes.filter((each) => this.#queuedAt.has(each.id));
        await database.transaction(async (transaction) => {
            for (const { id, outcome } of arrived) {
                if ("value" in outcome) {
                    this.#results.set(id, outcome.value);
                    await this.#options.executed?.(transaction, id, outcome.value);
                } else {
                    await this.#options.rejected?.(transaction, [id]);
                }
            }
        });

        // settle the waiters, leaving the mutations to the settle loop no longer
        for (const { id, outcome } of arrived) {
            this.#queuedAt.delete(id);
            this.#end(
                id,
                "value" in outcome
                    ? { kind: "settled" }
                    : { kind: "rejected", error: outcome.error },
            );
        }
    }

    /** End a mutation's waiters with its outcome, recording how long it took from its queueing. */
    #end(id: string, outcome: sync.MutationState): void {
        // record the time from queueing to the outcome
        const seconds = (this.#store.clock.now() - Uuid.time(id)) / MILLISECONDS;
        this.#confirms.record(seconds, {
            "destack.mutation.outcome": outcome.kind === "rejected" ? "rejected" : "settled",
        });

        // end the waiters
        for (const waiter of this.#waiting.get(id) ?? []) {
            waiter.resolve(outcome);
        }
        this.#waiting.delete(id);
    }

    /** Listen to how the homes end the mutations a party queued here, until the returned stop. */
    listen(
        origin: string,
        listener: (outcomes: readonly sync.MutationOutcome[]) => void,
    ): (unsent: readonly sync.MutationOutcome[]) => void {
        // know a new party until it closes
        let party = this.#parties.get(origin);
        if (party === undefined) {
            party = { listeners: new Set(), held: [] };
            this.#parties.set(origin, party);
            this.#close(origin);
        }

        // listen, handing over what arrived while none listened
        party.listeners.add(listener);
        const held = party.held;
        party.held = [];
        if (held.length > 0) {
            listener(held);
        }

        // stop listening, handing the outcomes the listener could not send to the party again
        const listening = party;

        return (unsent) => {
            listening.listeners.delete(listener);
            this.#deliver(origin, unsent);
        };
    }

    /** Forget a party and the outcomes held for it once it closes. */
    #close(origin: string): void {
        const signal = this.#closing.signal;
        this.#options.closed?.(origin, signal).then(
            () => this.#parties.delete(origin),
            (error: unknown) => {
                if (!signal.aborted) {
                    this.#instruments.captureException(error, { isEscaped: true });
                }
            },
        );
    }

    /** Hand outcomes to one listener of their party, holding them while none listens. */
    #deliver(origin: string, outcomes: readonly sync.MutationOutcome[]): void {
        // hand them to a listener, else hold them for a party not closed yet
        const party = this.#parties.get(origin);
        const [listener] = party?.listeners ?? [];
        if (party === undefined || outcomes.length === 0) {
            return;
        } else if (listener !== undefined) {
            listener(outcomes);
        } else {
            party.held.push(...outcomes);
        }
    }

    /** Read the party a subject's queued mutations belong to, the anonymous one for none. */
    static origin(subject: Subject | undefined): string {
        return canonicalize(subject ?? null);
    }

    /** Push the queued mutations until aborted, retrying transient failures. */
    async push(signal: AbortSignal): Promise<void> {
        // push as this node's log, under which a queueing receiver publishes the outcomes
        const { database, clock } = this.#store;
        const origin = (await database.log.position()).log;
        let failures = 0;
        while (!signal.aborted) {
            // wait for pending mutations
            if (!(await this.prediction.wait(database, signal))) {
                return;
            }
            const pending = await this.prediction.send(database, { limit: this.#mutations });

            // push the leading mutations of one home, a branch's edits as one push to the branch
            const { home, mutations } = this.#leading(pending.map((entry) => this.#pushed(entry)));
            const written = new Map(
                pending.map((entry) => [
                    entry.id,
                    entry.calls.map((call) => this.#type(call).table),
                ]),
            );
            const edits = new Set(
                pending.flatMap((entry) => (entry.branch === undefined ? [] : [entry.id])),
            );
            const origins = new Map(pending.map((entry) => [entry.id, entry.origin]));
            try {
                const chain = await this.#chain(home);
                const remote = present(
                    await this.#store.remoteOf(nth(nth(mutations, 0).calls, 0)),
                    `the remote reaching '${home}'`,
                );
                await this.#instruments.span(
                    "destack.outbox.push",
                    { "destack.outbox.mutations": mutations.length },
                    async () => {
                        const result = await remote.push(
                            home,
                            { mutations, chain, origin },
                            signal,
                        );
                        await this.#record(result, edits, { home, written, origins });
                    },
                    { kind: "client" },
                );
                failures = 0;
            } catch (error) {
                // stop on abort or a final failure, retrying a concurrent writer's conflict and warning of other retries
                const isConflict = CallError.of(error)?.code === "CONFLICT";
                const isFinal = Replay.failure(error) !== undefined;
                if (signal.aborted) {
                    return;
                } else if (isFinal) {
                    this.#instruments.captureException(error, { isEscaped: true });

                    return;
                } else if (!isConflict) {
                    this.#instruments.captureException(error, { level: "warn" });
                }

                // back off, drawn from the leading mutation's request
                failures += 1;
                const leading = present(mutations[0], "the leading mutation");
                const drawn = RetryPolicy.draw(`${leading.id} ${failures}`);
                await RetryPolicy.pause(this.#retry, failures, clock, signal, () => drawn);
            }
        }
    }

    /** Wait for a mutation's outcome, answering what its home answered or its rejection. */
    async outcome(id: string, origin: string): Promise<JsonValue | undefined> {
        // wait for the settle loop
        const outcome = await new Promise<sync.MutationState>((resolve, reject) => {
            this.#waiting.set(id, [...(this.#waiting.get(id) ?? []), { resolve, reject }]);
            if (this.#settling === undefined) {
                this.#settling = this.#settle();
            }
        });

        // throw a rejection once, else answer what the home answered this outbox
        if (outcome.kind === "rejected") {
            await this.prediction.forget(this.#store.database, origin, id);
            throw Failure.error(outcome.error);
        }
        const results = this.#results.get(id);
        this.#results.delete(id);

        return results;
    }

    /** Close the outbox, failing the outcome waits. */
    close(): void {
        this.#closing.abort();
    }

    /** Predict a queued mutation again, leaving one its home refuses for good. */
    async #predict(transaction: DatabaseConnection, pending: sync.Mutation): Promise<void> {
        try {
            await transaction.transaction(async (savepoint) => {
                const home = this.#home(nth(pending.calls, 0));
                const caller = await this.#store.callerContext(home, savepoint);
                await this.#store.predict(savepoint, pending, caller);
            });
        } catch (error) {
            if (Replay.failure(error) === undefined) {
                throw error;
            }
        }
    }

    /** List the tables a call's prediction may read, by SQL name. */
    #reads(call: sync.Call, tables: readonly Table[]): readonly string[] {
        // read the type's and parent's tables for a standard method, else every predicted table
        const object = this.#type(call);
        const parent = object.parent?.object;
        const isStandard = STANDARD_KINDS.has(object.method(call.method).kind) && parent !== "any";
        const read = isStandard
            ? [object.table, ...(parent === undefined ? [] : [parent.table])]
            : tables;

        return read.map((table) => table[TABLE].sqlName);
    }

    /** Read the home a call runs at: the scope its type reads it in. */
    #home(call: sync.Call): string {
        return this.#type(call).readScope(call.method, call.input);
    }

    /** Find a call's followed type. */
    #type(call: sync.Call): ObjectType {
        return present(
            ObjectType.find(this.#store.replicated, call),
            `the followed type of '${call.type.type}'`,
        );
    }

    /** Read the scopes the replica of a home reads, nearest first. */
    #chain(home: string): Promise<readonly string[]> {
        const name = Shape.replicaName(QUERIES_PUBLICATION, home);

        return new sync.Replica({ name, scope: home, tables: [] }).chain(this.#store.database);
    }

    /** Wrap a queued mutation as pushed: a branch's edits as one call to the branch. */
    #pushed(entry: sync.Mutation & { readonly branch?: string }): sync.Mutation {
        // push a main-line mutation as it is, refusing a branch edit without branch types
        const types = this.#options.branch;
        if (entry.branch === undefined) {
            return { id: entry.id, calls: entry.calls };
        } else if (types === undefined) {
            throw new TypeError("an outbox without branch types cannot push a branch edit");
        }

        // append the edits to the branch in the scope they act in
        const field = types.object.route.field;
        const home = this.#home(nth(entry.calls, 0));
        const push = types.object.call("push", {
            id: entry.branch,
            calls: entry.calls,
            ...(field === undefined ? {} : { [field]: home }),
        });

        return { id: entry.id, calls: [push] };
    }

    /** Check branches out over the replicas: rows, openness and applied edits. */
    #branches(types: BranchType): sync.BranchSource {
        const branches = types.object.table;
        const objects = this.#store.replicated;

        return {
            tables: [branches[TABLE].sqlName, types.change.table[TABLE].sqlName],
            isOpen: async (transaction, id) => {
                const row = await Snapshot.live(transaction)
                    .read(branches)
                    .findFirst({ where: { id } });

                return row === undefined || BranchType.record(row).state === "open";
            },
            apply: (transaction, id) =>
                new Branch(types, Snapshot.live(transaction), id).apply(transaction, objects),
        };
    }

    /** Take the leading pushed mutations that share the first one's home. */
    #leading(pushed: readonly sync.Mutation[]): {
        readonly home: string;
        readonly mutations: sync.Mutation[];
    } {
        // read each mutation's home from its first call
        const homes = pushed.map((entry) => this.#home(nth(entry.calls, 0)));
        const home = nth(homes, 0);
        const other = homes.findIndex((each) => each !== home);

        return { home, mutations: pushed.slice(0, other === -1 ? pushed.length : other) };
    }

    /** Record a push's outcomes and queued mutations, then publish the outcomes by party. */
    async #record(
        result: PushResult,
        edits: ReadonlySet<string>,
        pushed: {
            readonly home: string;
            readonly written: ReadonlyMap<string, readonly Table[]>;
            readonly origins: ReadonlyMap<string, string>;
        },
    ): Promise<void> {
        // mark what a queueing receiver took first, so no settle ends it before its outcome arrives
        const { outcomes, queued, watermark } = result;
        for (const id of queued) {
            this.#queuedAt.add(id);
        }

        // record the whole push in one transaction, reading the home's followed tables once
        const { database } = this.#store;
        const followed = await this.#store.sink.followedTables(pushed.home);
        await database.transaction(
            async (transaction) => {
                // acknowledge each executed mutation with its result, collecting the ones no replica confirms and the rejected
                const settled: string[] = [];
                const rejected: string[] = [];
                for (const { id, outcome } of outcomes) {
                    if ("value" in outcome) {
                        await this.prediction.acknowledge(transaction, id, watermark);
                        const tables = found(pushed.written, id);
                        if (!tables.some((table) => followed.has(table))) {
                            settled.push(id);
                        }
                        // keep the results for this node's own waiters, another party's reaching it on its pages
                        if (this.#waiting.has(id)) {
                            this.#results.set(id, outcome.value);
                        }
                        if (!edits.has(id)) {
                            await this.#options.executed?.(transaction, id, outcome.value);
                        }
                    } else {
                        await this.prediction.reject(transaction, id, outcome.error);
                        rejected.push(id);
                    }
                }

                // acknowledge each mutation a queueing receiver took at its position, its outcome to arrive on a page
                for (const id of queued) {
                    await this.prediction.acknowledge(transaction, id, watermark);
                    const tables = found(pushed.written, id);
                    if (!tables.some((table) => followed.has(table))) {
                        settled.push(id);
                    }
                }

                // drop what the rejected left, then settle the unconfirmed and predict the rest again, once
                if (rejected.length > 0) {
                    await this.#options.rejected?.(transaction, rejected);
                }
                if (settled.length > 0) {
                    await this.prediction.settle(transaction, settled);
                } else if (rejected.length > 0) {
                    await transaction.log.asReplica(() => this.prediction.revert(transaction));
                    await this.prediction.replay(transaction);
                }
            },
            { constraints: "deferred" },
        );

        // publish the outcomes no waiter here takes to the parties that queued them, such as tabs, holding them while none listens
        const published = Map.groupBy(
            outcomes.filter((each) => !this.#waiting.has(each.id)),
            (each) => found(pushed.origins, each.id),
        );
        for (const [origin, ended] of published) {
            this.#deliver(origin, ended);
        }
    }

    /** Settle waiting mutations as their outcomes arrive. */
    async #settle(): Promise<void> {
        // settle known outcomes
        const { database } = this.#store;
        let failure: Error | undefined;
        try {
            const isSettled = await database.log.waitUntil(async () => {
                const outcomes = await this.prediction.outcomes(database, [
                    ...this.#waiting.keys(),
                ]);
                for (const [id, outcome] of outcomes) {
                    // leave a mutation a queueing receiver took to the outcome its page brings
                    const isEnded = outcome.kind === "settled" || outcome.kind === "rejected";
                    if (isEnded && !this.#queuedAt.has(id)) {
                        this.#end(id, outcome);
                    }
                }

                return this.#waiting.size === 0;
            }, this.#closing.signal);
            if (!isSettled) {
                failure = new Error("the outbox closed before its mutations settled");
            }
        } catch (error) {
            failure = error instanceof Error ? error : new Error(String(error));
        }

        // fail the rest, or restart for late waiters
        this.#settling = undefined;
        if (failure !== undefined) {
            for (const { reject } of [...this.#waiting.values()].flat()) {
                reject(failure);
            }
            this.#waiting.clear();
        } else if (this.#waiting.size > 0) {
            this.#settling = this.#settle();
        }
    }
}
