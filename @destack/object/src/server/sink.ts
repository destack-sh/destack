import type { ChainOptions } from "@destack/access/authorizer";
import { CHAIN_PUBLICATION } from "@destack/access";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { Scope, type ObjectTypeReference } from "@destack/sync";
import {
    asc,
    DatabaseError,
    eq,
    isNotNull,
    Key,
    lte,
    now,
    sql,
    TABLE,
    type Table,
    type Change,
} from "@destack/db";
import type {
    Cursor,
    DatabaseConnection,
    Controller,
    Following,
    Snapshot,
} from "@destack/db/database";
import { canonicalize, Digest, schema, present } from "@destack/schema";
import * as sync from "@destack/sync";
import * as syncDataflow from "@destack/sync/dataflow";
import { untilAborted, waitAborted } from "@destack/environment";
import type { ObjectStore, Subscriber } from "./store.ts";
import { replicaRequest } from "../replica/request.ts";
import { followKey } from "../replica/key.ts";

/** The cursor deciding the replicas a store follows, whose checkpoint each decision moves. */
const SUBSCRIPTIONS_CURSOR = "destack.replica.request";

/** How each change moves the count of replicated rows. */
const ROW_DELTAS = { insert: 1, update: 0, delete: -1 } as const;

/** The cursor keeping the replicated rows within the store's budget. */
const BUDGET_CURSOR = "destack.replica.budget";

/** One requested replica as its row stores it. */
type ReplicaRequest = typeof replicaRequest.$inferSelect;

/** The replicas a store follows: what its subscriber asks for, followed and dropped by key. */
export class ObjectSink {
    /** The store whose database stores the replicas. */
    readonly store: ObjectStore;
    /** The replicated rows at a log sequence, absent before the first count. */
    #counted: { readonly sequence: number; readonly rows: number } | undefined;
    /** The keys the budget evicted in this process, which a listing must start after. */
    #evictions = 0;
    /** The evictions a finished listing started after. */
    #listedEvictions = 0;
    /** The log position the requests were listed at, absent before this process listed them. */
    #listed: number | undefined;

    /** Follow the replicas a store's subscriber asks for into its database. */
    constructor(store: ObjectStore) {
        this.store = store;
    }

    /** Build a subscription's replica key from its name and scope, shared per replica. */
    static #key(subscription: Pick<sync.Subscription, "name" | "scope">): string {
        return `${subscription.name} ${subscription.scope}`;
    }

    /** Wait until some followed replicas have a watermark, which the late ones request. */
    async wait(
        watermark: sync.Watermark,
        replicas: readonly sync.SubscriptionRecord[],
        signal: AbortSignal,
    ): Promise<boolean> {
        // list the replicas yet to have the watermark, none for replicas caught up
        const { database } = this.store;
        const homes = await new sync.ReplicaSet(database).homes(replicas);
        const late = new Set(
            sync.ReplicaSet.late(homes, watermark).map((home) => ObjectSink.#key(home)),
        );

        // request each late one at the store's position now, as this follower's log, with the position it waits for
        const followed = late.size === 0 ? [] : await this.followed();
        const revision = await database.log.position();
        const origin = revision.log;
        const { log, epoch, sequence } = watermark;
        await Promise.all(
            followed
                .filter((entry) => late.has(ObjectSink.#key(entry.subscription)))
                .map((entry) =>
                    entry.publisher.request({
                        ...entry.subscription,
                        revision,
                        origin,
                        waiting: { log, epoch, sequence },
                    }),
                ),
        );

        // wait, recording a wait that gives up with the late replicas' positions and how long it waited
        const started = this.store.clock.now();
        const isReached = await new sync.ReplicaSet(database).waitAt(replicas, watermark, signal);
        if (!isReached) {
            const waited = this.store.clock.now() - started;
            const { log: telemetry } = TelemetryScope.of(
                import.meta.destack.package,
                this.store.environment.telemetry,
            );
            const positions = await new sync.ReplicaSet(database).homes(replicas);
            for (const { name, scope, home } of sync.ReplicaSet.late(positions, watermark)) {
                telemetry.warn("destack.replica.waited", {
                    "destack.replica.name": name,
                    "destack.replica.scope": scope,
                    ...(home === undefined ? {} : { "destack.replica.sequence": home.sequence }),
                    "destack.watermark.log": log,
                    "destack.watermark.epoch": epoch,
                    "destack.watermark.sequence": sequence,
                    "destack.wait.duration": waited,
                });
            }
        }

        return isReached;
    }

    /** Build the cursor listing the requested replicas at each start and as their tables change. */
    cursor(subscriber: Subscriber): Cursor {
        return {
            name: SUBSCRIPTIONS_CURSOR,
            tables: this.#replicaWatches(),
            take: async () => this.#request(subscriber),
        };
    }

    /** Build the cursor dropping the earliest lapsing key while the rows exceed a budget. */
    budget(rows: number): Cursor {
        const cursor = { name: BUDGET_CURSOR, tables: this.#budgeted() };

        return {
            ...cursor,
            take: async () => {
                // evict past the budget once the replicas follow their requests, then move past the changes
                const { database } = this.store;
                const position = await database.log.position();
                await this.#evict(rows);
                await database.log.advance(cursor, position.sequence, this.store.clock.now());
            },
        };
    }

    /** Drop the earliest lapsing key past a budget, refusing an overflow. */
    async #evict(budget: number): Promise<void> {
        // count once the requests are listed past the last eviction and the replicas follow them, so a dropped key's rows left first
        const { database } = this.store;
        const view = database.snapshot();
        const isListed = this.#listedEvictions === this.#evictions;
        if (!isListed || !(await this.#isSettled(view))) {
            return;
        }
        const rows = await this.#count();
        if (rows <= budget) {
            return;
        }

        // drop the earliest lapsing key, refusing a budget the open keys exceed
        const [earliest] = await database
            .select()
            .from(followKey)
            .where(isNotNull(followKey.expiresAt))
            .orderBy(asc(followKey.expiresAt))
            .limit(1);
        if (earliest === undefined) {
            throw sync.SyncError.of(
                "CAPACITY_EXHAUSTED",
                `local replica has ${rows} rows, more than ${budget}, for open and forever followed queries`,
            );
        }
        await database.delete(followKey).where(Key.match(followKey, earliest));
        this.#evictions += 1;
    }

    /** Report whether each followed replica follows its request as it stands. */
    async #isSettled(view: Snapshot): Promise<boolean> {
        const requested = await view.read(replicaRequest).findMany({ where: { action: "follow" } });
        const held = new Map(
            (await sync.ReplicaSet.subscriptions(view)).map((replica) => [
                ObjectSink.#key(replica),
                canonicalize(replica.parameters),
            ]),
        );

        return requested.every(
            (request) => held.get(request.key) === canonicalize(request.subscription.parameters),
        );
    }

    /** Count the replicated object rows. */
    async #count(): Promise<number> {
        // count on from the last count through the log, recounting once the log dropped its changes
        const { database } = this.store;
        const tables = this.#budgeted().filter((table) => table !== followKey);
        if (this.#counted !== undefined) {
            try {
                let { sequence, rows } = this.#counted;
                for (let after = sequence; ; after = sequence) {
                    const page = await database.log.read({ tables, after });
                    for (const change of page.changes) {
                        rows += ROW_DELTAS[change.operation];
                    }
                    sequence = Math.max(sequence, page.sequence);
                    if (page.sequence <= after) {
                        break;
                    }
                }
                this.#counted = { sequence, rows };

                return rows;
            } catch (error) {
                if (!DatabaseError.is(error, "CHANGES_NOT_FOUND")) {
                    throw error;
                }
            }
        }

        // count every table at one log position
        this.#counted = await database.transaction(
            async (transaction) => {
                // read the position and the counts together
                const sequence = (await transaction.log.position()).sequence;
                const counts = tables.map((table) => sql`(SELECT count(*) FROM ${table})`);
                const [total] = await transaction.execute(
                    sql`SELECT ${sql.join(counts, sql` + `)} AS rows`,
                    schema.object({
                        rows: schema.union([schema.number(), schema.string(), schema.bigint()]),
                    }),
                );

                return { sequence, rows: Number(present(total, "the replicated row count").rows) };
            },
            { isReadOnly: true },
        );

        return this.#counted.rows;
    }

    /** List the tables whose changes may push the replicas past their budget. */
    #budgeted(): Table[] {
        const replicated = this.store.replicated.filter((object) => object.storage !== "local");

        return [...new Set([...replicated.map((object) => object.table), followKey])];
    }

    /** List the tables the store follows at a scope, whose replicas confirm their changes. */
    async followedTables(scope: string): Promise<ReadonlySet<Table>> {
        // read the replicas requested at the scope, and the keys followed on demand there before their request
        const view = this.store.database.snapshot();
        const requested = await view
            .read(replicaRequest)
            .findMany({ where: { action: "follow", scope } });
        const keys = await view.read(followKey).findMany({ where: { scope } });

        // read the replicas whose chain reads the scope, such as an enclosed room's of its home
        const chained: syncDataflow.Replica[] = [];
        for (const subscription of await sync.ReplicaSet.subscriptions(view)) {
            const replica = this.store.source.buildReplica(subscription);
            if ((await replica.chain(this.store.database)).includes(scope)) {
                chained.push(replica);
            }
        }

        return new Set([
            ...requested.flatMap(
                (request) => this.store.source.buildReplica(request.subscription).tables,
            ),
            ...keys.flatMap((key) => this.#keyTables(key.shape, key.parameters)),
            ...chained.flatMap((replica) => replica.tables),
        ]);
    }

    /** List the tables a followed key's queries read. */
    #keyTables(shape: string, parameters: string): Table[] {
        // read the key's queries through the on-demand follow of its shape
        const follow = this.store.queriesFollow;
        if (follow?.shape.name !== shape) {
            return [];
        }
        const queries = follow.shape.queries(follow.shape.parameters.parse(JSON.parse(parameters)));

        return queries.flatMap((query) =>
            this.store.followed.objects
                .filter(
                    (object) => object.name === query.object && object.package.id === query.package,
                )
                .map((object) => object.table),
        );
    }

    /** Build the controller removing the keys whose followers let them lapse, once they lapse. */
    lapse(): Controller<typeof followKey> {
        return {
            name: "destack.follow_key.lapse",
            table: followKey,
            when: { expiresAt: { lte: now } },
            by: [],
            reconcile: async () => {
                const { database, clock } = this.store;
                await database.delete(followKey).where(lte(followKey.expiresAt, clock.now()));
            },
        };
    }

    /** Build the controller following each requested replica and dropping each unrequested one. */
    controller(subscriber: Subscriber): Controller<typeof replicaRequest> {
        return {
            name: "destack.replica.follow",
            table: replicaRequest,
            when: {},
            concurrency: Infinity,
            follow: async (rows, following) => {
                const key = present(rows[0], "a requested replica").key;
                await this.#follow(subscriber, key, following);
            },
        };
    }

    /** List the replicas the store follows, as their requests stand. */
    async followed(): Promise<
        {
            readonly subscription: sync.SubscriptionRecord;
            readonly publisher: sync.Publisher;
            readonly replica: syncDataflow.Replica;
        }[]
    > {
        // follow nothing without a subscriber
        const subscriber = this.store.subscriber;
        if (subscriber === undefined) {
            return [];
        }

        // read the followed requests in key order
        const rows = await this.store.database
            .snapshot()
            .read(replicaRequest)
            .findMany({ where: { action: "follow" }, orderBy: { key: "asc" } });

        return rows.map((row) => ({
            subscription: row.subscription,
            publisher: subscriber.publisher(present(row.home, `the home of replica ${row.key}`)),
            replica: this.store.source.buildReplica(row.subscription),
        }));
    }

    /** List the replicas the store waits for: its own chain's, each up to the universe. */
    async gates(): Promise<sync.SubscriptionRecord[]> {
        // take the chain replicas of the store's scope, leaving followed shapes, which gate nothing
        const view = this.store.database.snapshot();
        const ancestry = new Set(await this.ancestry(this.store.scope, view));
        const chain = ObjectSink.chainName(this.store.scope);

        return (await this.chains(this.store.scope, view))
            .map(({ subscription }) => subscription)
            .filter(
                (subscription) => subscription.name === chain && ancestry.has(subscription.scope),
            );
    }

    /** List the requested replicas, reusing untouched keys, and write the changed requests. */
    async #request(subscriber: Subscriber): Promise<void> {
        // read the changes since the last listing, listing every key afresh at the first or once the log dropped them
        const { database } = this.store;
        const evictions = this.#evictions;
        const position = await database.log.position();
        const changes = await this.#changes(position.sequence);

        // list the requests
        const view = database.snapshot();
        const entries = await subscriber.subscriptions(view, changes);
        const named = new Set(entries.map((entry) => ObjectSink.#key(entry.subscription)));
        const held = await sync.ReplicaSet.subscriptions(view);
        const requests = await Promise.all(
            [
                ...held
                    .filter((replica) => !named.has(ObjectSink.#key(replica)))
                    .map((replica) => ({
                        action: "drop" as const,
                        subscription: replica,
                        home: null,
                    })),
                ...entries.map(({ subscription, home }) => ({
                    action: "follow" as const,
                    subscription,
                    home,
                })),
            ].map(async (request) => ({
                ...request,
                key: ObjectSink.#key(request.subscription),
                scope: request.subscription.scope,
                digest: await Digest.json(
                    { action: request.action, subscription: request.subscription },
                    this.store.environment.crypto,
                ),
            })),
        );

        // write the requests whose digest changed, and remove the ones no longer asked for
        const stored = new Map(
            (await view.read(replicaRequest).findMany()).map((row) => [row.key, row.digest]),
        );
        const changed = requests.filter((request) => stored.get(request.key) !== request.digest);
        const kept = new Set(requests.map((request) => request.key));
        const removed = [...stored.keys()].filter((key) => !kept.has(key));
        if (changed.length > 0 || removed.length > 0) {
            await database.transaction(async (transaction) => {
                for (const request of changed) {
                    await transaction
                        .insert(replicaRequest)
                        .values(request)
                        .onConflictDoUpdate({ target: replicaRequest.key, set: request });
                }
                for (const key of removed) {
                    await transaction.delete(replicaRequest).where(eq(replicaRequest.key, key));
                }
            });
        }
        this.#listed = position.sequence;
        this.#listedEvictions = evictions;
        await database.log.advance(
            { name: SUBSCRIPTIONS_CURSOR, tables: this.#replicaWatches() },
            position.sequence,
            this.store.clock.now(),
        );
    }

    /** Read the watched changes since the last listing up to a sequence, absent to list afresh. */
    async #changes(through: number): Promise<readonly Change[] | undefined> {
        // list afresh at the first listing
        const after = this.#listed;
        if (after === undefined) {
            return undefined;
        }

        // read every page up to the sequence, listing afresh once the log dropped them or none is logged
        try {
            const changes: Change[] = [];
            let sequence = after;
            while (sequence < through) {
                const page = await this.store.database.log.read({
                    tables: this.#replicaWatches(),
                    after: sequence,
                });
                if (page.changes.length === 0) {
                    break;
                }
                changes.push(...page.changes);
                sequence = page.sequence;
            }

            return changes.length === 0 ? undefined : changes;
        } catch (error) {
            if (!DatabaseError.is(error, "CHANGES_NOT_FOUND")) {
                throw error;
            }

            return undefined;
        }
    }

    /** List the tables whose changes decide the replicas again, none the follows write. */
    #replicaWatches(): Table[] {
        // watch the scopes, the replicated scope types and the rows the follows key on
        const scopes = this.store.replicated
            .filter((object) => object.policy.definition.scope === true)
            .map((object) => object.table);

        return [...new Set([Scope.table, ...scopes, ...this.store.watched])];
    }

    /** Follow or drop one requested replica until stopped, revised as its request changes. */
    async #follow(subscriber: Subscriber, key: string, following: Following): Promise<void> {
        // follow the request as it stands, then each revision of it
        const { signal } = following;
        let request = await this.#read(key);
        while (request !== undefined && !signal.aborted) {
            // revise the follow once the request changes, requesting it at the log's position now
            const revised = new AbortController();
            const revision = await this.store.database.log.position();
            const { refresh } = this.store;
            const current: Request = {
                subscription: {
                    ...request.subscription,
                    revision,
                    ...(refresh === undefined ? {} : { refresh }),
                },
                waking: new AbortController(),
            };
            const watching = this.#watch(key, request, current, revised, following);

            // follow a requested replica, or drop an unrequested one and wait for a change
            const until = AbortSignal.any([signal, revised.signal]);
            if (request.action === "follow") {
                await this.#followReplica(subscriber, request, current, until, following);
            } else {
                await syncDataflow.Replica.drop(this.store.database, {
                    ...request.subscription,
                    tables: [],
                });
                following.synced();
                await waitAborted(until);
            }
            revised.abort();
            await watching;
            request = await this.#read(key);
        }

        // hold a removed request's key until the loop stops it
        following.synced();
        await waitAborted(signal);
    }

    /** Read one replica's request, absent once removed. */
    #read(key: string): Promise<ReplicaRequest | undefined> {
        const { database } = this.store;

        return database.state.scope.span("destack.replica.request.read", {}, () =>
            database.snapshot().read(replicaRequest).findFirst({ where: { key } }),
        );
    }

    /** Watch a followed request: move a reshaped one in place, and revise any other change. */
    async #watch(
        key: string,
        request: ReplicaRequest,
        current: Request,
        revised: AbortController,
        following: Following,
    ): Promise<void> {
        let watched = request;
        while (!revised.signal.aborted) {
            // wait for the request's row to change
            await untilAborted(following.changed(), revised.signal);
            const next = revised.signal.aborted ? watched : await this.#read(key);

            // keep an unchanged request
            if (next?.digest === watched.digest) {
                watched = next;
            }
            // move a follow of other parameters over the same tables in place, waking its request
            else if (
                next !== undefined &&
                watched.action === "follow" &&
                next.action === "follow" &&
                this.#isReshape(watched.subscription, next.subscription)
            ) {
                const revision = await this.store.database.log.position();
                const { refresh } = this.store;
                current.subscription = {
                    ...next.subscription,
                    revision,
                    ...(refresh === undefined ? {} : { refresh }),
                };
                current.waking.abort();
                current.waking = new AbortController();
                watched = next;
            }
            // revise any other change
            else {
                revised.abort();
            }
        }
    }

    /** Follow a requested replica, failing on its source's refusal unless the request changed. */
    async #followReplica(
        subscriber: Subscriber,
        request: ReplicaRequest,
        current: Request,
        until: AbortSignal,
        following: Following,
    ): Promise<void> {
        // read the replica and its publisher
        const publisher = subscriber.publisher(present(request.home, `the home of ${request.key}`));
        const replica = this.store.source.buildReplica(request.subscription);
        const publication = this.store.source.publication(request.subscription);
        const { audience } = publication;

        // rebase the outbox's predictions onto the pages of a replica of the store's queries
        const isQueries = publication === this.store.source.queriesPublication;
        const prediction = isQueries ? this.store.outbox?.prediction : undefined;
        const { synced } = following;
        try {
            // follow the replica until stopped
            const follow = replica.follow(
                this.store.database,
                (from, stream, subscription) =>
                    publisher.stream(
                        { ...present(subscription, "the followed subscription"), ...from },
                        stream,
                    ),
                until,
                {
                    get subscription() {
                        return current.subscription;
                    },
                    audience,
                    synced,
                    lag: this.store.authorizer.lag,
                    received: (page) => this.store.receive(page),
                    ...(prediction === undefined ? {} : { prediction }),
                },
            );
            await Promise.all([
                follow,
                ObjectSink.#forward(current, publisher, this.store.database, until),
            ]);
        } catch (error) {
            // end quietly on the follow's own abort, failing on any other error
            const isAbort =
                until.aborted &&
                (error === until.reason || (error instanceof Error && error.name === "AbortError"));
            if (isAbort) {
                return;
            }
            this.store.refuse(error);
            throw error;
        }
    }

    /** Send a follow's request to its publisher each time its parameters move. */
    static async #forward(
        current: Request,
        publisher: sync.Publisher,
        database: DatabaseConnection,
        signal: AbortSignal,
    ): Promise<void> {
        let waking = current.waking;
        while (!signal.aborted) {
            // wait for the subscription to move, taking the next move's signal before requesting
            await waitAborted(AbortSignal.any([signal, waking.signal]));
            waking = current.waking;

            // request the moved subscription as this follower's log, failing the follow on a refusal
            if (!signal.aborted) {
                const origin = (await database.log.position()).log;
                await publisher.request({ ...current.subscription, origin });
            }
        }
    }

    /** Decide whether a follow changes only its parameters, over the same tables. */
    #isReshape(subscription: sync.SubscriptionRecord, next: sync.SubscriptionRecord): boolean {
        // compare the subscriptions beside their parameters, and the tables
        const { parameters: _parameters, ...identity } = subscription;
        const { parameters: _next, ...nextIdentity } = next;
        const tables = (record: sync.SubscriptionRecord) =>
            this.store.source.buildReplica(record).tables.map((table) => table[TABLE].sqlName);

        return (
            canonicalize(identity) === canonicalize(nextIdentity) &&
            canonicalize(tables(subscription)) === canonicalize(tables(next))
        );
    }

    /** Read a store's scope and the scopes above it, up to the universe. */
    async ancestry(target: string, view: Snapshot): Promise<readonly string[]> {
        // read the chain above a placed scope, the universe alone above an unplaced one
        const kind = await this.#chain(target, view);

        return kind === undefined
            ? [...new Set([target, Scope.universe.id])]
            : this.#ancestry(view, target, kind);
    }

    /** List the replicas a store's decisions read, each with the scope whose home publishes it. */
    async chains(
        target: string,
        view: Snapshot,
    ): Promise<
        {
            readonly subscription: sync.SubscriptionRecord;
            readonly home: string;
        }[]
    > {
        // follow nothing at the universe, nor for a scope without a known chain
        const kind = await this.#chain(target, view);
        if (kind === undefined) {
            return [];
        }

        // replicate the scope chain, and the universe rows it reads
        const served = this.store.objects.filter((object) => object.storage === "durable");
        const known = [...this.store.objects, ...this.store.replicated];
        const isLiving = served.some((object) =>
            object.scopes.some((scope) => !known.some((type) => type.same(scope))),
        );
        const isScoped = isLiving || kind.kind === "home";
        const scoped = await this.#scopeChain(view, target, kind, isScoped);

        // leave the scope chain's scopes to it, else let the via replicas cover the store's scope
        const chained = new Set(
            isScoped ? [target, ...scoped.map((subscription) => subscription.scope)] : [],
        );

        // read the replicated scopes outside the chain that served objects live in or relate to
        const living = this.store.replicated.filter((scope) =>
            served.some((object) =>
                [...object.scopes, ...object.related].some((type) => type.same(scope)),
            ),
        );
        const outside = await Promise.all(
            living.map(async (scope) => {
                // read the scope rows the database has
                const rows = await view.read(scope.table).findMany();

                // leave the chain's scopes, and the partitions other stores home with every object inside them
                const { partitions } = this.store;
                const stored = rows
                    .map((row) => ({
                        id: schema.string().parse(row["id"]),
                        scope: schema.string().parse(row["scope"]),
                    }))
                    .filter(
                        (row) =>
                            !chained.has(row.id) &&
                            (!scope.isPartition || partitions.includes(row.id)),
                    );

                // leave the rows a subject's follow brought, which their source decided for that subject
                const brought = await this.#brought(scope.table, stored, view);

                return stored.filter((row) => !brought.has(Key.name(scope.table, row)));
            }),
        );

        // replicate the chains of the store's outside scopes from the universe
        const via = await this.#viaChain(view, target, outside.flat());

        return [
            ...scoped.map((subscription) => ({ subscription, home: target })),
            ...via.map((subscription) => ({ subscription, home: Scope.universe.id })),
        ];
    }

    /** List a scope chain's replicas: each scope's access rows and each type it propagates. */
    async #scopeChain(
        view: Snapshot,
        target: string,
        options: ChainOptions,
        isScoped: boolean,
    ): Promise<sync.SubscriptionRecord[]> {
        // read the scope and its ancestors, up to the universe
        const chain = await this.#ancestry(view, target, options);
        const ancestors = chain.slice(1);

        // replicate the universe's grants alone for a store outside its scope chain
        let replicated: string[] = [Scope.universe.id];
        if (isScoped) {
            // replicate a home's chain above it, and a follower's from itself
            replicated = options.kind === "home" ? ancestors : chain;
        }

        // replicate each scope's access rows and propagated types for the target, in one replica of the nearest
        const [nearest] = replicated;
        if (nearest === undefined) {
            throw new TypeError(`the chain of '${target}' replicates no scope`);
        }

        return [
            this.store.source.chainPublication.subscription({
                name: ObjectSink.chainName(target),
                scope: nearest,
                target,
                parameters: {
                    scopes: chain,
                    local: [...this.store.authorizer.local],
                    // leave out the propagated rows of a scope the store serves beside its own, which it originates
                    replicated: replicated.map((scope) => ({
                        scope,
                        types: isScoped ? this.#propagatedTypes(scope) : [],
                    })),
                    via: [],
                    between: [],
                },
            }),
        ];
    }

    /** Read a scope's known chain, absent at the universe, without a chain or while pending. */
    async #chain(target: string, view: Snapshot): Promise<ChainOptions | undefined> {
        const chain = target === Scope.universe.id ? undefined : await this.store.chain(view);

        return chain?.kind === "pending" ? undefined : chain;
    }

    /** Read a placed scope, its parent and the parent's ancestors, up to the universe. */
    async #ancestry(view: Snapshot, target: string, options: ChainOptions): Promise<string[]> {
        // read the parent's row
        const [row] = await Scope.read(view, [options.parent]);

        return [...new Set([target, options.parent, ...(row?.ancestors ?? []), Scope.universe.id])];
    }

    /** List the one replica of the chains above a follower's outside scopes, in the universe. */
    async #viaChain(
        view: Snapshot,
        target: string,
        via: readonly { readonly id: string; readonly scope: string | null }[],
    ): Promise<sync.SubscriptionRecord[]> {
        // request nothing without a replicated scope
        if (via.length === 0) {
            return [];
        }

        // read the replicated scopes' ancestors and containing scopes in one query
        const replicated = new Set(via.map((object) => object.id));
        const rows = await view
            .read(Scope.table)
            .findMany({ where: { scope: { in: [...replicated] } } });
        const containing = [
            ...via.flatMap((object) => (object.scope === null ? [] : [object.scope])),
            ...rows.flatMap((row) => row.ancestors),
        ];
        const between = [
            ...new Set(
                containing.filter((scope) => scope !== Scope.universe.id && !replicated.has(scope)),
            ),
        ].toSorted();
        const outside = { via: [...replicated].toSorted(), between };

        // replicate the outside chains at the universe with the rows they store, in one replica
        return [
            this.store.source.chainPublication.subscription({
                name: ObjectSink.viaName(target),
                scope: Scope.universe.id,
                target,
                parameters: {
                    scopes: [],
                    local: [],
                    replicated: [{ scope: Scope.universe.id, types: this.#outsideTypes() }],
                    ...outside,
                },
            }),
        ];
    }

    /** List the replicated types a scope propagates to the store, less those it originates. */
    #propagatedTypes(scope: string): ObjectTypeReference[] {
        // originate the propagated rows of a scope served beside the store's own, as isReplica decides
        const { authorizer, partitions } = this.store;
        const isServed = partitions.slice(1).includes(scope);

        return authorizer.replicated.filter(
            (type) =>
                authorizer.propagates(type, scope) &&
                !(isServed && authorizer.requireMapping(type).propagated !== undefined),
        );
    }

    /** List the propagated and scoped types that outside scopes store for the store. */
    #outsideTypes(): ObjectTypeReference[] {
        const { authorizer } = this.store;

        return [
            ...authorizer.replicated.filter(
                (type) => authorizer.requireMapping(type).propagated !== undefined,
            ),
            ...authorizer.scoped,
        ];
    }

    /** Build the name of a target's chain replicas, one record per follower. */
    static chainName(target: string): string {
        return `${CHAIN_PUBLICATION}:${target}`;
    }

    /** Build the name of the replica of the outside chains a target reads. */
    static viaName(target: string): string {
        return `${CHAIN_PUBLICATION}:${target}:via`;
    }

    /** List the keys of some scope rows a follow on a subject's behalf stores here. */
    async #brought(
        table: Table,
        rows: readonly { readonly id: string; readonly scope: string }[],
        view: Snapshot,
    ): Promise<Set<string>> {
        // read the replicas that follow on a subject's behalf
        if (rows.length === 0) {
            return new Set();
        }
        const following = await view
            .read(sync.replicaSubscription)
            .findMany({ where: { subjectId: { isNotNull: true } } });
        const names = new Set(following.map((entry) => `${entry.name} ${entry.scope}`));

        // read which of the rows they store
        const held = await view.read(sync.replicaKey).findMany({
            where: {
                table: table[TABLE].sqlName,
                key: { in: rows.map((row) => Key.name(table, row)) },
            },
        });

        return new Set(
            held
                .filter((entry) => names.has(`${entry.name} ${entry.scope}`))
                .map((entry) => entry.key),
        );
    }
}

/** The subscription a follow requests now, and the signal waking its request once it moves. */
interface Request {
    /** The subscription at its latest revision. */
    subscription: sync.Subscription;
    /** Aborts once the subscription moves. */
    waking: AbortController;
}
