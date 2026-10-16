import * as db from "@destack/db";
import type { Extras } from "@destack/db";
import { type Duration, mapValues } from "@destack/schema";
import * as sync from "@destack/sync";
import type { AggregateRow } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { ObjectQuery } from "../replica/replica.ts";
import type { AggregateOptions, FindOptions, FindResult } from "../query/item.ts";
import { type NestedItem, relateItems } from "../query/item.ts";
import type { LiveQuery } from "./client.ts";

/** How long a closed live query stays followed: for a while, always, or not at all when absent. */
export interface SubscribeOptions {
    /** How long the closed query stays followed, as Zero's time to live. */
    readonly ttl?: Duration | "forever";
}

/** A query's items a client holds live, with the query its open form compiled to. */
export interface LiveItems extends LiveQuery {
    /** The compiled query, whose includes decide how the items nest. */
    readonly query: sync.Query;
}

/** Hold queries in their open form live over a client's replica. */
export interface Subscriber {
    /** Hold a query's items live. */
    readonly rows: (
        object: ObjectType,
        query: Omit<ObjectQuery, "object">,
        options: SubscribeOptions,
    ) => LiveItems;
    /** Hold an aggregate query's groups live. */
    readonly results: (
        object: ObjectType,
        query: Omit<ObjectQuery, "object">,
        options: SubscribeOptions,
    ) => LiveQuery<readonly sync.AggregateRow[]>;
}

/** The relational queries of one object type. */
export class RelationalQueryBuilder<Object extends ObjectType, Objects extends ObjectType> {
    /** Hold queries live over the client's replica. */
    readonly #subscriber: Subscriber;
    /** The queried object type. */
    readonly #object: Object;

    /** Query one object type through a client's subscriptions. */
    constructor(subscriber: Subscriber, object: Object) {
        this.#subscriber = subscriber;
        this.#object = object;
    }

    /** Query the rows with their relations. */
    findMany<
        const Computed extends Extras = {},
        const Options extends FindOptions<Object, Objects, "many", Computed> = {},
    >(
        options?: Options & { readonly extras?: Computed },
    ): RelationalQuery<readonly FindResult<Object, Objects, Options>[]> {
        // select and nest each item as the options ask
        const opened: Omit<ObjectQuery, "object"> = options ?? {};
        const select = RelationalQuery.selecting(opened, (item) =>
            mapValues<NestedItem, FindResult<Object, Objects, Options>>(item, (value) => value),
        );

        return new RelationalQuery(this.#object, opened, (ttl) => {
            const live = this.#subscriber.rows(this.#object, opened, ttl);

            return RelationalQuery.presented(live, (items) =>
                items.map((item) => select(item, live.query)),
            );
        });
    }

    /** Query the first row with its relations. */
    findFirst<
        const Computed extends Extras = {},
        const Options extends Omit<FindOptions<Object, Objects, "many", Computed>, "limit"> = {},
    >(
        options?: Options & { readonly extras?: Computed },
    ): RelationalQuery<FindResult<Object, Objects, Options> | undefined> {
        // select and nest the first item as the options ask
        const opened: Omit<ObjectQuery, "object"> = options ?? {};
        const select = RelationalQuery.selecting(opened, (item) =>
            mapValues<NestedItem, FindResult<Object, Objects, Options>>(item, (value) => value),
        );
        const first = { ...opened, limit: 1 };

        return new RelationalQuery(this.#object, first, (ttl) => {
            const live = this.#subscriber.rows(this.#object, first, ttl);

            return RelationalQuery.presented(live, ([item]) =>
                item === undefined ? undefined : select(item, live.query),
            );
        });
    }

    /** Measure the rows per group. */
    aggregate<const Options extends AggregateOptions<Object, Objects>>(
        options: Options,
    ): RelationalQuery<readonly AggregateRow<Options>[]> {
        // measure the groups the options ask for
        const opened: AggregateOptions<ObjectType, ObjectType> = options;
        const { where, ...aggregate } = opened;
        const asked = { ...(where === undefined ? {} : { where }), aggregate };

        return new RelationalQuery(this.#object, asked, (ttl) =>
            RelationalQuery.presented(this.#subscriber.results(this.#object, asked, ttl), (rows) =>
                rows.map((row) =>
                    mapValues<sync.AggregateRow, AggregateRow<Options>>(row, (value) => value),
                ),
            ),
        );
    }
}

/** A lazy query of a client's replica, read once when awaited and live when subscribed. */
export class RelationalQuery<Value> implements PromiseLike<Value> {
    /** The queried object type. */
    readonly object: ObjectType;
    /** The query in its open form. */
    readonly options: Omit<ObjectQuery, "object">;
    /** Hold the query live over the client's replica, its reads shaped into the query's value. */
    readonly #follow: (options: SubscribeOptions) => LiveQuery<Value>;

    /** Describe a query of one object type, followed live with its value shaped from its reads. */
    constructor(
        object: ObjectType,
        options: Omit<ObjectQuery, "object">,
        follow: (options: SubscribeOptions) => LiveQuery<Value>,
    ) {
        this.object = object;
        this.options = options;
        this.#follow = follow;
    }

    /** Select a query's items, reusing each unchanged item's object across reads. */
    static selecting<Result>(
        options: db.FindOptions,
        shape: (item: NestedItem) => Result,
    ): (item: sync.Item, query: sync.Query) => Result {
        const selected = new WeakMap<sync.Item, Result>();

        return (item, query) => {
            // reuse the selection of an unchanged item
            const known = selected.get(item);
            if (known !== undefined) {
                return known;
            }

            // select and shape a new item once
            const created = shape(RelationalQuery.select(item, options, query));
            selected.set(item, created);

            return created;
        };
    }

    /** Select the fields and nest each relation's selected items, as the query relates. */
    static select(item: sync.Item, options: db.FindOptions, query: sync.Query): NestedItem {
        // take the selected columns and every extra
        const columns = options.columns ?? {};
        const extras = options.extras ?? {};
        const isPicked = Object.values(columns).includes(true);
        const isSelected = (name: string) =>
            isPicked ? columns[name] === true : columns[name] !== false;
        const fields = Object.entries(item.row).filter(
            ([name]) => Object.hasOwn(extras, name) || isSelected(name),
        );

        // nest each relation's selected items, a one relation's item or null
        const relations = db.FindOptions.includes(options).map(
            ([name, relation]): [string, NestedItem[string]] => {
                const include = sync.included(query, name);
                const selected = (item.with[name] ?? []).map((entry) =>
                    RelationalQuery.select(entry, relation, include.query),
                );

                return [name, relateItems(include.relation, selected)];
            },
        );

        return Object.fromEntries([...fields, ...relations]);
    }

    /** Hold the query live over the client's replica until closed. */
    subscribe(options: SubscribeOptions = {}): LiveQuery<Value> {
        return this.#follow(options);
    }

    /** Read the query once, following it only until the replica has it. */
    then<Fulfilled = Value, Rejected = never>(
        fulfilled?: ((value: Value) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.#read().then(fulfilled, rejected);
    }

    /** Follow the query until the replica has it, read it, and close it. */
    async #read(): Promise<Value> {
        const live = this.subscribe();
        try {
            await live.ready;

            return await live.read();
        } finally {
            await live.close();
        }
    }

    /** Shape each read of a live query into a value. */
    static presented<Read, Value>(
        live: LiveQuery<Read>,
        shape: (read: Read) => Value,
    ): LiveQuery<Value> {
        return {
            ready: live.ready,
            read: async () => shape(await live.read()),
            watch: (signal) => RelationalQuery.watched(live, shape, signal),
            close: () => live.close(),
        };
    }

    /** Shape each watched read of a live query into a value. */
    static async *watched<Read, Value>(
        live: LiveQuery<Read>,
        shape: (read: Read) => Value,
        signal: AbortSignal,
    ): AsyncGenerator<Value> {
        for await (const read of live.watch(signal)) {
            yield shape(read);
        }
    }
}
