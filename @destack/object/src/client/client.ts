import { ObjectReference } from "@destack/sync";
import type { Item, Subject } from "@destack/sync";
import {
    FindOptions,
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
    declareState,
    type LogPosition,
    Snapshot,
    Transaction,
} from "@destack/db";
import {
    aligned,
    canonicalize,
    Clock,
    Digest,
    Duration,
    found,
    type Identifier,
    type JsonObject,
    type JsonValue,
    mapValues,
    present,
    schema,
} from "@destack/schema";
import type { Client } from "@destack/runtime";
import { createClient, type ClientOptions } from "@destack/runtime/client";
import type { Package } from "@destack/package";
import { errorOf, CallError } from "@destack/runtime/error";
import { Replay } from "../audit/replay.ts";
import { Authorization, Authorizer, decisionTables } from "@destack/access";
import { RequestId } from "@destack/runtime/request";
import { Observable } from "@destack/runtime/observable";
import { RetryPolicy } from "@destack/runtime/timer";
import * as sync from "@destack/sync";
import { Branch, type BranchChange, BranchType } from "../branch/index.ts";
import { type StoredStep, UndoStack, undoEntry } from "./undo.ts";
import { Call, type ObjectCall } from "../method/call.ts";
import { Method } from "../method/method.ts";
import { BatchItem } from "../method/record.ts";

import type { CallableName, CallInput, CallOutput, MutationName } from "../method/procedure.ts";
import {
    ACCESS_SHAPE,
    EPHEMERAL_SHAPE,
    EXTERNAL_SHAPE,
    ObjectQuery,
    PUSH_MUTATIONS,
    QUERIES_SHAPE,
    replicaProcedures,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ObjectKey, ObjectType, type ObjectStorage, type PermissionOf } from "../object/object.ts";
import { ObjectAccess } from "../object/access.ts";

import type { NestedItem } from "../query/item.ts";
import { related } from "../query/query.ts";
import { TextPresenter } from "../text/chunk.ts";
import {
    RelationalQuery,
    RelationalQueryBuilder,
    type LiveItems,
    type Subscriber,
    type SubscribeOptions,
} from "./query.ts";

/** The method kinds with predictions that read only their object's and parent's tables. */
const STANDARD_KINDS: ReadonlySet<string> = new Set([
    "create",
    "createMany",
    "update",
    "updateMany",
    "delete",
    "deleteMany",
]);

/** The default retry of transient failures: a second, doubling, at most a minute. */
const RETRY = RetryPolicy.of({ maximumInterval: 60_000 });

/** The hexadecimal digest digits identifying a query: 64 bits, safe among a client's queries. */
const QUERY_NAME_LENGTH = 16;

/** The storages beside the durable replica: ephemeral and external replicas, and local objects. */
const SEPARATE_STORAGES = ["ephemeral", "external", "local"] as const;

/** The default local log window, in milliseconds: a minute, far above live queries' lag. */
const LOCAL_LOG_MILLISECONDS = 60_000;

/** The row count change of each logged operation. */
const ROW_DELTAS = { insert: 1, update: 0, delete: -1 } as const;

/** The queries a replica last subscribed, by name. */
const SUBSCRIBED_QUERIES = schema.record(schema.string(), ObjectQuery);

/** A shape's parameters in JSON form. */
const PARAMETERS = schema.jsonObject();

/** The results of a pushed mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.unknown());

/** A mutation a client predicted. */
export interface Submission<Value> {
    /** The result the local prediction returned, once the mutation is queued. */
    readonly predicted: Promise<Value>;
    /** Settles once the replica has the server's changes, or rejects with its failure. */
    readonly confirmed: Promise<void>;
}

/** A live query a client reads from its replica, predictions included. */
export interface LiveQuery<Value = readonly Item[]> {
    /** Settles once the replica has the query, or once the query closes. */
    readonly ready: Promise<void>;
    /** Read the query's value: its items with their includes, or its groups when aggregating. */
    readonly read: () => Promise<Value>;
    /** Read the query again after every local commit until the signal aborts. */
    readonly watch: (signal: AbortSignal) => AsyncGenerator<Value>;
    /** Stop following the query. */
    readonly close: () => Promise<void>;
}

/** The changes a branch makes over the main line, as a client follows them. */
export interface BranchDiff {
    /** Settles once the replica has the branch's rows. */
    readonly ready: Promise<void>;
    /** Read the changes, one per object. */
    readonly read: () => Promise<BranchChange[]>;
    /** Read the changes again after every change of the branch's rows, until the signal aborts. */
    readonly watch: (signal: AbortSignal) => AsyncGenerator<BranchChange[]>;
    /** Stop following the branch. */
    readonly close: () => Promise<void>;
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
    readonly call: <Object extends ObjectType>(object: Object) => MutationCalls<Object>;
}

/** A mutation that also calls methods by name, as recorded calls refer to them. */
interface ReplayMutation extends Mutation {
    /** Call an object type's mutating method by name in the mutation, returning its prediction. */
    readonly invoke: (object: ObjectType, name: string, input: JsonObject) => Promise<unknown>;
}

/** What a client keeps and follows. */
export interface ClientInspection {
    /** The scope whose objects it keeps. */
    readonly scope: string;
    /** The rows its replica keeps, by object type name. */
    readonly rows: Readonly<Record<string, number>>;
    /** The most object rows the replica keeps, absent for no limit. */
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
    /** The replica of the scope. */
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

/** How a client keeps one scope's objects in its local database. */
export interface ObjectClientOptions<Objects extends Readonly<Record<string, ObjectType>>> {
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
    /** The client the database lives on, whose partition keeps local objects scoped to clients. */
    readonly client?: Subject;
    /** Where and how to reach the package serving the objects. */
    readonly endpoint: ClientOptions;
    /** The client's clock for log compaction, query keep and access decisions. */
    readonly clock: Clock;
    /** Where and how to reach the machine serving a moved scope now. */
    readonly reconnect: (machine: Identifier<"machine">) => ClientOptions;
    /** Where and how to reach the home of an enclosing scope whose rows the client changes. */
    readonly homes?: (scope: string) => ClientOptions;
    /** The party sharing the database, a new one by default. */
    readonly origin?: string;
    /** How often merged changes arrive, such as for a background tab. */
    readonly refresh?: { readonly every: Duration };
    /** The most object rows the local replica keeps, absent for no limit. */
    readonly storage?: { readonly rows: number };
    /** How long the local log keeps changes, a minute by default. */
    readonly log?: { readonly keep: Duration };
    /** How transient failures retry, over the defaults. */
    readonly retry?: Partial<RetryPolicy>;
    /** The most mutations one push sends, the server's most by default. */
    readonly push?: { readonly mutations: number };
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
    /** Keep queries live over the local replica, as the client's relational queries do. */
    readonly #subscriber: Subscriber = {
        rows: (object, query, options) => this.#live(object, query, options),
        results: (object, query, options) => this.#measured(object, query, options),
    };
    /** The presentation of read rows' texts, each row and list presented once. */
    readonly #presenter: TextPresenter;
    /** The scope whose objects the client keeps. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The principals whose partitions keep the client's local objects: caller and client. */
    readonly #owners: readonly Subject[];
    /** The replica of the scope's durable objects. */
    readonly replica: sync.Replica;
    /** The replica of the scope's ephemeral objects, absent when the client keeps none. */
    readonly ephemeral: sync.Replica | undefined;
    /** The replica of the scope's external objects, absent when the client keeps none. */
    readonly external: sync.Replica | undefined;
    /** The replica of the access rows the caller's own checks read along the scope chain. */
    readonly access: sync.Replica;
    /** The authorizer deciding the caller's checks over the replicas. */
    readonly #authorizer: Authorizer;
    /** The identifier of this client instance, the writer owning its ephemeral objects. */
    readonly writerId: string;
    /** What the replica shows over the server's rows: queued mutations and the branch. */
    readonly prediction: sync.Prediction;
    /** The scope's branch types, absent for a client without branches. */
    readonly branch: BranchType | undefined;
    /** The checked-out branch and its rows, followed into the replica. */
    #branchFollows: readonly LiveQuery[] = [];
    /** The party's undo and redo stacks. */
    readonly #undoStack: UndoStack;
    /** The party sharing the database, such as one browser tab. */
    readonly origin: string;
    /** The replica procedures of the machine serving the scope now. */
    #replica: Client<ReplicaProcedures>;
    /** Connect to the replica procedures of the machine serving a moved scope now. */
    readonly #reconnect: (machine: Identifier<"machine">) => Client<ReplicaProcedures>;
    /** Reach the replica procedures of a scope's home, the client's replica for its scope. */
    readonly #home: (scope: string) => Client<ReplicaProcedures>;
    /** Whether the client reaches the homes of scopes other than its own. */
    readonly #isReachingHomes: boolean;
    /** Aborts the streams following the machine that served a moved scope before. */
    #moves = new AbortController();
    /** The local feed of watched queries. */
    readonly #local: sync.Feed;
    /** The listeners of the events sent to each object, by topic. */
    readonly #listeners = new Map<string, Set<(event: JsonValue) => void>>();
    /** How often merged changes arrive, absent for one page per change. */
    readonly #refresh: { readonly every: Duration } | undefined;
    /** The most object rows the local replica keeps, absent for no limit. */
    readonly #storage: { readonly rows: number } | undefined;
    /** How long the local log keeps changes, in milliseconds. */
    readonly #logMilliseconds: number;
    /** How transient failures retry. */
    readonly #retry: RetryPolicy;
    /** The clock the client reads time at. */
    readonly clock: Clock;
    /** Aborts outcome waits and lookups on close. */
    readonly #closing = new AbortController();
    /** The object rows the replica kept at a log sequence, absent before the first count. */
    #counted: { readonly sequence: number; readonly rows: number } | undefined;
    /** The open followed queries. */
    readonly #subscriptions = new Set<Subscription>();
    /** Wake the local watches once a page reads other scopes. */
    readonly #rescoping = new Set<() => void>();
    /** The scopes the objects' replica last read, canonicalized. */
    #scopes: string | undefined;
    /** How often the objects' replica read other scopes, which watches compare across reads. */
    #generation = 0;
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
    private constructor(options: ObjectClientOptions<Objects> & { readonly origin: string }) {
        // keep the options, refusing a push larger than the server takes
        this.database = options.database;
        this.#presenter = new TextPresenter();
        this.#pushMutations = ObjectClient.#pushMutationsOf(options.push);
        this.#refresh = options.refresh;
        this.#storage = options.storage;
        this.#logMilliseconds =
            options.log === undefined
                ? LOCAL_LOG_MILLISECONDS
                : Duration.milliseconds(options.log.keep);
        this.#retry = { ...RETRY, ...options.retry };
        this.clock = options.clock;
        this.origin = options.origin;
        this.writerId = RequestId.create(options.clock.now());
        this.#undoStack = new UndoStack(this);

        // keep the objects of the scope for the caller, and the principals keeping local objects
        this.objects = ObjectType.served(Object.values(options.objects));
        this.query = ObjectClient.#queries(this, options.objects);
        this.scope = options.scope;
        this.caller = options.caller;
        this.#owners = [options.caller, ...(options.client === undefined ? [] : [options.client])];

        // reach the package serving the replicated objects
        const replicated = this.objects.filter((object) => object.storage !== "local");
        const served = ObjectClient.#served(replicated, options.package);
        this.#replica = ObjectClient.replica(served, options.endpoint);
        this.#reconnect = (machine) => ObjectClient.replica(served, options.reconnect(machine));
        this.#isReachingHomes = options.homes !== undefined;
        this.#home = ObjectClient.homes(served, this.scope, () => this.#replica, options.homes);

        // replicate durable objects with their predictions, and ephemeral and external objects apart
        const tables = ObjectClient.#storedTables(this.objects, "durable");
        this.replica = new sync.Replica({ name: "objects", scope: options.scope, tables });
        this.branch = options.branch;
        this.prediction = this.#prediction(tables, options.branch);
        this.ephemeral = ObjectClient.#separateReplica(
            "ephemeral",
            options.scope,
            this.#stored("ephemeral"),
        );
        this.external = ObjectClient.#separateReplica(
            "external",
            options.scope,
            this.#stored("external"),
        );

        // replicate the caller's access rows, and decide its checks over the replicas
        this.access = new sync.Replica({
            name: "access",
            scope: options.scope,
            tables: decisionTables,
        });
        this.#authorizer = ObjectAccess.authorizer(this.objects, [], (table) =>
            options.database.replicates(table),
        );

        // read every replica and the local objects through one local feed
        const others = SEPARATE_STORAGES.flatMap((storage) => this.#stored(storage));
        this.#local = new sync.Feed(options.database, [...tables, ...others, sync.replicaResult], {
            upstream: this.#upstream(this.objects),
            clock: this.clock,
        });
    }

    /** List the tables of the client's objects of one storage. */
    #stored(storage: ObjectStorage): Table[] {
        return ObjectClient.#storedTables(this.objects, storage);
    }

    /** Show the server's rows with the queued mutations and the checked-out branch over them. */
    #prediction(tables: readonly Table[], branch: BranchType | undefined): sync.Prediction {
        return new sync.Prediction({
            tables,
            predict: (transaction, pending) => this.#replay(transaction, pending),
            reads: (call) => this.#reads(call, tables),
            ...(branch === undefined
                ? {}
                : { branches: ObjectClient.#branchSource(branch, this.objects) }),
        });
    }

    /** Measure each query through the replica with its root. */
    #upstream(objects: readonly ObjectType[]): sync.Upstream {
        // map tables to storages, and storages to their replicas
        const storages = new Map(objects.map((object) => [object.table, object.storage]));
        const upstreams = new Map<ObjectStorage, sync.Upstream>([
            ["durable", this.replica.upstream(this.database, this.prediction)],
            ...(this.ephemeral === undefined
                ? []
                : [["ephemeral", this.ephemeral.upstream(this.database, undefined)] as const]),
            ...(this.external === undefined
                ? []
                : [["external", this.external.upstream(this.database, undefined)] as const]),
        ]);
        const readSource = (node: sync.Node) => {
            // measure a root's query through its storage's replica, refusing a storage without one
            const storage = storages.get(ObjectClient.#rootOf(node).table) ?? "durable";
            const upstream = upstreams.get(storage);
            if (upstream === undefined) {
                throw new TypeError(
                    `no ${storage} replica for table ${ObjectClient.#rootOf(node).table[TABLE].sqlName}`,
                );
            }

            return upstream;
        };

        return {
            watches: [...upstreams.values()].flatMap((upstream) => upstream.watches),
            groups: (node) => readSource(node).groups(node),
            group: (change) =>
                [...upstreams.values()]
                    .map((upstream) => upstream.group(change))
                    .find((group) => group !== undefined),
            isMeasured: (node) => {
                // measure here every node of local objects, and at the source only nodes within the root's storage
                const storage = storages.get(ObjectClient.#rootOf(node).table);
                if (storage === "local") {
                    return false;
                }
                for (let at: sync.Node | undefined = node; at !== undefined; at = at.parent) {
                    if (storages.get(at.table) !== storage) {
                        return false;
                    }
                }

                return true;
            },
        };
    }

    /** Create the local tables and keep one scope's objects in the database. */
    static async open<const Objects extends Readonly<Record<string, ObjectType>>>(
        options: ObjectClientOptions<Objects> & {
            /** Whether another party created the local tables. */
            readonly isMigrated?: boolean;
        },
    ): Promise<ObjectClient<Objects>> {
        // create the local tables
        if (options.isMigrated !== true) {
            await ObjectClient.migrate(options.database, options.objects);
        }

        return new ObjectClient({
            ...options,
            origin: options.origin ?? RequestId.create(options.clock.now()),
        });
    }

    /** Build the relational queries of object types, by key. */
    static #queries<Objects extends Readonly<Record<string, ObjectType>>>(
        client: ObjectClient,
        objects: Objects,
    ): ObjectQueries<Objects> {
        return mapValues<Objects, ObjectQueries<Objects>>(
            objects,
            (object) => new RelationalQueryBuilder(client.#subscriber, object),
        );
    }

    /** Build the mutators of object types, by key. */
    static #mutators<Objects extends Readonly<Record<string, ObjectType>>>(
        client: ObjectClient,
        objects: Objects,
    ): ObjectMutators<Objects> {
        return mapValues<Objects, ObjectMutators<Objects>>(objects, (object) =>
            client.mutate(object),
        );
    }

    /** Read the package serving the objects: the one they share, or the one the client sets. */
    static #served(objects: readonly ObjectType[], named: Package | undefined): Package {
        // take the given package, or require the objects to share one
        const packages = new Map(objects.map((object) => [object.package.id, object.package]));
        const [only, ...others] = packages.values();
        if (named !== undefined) {
            return named;
        } else if (only === undefined || others.length > 0) {
            throw new TypeError(
                "a client with objects of several packages needs the serving package",
            );
        }

        return only;
    }

    /** Write a branch's rows from the client's replica over its other rows. */
    static #branchSource(types: BranchType, objects: readonly ObjectType[]): sync.BranchSource {
        const branches = types.object.table;

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

    /** Reach a scope's home replica procedures: the client's replica, else each home once. */
    static homes(
        served: Package,
        scope: string,
        own: () => Client<ReplicaProcedures>,
        homes: ((scope: string) => ClientOptions) | undefined,
    ): (home: string) => Client<ReplicaProcedures> {
        const known = new Map<string, Client<ReplicaProcedures>>();

        return (home) => {
            // reach the client's scope at its replica, refusing another scope without homes
            if (home === scope) {
                return own();
            } else if (homes === undefined) {
                throw new CallError("BAD_REQUEST", {
                    message: `the client of ${scope} cannot reach the home of ${home}`,
                });
            }

            // connect to each other scope's home once
            const connected = known.get(home) ?? ObjectClient.replica(served, homes(home));
            known.set(home, connected);

            return connected;
        };
    }

    /** Call the replica procedures of the serving package, speaking its release. */
    static replica(served: Package, endpoint: ClientOptions): Client<ReplicaProcedures> {
        const router = { replica: replicaProcedures };

        return createClient({ version: served.version, router }, endpoint).replica;
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

    /** Create or migrate a client's local tables for some object types and further tables. */
    static async migrate(
        database: DatabaseConnection,
        objects: Readonly<Record<string, ObjectType>>,
        own: readonly Table[] = [],
    ): Promise<void> {
        const { replicated, local } = ObjectClient.#divide(objects);
        await database.migrate(replicated, {
            isReplica: true,
            states: [declareState([...local, ...own], database.dialect)],
        });
    }

    /** List the tables of a client's local database with an unapplied declaration, by name. */
    static async unapplied(
        database: DatabaseConnection,
        objects: Readonly<Record<string, ObjectType>>,
    ): Promise<string[]> {
        const { replicated, local } = ObjectClient.#divide(objects);

        return [
            ...(await database.unapplied(replicated, { isReplica: true })),
            ...(await database.unapplied(local)),
        ];
    }

    /** Split a client's tables into replicated tables and local object tables. */
    static #divide(objects: Readonly<Record<string, ObjectType>>): {
        readonly replicated: Table[];
        readonly local: Table[];
    } {
        const local = ObjectClient.#storedTables(
            ObjectType.served(Object.values(objects)),
            "local",
        );

        return {
            replicated: ObjectClient.tables(objects).filter((table) => !local.includes(table)),
            local,
        };
    }

    /** Read and change some of the client's object types by key, refusing types it lacks. */
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
        return mapValues<Readonly<Record<string, Method>>, Mutator<Object>>(
            this.#routed(object, true),
            (_method, name) => (input: JsonObject) => this.call(object, name, input),
        );
    }

    /** Call an object type's mutating method by name as one mutation, as recorded calls do. */
    call(object: ObjectType, name: string, input: JsonObject): Submission<unknown> {
        // send an ephemeral call at once, and write a local one at once in the client's database
        if (object.storage === "ephemeral" || object.storage === "local") {
            const call = this.#recordCall(object, name, input);
            const done =
                object.storage === "ephemeral" ? this.#send(call) : this.#write(object, name, call);
            const confirmed = done.then(() => {});

            // mark the confirmation handled, and answer its failure in the returned submission
            confirmed.catch(() => {});

            return { predicted: done, confirmed };
        }

        // predict only the call, on the main line for a branch's own methods
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

    /** Predict several calls together, on the main line when asked, as one mutation. */
    #mutation<Value>(
        run: (mutation: ReplayMutation) => Promise<Value>,
        isMainLine: boolean,
    ): Submission<Value> {
        // predict every call
        const id = RequestId.create(this.clock.now());
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
                    // reject ephemeral and local objects, and branch calls on a branch
                    if (object.storage === "ephemeral" || object.storage === "local") {
                        throw new TypeError(
                            `${object.storage} object ${object.name} is written one call at a time`,
                        );
                    } else if (checkedOut !== undefined && this.#isBranchType(object)) {
                        throw new TypeError(
                            `${object.name} is written through mutate, on the main line`,
                        );
                    }

                    // record the call in the scope it acts in, refusing a second home
                    const scope = await this.#actingScope(database, object, name, input);
                    const entry = this.#recordCall(object, name, input, scope);
                    this.#requireOneHome(calls, entry);
                    calls.push(entry);
                    const stepped = await this.step(database, object, name, entry.input);
                    steps.push(stepped.step);

                    return stepped.result;
                };
                const result = await run({
                    call: (object) =>
                        mapValues<Readonly<Record<string, Method>>, MutationCalls<typeof object>>(
                            this.#routed(object, true),
                            (_method, name) => (input: JsonObject) => invoke(object, name, input),
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

        // mark the confirmation handled, and answer its failure in the returned submission
        confirmed.catch(() => {});

        return { predicted, confirmed };
    }

    /** Predict recorded calls and queue them as one mutation. */
    submit(calls: readonly sync.Call[]): Submission<void> {
        return this.#mutation(async (mutation) => {
            for (const entry of calls) {
                const { object, name } = this.method(entry);
                await mutation.invoke(object, name, entry.input);
            }
        }, false);
    }

    /** Call an object type's reading methods on the server, refusing local-only objects. */
    read<Object extends ObjectType>(object: Object): Reader<Object> {
        if (object.storage === "local") {
            throw new TypeError(`local object ${object.name} is read through its queries`);
        }

        return mapValues<Readonly<Record<string, Method>>, Reader<Object>>(
            this.#routed(object, false),
            (_method, name) => (input: JsonObject) => {
                // read in the client's scope
                const call = this.#recordCall(object, name, input);

                return this.#call((replica) => replica.call({ scope: this.scope, call }));
            },
        );
    }

    /** Check out a branch by identifier, following its rows, or the main line if undefined. */
    async checkout(branch: string | undefined): Promise<void> {
        // follow the branch's rows into the replica
        const types = this.branch;
        if (branch !== undefined && types === undefined) {
            throw new TypeError("a client without branch types cannot check out a branch");
        }
        await Promise.all(this.#branchFollows.map((follow) => follow.close()));
        this.#branchFollows =
            branch === undefined || types === undefined
                ? []
                : [
                      this.#live(types.object, { where: { id: branch } }),
                      this.#live(types.change, { where: { parentId: branch } }),
                  ];

        // show it over the main line
        await this.prediction.checkout(this.database, branch);
    }

    /** Follow the changes a branch makes over the main line, one per object. */
    diff(branch: string): BranchDiff {
        // follow the branch's rows into the replica
        const types = this.branch;
        if (types === undefined) {
            throw new TypeError("a client without branch types cannot follow a branch");
        }
        const rows = this.#live(types.change, { where: { parentId: branch } });
        const read = () =>
            new Branch(types, Snapshot.live(this.database), branch).changes(this.objects);

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

        return types !== undefined && types.has(object);
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
        await this.#call((replica) =>
            replica.broadcast({
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
        let wake = () => ObjectClient.#idle();
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

            // push the leading mutations of one home, a branch's edits as one push to the branch
            const { home, mutations } = this.#leading(pending.map((entry) => this.#pushed(entry)));
            const edits = new Set(
                pending.flatMap((entry) => (entry.branch === undefined ? [] : [entry.id])),
            );
            try {
                const result = await this.#pushTo(home, mutations, signal);
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
                    if (Replay.failure(error) !== undefined) {
                        return;
                    }
                }

                // back off
                failures += 1;
                await RetryPolicy.pause(this.#retry, failures, this.clock, signal);
            }
        }
    }

    /** Follow every party's queries and the caller's access rows until aborted. */
    async follow(signal: AbortSignal, report: (error: unknown) => void): Promise<void> {
        this.#report = report;
        await Promise.all(this.#follows().map((follow) => this.#follow(follow, signal, report)));
    }

    /** List the client's followed replicas. */
    #follows(): ClientFollow[] {
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
                              writerId: this.writerId,
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

    /** Follow one replica's shape, subscribing again once its parameters change. */
    async #follow(
        follow: ClientFollow,
        signal: AbortSignal,
        report: (error: unknown) => void,
    ): Promise<void> {
        // register the replica, and read its parameters as they are now
        const parameters = async () => PARAMETERS.parse(await follow.parameters());
        await follow.replica.register(this.database, { clock: this.clock });
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
            const replica = this.#replica;
            let isFinal = false;
            try {
                // follow the replica, resetting failures on each page
                await this.#followPages(follow, subscribed, replica, stream, resetFailures);
            } catch (error) {
                // restart on change or move, report other failures
                if (signal.aborted) {
                    return;
                } else if (stream.aborted || this.#redirect(error, replica)) {
                    continue;
                }
                report(error);
                isFinal = Replay.failure(error) !== undefined;
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
            await RetryPolicy.pause(this.#retry, failures, this.clock, signal);
        }
    }

    /** Drop the closed queries whose keep expired. */
    async #dropExpired(): Promise<void> {
        await this.database
            .delete(subscription)
            .where(lte(sql`${subscription.releasedAt} + ${subscription.keep}`, this.clock.now()));
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

    /** Follow a replica's shape from where the replica is, applying each page it streams. */
    async #followPages(
        follow: ClientFollow,
        parameters: JsonObject,
        procedures: Client<ReplicaProcedures>,
        stream: AbortSignal,
        onPage: () => void,
    ): Promise<void> {
        // stream the replica's shape from where the replica is
        const { replica, prediction } = follow;
        const followed: sync.Subscription = {
            name: replica.name,
            shape: follow.shape,
            scope: this.scope,
            below: this.scope,
            parameters,
            ...(this.#refresh === undefined ? {} : { refresh: this.#refresh }),
        };
        const from = await replica.resume(this.database, followed);
        const pages = await procedures.stream({ ...followed, ...from }, { signal: stream });
        let isGrown = false;
        for await (const page of replica.apply(this.database, pages, {
            subscription: followed,
            clock: this.clock,
            ...(prediction === undefined ? {} : { prediction }),
        })) {
            // deliver the page's broadcasts and scopes
            onPage();
            this.#broadcast(page);
            if (replica === this.replica) {
                this.#rescope(page);
            }

            // compact the local log
            const now = this.clock.now();
            await this.database.log.compact(now - this.#logMilliseconds, now);

            // evict once the predicted replica grew
            isGrown ||= page.reset || page.changes.some((change) => change.operation === "insert");
            if (page.complete && prediction !== undefined && isGrown) {
                isGrown = false;
                await this.#queueEviction();
            }
        }
    }

    /** Wake the watches over the objects' replica once its page reads other scopes. */
    #rescope(page: sync.Page): void {
        const scopes = page.scopes === undefined ? undefined : canonicalize(page.scopes);
        if (scopes !== undefined && scopes !== this.#scopes) {
            this.#scopes = scopes;
            this.#generation += 1;
            for (const wake of this.#rescoping) {
                wake();
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

    /** Check whether the caller has a permission on an object, over the client's replica. */
    async check<Object extends ObjectType>(
        object: Object,
        id: string,
        permission: PermissionOf<Object>,
    ): Promise<boolean> {
        const authorization = new Authorization(
            this.#authorizer,
            Transaction.reading(this.database),
            () => ({
                subjects: [this.caller],
                now: this.clock.now(),
                attributes: {},
            }),
        );
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
    queryType<Object extends ObjectType>(
        object: Object,
    ): RelationalQueryBuilder<Object, Objects[keyof Objects]> {
        return new RelationalQueryBuilder(this.#subscriber, object);
    }

    /** Keep a query's items live over the local replica with predictions until closed. */
    #live(
        object: ObjectType,
        query: Omit<ObjectQuery, "object"> = {},
        options: SubscribeOptions = {},
    ): LiveItems {
        const followed = this.#followed(object, query, options);
        const { compiled, named } = followed;

        return {
            query: compiled,
            ready: followed.ready,
            read: async () =>
                this.#presenter.present(
                    await this.#items(await named, compiled),
                    compiled,
                    this.objects,
                ),
            watch: (signal) => this.#watch(named, compiled, signal),
            close: () => followed.close(),
        };
    }

    /** Keep an aggregate query's groups live over the local replica until closed. */
    #measured(
        object: ObjectType,
        query: Omit<ObjectQuery, "object">,
        options: SubscribeOptions,
    ): LiveQuery<readonly sync.AggregateRow[]> {
        // read local groups here, and other groups through their replica
        const followed = this.#followed(object, query, options);
        const { compiled, named } = followed;
        const isDurable = object.storage === "durable";

        return {
            ready: followed.ready,
            read: async () =>
                object.storage === "local"
                    ? (await this.#dataflow(await named, compiled)).results(await named)
                    : this.#pickReplica(object.storage).results(
                          this.database,
                          await named,
                          compiled,
                          isDurable ? this.prediction : undefined,
                      ),
            watch: (signal) => this.#watchResults(named, compiled, signal),
            close: () => followed.close(),
        };
    }

    /** Follow a query in its open form over the local replica until closed. */
    #followed(
        object: ObjectType,
        query: Omit<ObjectQuery, "object">,
        options: SubscribeOptions,
    ): Follow {
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

        // mark readiness handled, and answer its failure in the returned query's ready
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
                            releasedAt: this.clock.now(),
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
    async release(object: ObjectType, query: Omit<ObjectQuery, "object"> = {}): Promise<void> {
        // find the query's closed subscriptions
        const split = this.#split(object, query);
        const name = await ObjectClient.#queryName(split.query);
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

    /** Read the parties with subscriptions or mutations in the shared replica. */
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

    /** Take the leading pushed mutations that share the first one's home. */
    #leading(pushed: readonly sync.Mutation[]): {
        readonly home: string;
        readonly mutations: sync.Mutation[];
    } {
        // read each mutation's home from its first call
        const homes = pushed.map((entry) => this.#readHome(aligned(entry.calls, 0)));
        const home = aligned(homes, 0);
        const other = homes.findIndex((each) => each !== home);

        return { home, mutations: pushed.slice(0, other === -1 ? pushed.length : other) };
    }

    /** Push mutations to their home, answering their outcomes and the replica's watermark. */
    async #pushTo(
        home: string,
        mutations: readonly sync.Mutation[],
        signal: AbortSignal,
    ): Promise<Pick<PushResult, "outcomes" | "watermark">> {
        // push the client's own scope's mutations to its replica
        if (home === this.scope) {
            return this.#call((replica) =>
                replica.push({ scope: this.scope, mutations: [...mutations] }, { signal }),
            );
        }

        // push an enclosing scope's mutations to its home
        const pushed = await this.#home(home).push(
            { scope: home, mutations: [...mutations] },
            { signal },
        );

        // read the replica's watermark, waiting for the home's commit when the chain has the home
        const chain = await this.replica.chain(this.database);
        const homes = chain.includes(home) ? [pushed.watermark] : [];
        const watermark = await this.#call((replica) =>
            replica.watermark({ scope: this.scope, homes }, { signal }),
        );

        return { outcomes: pushed.outcomes, watermark };
    }

    /** Wrap a branch's edit as one call appending its calls to the branch. */
    #pushed(entry: sync.Mutation & { readonly branch?: string }): sync.Mutation {
        // push a main-line mutation as it is, refusing a branch edit without branch types
        const types = this.branch;
        if (entry.branch === undefined) {
            return { id: entry.id, calls: entry.calls };
        } else if (types === undefined) {
            throw new TypeError("a client without branch types cannot push a branch edit");
        }
        const push = this.#recordCall(types.object, "push", {
            id: entry.branch,
            calls: entry.calls,
        });

        return { id: entry.id, calls: [push] };
    }

    /** Record pushed outcomes and revert rejected predictions, leaving branch edits unsettled. */
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

        // keep evicting after a failed eviction, and answer its failure in the returned promise
        this.#evictions = eviction.catch(() => {});

        return eviction;
    }

    /** Drop the oldest closed kept query while the replica exceeds its budget. */
    async #evict(): Promise<sync.SyncError | undefined> {
        // evict only a budgeted replica subscribed to the current queries
        const subscribed = ObjectClient.#queriesOf(await this.replica.subscribed(this.database));
        if (
            this.#storage === undefined ||
            !ObjectClient.#isSame(subscribed, await this.#subscribed("durable"))
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
                `local replica has ${rows} rows, more than ${this.#storage.rows}, for open and always kept queries`,
            );
        }
        await this.database
            .delete(subscription)
            .where(or(eq(subscription.id, oldest.id), eq(subscription.root, oldest.id)));

        return undefined;
    }

    /** Count the replica's object rows. */
    async #count(): Promise<number> {
        // count on from the last count through the log, leaving out local objects
        const replicated = this.objects.filter((object) => object.storage !== "local");
        const tables = replicated.map((object) => object.table);
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
                const counts = replicated.map(
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
                    gt(sql`${subscription.releasedAt} + ${subscription.keep}`, this.clock.now()),
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

    /** Keep a live union of several queries in one order and limit, with each entry's member. */
    union<const Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>>(
        members: Members,
        options: UnionOptions<MemberItem<Members>>,
    ): LiveQuery<readonly UnionEntry<Members>[]> {
        // refuse members ordering or limiting themselves, and follow each in the union's order and limit
        for (const [name, member] of Object.entries(members)) {
            if (member.options.orderBy !== undefined || member.options.limit !== undefined) {
                throw new TypeError(`union member ${name} cannot order or limit itself`);
            }
        }
        const orderBy = OrderBy.schema.parse(options.orderBy);
        const followed = Object.entries(members).map(([name, member]) => ({
            name,
            table: member.object.table,
            select: RelationalQuery.selecting(member.options, (item) => item),
            live: this.#live(
                member.object,
                { ...member.options, orderBy, limit: options.limit },
                options.keep === undefined ? {} : { keep: options.keep },
            ),
        }));
        const byName = new Map(followed.map((member) => [member.name, member]));
        const merge = (lists: readonly (readonly Item[])[]) => {
            // order the members' rows and select each row's item
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

                return mapValues<
                    { readonly name: string; readonly row: NestedItem },
                    UnionEntry<Members>
                >({ name, row: select(found(items, row), live.query) }, (value) => value);
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
        const watched = this.#rescoped(named, query, signal, (name, read, stop) =>
            this.#local.watch(name, read, stop),
        );
        for await (const items of watched) {
            yield this.#presenter.present(items, query, this.objects);
        }
    }

    /** Watch an aggregate query's groups through the local feed until aborted. */
    #watchResults(
        named: Promise<string>,
        query: sync.Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly sync.AggregateRow[]> {
        return this.#rescoped(named, query, signal, (name, read, stop) =>
            this.#local.watchResults(name, read, stop),
        );
    }

    /** Watch a query over the scopes the replica reads, again each time its pages change them. */
    async *#rescoped<Result>(
        named: Promise<string>,
        query: sync.Query,
        signal: AbortSignal,
        watch: (name: string, query: sync.Query, signal: AbortSignal) => AsyncGenerator<Result>,
    ): AsyncGenerator<Result> {
        const name = await named;
        while (!signal.aborted) {
            // read the scopes, again when a page changed them meanwhile
            const generation = this.#generation;
            const read = await this.#read(query);
            if (generation !== this.#generation) {
                continue;
            }

            // watch over them until a page brings others
            const rescoping = new AbortController();
            const wake = () => rescoping.abort();
            this.#rescoping.add(wake);
            try {
                yield* watch(name, read, AbortSignal.any([signal, rescoping.signal]));
            } finally {
                this.#rescoping.delete(wake);
            }
        }
    }

    /** Widen a durable query to every scope of its type the replica last read. */
    async #read(query: sync.Query): Promise<sync.Query> {
        // keep a query of no durable type, whose replica reads no chain
        const object = this.objects.find((type) => type.table === query.table);
        if (object?.storage !== "durable") {
            return query;
        }

        // read the scopes holding the type's rows, which the source chose to send
        const held = object.table[TABLE].column("scope").definition.schema;
        const read = await this.replica.chain(this.database);

        return { ...query, scopes: read.filter((scope) => held.safeParse(scope).success) };
    }

    /** Read a query's items from the local replica. */
    async #items(name: string, query: sync.Query): Promise<readonly Item[]> {
        return (await this.#dataflow(name, query)).read(name);
    }

    /** Load a query over the local database through the local feed's upstream. */
    async #dataflow(name: string, query: sync.Query): Promise<sync.Dataflow> {
        // read through the local feed's upstream
        const upstream = present(this.#local.upstream, "the local feed's upstream");
        const dataflow = new sync.Dataflow(
            { [name]: await this.#read(query) },
            {
                audience: sync.EVERYONE,
                database: this.database,
                upstream,
                changesThrough: sync.Dataflow.changesThrough(this.database),
                isMaterialized: true,
            },
        );
        await dataflow.load(await sync.View.latest(this.database));

        return dataflow;
    }

    /** Split a query by storage: its own storage's query, and key lookups of the other's rows. */
    #split(object: ObjectType, options: Omit<ObjectQuery, "object">): StorageQuery {
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
        const nested: Record<string, FindOptions> = {};
        for (const [name, relation] of FindOptions.includes({ with: included ?? {} })) {
            const joined = related(object, name, this.objects);
            if (joined.object.storage === object.storage) {
                const child = this.#split(joined.object, relation);
                const { object: _object, ...childOptions } = child.query;
                nested[name] = childOptions;
                lookups.push(
                    ...child.lookups.map((lookup) => ({ ...lookup, path: [name, ...lookup.path] })),
                );
            } else {
                const read: FindOptions = relation;
                const { orderBy: _orderBy, limit: _limit, ...rows } = read;
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
    #lookup(object: ObjectType, relation: string, options: FindOptions): Lookup {
        const { object: target, relation: joined } = related(object, relation, this.objects);
        if (joined.on.kind !== "key") {
            throw new CallError("BAD_REQUEST", {
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
                throw new CallError("BAD_REQUEST", {
                    message: `object ${object.name} cannot relate ${via}: it has another storage`,
                });
            }
        }
    }

    /** Follow a storage query and the lookups its rows key. */
    #subscribe(split: StorageQuery, root: string | undefined, signal: AbortSignal): Subscription {
        // key a local query, or subscribe a replicated one
        const id = RequestId.create(this.clock.now());
        const isLocal = split.object.storage === "local";
        const named = isLocal
            ? ObjectClient.#queryName(split.query)
            : this.#insertSubscription(split, id, root);

        // read a local query at once, or wait until the replica subscribes to it
        const replicated = named.then(async (name) => {
            if (!isLocal) {
                const replica = this.#pickReplica(split.object.storage);
                await this.database.log.until(
                    async () =>
                        Object.hasOwn(
                            ObjectClient.#queriesOf(await replica.subscribed(this.database)),
                            name,
                        ),
                    signal,
                );
            }
        });
        const lookups = new Map<Lookup, LookupSubscription>();
        const ids = () => [id, ...[...lookups.values()].flatMap((entry) => entry.subscribed.ids())];
        if (split.lookups.length === 0) {
            return { named, ready: replicated, done: named.then(() => {}), ids };
        }

        // key lookups from the query's rows, ready once the replica and the first lookups are
        const compiled = this.#compile(split.query);
        const { promise: ready, resolve: isReady } = Promise.withResolvers<void>();
        const signalReady = async () => {
            await replicated;
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

        return { named, ready: Promise.race([ready, done.then(() => replicated)]), done, ids };
    }

    /** Record a storage query as subscribed under its name, returning the name. */
    async #insertSubscription(
        split: StorageQuery,
        id: string,
        root: string | undefined,
    ): Promise<string> {
        const name = await ObjectClient.#queryName(split.query);
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
                    await this.#replaceLookup(
                        lookup,
                        ObjectClient.#keysOf(rows, lookup),
                        owner,
                        lookups,
                        signal,
                    );
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

    /** Replace a lookup with changed keys: stop the subscription and subscribe the new keys. */
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
                ObjectClient.#keyedTarget(lookup, keys),
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
                    const { object, name } = this.method(entry);
                    await this.#predict(savepoint, object, name, Call.upgrade(object, name, entry));
                }
            });
        } catch (error) {
            // keep a mutation the server would refuse for good pending unpredicted
            if (Replay.failure(error) === undefined) {
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
        const hasTarget = !Method.loads(method) || before !== undefined;
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
            object: object.typeReference,
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

        // split off routing fields, acting in the call's scope, else the type's scope here
        const { id, revision, ...rest } = input;
        const field = object.route.field;
        const fields = field === undefined ? rest : ObjectClient.#omitField(rest, field);
        const targetId = id === undefined ? undefined : schema.string().parse(id);
        const named = field === undefined ? undefined : rest[field];
        const scope = typeof named === "string" ? named : this.#readScope(object);
        const call: Call = new Call({
            object,
            name,
            method,
            scope,
            chain: object.storage === "local" ? [scope] : await this.replica.chain(database),
            input: fields,
            ...(targetId === undefined ? {} : { id: targetId }),
            target: undefined,
            prepared: undefined,
            caller:
                this.caller === undefined
                    ? { subjects: [] }
                    : { subject: this.caller, subjects: [this.caller] },
            now: this.clock.now(),
            location: { kind: "client", database },
            objects: this.objects,
            run: (entry, _caller, mutates, code) => {
                const nested = Call.resolve(this.objects, entry, mutates, code);

                return this.#predict(database, nested.object, nested.name, nested.input);
            },
        });

        // load the target at the given revision
        const table = object.table;
        const [target] =
            Method.loads(method) && targetId !== undefined
                ? await database
                      .select()
                      .from(table)
                      .where(eq(table[TABLE].column("id"), targetId))
                : [];
        if (Method.loads(method) && target === undefined) {
            throw new CallError("NOT_FOUND", { message: `no ${object.name} ${targetId}` });
        }
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw new CallError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        return method.execute(target === undefined ? call : call.with({ target: target }), {
            kind: "client",
            database,
        });
    }

    /** Call the replica procedures, forwarding once to the machine now serving a moved scope. */
    async #call<Value>(
        send: (replica: Client<ReplicaProcedures>) => Promise<Value>,
    ): Promise<Value> {
        const replica = this.#replica;
        try {
            return await send(replica);
        } catch (error) {
            // retry once after a move
            if (!this.#redirect(error, replica)) {
                throw error;
            }

            return send(this.#replica);
        }
    }

    /** Reconnect to the machine serving a moved scope, returning whether it was a move. */
    #redirect(error: unknown, replica: Client<ReplicaProcedures>): boolean {
        // ignore other failures
        const moved = CallError.data(error, "MISDIRECTED_REQUEST");
        if (moved === undefined || !("machine" in moved)) {
            return false;
        }

        // reconnect once per move
        if (replica === this.#replica) {
            this.#replica = this.#reconnect(moved.machine);
            this.#moves.abort();
            this.#moves = new AbortController();
        }

        return true;
    }

    /** Send one ephemeral call as this client. */
    async #send(call: sync.Call): Promise<unknown> {
        // push only the call
        const { outcomes } = await this.#call((replica) =>
            replica.push({
                scope: this.scope,
                mutations: [{ id: RequestId.create(this.clock.now()), calls: [call] }],
                writerId: this.writerId,
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

    /** Write one local call in the client's database, returning its result. */
    #write(object: ObjectType, name: string, call: sync.Call): Promise<unknown> {
        return this.database.transaction((transaction) =>
            this.#predict(transaction, object, name, call.input),
        );
    }

    /** Read an object type's scope here: the client's scope, or a local type's partition. */
    #readScope(object: ObjectType): string {
        // keep replicated objects in the client's scope
        if (object.storage !== "local") {
            return this.scope;
        }

        // keep local objects in the partition of the principal their scope type stands for
        const type = present(object.scopes[0], `the scope type of local object ${object.name}`);
        const owner = this.#owners.find((subject) => type.policy.is(subject));
        if (owner === undefined) {
            throw new TypeError(`the client has no ${type.name} for local object ${object.name}`);
        }

        return owner.id;
    }

    /** Record a call in a scope, the type's scope by default, with identifiers for creations. */
    #recordCall(
        object: ObjectType,
        name: string,
        input: JsonObject,
        scope = this.#readScope(object),
    ): sync.Call {
        // generate the identifiers of a creation and of each item of a bulk creation
        const { kind } = object.method(name);
        const generated =
            kind === "create" && input["id"] === undefined
                ? { id: object.generateId() }
                : kind === "createMany"
                  ? {
                        items: BatchItem.array()
                            .parse(input["items"])
                            .map((item) => ({
                                id: object.generateId(),
                                ...item,
                            })),
                    }
                  : {};

        // place the scope in the route field
        const field = object.route.field;
        const scoped = field === undefined ? {} : { [field]: scope };

        return Call.record(object, name, { ...input, ...generated, ...scoped });
    }

    /** Read the scope a call acts in: its own, its target's, else the client's. */
    async #actingScope(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: JsonObject,
    ): Promise<string> {
        // take the call's own scope
        const field = object.route.field;
        const named = field === undefined ? undefined : input[field];
        if (typeof named === "string") {
            return named;
        }

        // take the scope of the target the replica keeps, such as an enclosing scope's inherited row
        const id = input["id"];
        const row =
            Method.loads(object.method(name)) && typeof id === "string"
                ? await this.row(database, object, id)
                : undefined;

        return row === undefined ? this.scope : ObjectKey.parse(row).scope;
    }

    /** Refuse a call whose home is not the home of the mutation's calls before it. */
    #requireOneHome(calls: readonly sync.Call[], call: sync.Call): void {
        // require one home for every call
        const home = this.#readHome(call);
        const [first] = calls;
        const before = first === undefined ? home : this.#readHome(first);
        if (home !== before) {
            throw new CallError("BAD_REQUEST", {
                message: `a mutation changes rows of one home, and its calls change rows of ${before} and ${home}`,
            });
        }

        // require a home the client reaches
        if (home !== this.scope && !this.#isReachingHomes) {
            throw new CallError("BAD_REQUEST", {
                message: `the client of ${this.scope} cannot reach the home of ${home}`,
            });
        }
    }

    /** Read the scope whose home runs a recorded call. */
    #readHome(call: sync.Call): string {
        const { object, name } = this.method(call);

        return object.readScope(name, call.input);
    }

    /** List the tables a call's prediction may read, by SQL name. */
    #reads(call: sync.Call, tables: readonly Table[]): readonly string[] {
        // read the object's and parent's tables for a standard method, else every table
        const { object, method } = this.method(call);
        const parent = object.parent?.object;
        const isStandard = STANDARD_KINDS.has(method.kind) && parent !== "any";
        const read = isStandard
            ? [object.table, ...(parent === undefined ? [] : [parent.table])]
            : tables;

        return read.map((table) => table[TABLE].sqlName);
    }

    /** Find the object type and method of a recorded call. */
    method(call: Pick<ObjectCall, "type" | "method">): {
        readonly object: ObjectType;
        readonly name: string;
        readonly method: Method;
    } {
        const object = Call.findType(this.objects, call);
        if (!object) {
            throw new TypeError(`the client has no object ${call.type.type}`);
        }

        return { object, name: call.method, method: object.method(call.method) };
    }

    /** Pick a storage's replica, events beside durable rows, refusing one the client lacks. */
    #pickReplica(storage: ObjectStorage): sync.Replica {
        // pick the storage's replica
        const replicas = {
            durable: this.replica,
            events: this.replica,
            ephemeral: this.ephemeral,
            external: this.external,
            local: undefined,
        };
        const replica = replicas[storage];
        if (replica === undefined) {
            throw new TypeError(`the client has no replica of ${storage} objects`);
        }

        return replica;
    }

    /** Compile one object query of the scope its objects live in here. */
    #compile(query: ObjectQuery): sync.Query {
        // compile in the scope of the query's object type
        const object = this.objects.find((type) => type.name === query.object);
        const chain = [this.#readScope(present(object, `the client's object ${query.object}`))];
        const [compiled] = Object.values(ObjectType.queries(this.objects, { query }, chain));

        return present(compiled, `the compiled query of ${query.object}`);
    }

    /** List an object type's routed methods of one mutability, refusing a type the client lacks. */
    #routed(object: ObjectType, mutates: boolean): Readonly<Record<string, Method>> {
        // require a known object type
        if (!this.objects.some((type) => type.same(object))) {
            throw new TypeError(`the client has no object ${object.name}`);
        }

        return Object.fromEntries(
            Object.entries(object.methods).filter(
                ([, method]) => method.mutates === mutates && method.isInternal !== true,
            ),
        );
    }

    /** Read the most mutations one push sends, refusing a push larger than the server takes. */
    static #pushMutationsOf(push: { readonly mutations: number } | undefined): number {
        const mutations = push?.mutations ?? PUSH_MUTATIONS;
        if (!Number.isInteger(mutations) || mutations < 1 || mutations > PUSH_MUTATIONS) {
            throw new TypeError(`a push needs 1 to ${PUSH_MUTATIONS} mutations`);
        }

        return mutations;
    }

    /** List the tables of the object types kept in one storage. */
    static #storedTables(objects: readonly ObjectType[], storage: ObjectStorage): Table[] {
        return objects.filter((object) => object.storage === storage).map((object) => object.table);
    }

    /** Replicate some tables in a separate replica, absent for no tables. */
    static #separateReplica(
        name: string,
        scope: string,
        tables: readonly Table[],
    ): sync.Replica | undefined {
        return tables.length === 0 ? undefined : new sync.Replica({ name, scope, tables });
    }

    /** Read the distinct keys of a lookup's parent rows, in their JSON form, sorted. */
    static #keysOf(rows: readonly Item[], lookup: Lookup): Exclude<Scalar, null>[] {
        // follow the include path
        let reached: readonly Item[] = rows;
        for (const name of lookup.path) {
            reached = reached.flatMap((item) => present(item.with[name], `the ${name} include`));
        }

        // collect the distinct keys of the parent rows, in their JSON form
        const column = lookup.target.object.table[TABLE].column(lookup.column).definition;
        const keys = new Set<Exclude<Scalar, null>>();
        for (const item of reached) {
            const value = item.row[lookup.parent];
            if (value === null || value === undefined) {
                continue;
            } else if (typeof value === "bigint") {
                keys.add(ObjectClient.#scalarOf(column.toJson(value)));
            } else {
                keys.add(ObjectClient.#scalarOf(column.toJson(ObjectClient.#scalarOf(value))));
            }
        }

        return [...keys].toSorted((left, right) => String(left).localeCompare(String(right)));
    }

    /** Follow a query node up to its root. */
    static #rootOf(node: sync.Node): sync.Node {
        return node.parent === undefined ? node : ObjectClient.#rootOf(node.parent);
    }

    /** Wait for nothing, until a reader waiting for events replaces it. */
    static #idle(): void {}

    /** Require a lookup key value to be a scalar, as key columns keep them. */
    static #scalarOf(value: unknown): Exclude<Scalar, null> {
        if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
            return value;
        }

        throw new TypeError(`a lookup key is ${typeof value}, not a scalar`);
    }

    /** Narrow a lookup's target query to the given keys. */
    static #keyedTarget(lookup: Lookup, keys: readonly Exclude<Scalar, null>[]): StorageQuery {
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

    /** Read the queries a replica completed a run of, none before its first run. */
    static #queriesOf(
        subscribed: sync.SubscriptionRecord | undefined,
    ): Readonly<Record<string, ObjectQuery>> {
        // read no queries before the first run
        const queries = subscribed?.parameters["queries"];

        return queries === undefined ? {} : SUBSCRIBED_QUERIES.parse(queries);
    }

    /** Name a storage query by a prefix of its content's digest. */
    static async #queryName(query: ObjectQuery): Promise<string> {
        return (await Digest.json(query)).slice(0, QUERY_NAME_LENGTH);
    }

    /** Decide whether stored queries are the queries given. */
    static #isSame(stored: unknown, queries: Readonly<Record<string, ObjectQuery>>): boolean {
        return canonicalize(stored) === canonicalize(queries);
    }

    /** Leave out one field of an input. */
    static #omitField(input: JsonObject, field: string): JsonObject {
        const { [field]: _omitted, ...rest } = input;

        return rest;
    }
}

/** What the server answers a push. */
type PushResult = Awaited<ReturnType<Client<ReplicaProcedures>["push"]>>;

/** What the server made of one pushed mutation. */
type PushOutcome = PushResult["outcomes"][number];

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
    /** How long the closed query stays followed in milliseconds, absent until released. */
    keep: integer("keep"),
});

/** A replica the client follows: replica, shape and parameters, and the prediction it rebases. */
interface ClientFollow {
    /** The local replica. */
    readonly replica: sync.Replica;
    /** The followed shape. */
    readonly shape: string;
    /** Read the current subscription parameters: followed queries and the ephemeral writer. */
    readonly parameters: () => Promise<{
        readonly queries?: Readonly<Record<string, ObjectQuery>>;
        readonly writerId?: string;
    }>;
    /** The prediction rebased onto the replica, absent for replicas nothing predicts. */
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
    /** The parent rows' column with the key. */
    readonly parent: string;
    /** The condition the join adds, such as an attachment's host type. */
    readonly where?: Condition;
    /** The looked-up objects' query, before keying. */
    readonly target: StorageQuery;
}

/** A query in its open form followed over the local replica until closed. */
interface Follow {
    /** The compiled query. */
    readonly compiled: sync.Query;
    /** The name of the query its storage follows, once subscribed. */
    readonly named: Promise<string>;
    /** Settles once the replica has the query. */
    readonly ready: Promise<void>;
    /** Stop following the query. */
    readonly close: () => Promise<void>;
}

/** A storage query followed until closed, with the lookups its rows keyed so far. */
interface Subscription {
    /** The name of the storage query, once subscribed. */
    readonly named: Promise<string>;
    /** Settles once the storage's replica has it and its first lookups are ready. */
    readonly ready: Promise<void>;
    /** Settles once the query stops, rejecting with a failed lookup. */
    readonly done: Promise<void>;
    /** List the subscriptions the query and its current lookups have. */
    readonly ids: () => string[];
}

/** A lookup's subscription to the keys its rows held last. */
interface LookupSubscription {
    /** The canonical keys subscribed. */
    readonly keys: string;
    /** The subscription to the keyed objects. */
    readonly subscribed: Subscription;
    /** Stops the subscription. */
    readonly stop: AbortController;
}
