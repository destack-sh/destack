import type { Extras } from "@destack/db";
import type { Duration } from "@destack/schema";
import * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { OpenQueryOptions, RelationOptions } from "../replica/replica.ts";
import type { Group } from "@destack/db";
import type { AggregateOptions, FindOptions, FindResult } from "../query/item.ts";
import { relate, type NestedItem } from "../query/item.ts";
import type { LiveQuery } from "./client.ts";

/** How long a closed live query stays followed: for a while, always, or not at all when absent. */
export interface SubscribeOptions {
    /** How long the closed query stays followed. */
    readonly keep?: Duration | "always";
}

/** A query's items a client keeps live, with the query its open form compiled to. */
export interface LiveItems extends LiveQuery {
    /** The compiled query, whose includes decide how the items nest. */
    readonly query: sync.Query;
}

/** Keep queries in their open form live over a client's copy. */
export interface Subscriber {
    /** Keep a query's items live. */
    rows(object: ObjectType, query: OpenQueryOptions, options: SubscribeOptions): LiveItems;
    /** Keep an aggregate query's groups live. */
    results(
        object: ObjectType,
        query: OpenQueryOptions,
        options: SubscribeOptions,
    ): LiveQuery<readonly sync.AggregateRow[]>;
}

/** The relational queries of one object type. */
export class RelationalQueryBuilder<Object extends ObjectType, Objects extends ObjectType> {
    /** Keep queries live over the client's copy. */
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
    ): RelationalQuery<readonly FindResult<Object, Objects, Options>[]>;
    /**
     * Query the rows with their relations, typed by the signature above.
     *
     * @construct the rows come from the query the options shape over this builder's object, which is how FindResult maps the options.
     */
    findMany(options: OpenQueryOptions = {}): RelationalQuery<readonly NestedItem[]> {
        const select = RelationalQuery.selecting(options);

        return new RelationalQuery(this.#object, options, (keep) => {
            const live = this.#subscriber.rows(this.#object, options, keep);

            return presented(live, (items) => items.map((item) => select(item, live.query)));
        });
    }

    /** Query the first row with its relations. */
    findFirst<
        const Computed extends Extras = {},
        const Options extends Omit<FindOptions<Object, Objects, "many", Computed>, "limit"> = {},
    >(
        options?: Options & { readonly extras?: Computed },
    ): RelationalQuery<FindResult<Object, Objects, Options> | undefined>;
    /**
     * Query the first row with its relations, typed by the signature above.
     *
     * @construct the first row comes from the query the options shape, which is how FindResult maps the options.
     */
    findFirst(options: OpenQueryOptions = {}): RelationalQuery<NestedItem | undefined> {
        const select = RelationalQuery.selecting(options);
        const first = { ...options, limit: 1 };

        return new RelationalQuery(this.#object, first, (keep) => {
            const live = this.#subscriber.rows(this.#object, first, keep);

            return presented(live, ([item]) =>
                item === undefined ? undefined : select(item, live.query),
            );
        });
    }

    /** Measure the rows per group. */
    aggregate<const Options extends AggregateOptions<Object, Objects>>(
        options: Options,
    ): RelationalQuery<readonly Group<Options>[]>;
    /**
     * Measure the rows per group, typed by the signature above.
     *
     * @construct each group holds the measures the options request over this builder's object, which is how Group maps the options.
     */
    aggregate(
        options: AggregateOptions<ObjectType, ObjectType>,
    ): RelationalQuery<readonly sync.AggregateRow[]> {
        const { where, ...aggregate } = options;
        const query = { ...(where === undefined ? {} : { where }), aggregate };

        return new RelationalQuery(this.#object, query, (keep) =>
            this.#subscriber.results(this.#object, query, keep),
        );
    }
}

/** A lazy query of a client's copy, read once when awaited and live when subscribed. */
export class RelationalQuery<Value> implements PromiseLike<Value> {
    /** The queried object type. */
    readonly object: ObjectType;
    /** The query in its open form. */
    readonly options: OpenQueryOptions;
    /** Keep the query live over the client's copy, its reads shaped into the query's value. */
    readonly #follow: (options: SubscribeOptions) => LiveQuery<Value>;

    /** Describe a query of one object type, followed live with its value shaped from what it reads. */
    constructor(
        object: ObjectType,
        options: OpenQueryOptions,
        follow: (options: SubscribeOptions) => LiveQuery<Value>,
    ) {
        this.object = object;
        this.options = options;
        this.#follow = follow;
    }

    /** Select items for a query, keeping the selected object of each unchanged item across reads. */
    static selecting(options: RelationOptions): (item: sync.Item, query: sync.Query) => NestedItem {
        const selected = new WeakMap<sync.Item, NestedItem>();

        return (item, query) => {
            // reuse the selection of an unchanged item
            const known = selected.get(item);
            if (known !== undefined) {
                return known;
            }

            // select a new item once
            const created = RelationalQuery.select(item, options, query);
            selected.set(item, created);

            return created;
        };
    }

    /** Keep the fields a query's columns select and nest each relation's selected items, as the compiled query relates them. */
    static select(item: sync.Item, options: RelationOptions, query: sync.Query): NestedItem {
        // keep the selected columns and every extra
        const columns = options.columns ?? {};
        const extras = options.extras ?? {};
        const isPicked = Object.values(columns).includes(true);
        const isSelected = (name: string) =>
            isPicked ? columns[name] === true : columns[name] !== false;
        const fields = Object.entries(item.row).filter(
            ([name]) => Object.hasOwn(extras, name) || isSelected(name),
        );

        // nest each relation's selected items, a one relation's item or null
        const relations = Object.entries(options.with ?? {}).map(
            ([name, relation]): [string, NestedItem[string]] => {
                const include = sync.included(query, name);
                const selected = (item.with[name] ?? []).map((entry) =>
                    RelationalQuery.select(entry, relation === true ? {} : relation, include.query),
                );

                return [name, relate(include.relation, selected)];
            },
        );

        return Object.fromEntries([...fields, ...relations]);
    }

    /** Keep the query live over the client's copy until closed. */
    subscribe(options: SubscribeOptions = {}): LiveQuery<Value> {
        return this.#follow(options);
    }

    /** Read the query once, following it only until the copy has it. */
    then<Fulfilled = Value, Rejected = never>(
        fulfilled?: ((value: Value) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.#read().then(fulfilled, rejected);
    }

    /** Follow the query until the copy has it, read it, and close it. */
    async #read(): Promise<Value> {
        const live = this.subscribe();
        try {
            await live.ready;

            return await live.read();
        } finally {
            await live.close();
        }
    }
}

/** Shape each read of a live query into a value. */
function presented<Read, Value>(
    live: LiveQuery<Read>,
    shape: (read: Read) => Value,
): LiveQuery<Value> {
    return {
        ready: live.ready,
        read: async () => shape(await live.read()),
        watch: (signal) => watched(live, shape, signal),
        close: () => live.close(),
    };
}

/** Shape each watched read of a live query into a value. */
async function* watched<Read, Value>(
    live: LiveQuery<Read>,
    shape: (read: Read) => Value,
    signal: AbortSignal,
): AsyncGenerator<Value> {
    for await (const read of live.watch(signal)) {
        yield shape(read);
    }
}
