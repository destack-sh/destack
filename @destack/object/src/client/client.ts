import { ObjectReference } from "@destack/sync";
import type { Item, Subject } from "@destack/sync";
import {
    and,
    asc,
    count,
    defineTable,
    eq,
    gt,
    integer,
    inArray,
    isNotNull,
    isNull,
    json,
    lte,
    or,
    sql,
    text,
    type DatabaseConnection,
    TABLE,
    type Table,
    Condition,
    Namespace,
    type Extras,
    Predicate,
    Order,
    OrderBy,
    Row,
    type Scalar,
    Expression,
    DatabaseError,
    type LogPosition,
    Snapshot,
} from "@destack/db";
import {
    Digest,
    Duration,
    schema,
    canonicalize,
    aligned,
    found,
    present,
    type JsonObject,
    type JsonValue,
} from "@destack/schema";
import type { Client } from "@destack/service";
import { createClient, type ClientOptions } from "@destack/service/client";
import type { Package } from "@destack/package";
import { errorOf, ServiceError } from "@destack/service/error";
import { Moved } from "@destack/directory";
import { Journal } from "@destack/audit";
import { Authorization, Authorizer, decisionTables } from "@destack/access";
import { RequestId } from "@destack/service/request";
import { Observable } from "@destack/service/observable";
import { RetryPolicy } from "@destack/service/timer";
import * as sync from "@destack/sync";
import { Branch, type BranchChange, BranchType } from "../branch/index.ts";
import { type StoredStep, UndoStack, undoEntry } from "./undo.ts";
import { Call } from "../method/call.ts";
import type { Method } from "../method/method.ts";

import type { CallableName, CallInput, CallOutput, MutationName } from "../method/procedure.ts";
import {
    ACCESS_SHAPE,
    EPHEMERAL_SHAPE,
    EXTERNAL_SHAPE,
    ObjectQuery,
    PUSH_MUTATIONS,
    QUERIES_SHAPE,
    replicaProcedures,
    type OpenQueryOptions,
    type RelationOptions,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ObjectType, type ObjectStorage, type PermissionOf } from "../object/object.ts";

import type { NestedItem } from "../query/item.ts";
import { related } from "../query/query.ts";
import { Chunk } from "../text/chunk.ts";
import {
    RelationalQuery,
    RelationalQueryBuilder,
    type LiveItems,
    type Subscriber,
    type SubscribeOptions,
} from "./query.ts";

/** The method kinds with predictions that read only their object's and parent's tables. */
const STANDARD_KINDS: ReadonlySet<string> = new Set(["create", "update", "delete", "updateMany"]);

/** The default retry of transient failures: a second, doubling, at most a minute. */
const RETRY = RetryPolicy.of({ maximumInterval: 60_000 });

/** The hexadecimal digest digits naming a query: 64 bits, safe among a client's queries. */
const QUERY_NAME_LENGTH = 16;

/** The default local log window, in milliseconds: a minute, far above live queries' lag. */
const LOCAL_LOG_MILLISECONDS = 60_000;

/** The row count change of each logged operation. */
const ROW_DELTAS = { insert: 1, update: 0, delete: -1 } as const;

/** The queries a copy last subscribed, by name. */
const SUBSCRIBED_QUERIES = schema.record(schema.string(), ObjectQuery);

/** A shape's parameters in JSON form. */
const PARAMETERS = schema.record(schema.string(), schema.json());

/** The results of a pushed mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.unknown());

/** A mutation a client predicted. */
export interface Submission<Value> {
    /** The result the local prediction returned, once the mutation is queued. */
    readonly predicted: Promise<Value>;
    /** Settles once the replica has the server's changes, or rejects with its failure. */
    readonly confirmed: Promise<void>;
}

/** A live query a client reads from its copy, predictions included. */
export interface LiveQuery<Value = readonly Item[]> {
    /** Settles once the copy has the query, or once the query closes. */
    readonly ready: Promise<void>;
    /** Read the query's value: its items with what they include, or its groups when it aggregates. */
    read(): Promise<Value>;
    /** Read the query again after every local commit until the signal aborts. */
    watch(signal: AbortSignal): AsyncGenerator<Value>;
    /** Stop following the query. */
    close(): Promise<void>;
}

/** The changes a branch makes over the main line, as a client follows them. */
export interface BranchDiff {
    /** Settles once the copy has the branch's rows. */
    readonly ready: Promise<void>;
    /** Read the changes, one per object. */
    read(): Promise<BranchChange[]>;
    /** Read the changes again after every change of the branch's rows, until the signal aborts. */
    watch(signal: AbortSignal): AsyncGenerator<BranchChange[]>;
    /** Stop following the branch. */
    close(): Promise<void>;
}

/** The mutating methods of an object type, one pending mutation per call. */
export type Mutator<Object extends ObjectType> = {
    readonly [Name in MutationName<Object>]: (
        input: CallInput<Object, Name>,
    ) => Submission<CallOutput<Object, Name>>;
};

/** The mutating methods of an object type within a mutation, returning predictions. */
export type MutationCalls<Object extends ObjectType> = {
    readonly [Name in MutationName<Object>]: (
        input: CallInput<Object, Name>,
    ) => Promise<CallOutput<Object, Name>>;
};

/** The read-only methods of an object type, read from the server. */
export type Reader<Object extends ObjectType> = {
    readonly [Name in QueryName<Object>]: (
        input: CallInput<Object, Name>,
    ) => Promise<CallOutput<Object, Name>>;
};

/** Several calls a client makes as one atomic mutation. */
export interface Mutation {
    /** Call an object type's mutating methods within the mutation. */
    call<Object extends ObjectType>(object: Object): MutationCalls<Object>;
}

/** A mutation that also calls methods by name, as recorded calls name them. */
interface ReplayMutation extends Mutation {
    /** Call an object type's mutating method by name within the mutation, returning its prediction. */
    invoke(object: ObjectType, name: string, input: JsonObject): Promise<unknown>;
}

/** What a client keeps and follows. */
export interface ClientInspection {
    /** The scope whose objects it keeps. */
    readonly scope: string;
    /** The rows its copy keeps, by object type name. */
    readonly rows: Readonly<Record<string, number>>;
    /** The most object rows the copy keeps, absent for no limit. */
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
    /** The queued mutations by outcome. */
    readonly mutations: {
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

/** The relational queries of a client's object types, by key. */
export type ObjectQueries<Objects extends Readonly<Record<string, ObjectType>>> = {
    readonly [Key in keyof Objects]: RelationalQueryBuilder<Objects[Key], Objects[keyof Objects]>;
};

/** How a union orders and limits its members' rows, and how long it stays followed once closed. */
export interface UnionOptions<Read = Row> {
    /** How the merged rows sort, by fields every member's rows share. */
    readonly orderBy: OrderBy<Read>;
    /** The most rows the union keeps. */
    readonly limit: number;
    /** How long the closed union stays followed. */
    readonly keep?: Duration;
}

/** The items of a union's members, whose shared fields the union orders by. */
type MemberItem<Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>> =
    Members[keyof Members] extends RelationalQuery<readonly (infer Selected)[]> ? Selected : never;

/** One entry of a union: a member's name and a row its query selects. */
export type UnionEntry<
    Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>,
> = {
    readonly [Name in keyof Members & string]: {
        readonly name: Name;
        readonly row: Members[Name] extends RelationalQuery<readonly (infer Selected)[]>
            ? Selected
            : never;
    };
}[keyof Members & string];

/** The mutators of object types, by key. */
export type ObjectMutators<Objects extends Readonly<Record<string, ObjectType>>> = {
    readonly [Key in keyof Objects]: Mutator<Objects[Key]>;
};

/** Some of a client's object types by key: their relational queries and their mutators. */
export interface ObjectAccess<Objects extends Readonly<Record<string, ObjectType>>> {
    /** The relational queries, by key. */
    readonly query: ObjectQueries<Objects>;
    /** The mutators, by key. */
    readonly mutate: ObjectMutators<Objects>;
}

/** One scope's objects on a client. */
export class ObjectClient<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The local database with the replica and the queued mutations. */
    readonly database: DatabaseConnection;
    /** The object types the client keeps, with the chunk types serving their texts. */
    readonly objects: readonly ObjectType[];
    /** The relational queries of the object types, by key. */
    readonly query: ObjectQueries<Objects>;
    /** Keep queries live over the local copy, as the client's relational queries do. */
    readonly #subscriber: Subscriber = {
        rows: (object, query, options) => this.#live(object, query, options),
        results: (object, query, options) => this.#measured(object, query, options),
    };
    /** The scope whose objects the client keeps. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The copy of the scope's durable objects. */
    readonly replica: sync.Replica;
    /** The copy of the scope's ephemeral objects, absent when the client keeps none. */
    readonly ephemeral: sync.Replica | undefined;
    /** The copy of the scope's external objects, absent when the client keeps none. */
    readonly external: sync.Replica | undefined;
    /** The copy of the access rows the caller's own checks read along the scope chain. */
    readonly access: sync.Replica;
    /** The authorizer deciding the caller's checks over the copies. */
    readonly #authorizer: Authorizer;
    /** The client's own identifier, owning its ephemeral objects. */
    readonly clientId: string;
    /** What the copy shows over the server's rows: queued mutations and the checked-out branch. */
    readonly prediction: sync.Prediction;
    /** The scope's branch types, absent for a client without branches. */
    readonly branch: BranchType | undefined;
    /** The checked-out branch and its rows, followed into the copy. */
    #branchFollows: readonly LiveQuery[] = [];
    /** The party's undo and redo stacks. */
    readonly #undoStack: UndoStack;
    /** The party sharing the database, such as one browser tab. */
    readonly origin: string;
    /** The replica procedures of the cell serving the scope now. */
    #service: Client<ReplicaProcedures>;
    /** Connect to the replica procedures of the cell serving a moved scope now. */
    readonly #reconnect: (cell: string) => Client<ReplicaProcedures>;
    /** Aborts the streams following the cell that served a moved scope before. */
    #moves = new AbortController();
    /** The input field naming the scope. */
    readonly #field: string | undefined;
    /** The local feed of watched queries. */
    readonly #local: sync.Feed;
    /** The listeners of the events sent to each object, by topic. */
    readonly #listeners = new Map<string, Set<(event: JsonValue) => void>>();
    /** How often merged changes arrive, absent for one page per change. */
    readonly #refresh: { readonly every: Duration } | undefined;
    /** The most object rows the local copy keeps, absent for no limit. */
    readonly #storage: { readonly rows: number } | undefined;
    /** How long the local log keeps changes, in milliseconds. */
    readonly #logMilliseconds: number;
    /** How transient failures retry. */
    readonly #retry: RetryPolicy;
    /** Aborts outcome waits and lookups on close. */
    readonly #closing = new AbortController();
    /** The object rows the copy kept at a log sequence, absent before the first count. */
    #counted: { readonly sequence: number; readonly rows: number } | undefined;
    /** The open followed queries. */
    readonly #subscriptions = new Set<Subscription>();
    /** The queued evictions, run one at a time. */
    #evictions: Promise<void> = Promise.resolve();
    /** The mutations waiting for their outcome, by identifier. */
    readonly #waiting = new Map<
        string,
        {
            readonly resolve: (outcome: sync.MutationState) => void;
            readonly reject: (error: Error) => void;
        }
    >();
    /** The running settle loop, absent while none wait. */
    #settling: Promise<void> | undefined;
    /** The most mutations one push sends. */
    readonly #pushMutations: number;
    /** Report the follow loop's failures, absent until it follows. */
    #report: ((error: unknown) => void) | undefined;

    /** Keep one scope's objects in a local database with existing tables. */
    private constructor(options: {
        readonly database: DatabaseConnection;
        readonly objects: Objects;
        readonly package?: Package;
        readonly branch?: BranchType;
        readonly scope: string;
        readonly caller: Subject;
        readonly endpoint: ClientOptions;
        readonly reconnect: (cell: string) => ClientOptions;
        readonly origin: string;
        readonly refresh?: { readonly every: Duration };
        readonly storage?: { readonly rows: number };
        readonly log?: { readonly keep: Duration };
        readonly retry?: Partial<RetryPolicy>;
        readonly push?: { readonly mutations: number };
    }) {
        // keep the options, refusing a push larger than the server takes
        this.database = options.database;
        this.#pushMutations = pushMutationsOf(options.push);
        this.#refresh = options.refresh;
        this.#storage = options.storage;
        this.#logMilliseconds =
            options.log === undefined
                ? LOCAL_LOG_MILLISECONDS
                : Duration.milliseconds(options.log.keep);
        this.#retry = { ...RETRY, ...options.retry };
        this.origin = options.origin;
        this.#undoStack = new UndoStack(this);
        this.objects = ObjectType.served(Object.values(options.objects));
        this.query = ObjectClient.#queries(this, options.objects);
        this.scope = options.scope;
        this.caller = options.caller;
        const served = ObjectClient.#served(this.objects, options.package);
        this.#service = ObjectClient.#replica(served, options.endpoint);
        this.#reconnect = (cell) => ObjectClient.#replica(served, options.reconnect(cell));

        // require one scope field, and copy durable objects with their predictions
        this.#field = scopeFieldOf(this.objects);
        const tables = storedTables(this.objects, "durable");
        this.replica = new sync.Replica({ name: "objects", scope: options.scope, tables });
        this.branch = options.branch;
        this.prediction = new sync.Prediction({
            tables,
            predict: (transaction, pending) => this.#replay(transaction, pending),
            reads: (call) => this.#reads(call, tables),
            ...(options.branch === undefined
                ? {}
                : { branches: ObjectClient.#branchSource(options.branch, this.objects) }),
        });

        // copy ephemeral and external objects apart
        const ephemeral = storedTables(this.objects, "ephemeral");
        const external = storedTables(this.objects, "external");
        this.clientId = RequestId.create();
        this.ephemeral = separateReplica("ephemeral", options.scope, ephemeral);
        this.external = separateReplica("external", options.scope, external);

        // copy the caller's access rows, and decide its checks over the copies
        this.access = new sync.Replica({
            name: "access",
            scope: options.scope,
            tables: decisionTables,
        });
        this.#authorizer = ObjectType.authorizer(this.objects, [], (table) =>
            options.database.copies(table),
        );

        // read every copy through one local feed
        this.#local = new sync.Feed(
            options.database,
            [...tables, ...ephemeral, ...external, sync.replicaResult],
            { upstream: this.#upstream(this.objects) },
        );
    }

    /** Measure each query through the copy with its root. */
    #upstream(objects: readonly ObjectType[]): sync.Upstream {
        // map tables to storages, and storages to their copies
        const storages = new Map(objects.map((object) => [object.table, object.storage]));
        const copies = new Map<ObjectStorage, sync.Upstream>([
            ["durable", this.replica.upstream(this.database, this.prediction)],
            ...(this.ephemeral === undefined
                ? []
                : [["ephemeral", this.ephemeral.upstream(this.database, undefined)] as const]),
            ...(this.external === undefined
                ? []
                : [["external", this.external.upstream(this.database, undefined)] as const]),
        ]);
        const sourceOf = (node: sync.Node) => {
            // measure a root's query through its storage's copy, refusing a storage without one
            const storage = storages.get(rootOf(node).table) ?? "durable";
            const copy = copies.get(storage);
            if (copy === undefined) {
                throw new TypeError(
                    `no ${storage} copy measures table ${rootOf(node).table[TABLE].sqlName}`,
                );
            }

            return copy;
        };

        return {
            watches: [...copies.values()].flatMap((copy) => copy.watches),
            groups: (node) => sourceOf(node).groups(node),
            groupOf: (change) =>
                [...copies.values()]
                    .map((copy) => copy.groupOf(change))
                    .find((group) => group !== undefined),
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

    /** Create the local tables, then keep one scope's objects in the database. */
    static async open<const Objects extends Readonly<Record<string, ObjectType>>>(options: {
        /** The local database. */
        readonly database: DatabaseConnection;
        /** The object types to keep, by key. */
        readonly objects: Objects;
        /** The package serving the objects, required when they span packages. */
        readonly package?: Package;
        /** The scope's branch types, among the objects, to check out and edit branches. */
        readonly branch?: BranchType;
        /** The scope whose objects to keep. */
        readonly scope: string;
        /** The calling principal. */
        readonly caller: Subject;
        /** Where and how to reach the service serving the objects. */
        readonly endpoint: ClientOptions;
        /** Where and how to reach the cell serving a moved scope now. */
        readonly reconnect: (cell: string) => ClientOptions;
        /** The party sharing the database, a new one by default. */
        readonly origin?: string;
        /** Whether another party created the local tables. */
        readonly isMigrated?: boolean;
        /** How often merged changes arrive, such as for a background tab. */
        readonly refresh?: { readonly every: Duration };
        /** The most object rows the local copy keeps, absent for no limit. */
        readonly storage?: { readonly rows: number };
        /** How long the local log keeps changes, a minute by default. */
        readonly log?: { readonly keep: Duration };
        /** How transient failures retry, over the defaults. */
        readonly retry?: Partial<RetryPolicy>;
        /** The most mutations one push sends, the server's most by default. */
        readonly push?: { readonly mutations: number };
    }): Promise<ObjectClient<Objects>> {
        // create the local tables
        if (options.isMigrated !== true) {
            await options.database.migrate(ObjectClient.tables(options.objects), {
                isReplica: true,
            });
        }

        return new ObjectClient({
            ...options,
            origin: options.origin ?? RequestId.create(),
        });
    }

    /** Build the relational queries of object types, by key. */
    static #queries<Objects extends Readonly<Record<string, ObjectType>>>(
        client: ObjectClient,
        objects: Objects,
    ): ObjectQueries<Objects>;
    /**
     * Build the relational queries of object types, by key.
     *
     * @construct each key gets the builder of its own object type, which is how ObjectQueries maps the objects.
     */
    static #queries(
        client: ObjectClient,
        objects: Readonly<Record<string, ObjectType>>,
    ): Readonly<Record<string, RelationalQueryBuilder<ObjectType, ObjectType>>> {
        return Object.fromEntries(
            Object.entries(objects).map(([key, object]) => [
                key,
                new RelationalQueryBuilder(client.#subscriber, object),
            ]),
        );
    }

    /** Build the mutators of object types, by key. */
    static #mutators<Objects extends Readonly<Record<string, ObjectType>>>(
        client: ObjectClient,
        objects: Objects,
    ): ObjectMutators<Objects>;
    /**
     * Build the mutators of object types, by key.
     *
     * @construct each key gets the mutator of its own object type, which is how ObjectMutators maps the objects.
     */
    static #mutators(
        client: ObjectClient,
        objects: Readonly<Record<string, ObjectType>>,
    ): Readonly<Record<string, Mutator<ObjectType>>> {
        return Object.fromEntries(
            Object.entries(objects).map(([key, object]) => [key, client.mutate(object)]),
        );
    }

    /** Read the package serving the objects: the one they share, or the one the client sets. */
    static #served(objects: readonly ObjectType[], named: Package | undefined): Package {
        // take the named package, or require the objects to share one
        const packages = new Map(objects.map((object) => [object.package.id, object.package]));
        const [only, ...others] = packages.values();
        if (named !== undefined) {
            return named;
        } else if (only === undefined || others.length > 0) {
            throw new TypeError("a client with objects of several packages names the serving one");
        }

        return only;
    }

    /** Write a branch's rows from the client's copy over its other rows. */
    static #branchSource(types: BranchType, objects: readonly ObjectType[]): sync.BranchSource {
        const branches = types.object.table;

        return {
            tables: [branches[TABLE].sqlName, types.row.table[TABLE].sqlName],
            isOpen: async (transaction, id) => {
                const row = await Snapshot.live(transaction).row(branches, { id });

                return row === null || BranchType.record(row).state === "open";
            },
            apply: (transaction, id) => new Branch(types, transaction, id).apply(objects),
        };
    }

    /** Call the replica procedures of the serving package's service, speaking its release. */
    static #replica(served: Package, endpoint: ClientOptions): Client<ReplicaProcedures> {
        const router = { replica: replicaProcedures };

        return createClient({ package: served, router }, endpoint).replica;
    }

    /** List the tables a client's local database keeps for some object types. */
    static tables(objects: Readonly<Record<string, ObjectType>>): Table[] {
        return [
            ...new Set([
                ...decisionTables,
                ...ObjectType.served(Object.values(objects)).map((object) => object.table),
            ]),
            ...sync.replicaTables,
            ...sync.predictionTables,
            subscription,
            undoEntry,
        ];
    }

    /** Read and change some of the client's object types by key, refusing types the client does not keep. */
    of<const Selected extends Readonly<Record<string, ObjectType>>>(
        objects: Selected,
    ): ObjectAccess<Selected> {
        // require kept types
        for (const object of Object.values(objects)) {
            if (!this.objects.some((type) => type.same(object))) {
                throw new TypeError(`the client has no object ${object.name}`);
            }
        }

        return {
            query: ObjectClient.#queries(this, objects),
            mutate: ObjectClient.#mutators(this, objects),
        };
    }

    /** Call an object type's mutating methods, one mutation per call. */
    mutate<Object extends ObjectType>(object: Object): Mutator<Object> {
        return this.#methods<Object>(object, true, (name, input) => this.call(object, name, input));
    }

    /** Call an object type's mutating method by name as one mutation, as recorded calls name it. */
    call(object: ObjectType, name: string, input: JsonObject): Submission<unknown> {
        // send an ephemeral call at once
        if (object.storage === "ephemeral") {
            const sent = this.#send(this.#recordCall(object, name, input));
            const confirmed = sent.then(() => {});
            confirmed.catch(() => {});

            return { predicted: sent, confirmed };
        }

        // predict the call alone, on the main line for a branch's own methods
        let result: unknown;
        const pending = this.#mutation(async (mutation) => {
            result = await mutation.invoke(object, name, input);
        }, this.#isBranchType(object));

        return {
            predicted: pending.predicted.then(() => result),
            confirmed: pending.confirmed,
        };
    }

    /** Predict several calls together and queue them as one mutation. */
    mutation<Value>(run: (mutation: Mutation) => Promise<Value>): Submission<Value> {
        return this.#mutation(run, false);
    }

    /** Predict several calls together, on the main line when asked, and add them as one mutation. */
    #mutation<Value>(
        run: (mutation: ReplayMutation) => Promise<Value>,
        isMainLine: boolean,
    ): Submission<Value> {
        // predict every call
        const id = RequestId.create();
        const predicted = this.prediction.add(
            this.database,
            id,
            this.origin,
            async (database) => {
                // collect calls and steps
                const calls: sync.Call[] = [];
                const steps: StoredStep[] = [];
                const checkedOut = isMainLine
                    ? undefined
                    : await this.prediction.checkedOut(database);
                const invoke = async (object: ObjectType, name: string, input: JsonObject) => {
                    // reject ephemeral objects, and branch calls on a branch
                    if (object.storage === "ephemeral") {
                        throw new TypeError(
                            `ephemeral object ${object.name} is written one call at a time`,
                        );
                    } else if (checkedOut !== undefined && this.#isBranchType(object)) {
                        throw new TypeError(
                            `${object.name} is written through mutate, on the main line`,
                        );
                    }

                    // record and predict the call
                    const entry = this.#recordCall(object, name, input);
                    calls.push(entry);
                    const stepped = await this.step(database, object, name, entry.input);
                    steps.push(stepped.step);

                    return stepped.result;
                };
                const result = await run({
                    call: (object) =>
                        this.#methods<typeof object>(object, true, (name, input) =>
                            invoke(object, name, input),
                        ),
                    invoke,
                });

                // remember for undo
                await this.#undoStack.remember(database, id, steps);

                return { calls, result };
            },
            { isMainLine },
        );

        // confirm with the server's outcome
        const confirmed = predicted.then(() => this.outcome(id));
        confirmed.catch(() => {});

        return { predicted, confirmed };
    }

    /** Predict recorded calls and queue them as one mutation. */
    submit(calls: readonly sync.Call[]): Submission<void> {
        return this.#mutation(async (mutation) => {
            for (const entry of calls) {
                const { object, name } = this.method(entry.method);
                await mutation.invoke(object, name, entry.input);
            }
        }, false);
    }

    /** Call an object type's reading methods on the server. */
    read<Object extends ObjectType>(object: Object): Reader<Object> {
        return this.#methods<Object>(object, false, (name, input): Promise<unknown> =>
            this.#call((service) =>
                service.call({ scope: this.scope, call: this.#recordCall(object, name, input) }),
            ),
        );
    }

    /** Check out a branch by its identifier, following its rows, or the main line when undefined. */
    async checkout(branch: string | undefined): Promise<void> {
        // follow the branch's rows into the copy
        const types = this.branch;
        if (branch !== undefined && types === undefined) {
            throw new TypeError("a client without branch types checks out no branch");
        }
        await Promise.all(this.#branchFollows.map((follow) => follow.close()));
        this.#branchFollows =
            branch === undefined || types === undefined
                ? []
                : [
                      this.#live(types.object, { where: { id: branch } }),
                      this.#live(types.row, { where: { parentId: branch } }),
                  ];

        // show it over the main line
        await this.prediction.checkout(this.database, branch);
    }

    /** Follow the changes a branch makes over the main line, one per object. */
    diff(branch: string): BranchDiff {
        // follow the branch's rows into the copy
        const types = this.branch;
        if (types === undefined) {
            throw new TypeError("a client without branch types follows no branch");
        }
        const rows = this.#live(types.row, { where: { parentId: branch } });
        const read = () => new Branch(types, this.database, branch).changes(this.objects);

        return {
            ready: rows.ready,
            read,
            async *watch(signal) {
                // read the changes again after each change of the rows
                const changes = rows.watch(signal);
                try {
                    for (
                        let next = await changes.next();
                        next.done !== true;
                        next = await changes.next()
                    ) {
                        yield await read();
                    }
                } finally {
                    await changes.return(undefined);
                }
            },
            close: () => rows.close(),
        };
    }

    /** Read the checked-out branch, absent on the main line. */
    checkedOut(): Promise<string | undefined> {
        return this.prediction.checkedOut(this.database);
    }

    /** Report whether an object type keeps branches, whose calls stay on the main line. */
    #isBranchType(object: ObjectType): boolean {
        const types = this.branch;

        return types !== undefined && types.keeps(object);
    }

    /** Undo this party's latest mutation, predicting false when none is left. */
    undo(): Submission<boolean> {
        return this.#undoStack.toggle("done", "undone");
    }

    /** Redo this party's latest undo, predicting false when none is left. */
    redo(): Submission<boolean> {
        return this.#undoStack.toggle("undone", "done");
    }

    /** Close the client and fail unsettled mutations. */
    async close(): Promise<void> {
        this.#closing.abort();
        await Promise.all([...this.#subscriptions].map((subscribed) => subscribed.done));
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
        const topic = ObjectReference.key(object.reference(this.scope, id));
        const arrived: JsonValue[] = [];
        let wake = idle;
        const listener = (event: JsonValue) => {
            arrived.push(event);
            wake();
        };
        const listeners = this.#listeners.get(topic) ?? new Set();
        this.#listeners.set(topic, listeners.add(listener));
        const abort = () => wake();
        signal.addEventListener("abort", abort);
        try {
            // yield them in order, waiting while none is left
            while (!signal.aborted) {
                const event = arrived.shift();
                if (event === undefined) {
                    const { promise, resolve } = Promise.withResolvers<void>();
                    wake = resolve;
                    await promise;
                } else {
                    yield event;
                }
            }
        } finally {
            signal.removeEventListener("abort", abort);
            listeners.delete(listener);
        }
    }

    /** Push the queued mutations in order until aborted. */
    async push(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        let failures = 0;
        while (!signal.aborted) {
            // wait for pending mutations
            if (!(await this.prediction.wait(this.database, signal))) {
                return;
            }
            const pending = await this.prediction.pending(this.database, {
                limit: this.#pushMutations,
            });

            // push them, a branch's edits as one push to the branch
            const mutations = pending.map((entry) => this.#pushed(entry));
            const edits = new Set(
                pending.flatMap((entry) => (entry.branch === undefined ? [] : [entry.id])),
            );
            try {
                const result = await this.#call((service) =>
                    service.push({ scope: this.scope, mutations }, { signal }),
                );
                await this.#record(result.outcomes, result.watermark, edits);
                failures = 0;
            } catch (error) {
                // stop on abort or a final failure, retrying a concurrent writer's conflict
                const isConflict =
                    typeof error === "object" &&
                    error !== null &&
                    "code" in error &&
                    error.code === "CONFLICT";
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

    /** Follow every party's queries and the caller's access rows until aborted. */
    async follow(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        this.#report = report;
        await Promise.all(this.#copies().map((copy) => this.#follow(copy, signal, report)));
    }

    /** List the copies the client follows: its durable objects with their prediction, its ephemeral and external objects, and the caller's access rows. */
    #copies(): ClientCopy[] {
        return [
            {
                replica: this.replica,
                shape: QUERIES_SHAPE,
                parameters: async () => ({ queries: await this.#subscribed("durable") }),
                prediction: this.prediction,
            },
            ...(this.ephemeral === undefined
                ? []
                : [
                      {
                          replica: this.ephemeral,
                          shape: EPHEMERAL_SHAPE,
                          parameters: async () => ({
                              queries: await this.#subscribed("ephemeral"),
                              client: this.clientId,
                          }),
                      },
                  ]),
            ...(this.external === undefined
                ? []
                : [
                      {
                          replica: this.external,
                          shape: EXTERNAL_SHAPE,
                          parameters: async () => ({
                              queries: await this.#subscribed("external"),
                          }),
                      },
                  ]),
            { replica: this.access, shape: ACCESS_SHAPE, parameters: async () => ({}) },
        ];
    }

    /** Follow one copy's shape into its replica, subscribing again once its parameters change. */
    async #follow(
        copy: ClientCopy,
        signal: AbortSignal,
        report: (error: unknown) => void,
    ): Promise<void> {
        // register the copy, and read its parameters as they are now
        const parameters = async () => PARAMETERS.parse(await copy.parameters());
        await copy.replica.register(this.database);
        let failures = 0;
        const resetFailures = () => {
            failures = 0;
        };
        while (!signal.aborted) {
            // drop expired closed queries
            await this.#dropExpired();

            // restart once the parameters change
            const subscribed = await parameters();
            const isChanged = async () =>
                canonicalize(await parameters()) !== canonicalize(subscribed);
            const reshaped = new AbortController();
            const stream = AbortSignal.any([signal, reshaped.signal, this.#moves.signal]);
            this.#restartOnChange(isChanged, stream, reshaped, report);
            const service = this.#service;
            let isFinal = false;
            try {
                // follow the copy, resetting failures on each page
                await this.#followPages(copy, subscribed, service, stream, resetFailures);
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
                    isChanged,
                    AbortSignal.any([signal, this.#moves.signal]),
                );
                continue;
            }

            // back off
            failures += 1;
            await RetryPolicy.pause(this.#retry, failures, signal);
        }
    }

    /** Drop the closed queries whose keep expired. */
    async #dropExpired(): Promise<void> {
        await this.database
            .delete(subscription)
            .where(lte(sql`${subscription.releasedAt} + ${subscription.keep}`, Date.now()));
    }

    /** Abort a follow once its parameters change, reporting a failed watch. */
    #restartOnChange(
        isChanged: () => Promise<boolean>,
        stream: AbortSignal,
        reshaped: AbortController,
        report: (error: unknown) => void,
    ): void {
        void this.database.log.until(isChanged, stream).then(
            (changed) => changed && reshaped.abort(),
            (error: unknown) => {
                // report a failed watch and restart
                if (!stream.aborted) {
                    report(error);
                    reshaped.abort();
                }
            },
        );
    }

    /** Follow a copy's shape from where the copy is, applying each page it streams. */
    async #followPages(
        copy: ClientCopy,
        parameters: JsonObject,
        service: Client<ReplicaProcedures>,
        stream: AbortSignal,
        onPage: () => void,
    ): Promise<void> {
        // stream the copy's shape from where the copy is
        const { replica, prediction } = copy;
        const followed: sync.Subscription = {
            name: replica.name,
            shape: copy.shape,
            scope: this.scope,
            below: this.scope,
            parameters,
            ...(this.#refresh === undefined ? {} : { refresh: this.#refresh }),
        };
        const from = await replica.resume(this.database, followed);
        const pages = await service.stream({ ...followed, ...from }, { signal: stream });
        let isGrown = false;
        for await (const page of replica.apply(this.database, pages, {
            subscription: followed,
            ...(prediction === undefined ? {} : { prediction }),
        })) {
            // deliver broadcasts and compact the local log
            onPage();
            this.#broadcast(page);
            await this.database.log.compact(Date.now() - this.#logMilliseconds);

            // evict once the predicted copy grew
            isGrown ||= page.reset || page.changes.some((change) => change.operation === "insert");
            if (page.complete && prediction !== undefined && isGrown) {
                isGrown = false;
                await this.#queueEviction();
            }
        }
    }

    /** Deliver a page's broadcasts to the listeners of their topics. */
    #broadcast(page: sync.Page): void {
        for (const { topic, event } of page.broadcasts ?? []) {
            for (const listener of this.#listeners.get(topic) ?? []) {
                listener(event);
            }
        }
    }

    /** Decide whether the caller holds a permission on an object of the client's scope, as the client's copy of its access rows decides it. */
    async can<Object extends ObjectType>(
        object: Object,
        id: string,
        permission: PermissionOf<Object>,
    ): Promise<boolean> {
        const authorization = new Authorization(this.#authorizer, this.database, () => ({
            subjects: [this.caller],
            now: Date.now(),
            attributes: {},
        }));
        const decision = await authorization.check(
            object.permission(permission),
            object.reference(this.scope, id),
        );

        return decision.isAllowed;
    }

    /** Push the queued mutations and follow subscriptions until aborted. */
    async run(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        await Promise.all([this.push(signal, report), this.follow(signal, report)]);
    }

    /** Query one of the client's object types, as its keyed queries do. */
    queryOf<Object extends ObjectType>(
        object: Object,
    ): RelationalQueryBuilder<Object, Objects[keyof Objects]> {
        return new RelationalQueryBuilder(this.#subscriber, object);
    }

    /** Keep a query's items in its open form live over the local copy with predictions until closed. */
    #live(
        object: ObjectType,
        query: OpenQueryOptions = {},
        options: SubscribeOptions = {},
    ): LiveItems {
        const followed = this.#followed(object, query, options);
        const { compiled, named } = followed;

        return {
            query: compiled,
            ready: followed.ready,
            read: async () =>
                Chunk.present(await this.#items(await named, compiled), compiled, this.objects),
            watch: (signal) => this.#watch(named, compiled, signal),
            close: () => followed.close(),
        };
    }

    /** Keep an aggregate query's groups in its open form live over the local copy with predictions until closed. */
    #measured(
        object: ObjectType,
        query: OpenQueryOptions,
        options: SubscribeOptions,
    ): LiveQuery<readonly sync.AggregateRow[]> {
        // follow the query, reading its groups through the copy with its predictions
        const followed = this.#followed(object, query, options);
        const { compiled, named } = followed;
        const replica = this.#replicaOf(object.storage);
        const isDurable = object.storage === "durable";

        return {
            ready: followed.ready,
            read: async () =>
                replica.results(
                    this.database,
                    await named,
                    compiled,
                    isDurable ? this.prediction : undefined,
                ),
            watch: (signal) => this.#watchResults(named, compiled, signal),
            close: () => followed.close(),
        };
    }

    /** Follow a query in its open form over the local copy until closed. */
    #followed(object: ObjectType, query: OpenQueryOptions, options: SubscribeOptions): Follow {
        // split the query by storage
        if (!this.objects.some((type) => type.same(object))) {
            throw new TypeError(`the client has no object ${object.name}`);
        }
        const split = this.#split(object, query);
        const whole = ObjectQuery.parse({ object: object.name, ...query });
        const compiled = this.#compile(whole);

        // follow the storage query and its lookups
        const closed = new AbortController();
        const subscribed = this.#subscribe(
            split,
            undefined,
            AbortSignal.any([closed.signal, this.#closing.signal]),
        );
        this.#subscriptions.add(subscribed);
        subscribed.ready.catch(() => {});

        return {
            compiled,
            named: subscribed.named,
            ready: subscribed.ready,
            close: async () => {
                // stop following or mark released
                closed.abort();
                await subscribed.done;
                this.#subscriptions.delete(subscribed);
                const ids = subscribed.ids();
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
            const [counted] = await this.database.select({ rows: count() }).from(object.table);
            rows[object.name] = present(counted, `the count of ${object.name} rows`).rows;
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
            mutations: await this.prediction.inspect(this.database),
            feed: this.#local.inspect(),
        };
    }

    /** Stop following a closed query, even one kept always. */
    async release(object: ObjectType, query: OpenQueryOptions = {}): Promise<void> {
        // find the query's closed subscriptions
        const split = this.#split(object, query);
        const name = (await Digest.json(split.query)).slice(0, QUERY_NAME_LENGTH);
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

    /** Read the parties with subscriptions or mutations in the shared copy. */
    async origins(): Promise<Set<string>> {
        const rows = await this.database
            .selectDistinct({ origin: subscription.origin })
            .from(subscription);
        const queued = await this.prediction.origins(this.database);

        return new Set([...rows.map((row) => row.origin), ...queued]);
    }

    /** Forget a gone party's subscriptions and unread rejections. */
    async forget(origin: string): Promise<void> {
        await this.database.delete(subscription).where(eq(subscription.origin, origin));
        await this.prediction.forget(this.database, origin);
    }

    /** Wrap a branch's edit as one call appending its calls to the branch. */
    #pushed(entry: sync.Mutation & { readonly branch?: string }): sync.Mutation {
        // push a main-line mutation as it is, refusing a branch edit without branch types
        const types = this.branch;
        if (entry.branch === undefined) {
            return { id: entry.id, calls: entry.calls };
        } else if (types === undefined) {
            throw new TypeError("a client without branch types pushes no branch edit");
        }
        const push = this.#recordCall(types.object, "push", {
            id: entry.branch,
            calls: entry.calls,
        });

        return { id: entry.id, calls: [push] };
    }

    /** Record pushed mutations' outcomes and revert rejected predictions, leaving a branch edit's steps unsettled. */
    async #record(
        outcomes: readonly PushOutcome[],
        watermark: LogPosition,
        edits: ReadonlySet<string>,
    ): Promise<void> {
        // sort outcomes
        const rejected: { readonly id: string; readonly error: sync.Failure }[] = [];
        for (const { id, outcome } of outcomes) {
            // acknowledge an executed mutation
            if ("value" in outcome) {
                await this.prediction.acknowledge(this.database, id, watermark);
                if (!edits.has(id)) {
                    await this.#undoStack.settle(id, outcome.value);
                }
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
                    await this.prediction.reject(transaction, id, error);
                }
                await this.#undoStack.forget(
                    transaction,
                    rejected.map((entry) => entry.id),
                );
                await transaction.log.asReplica(() => this.prediction.revert(transaction));
                await this.prediction.replay(transaction);
            });
        }
    }

    /** Wait for a mutation's outcome, failing with the server's rejection. */
    async outcome(id: string): Promise<void> {
        // wait for the settle loop
        const outcome = await new Promise<sync.MutationState>((resolve, reject) => {
            this.#waiting.set(id, { resolve, reject });
            if (this.#settling === undefined) {
                this.#settling = this.#settle();
            }
        });

        // throw a rejection once
        if (outcome.kind === "rejected") {
            await this.prediction.forget(this.database, this.origin, id);
            throw errorOf(outcome.error);
        }
    }

    /** Settle waiting mutations as their outcomes arrive. */
    async #settle(): Promise<void> {
        // settle known outcomes
        let failure: Error | undefined;
        try {
            const isSettled = await this.database.log.until(async () => {
                const outcomes = await this.prediction.outcomes(this.database, [
                    ...this.#waiting.keys(),
                ]);
                for (const [id, outcome] of outcomes) {
                    if (outcome.kind !== "pending") {
                        found(this.#waiting, id).resolve(outcome);
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
        // evict only a budgeted copy subscribed to the current queries
        const subscribed = queriesOf(await this.replica.subscribed(this.database));
        if (
            this.#storage === undefined ||
            subscribed === undefined ||
            !isSame(subscribed, await this.#subscribed("durable"))
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
                `local copy has ${rows} rows, more than ${this.#storage.rows}, for open and always kept queries`,
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
        const tables = this.objects.map((object) => object.table);
        if (this.#counted !== undefined) {
            try {
                let { sequence, rows } = this.#counted;
                for (let reached = sequence; ; reached = sequence) {
                    const page = await this.database.log.read({ tables, after: reached });
                    for (const change of page.changes) {
                        rows += ROW_DELTAS[change.operation];
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
                const [total] = await transaction.execute(
                    sql`SELECT ${sql.join(counts, sql` + `)} AS rows`,
                    schema.object({
                        rows: schema.union([schema.number(), schema.string(), schema.bigint()]),
                    }),
                );
                if (total === undefined) {
                    throw new TypeError("the row count query returned no row");
                }

                return { sequence, rows: Number(total.rows) };
            },
            { isReadOnly: true },
        );
        this.#counted = counted;

        return counted.rows;
    }

    /** Read every party's followed queries of one storage by name. */
    async #subscribed(storage: ObjectStorage): Promise<Record<string, ObjectQuery>> {
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

    /** Keep a live union of several queries in one order and limit, each entry naming its member. */
    union<const Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>>(
        members: Members,
        options: UnionOptions<MemberItem<Members>>,
    ): LiveQuery<readonly UnionEntry<Members>[]>;
    /**
     * Keep a live union of several queries, each entry pairing its member's name with a row.
     *
     * @construct each entry pairs a member's name with a row its member's query selects, which is how UnionEntry maps the members.
     */
    union(
        members: Readonly<Record<string, RelationalQuery<readonly object[]>>>,
        options: Omit<UnionOptions, "orderBy"> & { readonly orderBy: unknown },
    ): LiveQuery<readonly { readonly name: string; readonly row: NestedItem }[]> {
        // refuse members ordering or limiting themselves, then follow each in the union's order and limit
        for (const [name, member] of Object.entries(members)) {
            if (member.options.orderBy !== undefined || member.options.limit !== undefined) {
                throw new TypeError(`union member ${name} orders or limits itself`);
            }
        }
        const orderBy = OrderBy.schema.parse(options.orderBy);
        const followed = Object.entries(members).map(([name, member]) => ({
            name,
            table: member.object.table,
            select: RelationalQuery.selecting(member.options),
            live: this.#live(
                member.object,
                { ...member.options, orderBy, limit: options.limit },
                options.keep === undefined ? {} : { keep: options.keep },
            ),
        }));
        const byName = new Map(followed.map((member) => [member.name, member]));
        const merge = (lists: readonly (readonly Item[])[]) => {
            // order the members' rows, then select each row's item
            const items = new Map(lists.flatMap((list) => list.map((item) => [item.row, item])));
            const merged = Order.merge(
                Order.of(orderBy),
                followed.map(({ name, table }, index) => ({
                    name,
                    table,
                    rows: aligned(lists, index).map((item) => item.row),
                })),
                options.limit,
            );

            return merged.map(({ name, row }) => {
                const { select, live } = found(byName, name);

                return { name, row: select(found(items, row), live.query) };
            });
        };

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

    /** Watch a query's items through the local feed until aborted. */
    async *#watch(
        named: Promise<string>,
        query: sync.Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly Item[]> {
        const name = await named;
        for await (const items of this.#local.watch(name, await this.#read(query), signal)) {
            yield Chunk.present(items, query, this.objects);
        }
    }

    /** Watch an aggregate query's groups through the local feed until aborted. */
    async *#watchResults(
        named: Promise<string>,
        query: sync.Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly sync.AggregateRow[]> {
        const name = await named;
        yield* this.#local.watchResults(name, await this.#read(query), signal);
    }

    /** Widen a query of inherited objects to the scope chain the copy last read. */
    async #read(query: sync.Query): Promise<sync.Query> {
        const object = this.objects.find((type) => type.table === query.table);
        if (object?.inherited === undefined) {
            return query;
        }

        return { ...query, scopes: object.scopesOf(await this.replica.chain(this.database)) };
    }

    /** Read a query's items from the local copy. */
    async #items(name: string, query: sync.Query): Promise<readonly Item[]> {
        // read through the local feed's upstream
        const upstream = present(this.#local.upstream, "the local feed's upstream");
        const dataflow = new sync.Dataflow(
            { [name]: await this.#read(query) },
            {
                audience: sync.EVERYONE,
                database: this.database,
                upstream,
                changesThrough: sync.changesThroughLog(this.database),
                isMaterialized: true,
            },
        );
        await dataflow.load(await sync.View.latest(this.database));

        return dataflow.read(name);
    }

    /** Split a query by storage: the query its own storage follows, and lookups of the other storage's rows by key. */
    #split(object: ObjectType, options: OpenQueryOptions): StorageQuery {
        // require conditions relating one storage
        const { with: included, extras, columns: _columns, ...own } = options;
        this.#requireOneStorage(object, own.where ?? {}, extras ?? {});
        const lookups: Lookup[] = [];

        // keep same-storage extras, and follow the rows other-storage extras measure by key
        const kept: Record<string, Expression> = {};
        const measured = new Set<string>();
        for (const [name, expression] of Object.entries(extras ?? {})) {
            const vias = [...Expression.lookups(expression), ...Expression.rollups(expression)].map(
                ({ via }) => via,
            );
            const foreign = vias.filter(
                (via) => related(object, via, this.objects).object.storage !== object.storage,
            );
            if (foreign.length === 0) {
                kept[name] = expression;
            }
            for (const via of foreign.filter((entry) => !measured.has(entry))) {
                measured.add(via);
                lookups.push(this.#lookup(object, via, {}));
            }
        }

        // keep same-storage relations, and look up other-storage ones by key
        const nested: Record<string, RelationOptions> = {};
        for (const [name, entry] of Object.entries(included ?? {})) {
            const relation = entry === true ? {} : entry;
            const joined = related(object, name, this.objects);
            if (joined.object.storage === object.storage) {
                const child = this.#split(joined.object, relation);
                const { object: _object, ...childOptions } = child.query;
                nested[name] = childOptions;
                lookups.push(
                    ...child.lookups.map((lookup) => ({ ...lookup, path: [name, ...lookup.path] })),
                );
            } else {
                const { orderBy: _orderBy, limit: _limit, ...rows } = relation;
                lookups.push(this.#lookup(object, name, rows));
            }
        }

        return {
            object,
            query: ObjectQuery.parse({
                object: object.name,
                ...own,
                ...(Object.keys(kept).length === 0 ? {} : { extras: kept }),
                ...(Object.keys(nested).length === 0 ? {} : { with: nested }),
            }),
            lookups,
        };
    }

    /** Look up the rows of another storage a relation joins by key. */
    #lookup(object: ObjectType, relation: string, options: RelationOptions): Lookup {
        const { object: target, relation: joined } = related(object, relation, this.objects);
        if (joined.on.kind !== "key") {
            throw new ServiceError("BAD_REQUEST", {
                message: `object ${object.name} relates ${relation} of another storage only by key`,
            });
        }

        return {
            path: [],
            column: joined.on.column,
            parent: joined.on.parent,
            ...(joined.where === undefined ? {} : { where: joined.where }),
            target: this.#split(target, options),
        };
    }

    /** Refuse conditions relating objects of another storage. */
    #requireOneStorage(object: ObjectType, where: Condition, extras: Extras): void {
        const relations = Predicate.relations(
            Condition.resolve(where, Namespace.fields(object.table, { extras })),
        );
        for (const { via } of relations) {
            if (related(object, via, this.objects).object.storage !== object.storage) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `object ${object.name} relates no ${via} of another storage`,
                });
            }
        }
    }

    /** Follow a storage query and the lookups its rows key. */
    #subscribe(split: StorageQuery, root: string | undefined, signal: AbortSignal): Subscription {
        // subscribe the storage query
        const id = RequestId.create();
        const named = this.#insertSubscription(split, id, root);

        // wait until the copy subscribes to it
        const replica = this.#replicaOf(split.object.storage);
        const copied = named.then(async (name) => {
            await this.database.log.until(
                async () =>
                    Object.hasOwn(queriesOf(await replica.subscribed(this.database)) ?? {}, name),
                signal,
            );
        });
        const lookups = new Map<Lookup, LookupSubscription>();
        const ids = () => [id, ...[...lookups.values()].flatMap((entry) => entry.subscribed.ids())];
        if (split.lookups.length === 0) {
            return { named, ready: copied, done: named.then(() => {}), ids };
        }

        // key lookups from the query's rows, ready once the copy and the first lookups are
        const compiled = this.#compile(split.query);
        const { promise: ready, resolve: isReady } = Promise.withResolvers<void>();
        const signalReady = async () => {
            await copied;
            await Promise.all([...lookups.values()].map((entry) => entry.subscribed.ready));
            isReady();
        };
        const done = this.#watchLookups(
            split,
            compiled,
            named,
            root ?? id,
            lookups,
            signal,
            signalReady,
        );
        done.catch((error: unknown) => {
            // report a failed lookup
            this.#report?.(error);
        });

        return { named, ready: Promise.race([ready, done.then(() => copied)]), done, ids };
    }

    /** Record a storage query as subscribed under its name, returning the name. */
    async #insertSubscription(
        split: StorageQuery,
        id: string,
        root: string | undefined,
    ): Promise<string> {
        const name = (await Digest.json(split.query)).slice(0, QUERY_NAME_LENGTH);
        await this.database.insert(subscription).values({
            id,
            origin: this.origin,
            name,
            root: root ?? null,
            query: schema.json().parse(split.query),
        });

        return name;
    }

    /** Watch a query's rows and replace each lookup whose keys changed, until stopped. */
    async #watchLookups(
        split: StorageQuery,
        compiled: sync.Query,
        named: Promise<string>,
        owner: string,
        lookups: Map<Lookup, LookupSubscription>,
        signal: AbortSignal,
        onReady: () => Promise<void>,
    ): Promise<void> {
        // watch the query's rows
        const name = await named;
        let isFirst = true;
        try {
            for await (const rows of this.#local.watch(name, await this.#read(compiled), signal)) {
                // replace each lookup whose keys changed
                for (const lookup of split.lookups) {
                    await this.#replaceLookup(lookup, keysOf(rows, lookup), owner, lookups, signal);
                }

                // signal ready after the first lookups
                if (isFirst) {
                    isFirst = false;
                    await onReady();
                }
            }
        } catch (error) {
            // rethrow unless stopped
            if (!signal.aborted) {
                throw error;
            }
        } finally {
            // wait for the lookups to stop
            await Promise.all([...lookups.values()].map((entry) => entry.subscribed.done));
        }
    }

    /** Replace a lookup whose keys changed: stop the current subscription and subscribe the new keys. */
    async #replaceLookup(
        lookup: Lookup,
        keys: readonly Exclude<Scalar, null>[],
        owner: string,
        lookups: Map<Lookup, LookupSubscription>,
        signal: AbortSignal,
    ): Promise<void> {
        // skip unchanged lookups
        const key = canonicalize(keys);
        const current = lookups.get(lookup);
        if (current?.keys === key) {
            return;
        }

        // stop the current lookup and drop its subscriptions
        if (current !== undefined) {
            current.stop.abort();
            await current.subscribed.done;
            await this.database
                .delete(subscription)
                .where(inArray(subscription.id, current.subscribed.ids()));
            lookups.delete(lookup);
        }

        // subscribe the new keys
        if (keys.length > 0) {
            const stop = new AbortController();
            const subscribed = this.#subscribe(
                keyedTarget(lookup, keys),
                owner,
                AbortSignal.any([signal, stop.signal]),
            );
            lookups.set(lookup, { keys: key, subscribed, stop });
        }
    }

    /** Predict a pending mutation again after a rebase. */
    async #replay(transaction: DatabaseConnection, pending: sync.Mutation): Promise<void> {
        try {
            await transaction.transaction(async (savepoint) => {
                for (const entry of pending.calls) {
                    const { object, name } = this.method(entry.method);
                    await this.#predict(savepoint, object, name, Call.upgrade(object, name, entry));
                }
            });
        } catch (error) {
            // keep a mutation the server would refuse for good pending unpredicted
            if (Journal.failure(error) === undefined) {
                throw error;
            }
        }
    }

    /** Predict one call as a step with the target's row before and after. */
    async step(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: sync.Call["input"],
        isInverse = false,
    ): Promise<{ readonly step: StoredStep; readonly result: unknown }> {
        // read the target around the prediction
        const id = typeof input["id"] === "string" ? input["id"] : undefined;
        const before = id === undefined ? undefined : await this.row(database, object, id);
        const method = object.method(name);
        const hasTarget = !method.target || before !== undefined;
        const result =
            isInverse && !hasTarget
                ? undefined
                : await this.#predict(database, object, name, input);
        const created = id ?? Call.resultId(result);
        const after = created === undefined ? undefined : await this.row(database, object, created);

        // keep the result and the rows in the JSON form the server answers
        const answered =
            result === undefined || method.result !== "object"
                ? result
                : object.table[TABLE].encode(Row.parse(result));
        const step: StoredStep = {
            object: object.name,
            release: object.package.version,
            name,
            input,
            ...(answered === undefined ? {} : { result: answered }),
            ...(before === undefined ? {} : { before: object.table[TABLE].encode(before) }),
            ...(after === undefined ? {} : { after: object.table[TABLE].encode(after) }),
        };

        return { step, result };
    }

    /** Read an object's row as the client has it, absent when it has none. */
    async row(
        database: DatabaseConnection,
        object: ObjectType,
        id: string,
    ): Promise<Row | undefined> {
        const table = object.table;
        const [row] = await database
            .select()
            .from(table)
            .where(eq(table[TABLE].column("id"), id));

        return row;
    }

    /** Predict one call over the local replica. */
    async #predict(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: JsonObject,
    ): Promise<unknown> {
        // skip unpredicted methods
        const method = object.method(name);
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
            chain: await this.replica.chain(database),
            input: fields,
            ...(targetId === undefined ? {} : { id: targetId }),
            database,
            caller: this.caller,
            now: Date.now(),
            objects: this.objects,
            run: (invoked, invokedName, invokedInput) =>
                this.#predict(database, invoked, invokedName, invokedInput),
        });

        // load the target at the named revision
        const table = object.table;
        const [target] =
            method.target && targetId !== undefined
                ? await database
                      .select()
                      .from(table)
                      .where(eq(table[TABLE].column("id"), targetId))
                : [];
        if (method.target && target === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${targetId}` });
        }
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw new ServiceError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        return method.execute(target === undefined ? call : call.with({ target: target }));
    }

    /** Call the replica procedures, forwarding a call once to the cell serving a moved scope now. */
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

    /** Reconnect to the cell serving a moved scope now, returning whether the failure was a move. */
    #redirect(error: unknown, service: Client<ReplicaProcedures>): boolean {
        // ignore other failures
        const moved = Moved.of(error);
        if (moved === undefined) {
            return false;
        }

        // reconnect once per move
        if (service === this.#service) {
            this.#service = this.#reconnect(moved.cell);
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
        const { outcome } = present(outcomes[0], "the pushed call's outcome");
        if (!("value" in outcome)) {
            throw errorOf(outcome.error);
        }
        const [result] = CALL_RESULTS.parse(outcome.value);

        return result;
    }

    /** Record a call with its scope and a generated identifier for creations. */
    #recordCall(object: ObjectType, name: string, input: JsonObject): sync.Call {
        const method = object.method(name);
        const generated =
            method.kind === "create" && input["id"] === undefined
                ? { id: object.generateId() }
                : {};

        return Call.record(object, name, { ...input, ...generated, ...this.#scoped() });
    }

    /** Name the client's scope in its route field. */
    #scoped(): Record<string, string> {
        return this.#field === undefined ? {} : { [this.#field]: this.scope };
    }

    /** Name the tables a call's prediction may read, by SQL name. */
    #reads(call: sync.Call, tables: readonly Table[]): readonly string[] {
        // read the object's and parent's tables for a standard method, else every table
        const { object, method } = this.method(call.method);
        const parent = object.parent?.object;
        const isStandard = STANDARD_KINDS.has(method.kind) && parent !== "any";
        const read = isStandard
            ? [object.table, ...(parent === undefined ? [] : [parent.table])]
            : tables;

        return read.map((table) => table[TABLE].sqlName);
    }

    /** Find the object type and method a recorded call names. */
    method(named: string): {
        readonly object: ObjectType;
        readonly name: string;
        readonly method: Method;
    } {
        // split the name
        const separator = named.lastIndexOf(".");
        const object = this.objects.find((entry) => entry.name === named.slice(0, separator));
        if (!object) {
            throw new TypeError(`the client has no object ${named.slice(0, separator)}`);
        }
        const name = named.slice(separator + 1);

        return { object, name, method: object.method(name) };
    }

    /** Pick the copy of a storage's objects, refusing a storage the client keeps none of. */
    #replicaOf(storage: ObjectStorage): sync.Replica {
        // pick the storage's copy
        const replicas = {
            durable: this.replica,
            ephemeral: this.ephemeral,
            external: this.external,
        };
        const replica = replicas[storage];
        if (replica === undefined) {
            throw new TypeError(`the client keeps no ${storage} objects`);
        }

        return replica;
    }

    /** Compile one object query of the client's scope. */
    #compile(query: ObjectQuery): sync.Query {
        const [compiled] = Object.values(ObjectType.queries(this.objects, { query }, [this.scope]));

        return present(compiled, `the compiled query of ${query.object}`);
    }

    /** Expose an object type's mutating methods, each submitting one mutation. */
    #methods<Object extends ObjectType>(
        object: Object,
        mutates: true,
        call: (name: string, input: JsonObject) => Submission<unknown>,
    ): Mutator<Object>;
    /** Expose an object type's mutating methods within a mutation, each returning its prediction. */
    #methods<Object extends ObjectType>(
        object: Object,
        mutates: true,
        call: (name: string, input: JsonObject) => Promise<unknown>,
    ): MutationCalls<Object>;
    /** Expose an object type's reading methods, each read from the server. */
    #methods<Object extends ObjectType>(
        object: Object,
        mutates: false,
        call: (name: string, input: JsonObject) => Promise<unknown>,
    ): Reader<Object>;
    /**
     * Expose an object type's mutating or reading methods through one call function.
     *
     * @construct each key is one of the object's non-system methods of the chosen mutability, calling through the function.
     */
    #methods(
        object: ObjectType,
        mutates: boolean,
        call: (name: string, input: JsonObject) => unknown,
    ): object {
        // require a known object type
        if (!this.objects.some((type) => type.same(object))) {
            throw new TypeError(`the client has no object ${object.name}`);
        }

        return Object.fromEntries(
            Object.entries(object.methods)
                .filter(([, method]) => method.mutates === mutates && method.isSystem !== true)
                .map(([name]) => [name, (input: JsonObject) => call(name, input)]),
        );
    }
}

/** What the server made of one pushed mutation. */
type PushOutcome = Awaited<ReturnType<Client<ReplicaProcedures>["push"]>>["outcomes"][number];

/** The names of an object type's methods that change nothing. */
type QueryName<Object extends ObjectType> = {
    [Name in CallableName<Object>]: Object["methods"][Name] extends { mutates: false }
        ? Name
        : never;
}[CallableName<Object>];

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

/** A copy the client follows: its replica, the shape and parameters it subscribes with, and the prediction it rebases. */
interface ClientCopy {
    /** The local copy. */
    readonly replica: sync.Replica;
    /** The followed shape. */
    readonly shape: string;
    /** Read the parameters to subscribe with now: the followed queries, and the writing client of ephemeral objects. */
    parameters(): Promise<{
        readonly queries?: Readonly<Record<string, ObjectQuery>>;
        readonly client?: string;
    }>;
    /** The prediction rebased onto the copy, absent for copies nothing predicts. */
    readonly prediction?: sync.Prediction;
}

/** The query one storage follows, and the lookups its rows key. */
interface StorageQuery {
    /** The object type. */
    readonly object: ObjectType;
    /** The query its storage follows, without the includes of the other storage. */
    readonly query: ObjectQuery;
    /** The lookups of the other storage below the query's rows. */
    readonly lookups: readonly Lookup[];
}

/** Other-storage objects a storage query's rows include by key. */
interface Lookup {
    /** The include path to the rows with the keys. */
    readonly path: readonly string[];
    /** The looked-up objects' column with a key. */
    readonly column: string;
    /** The parent rows' column naming the key. */
    readonly parent: string;
    /** The condition the join adds, such as an attachment's host type. */
    readonly where?: Condition;
    /** The looked-up objects' query, before keying. */
    readonly target: StorageQuery;
}

/** A query in its open form followed over the local copy until closed. */
interface Follow {
    /** The compiled query. */
    readonly compiled: sync.Query;
    /** The name of the query its storage follows, once subscribed. */
    readonly named: Promise<string>;
    /** Settles once the copy has the query. */
    readonly ready: Promise<void>;
    /** Stop following the query. */
    close(): Promise<void>;
}

/** A storage query followed until closed, with the lookups its rows keyed so far. */
interface Subscription {
    /** The name of the storage query, once subscribed. */
    readonly named: Promise<string>;
    /** Settles once the storage's copy has it and its first lookups are ready. */
    readonly ready: Promise<void>;
    /** Settles once the query stops, rejecting with a failed lookup. */
    readonly done: Promise<void>;
    /** List the subscriptions the query and its current lookups have. */
    ids(): string[];
}

/** A lookup's subscription to the keys its rows named last. */
interface LookupSubscription {
    /** The canonical keys subscribed. */
    readonly keys: string;
    /** The subscription to the keyed objects. */
    readonly subscribed: Subscription;
    /** Stops the subscription. */
    readonly stop: AbortController;
}

/** Read the most mutations one push sends, refusing a push larger than the server takes. */
function pushMutationsOf(push: { readonly mutations: number } | undefined): number {
    const mutations = push?.mutations ?? PUSH_MUTATIONS;
    if (!Number.isInteger(mutations) || mutations < 1 || mutations > PUSH_MUTATIONS) {
        throw new TypeError(`a push carries 1 to ${PUSH_MUTATIONS} mutations`);
    }

    return mutations;
}

/** Read the one input field naming the scope of every kept object type. */
function scopeFieldOf(objects: readonly ObjectType[]): string | undefined {
    const fields = new Set(objects.map((object) => object.route.field));
    if (fields.size !== 1) {
        throw new TypeError("a client keeps object types that name their scope in one field");
    }

    return [...fields][0];
}

/** List the tables of the object types kept in one storage. */
function storedTables(objects: readonly ObjectType[], storage: ObjectStorage): Table[] {
    return objects.filter((object) => object.storage === storage).map((object) => object.table);
}

/** Copy some tables in a separate replica, absent for no tables. */
function separateReplica(
    name: string,
    scope: string,
    tables: readonly Table[],
): sync.Replica | undefined {
    return tables.length === 0 ? undefined : new sync.Replica({ name, scope, tables });
}

/** Read the distinct keys a lookup's parent rows name, in their JSON form, sorted. */
function keysOf(rows: readonly Item[], lookup: Lookup): Exclude<Scalar, null>[] {
    // follow the include path
    let reached: readonly Item[] = rows;
    for (const name of lookup.path) {
        reached = reached.flatMap((item) => item.with[name] ?? []);
    }

    // collect the distinct keys the parent rows name, in their JSON form
    const column = lookup.target.object.table[TABLE].column(lookup.column).definition;
    const keys = new Set<Exclude<Scalar, null>>();
    for (const item of reached) {
        const value = item.row[lookup.parent];
        if (value === null || value === undefined) {
            continue;
        } else if (typeof value === "bigint") {
            keys.add(scalarOf(column.toJson(value)));
        } else {
            keys.add(scalarOf(column.toJson(scalarOf(value))));
        }
    }

    return [...keys].toSorted((left, right) => String(left).localeCompare(String(right)));
}

/** Follow a query node up to its root. */
function rootOf(node: sync.Node): sync.Node {
    return node.parent === undefined ? node : rootOf(node.parent);
}

/** Wait for nothing, until a reader waiting for events replaces it. */
function idle(): void {}

/** Require a lookup key value to be a scalar, as key columns keep them. */
function scalarOf(value: unknown): Exclude<Scalar, null> {
    if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
        return value;
    }

    throw new TypeError(`a lookup key is ${typeof value}, no scalar`);
}

/** Narrow a lookup's target query to the given keys. */
function keyedTarget(lookup: Lookup, keys: readonly Exclude<Scalar, null>[]): StorageQuery {
    const { where } = lookup.target.query;
    const conditions = [
        ...(where === undefined ? [] : [where]),
        { [lookup.column]: { in: keys } },
        ...(lookup.where === undefined ? [] : [lookup.where]),
    ];

    return {
        ...lookup.target,
        query: { ...lookup.target.query, where: { AND: conditions } },
    };
}

/** Read the queries a copy completed a run of, absent before its first run. */
function queriesOf(
    subscribed: sync.SubscriptionRecord | undefined,
): Readonly<Record<string, ObjectQuery>> | undefined {
    // read nothing before the first run
    if (subscribed === undefined) {
        return undefined;
    }
    const queries = subscribed.parameters["queries"];

    return queries === undefined ? {} : SUBSCRIBED_QUERIES.parse(queries);
}

/** Decide whether stored queries are the queries given. */
function isSame(stored: unknown, queries: Readonly<Record<string, ObjectQuery>>): boolean {
    return canonicalize(stored) === canonicalize(queries);
}

/** Leave out one field of an input. */
function omitField(input: JsonObject, field: string): JsonObject {
    const { [field]: _omitted, ...rest } = input;

    return rest;
}
