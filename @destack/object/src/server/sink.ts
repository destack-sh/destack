import { CHAIN_PUBLICATION, type ChainOptions } from "@destack/access";
import { Scope, type ObjectTypeReference } from "@destack/sync";
import { Key, type LogPosition, Snapshot, TABLE, type Table } from "@destack/db";
import { canonicalize, Digest, schema, present } from "@destack/schema";
import * as sync from "@destack/sync";
import type { FollowController, Delta } from "@destack/db";
import { until } from "@destack/environment";
import type { ObjectStore, Subscriber } from "./store.ts";

/** The replicas a store follows: what its subscriber asks for, followed and dropped by key. */
export class ObjectSink {
    /** The store whose database stores the replicas. */
    readonly store: ObjectStore;
    /** The replicas the store's subscriber asks for as last listed, by replica key. */
    readonly #listed = new Map<string, ReplicaEntry>();
    /** The latest listing of the replicas, each one run after the one before. */
    #listing: Promise<readonly string[]> | undefined;

    /** Follow the replicas a store's subscriber asks for into its database. */
    constructor(store: ObjectStore) {
        this.store = store;
    }

    /** Build a subscription's replica key from its name and scope, shared per replica. */
    static #key(subscription: Pick<sync.Subscription, "name" | "scope">): string {
        return `${subscription.name} ${subscription.scope}`;
    }

    /** Build the controller following requested replicas and dropping unrequested stored ones. */
    controller(subscriber: Subscriber): FollowController {
        // follow each listed replica under its key and lease
        return {
            name: "replica",
            mode: "follow",
            watches: this.#replicaWatches(subscriber),
            concurrency: Infinity,
            list: (view, delta) => this.#listInOrder(subscriber, view, delta),
            revision: (key) => this.#listed.get(key)?.digest,
            reconcile: (key, { signal, synced }) => this.#reconcileReplica(key, signal, synced),
        };
    }

    /** List the replicas the store follows as last listed, listing them first if none did yet. */
    async followed(): Promise<Extract<ReplicaIntent, { readonly action: "follow" }>[]> {
        // follow nothing without a subscriber
        const subscriber = this.store.subscriber;
        if (subscriber === undefined) {
            return [];
        }

        // wait for the latest listing, listing first if none ran yet
        await (this.#listing ?? this.#listInOrder(subscriber, Snapshot.live(this.store.database)));

        return [...this.#listed.values()].filter((entry) => entry.action === "follow");
    }

    /** List the replicas the store waits for: its own chain's, each up to the universe. */
    async gates(): Promise<Extract<ReplicaIntent, { readonly action: "follow" }>[]> {
        // take the scopes from the store's scope up to the universe
        const view = Snapshot.live(this.store.database);
        const ancestry = new Set(await this.ancestry(this.store.scope, view));

        // leave followed shapes, which report each key synced without gating the store
        const chain = ObjectSink.chainName(this.store.scope);

        return (await this.followed()).filter(
            ({ subscription }) => subscription.name === chain && ancestry.has(subscription.scope),
        );
    }

    /** List the replicas after the listing before, recording this one as the latest. */
    #listInOrder(
        subscriber: Subscriber,
        view: Snapshot,
        delta?: Delta,
    ): Promise<readonly string[]> {
        // wait for the listing before, whose caller sees its failure
        const previous = this.#listing?.catch(() => []) ?? Promise.resolve([]);
        const listing = previous.then(() => this.#list(subscriber, view, delta));
        this.#listing = listing;

        return listing;
    }

    /** List the requested replicas through a view, revising the listed ones, by replica key. */
    async #list(subscriber: Subscriber, view: Snapshot, delta?: Delta): Promise<readonly string[]> {
        // list the replicas, revising the listed ones at the log position the listing read from
        const revision = await this.store.database.log.position();
        const replicas = await this.#replicas(subscriber, view, delta);
        ObjectSink.#revise(this.#listed, replicas, revision);

        return [...replicas.keys()];
    }

    /** List the tables whose changes list the replicas again. */
    #replicaWatches(subscriber: Subscriber): Table[] {
        // watch the replicated scope types, the stored replicas and their keys
        const scopes = this.store.replicated
            .filter((object) => object.policy.definition.scope === true)
            .map((object) => object.table);

        return [
            ...new Set([
                Scope.table,
                sync.replica,
                sync.replicaSubscription,
                sync.replicaKey,
                ...scopes,
                ...(subscriber.watches ?? []),
            ]),
        ];
    }

    /** Follow requested replicas from publishers and drop unrequested ones, by replica key. */
    async #replicas(
        subscriber: Subscriber,
        view: Snapshot,
        delta?: Delta,
    ): Promise<Map<string, ReplicaIntent & { digest: string }>> {
        // drop the stored replicas no request asks for and follow the requested ones
        const entries = await subscriber.subscriptions(view, delta);
        const named = new Set(entries.map((entry) => ObjectSink.#key(entry.subscription)));
        const held = await sync.Replica.subscriptions(view);
        const actions: ReplicaIntent[] = [
            ...held
                .filter((replica) => !named.has(ObjectSink.#key(replica)))
                .map((replica) => ({ action: "drop" as const, subscription: replica })),
            ...entries.map(({ subscription, publisher, isAcquired }) => ({
                action: "follow" as const,
                subscription,
                replica: this.store.source.buildReplica(subscription),
                publisher,
                isAcquired,
            })),
        ];

        // digest each action with its subscription, the revision a listing compares
        return new Map(
            await Promise.all(
                actions.map(
                    async (entry): Promise<[string, ReplicaIntent & { digest: string }]> => {
                        const { action, subscription } = entry;
                        const digest = await Digest.json(
                            { action, subscription },
                            this.store.environment.crypto,
                        );

                        return [ObjectSink.#key(subscription), { ...entry, digest }];
                    },
                ),
            ),
        );
    }

    /** Follow or drop one listed replica until the controller stops it. */
    async #reconcileReplica(
        key: string,
        signal: AbortSignal,
        synced: () => void,
    ): Promise<undefined> {
        while (!signal.aborted) {
            // wait for the loop to stop a replica no longer listed
            const replica = this.#listed.get(key);
            if (replica === undefined) {
                synced();
                await until(signal);

                return undefined;
            }

            // follow a requested replica from the position it arrived at until its request changes
            const revised = AbortSignal.any([signal, replica.revised.signal]);
            if (replica.action === "follow") {
                await this.#followReplica(replica, revised, synced);
            }
            // drop a stored replica no request asks for and wait for a request asking for it again
            else {
                await sync.Replica.drop(this.store.database, {
                    ...replica.subscription,
                    tables: [],
                });
                synced();
                await until(revised);
            }
        }

        return undefined;
    }

    /** Follow a requested replica, failing on its source's refusal unless the request changed. */
    async #followReplica(
        replica: ReplicaEntry & { readonly action: "follow" },
        revised: AbortSignal,
        synced: () => void,
    ): Promise<void> {
        try {
            const { publisher, requested } = replica;
            const { audience } = this.store.source.publication(requested.subscription);
            const following = replica.replica.follow(
                this.store.database,
                (from, stream, subscription) =>
                    publisher.stream(
                        { ...present(subscription, "the followed subscription"), ...from },
                        stream,
                    ),
                revised,
                {
                    get subscription() {
                        return requested.subscription;
                    },
                    audience,
                    synced,
                    lag: this.store.authorizer.lag,
                },
            );
            await Promise.all([following, ObjectSink.#forward(requested, publisher, revised)]);
        } catch (error) {
            // fail a refused follow unless the loop revised its request meanwhile
            if (!revised.aborted) {
                throw error;
            }
        }
    }

    /** Send a follow's request to its publisher each time its parameters move. */
    static async #forward(
        requested: Request,
        publisher: sync.Publisher,
        signal: AbortSignal,
    ): Promise<void> {
        let waking = requested.waking;
        while (!signal.aborted) {
            // wait for the subscription to move, taking the next move's signal before requesting
            await until(AbortSignal.any([signal, waking.signal]));
            waking = requested.waking;

            // request the moved subscription, failing the follow on a refusal
            if (!signal.aborted) {
                await publisher.request(requested.subscription);
            }
        }
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

                // leave the rows a subject's follow brought, which their source decided for that subject
                const stored = rows
                    .map((row) => ({
                        id: schema.string().parse(row["id"]),
                        scope: schema.string().parse(row["scope"]),
                    }))
                    .filter((row) => !chained.has(row.id));
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

    /** List a scope chain's replicas: each scope's access rows and each type it hands down. */
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

        // replicate each scope's access rows and handed down types for the target, in one replica of the nearest
        const [nearest] = replicated;
        if (nearest === undefined) {
            throw new TypeError(`the chain of ${target} replicates no scope`);
        }

        return [
            this.store.source.chainPublication.subscription({
                name: ObjectSink.chainName(target),
                scope: nearest,
                target,
                parameters: {
                    scopes: chain,
                    local: [...this.store.authorizer.local],
                    // leave out the inherited rows of a scope the store serves beside its own, which it originates
                    replicated: replicated.map((scope) => ({
                        scope,
                        types: isScoped ? this.#handedDownTypes(scope) : [],
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

    /** List the replicated types a scope hands down to the store, less those it originates. */
    #handedDownTypes(scope: string): ObjectTypeReference[] {
        // originate the inherited rows of a scope served beside the store's own, as isReplica decides
        const { authorizer, partitions } = this.store;
        const isServed = partitions.slice(1).includes(scope);

        return authorizer.replicated.filter(
            (type) =>
                authorizer.handsDown(type, scope) &&
                !(isServed && authorizer.requireMapping(type).inherited !== undefined),
        );
    }

    /** List the replicated types outside scopes store for the store: inherited and scoped ones. */
    #outsideTypes(): ObjectTypeReference[] {
        const { authorizer } = this.store;

        return [
            ...authorizer.replicated.filter(
                (type) => authorizer.requireMapping(type).inherited !== undefined,
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

    /** Revise each listed replica whose action changed or left, and list the new ones. */
    static #revise(
        listed: Map<string, ReplicaEntry>,
        replicas: ReadonlyMap<string, ReplicaIntent & { digest: string }>,
        revision: LogPosition,
    ): void {
        for (const [key, replica] of listed) {
            // keep an unchanged replica
            const next = replicas.get(key);
            if (next?.digest === replica.digest) {
                continue;
            }

            // move a follow of other parameters over the same tables at its source, waking its request
            if (
                next !== undefined &&
                replica.action === "follow" &&
                next.action === "follow" &&
                ObjectSink.#isReshape(replica, next)
            ) {
                const { requested } = replica;
                requested.subscription = { ...next.subscription, revision };
                listed.set(key, { ...next, revised: replica.revised, requested });
                requested.waking.abort();
                requested.waking = new AbortController();
            }
            // abort and forget any other changed or left replica
            else {
                replica.revised.abort();
                listed.delete(key);
            }
        }

        // list the new replicas
        for (const [key, replica] of replicas) {
            if (!listed.has(key)) {
                listed.set(key, {
                    ...replica,
                    revised: new AbortController(),
                    requested: {
                        subscription: { ...replica.subscription, revision },
                        waking: new AbortController(),
                    },
                });
            }
        }
    }

    /** Decide whether a follow changes only its parameters, over the same tables. */
    static #isReshape(
        replica: Extract<ReplicaIntent, { readonly action: "follow" }>,
        next: Extract<ReplicaIntent, { readonly action: "follow" }>,
    ): boolean {
        // compare the subscriptions beside their parameters, and the tables
        const { parameters: _parameters, ...identity } = replica.subscription;
        const { parameters: _next, ...nextIdentity } = next.subscription;

        return (
            canonicalize(identity) === canonicalize(nextIdentity) &&
            canonicalize(ObjectSink.#tables(replica.replica)) ===
                canonicalize(ObjectSink.#tables(next.replica))
        );
    }

    /** List a replica's tables by SQL name. */
    static #tables(replica: sync.Replica): string[] {
        return replica.tables.map((table) => table[TABLE].sqlName);
    }
}

/** What the replica controller does with a replica: follow it from its publisher, or drop it. */
type ReplicaIntent =
    | {
          /** A request follows the replica. */
          readonly action: "follow";
          /** The subscription following the replica. */
          readonly subscription: sync.SubscriptionRecord;
          /** The replica the request follows into. */
          readonly replica: sync.Replica;
          /** The publisher streaming the replica's pages. */
          readonly publisher: sync.Publisher;
          /** Whether a use acquired the replica's key, not the store's declaration. */
          readonly isAcquired: boolean;
      }
    | {
          /** The stored replica drops for want of a request. */
          readonly action: "drop";
          /** The subscription the stored replica recorded. */
          readonly subscription: sync.SubscriptionRecord;
      };

/** A replica the replica controller lists, with the digest each listing compares. */
type ReplicaEntry = ReplicaIntent & {
    /** The digest of the action and subscription. */
    readonly digest: string;
    /** Aborts once a listing changes the action, or the subscription beyond its parameters. */
    readonly revised: AbortController;
    /** The subscription a follow requests now, which a reshape moves in place. */
    readonly requested: Request;
};

/** The subscription a follow requests now, and the signal waking its request once it moves. */
interface Request {
    /** The subscription at its latest revision. */
    subscription: sync.Subscription;
    /** Aborts once the subscription moves. */
    waking: AbortController;
}
