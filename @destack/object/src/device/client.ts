import type { Action } from "../declare/action.ts";
import type { ActionArguments } from "../declare/call.ts";
import type { ActionInputs } from "../declare/variable.ts";
import type { Place } from "../declare/place.ts";
import type { ActionContext } from "../declare/template.ts";
import { ObjectNode } from "../client/node.ts";
import { TextSpan } from "../sequence/index.ts";
import { textChunkTable } from "../text/table.ts";
import { ObjectReference, type Item, Subject, RequestId } from "@destack/sync";
import { TelemetryScope } from "@destack/telemetry";
import { ObjectError } from "../error/error.ts";
import {
    FindOptions,
    count,
    eq,
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
    declareState,
    Search,
} from "@destack/db";
import { type DatabaseConnection, Transaction } from "@destack/db/database";
import {
    canonicalize,
    Digest,
    type DigestCrypto,
    Duration,
    found,
    type JsonObject,
    JsonValue,
    mapValues,
    nth,
    present,
    schema,
    zip,
} from "@destack/schema";
import type { ObjectRemote } from "../replica/remote.ts";
import type { ObjectConnection } from "../replica/connection.ts";
import { Replay } from "../audit/replay.ts";
import { decisionTables } from "@destack/access";
import { Observable } from "../client/observable.ts";
import { RetryPolicy, type Clock, type Environment } from "@destack/environment";
import * as sync from "@destack/sync";
import * as syncDataflow from "@destack/sync/dataflow";
import { Failure } from "@destack/error";
import { Branch, BranchType } from "../branch/index.ts";
import { type StoredStep, UndoStack, undoEntry } from "../client/undo.ts";
import { Method } from "../method/method.ts";
import { BatchItem } from "../method/record.ts";

import {
    EPHEMERAL_PUBLICATION,
    EXTERNAL_PUBLICATION,
    ObjectQuery,
    QUERIES_PUBLICATION,
} from "../replica/replica.ts";
import { ObjectKey, ObjectType, type ObjectStorage, type PermissionOf } from "../object/object.ts";

import type { NestedItem } from "../query/item.ts";
import { RelationGraph } from "../query/query.ts";
import { TextPresenter } from "../client/presenter.ts";
import {
    RelationalQuery,
    ObjectQueryBuilder,
    type LiveItems,
    type Subscriber,
    type SubscribeOptions,
} from "../client/query.ts";

import { TextDocument } from "../client/text.ts";
import { MemoryBlobStorage } from "@destack/resource";
import { ObjectStore } from "../server/store.ts";
import { STORE_TABLES } from "../stack/db.ts";
import { Outbox } from "../server/outbox.ts";
import { type OnDemandFollow, type FollowKey, QUERY_TTL, Shape } from "../replica/shape.ts";
import { implementPackage, type PackageImplementation } from "../server/implementation.ts";
import { Package } from "../declare/package.ts";
import type {
    BranchDiff,
    ClientInspection,
    LiveQuery,
    MemberItem,
    MutationContext,
    Mutator,
    ObjectAccess,
    ObjectMutators,
    ObjectQueries,
    Reader,
    SearchOptions,
    SearchResult,
    Submission,
    UnionEntry,
    UnionOptions,
    Writer,
} from "../client/access.ts";
/** The hexadecimal digest digits identifying a query: 64 bits, safe among a client's queries. */
const QUERY_NAME_LENGTH = 16;

/** The follower of the keys a client's live queries follow, one per client database. */
const CLIENT_FOLLOWER = "client";

/** The follower of the keys a client follows forever, until released. */
const FOREVER_FOLLOWER = "forever";

/** The storages beside the durable replica: ephemeral and external replicas, and local objects. */
const SEPARATE_STORAGES = ["ephemeral", "external", "local"] as const;

/** How long the local log retains changes: a minute, past any tab's catch-up. */
const LOCAL_LOG_RETENTION = { minutes: 1 } as const;

/** A digit, which a word of an identifier or a date has. */
const DIGIT = /\p{Nd}/u;

/** Milliseconds per second: the monotonic clock reads milliseconds, durations record seconds. */
const MILLISECONDS = 1000;

/** The results of a pushed mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.unknown());

/** How a client replicates one scope's objects in its local database. */
export interface ObjectClientOptions<Objects extends Readonly<Record<string, ObjectType>>> {
    /** The local database. */
    readonly database: DatabaseConnection;
    /** The object types to replicate, by key. */
    readonly objects: Objects;
    /** The scope's branch types, among the objects, to check out and edit branches. */
    readonly branch?: BranchType;
    /** The scope whose objects to replicate. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The device the database lives on, whose partition stores local objects scoped to devices. */
    readonly device?: Subject;
    /** Where local objects live: served here by default, or followed from the remote. */
    readonly local?: "served" | "followed";
    /** The node the client follows and writes through, such as its machine or device. */
    readonly remote: ObjectRemote;
    /** Open the node toward each followed scope's home, the remote for every scope by default. */
    readonly publisher?: (home: string) => ObjectRemote;
    /** The environment the client runs in, whose clock compacts its log and decides access. */
    readonly environment: Environment;
    /** How often merged changes arrive, such as for a background tab. */
    readonly refresh?: { readonly every: Duration };
    /** The most object rows the local replica stores, absent for no limit. */
    readonly storage?: { readonly rows: number };
    /** How long the local log retains changes, a minute by default. */
    readonly log?: { readonly retention: Duration };
    /** How transient failures retry, over the defaults. */
    readonly retry?: Partial<RetryPolicy>;
    /** The most mutations one push sends, the server's most by default. */
    readonly push?: { readonly mutations: number };
    /** Wait until a party writing through this client closes, such as a tab of a device. */
    readonly closed?: (origin: string, signal: AbortSignal) => Promise<void>;
}

/** One scope's objects on a client. */
export class ObjectClient<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> implements ObjectNode<Objects> {
    /** The local database with the replica and the queued mutations. */
    readonly database: DatabaseConnection;
    /** The relations between the object types the client replicates, text types among them. */
    readonly relations: RelationGraph;
    /** The relational queries of the object types, by key. */
    readonly query: ObjectQueries<Objects>;
    /** Run queries live over the local replica, for its relational queries and its tabs. */
    readonly subscriber: Subscriber = {
        rows: (object, query, options) => this.#live(object, query, options),
        results: (object, query, options) => this.#measured(object, query, options),
    };
    /** The presentation of read rows' texts, each row and list presented once. */
    readonly #presenter: TextPresenter;
    /** The scope whose objects the client replicates. */
    readonly scope: string;
    /** The calling principal. */
    readonly caller: Subject;
    /** The principals whose partitions store the client's local objects: caller and client. */
    readonly #owners: readonly Subject[];
    /** The replica of the scope's durable objects. */
    readonly replica: syncDataflow.Replica;
    /** The replica of the scope's ephemeral objects, absent when the client has none. */
    readonly ephemeral: syncDataflow.Replica | undefined;
    /** The replica of the scope's external objects, absent when the client has none. */
    readonly external: syncDataflow.Replica | undefined;
    /** The replica of the access rows the caller's checks read along the scope chain. */
    readonly access: syncDataflow.Replica;
    /** The node's queued mutations, predicted here and pushed to the remote. */
    readonly #outbox: Outbox;
    /** The scope's branch types, absent for a client without branches. */
    readonly branch: BranchType | undefined;
    /** The checked-out branch and its rows, followed into the replica. */
    #branchFollows: readonly LiveQuery[] = [];
    /** The party's undo and redo stacks. */
    readonly undoStack: UndoStack;
    /** The party the client queues its mutations as: its caller, as the outbox keys parties. */
    readonly party: string;
    /** The node the client follows and writes through, such as its machine or device. */
    readonly #remote: ObjectRemote;
    /** The store of the client's node, which runs its predictions and its local objects' calls. */
    readonly store: ObjectStore;
    /** The shapes of the client's live queries by storage, which its store follows on demand. */
    readonly #follows: ReadonlyMap<ObjectStorage, OnDemandFollow<{ readonly query: ObjectQuery }>>;
    /** Whether the local objects live at the remote, as a tab's live at its device. */
    readonly #isLocalFollowed: boolean;
    /** The local feed of watched queries. */
    readonly #local: syncDataflow.Feed;
    /** The options the client opened with, which its scope clients derive theirs from. */
    readonly #options: ObjectClientOptions<Objects>;
    /** The client whose store this client shares, absent for the client owning its store. */
    readonly #base: ObjectClient<Objects> | undefined;
    /** Stop failing the open queries on the store's refusals. */
    readonly #stopRefusals: () => void;
    /** The environment the client runs in: its clock, network and randomness. */
    readonly environment: Environment;
    /** Aborts outcome waits, lookups and watches on close. */
    readonly #closing = new AbortController();
    /** The refusal the queries open now fail with once the source refuses a follow for good. */
    #refusal = new FollowRefusal();
    /** Settles once the replica has the caller's access rows, or false once the client closes. */
    #accessed: Promise<boolean> | undefined;
    /** The open followed queries. */
    readonly #subscriptions = new Set<Subscription>();
    /** The unfollows of closed queries still under way. */
    readonly #unfollows = new Set<Promise<void>>();
    /** How many live queries follow each key, by shape and key. */
    readonly #followCounts = new Map<string, number>();
    /** The client's captures, on the environment's telemetry. */
    readonly #instruments: TelemetryScope;
    /** The seconds each live query read of the replica takes. */
    readonly #reads;

    /** Replicate one scope's objects in a local database, over a base client's store when given. */
    private constructor(options: ObjectClientOptions<Objects>, base?: ObjectClient<Objects>) {
        // declare the client's tables to its database once
        this.database = options.database;
        this.#options = options;
        this.#base = base;
        if (base === undefined) {
            this.database.state.declare(ObjectClient.tables(options.objects));
        }

        // store the options, refusing a push larger than the server takes
        this.#presenter = new TextPresenter();
        this.environment = options.environment;
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            options.environment.telemetry,
        );
        this.#reads = this.#instruments.metric.histogram("destack.client.query.duration", {
            unit: "s",
            attributes: {},
        });
        this.party = Outbox.origin(options.caller);
        this.undoStack = new UndoStack(this);

        // replicate the scope's objects for the caller, and the principals storing local objects
        this.relations = base?.relations ?? new RelationGraph(Object.values(options.objects));
        this.query = ObjectQueryBuilder.of(this.subscriber, options.objects);
        this.scope = options.scope;
        this.caller = options.caller;
        this.#owners = [options.caller, ...(options.device === undefined ? [] : [options.device])];

        // follow each storage's live queries through one shape, predicting the party's calls in the node's store
        this.#remote = options.remote;
        this.#isLocalFollowed = options.local === "followed";
        this.#follows =
            base === undefined
                ? ObjectClient.#queryFollows(this.objects, this.#isLocalFollowed)
                : base.#follows;
        const tables = ObjectClient.#storedTables(this.objects, "durable");
        this.branch = options.branch;
        this.store = base?.store ?? this.#openStore(options);
        this.#outbox = present(this.store.outbox, "the client's outbox");
        this.#stopRefusals = this.store.listenRefusals((error) => {
            if (Replay.failure(error) !== undefined) {
                this.#refusal = this.#refusal.refuse(error);
            }
        });

        // read the store's replicas of the scope: durable, ephemeral and external objects
        this.replica = new syncDataflow.Replica({
            name: Shape.replicaName(QUERIES_PUBLICATION, options.scope),
            scope: options.scope,
            tables,
        });
        this.ephemeral = ObjectClient.#separateReplica(
            EPHEMERAL_PUBLICATION,
            options.scope,
            this.#stored("ephemeral"),
        );
        this.external = ObjectClient.#separateReplica(
            EXTERNAL_PUBLICATION,
            options.scope,
            this.#stored("external"),
        );

        // read the caller's access rows the store follows, and decide its checks over the replicas
        this.access = this.store.accessReplica(options.scope);

        // read every replica and the local objects through one local feed
        const others = SEPARATE_STORAGES.flatMap((each) => this.#stored(each));
        this.#local = new syncDataflow.Feed(
            options.database,
            [...tables, ...others, sync.replicaResult],
            {
                upstream: this.#upstream(this.objects),
            },
        );
    }

    /** Open the store of a base client's database, settling undo entries as the server answers. */
    #openStore(options: ObjectClientOptions<Objects>): ObjectStore {
        const { branch, storage, refresh } = options;

        return new ObjectStore({
            servers: ObjectClient.#servers(this.objects, [...new Set(this.#follows.values())], {
                isLocalServed: !this.#isLocalFollowed,
            }),
            principal: options.caller,
            database: options.database,
            blobs: new MemoryBlobStorage(options.environment.crypto),
            origin: { package: nth(this.objects, 0).package },
            chain: async () => undefined,
            remote: options.remote,
            ...(options.publisher === undefined ? {} : { publisher: options.publisher }),
            ...(options.device === undefined ? {} : { device: options.device }),
            ...(storage === undefined ? {} : { storage }),
            ...(refresh === undefined ? {} : { refresh }),
            log: { retention: options.log?.retention ?? LOCAL_LOG_RETENTION },
            outbox: {
                ...(branch === undefined ? {} : { branch }),
                ...(options.push === undefined ? {} : { mutations: options.push.mutations }),
                ...(options.retry === undefined ? {} : { retry: options.retry }),
                ...(options.closed === undefined ? {} : { closed: options.closed }),
                executed: (transaction, id, value) => UndoStack.settle(transaction, id, value),
                rejected: (transaction, ids) => UndoStack.forget(transaction, ids),
            },
            environment: options.environment,
        });
    }

    /** Serve each package's local object types, and follow the live queries' shapes on demand. */
    static #servers(
        objects: readonly ObjectType[],
        follows: readonly OnDemandFollow<{ readonly query: ObjectQuery }>[],
        options: { readonly isLocalServed: boolean },
    ): PackageImplementation[] {
        // group the served local types by their package, the first serving the follow
        const local = Map.groupBy(
            objects.filter((object) => object.storage === "local" && options.isLocalServed),
            (object) => object.package.id,
        );
        const owner = nth(objects, 0).package;
        const packages = new Map([
            [owner.id, owner],
            ...objects.map((each) => [each.package.id, each.package] as const),
        ]);

        return [...new Set([owner.id, ...local.keys()])].map((id) => {
            const served = local.get(id) ?? [];
            const handle = Package.of(present(packages.get(id), `package ${id}`), {
                objects: served,
            });

            return implementPackage(handle, {
                objects: served,
                ...(id === owner.id ? { follows } : {}),
            });
        });
    }

    /** Follow one shape of live queries on demand per publication of the followed types. */
    static #queryFollows(
        objects: readonly ObjectType[],
        isLocalFollowed: boolean,
    ): ReadonlyMap<ObjectStorage, OnDemandFollow<{ readonly query: ObjectQuery }>> {
        // group the followed types by the publication they follow through
        const replicated = objects.filter(
            (object) => object.storage !== "local" || isLocalFollowed,
        );
        const byPublication = Map.groupBy(replicated, (object) => Shape.publicationOf(object));

        // key each publication's shape by the storages of its types
        return new Map(
            [...byPublication].flatMap(([, members]) => {
                const follow = Shape.queried(members).onDemand(QUERY_TTL);

                return [...new Set(members.map((object) => object.storage))].map(
                    (storage) => [storage, follow] as const,
                );
            }),
        );
    }

    /** List the tables of the client's objects of one storage. */
    #stored(storage: ObjectStorage): Table[] {
        return ObjectClient.#storedTables(this.objects, storage);
    }

    /** Measure each query through the replica with its root. */
    #upstream(objects: readonly ObjectType[]): syncDataflow.Upstream {
        // map tables to storages, events as durable, and storages to their replicas
        const storages = new Map(
            objects.map((object) => [
                object.table,
                object.storage === "events" ? "durable" : object.storage,
            ]),
        );
        const upstreams = new Map<ObjectStorage, syncDataflow.Upstream>([
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
                    `no ${storage} replica for table '${ObjectClient.#rootOf(node).table[TABLE].sqlName}'`,
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

    /** Create the local tables and replicate one scope's objects in the database. */
    static async open<const Objects extends Readonly<Record<string, ObjectType>>>(
        options: ObjectClientOptions<Objects> & {
            /** Whether another party created the local tables. */
            readonly isMigrated?: boolean;
        },
    ): Promise<ObjectClient<Objects>> {
        // create the local tables, else read which tables the party that created them replicates
        if (options.isMigrated !== true) {
            await ObjectClient.migrate(options.database, options.objects);
        } else {
            await options.database.declareApplied(ObjectClient.tables(options.objects));
        }

        // let the keys a closed client's live queries left lapse, and follow its scope's access rows when it replicates any
        const client = new ObjectClient(options);
        await client.#lapseLeft();
        await client.#followAccess();

        return client;
    }

    /** Open the client of another scope over this client's database, store and outbox. */
    async at(scope: string): Promise<ObjectClient<Objects>> {
        // derive the scope's options, its remote toward the scope's home
        const options = this.#options;
        const remote = options.publisher?.(scope) ?? options.remote;
        const client = new ObjectClient({ ...options, scope, remote }, this.#base ?? this);

        // follow the scope's access rows when the client replicates any
        await client.#followAccess();

        return client;
    }

    /** Follow the scope's access rows when the client replicates any. */
    async #followAccess(): Promise<void> {
        const isReplicating = [...this.#follows.values()].some((follow) =>
            follow.shape.objects.some((type) => type.storage !== "local"),
        );
        if (isReplicating) {
            await this.store.follow(CLIENT_FOLLOWER, ObjectStore.ACCESS_FOLLOW, [
                { key: { scope: this.scope } },
            ]);
        }
    }

    /** Let the keys an earlier client's live queries left lapse. */
    async #lapseLeft(): Promise<void> {
        const left = (await this.store.followKeys()).filter(
            (key) => key.follower === CLIENT_FOLLOWER && key.expiresAt === null,
        );
        for (const follow of new Set(this.#follows.values())) {
            const keys = left
                .filter((key) => key.shape === follow.shape.name)
                .map(({ scope, parameters }) => ({
                    key: { scope, query: ObjectQuery.parse(parameters["query"]) },
                }));
            await this.store.unfollow(CLIENT_FOLLOWER, follow, keys, QUERY_TTL);
        }
    }

    /** Open a client over a database and a connection only it uses, until the stack closes. */
    static async start<const Objects extends Readonly<Record<string, ObjectType>>>(
        options: ObjectClientOptions<Objects> & { readonly remote: ObjectConnection },
        stack: AsyncDisposableStack,
    ): Promise<ObjectClient<Objects>> {
        // open the client, closed with the stack before its connection
        stack.defer(() => options.remote.close());
        const client = await ObjectClient.open(options);
        stack.defer(() => client.close());

        // push and follow until then, capturing a failure that ends the loops
        const stopping = new AbortController();
        const running = client
            .run(stopping.signal)
            .catch((error: unknown) =>
                client.#instruments.captureException(error, { isEscaped: true }),
            );
        stack.defer(async () => {
            stopping.abort();
            await running;
        });

        return client;
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

    /** Write a branch's rows from the client's replica over its other rows. */
    /** What the replicas show over their homes' rows: queued mutations and the branch. */
    get prediction(): syncDataflow.Prediction {
        return this.#outbox.prediction;
    }

    /** The writer of the client's ephemeral objects: its store's. */
    get writerId(): string {
        return this.store.writerId;
    }

    /** The object types the client replicates, with the text types serving their texts. */
    get objects(): readonly ObjectType[] {
        return this.relations.objects;
    }

    /** The clock the client reads time at, its environment's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** List the tables a client's local database has for some object types. */
    static tables(objects: Readonly<Record<string, ObjectType>>): Table[] {
        return [
            ...new Set([
                ...STORE_TABLES,
                ...decisionTables,
                ...new RelationGraph(Object.values(objects)).objects.map((object) => object.table),
                ...sync.replicaTables,
                ...sync.predictionTables,
                undoEntry,
            ]),
        ];
    }

    /** Create or migrate a client's local tables for some object types and further tables. */
    static async migrate(
        database: DatabaseConnection,
        objects: Readonly<Record<string, ObjectType>>,
        extra: readonly Table[] = [],
    ): Promise<void> {
        const { replicated, local } = ObjectClient.#divide(objects);
        await database.migrate(replicated, {
            isReplica: true,
            replicated: new Set(replicated.map((table) => table[TABLE].sqlName)),
            states: [declareState([...local, ...extra], database.dialect)],
        });
    }

    /** List the tables of a client's local database with an unapplied declaration, by name. */
    static async unapplied(
        database: DatabaseConnection,
        objects: Readonly<Record<string, ObjectType>>,
    ): Promise<string[]> {
        const { replicated, local } = ObjectClient.#divide(objects);

        return [
            ...(await database.unapplied(replicated, {
                isReplica: true,
                replicated: new Set(replicated.map((table) => table[TABLE].sqlName)),
            })),
            ...(await database.unapplied(local)),
        ];
    }

    /** Split a client's tables into replicated tables and local object tables. */
    static #divide(objects: Readonly<Record<string, ObjectType>>): {
        readonly replicated: Table[];
        readonly local: Table[];
    } {
        const local = ObjectClient.#storedTables(
            new RelationGraph(Object.values(objects)).objects,
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
        // require replicated types
        for (const object of Object.values(objects)) {
            if (!this.objects.some((type) => type.same(object))) {
                throw new TypeError(`the client has no object ${object.name}`);
            }
        }

        return {
            query: ObjectQueryBuilder.of(this.subscriber, objects),
            mutate: ObjectClient.#mutators(this, objects),
            mutation: (run) => this.mutation(run),
            undoStack: this.undoStack,
            call: (object, name, input) => this.call(object, name, input),
            check: (object, id, permission) => this.check(object, id, permission),
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
                object.storage === "ephemeral" || this.#isLocalFollowed
                    ? this.#send(call, this.#readScope(object))
                    : this.#write(object, name, call);
            const confirmed = done.then(() => {});

            // mark the confirmation handled, and answer its failure in the returned submission
            confirmed.catch(() => {});

            return { predicted: done, confirmed };
        }

        // predict only the call, on the main line for a branch's methods
        let result: unknown;
        const pending = this.#mutation(async (mutation) => {
            result = await mutation.invoke(object, name, input);
        }, this.#isBranchType(object));

        return {
            predicted: pending.predicted.then(() => result),
            confirmed: pending.confirmed,
        };
    }

    /** Run an action's call on resolved inputs as one mutation, predicted and undone as one. */
    perform<Inputs extends ActionInputs>(
        action: Action<Inputs>,
        values: ActionArguments<Inputs>,
    ): Submission<readonly unknown[]> {
        // run a branch type's calls on the main line
        const call = present(action.call, `the call of action '${action.name}'`);

        return ObjectNode.perform(this, action, values, (run) =>
            this.#mutation(run, this.#isBranchType(call.object)),
        );
    }

    /** Read what actions read here: who the client is for, its scope, the time, and some places. */
    context(places: readonly Place[] = []): ActionContext {
        return ObjectNode.context(this, places);
    }

    /** Follow objects by reference across types and scopes. */
    get(references: readonly ObjectReference[]): LiveQuery<ReadonlyMap<string, JsonObject>> {
        return ObjectNode.get(this, references);
    }

    /** Read objects by reference once: the rows the device has, else the server's answer. */
    fetch(references: readonly ObjectReference[]): Promise<readonly JsonObject[]> {
        return ObjectNode.fetch(this, references);
    }

    /** Open one text field of an object for editing. */
    text(object: ObjectType, id: string, field: string): TextDocument {
        return new TextDocument(this, object, id, field);
    }

    /** Predict several calls together and queue them as one mutation. */
    mutation<Value>(run: (mutation: MutationContext) => Promise<Value>): Submission<Value> {
        return this.#mutation(run, false);
    }

    /** Predict several calls together, on the main line when asked, as one mutation. */
    #mutation<Value>(
        run: (mutation: MutationContext) => Promise<Value>,
        isMainLine: boolean,
    ): Submission<Value> {
        // predict every call, as a new mutation or composed into the latest one
        const id = RequestId.create(this.environment);
        const queued = this.prediction.add(
            this.database,
            id,
            this.party,
            async (database, latest) => {
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
                            `${object.storage} object ${object.name} cannot be written in a batch`,
                        );
                    } else if (checkedOut !== undefined && this.#isBranchType(object)) {
                        throw new TypeError(
                            `${object.name} is a branch type: write it through mutate on the main line`,
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
                        mapValues<Readonly<Record<string, Method>>, Writer<typeof object>>(
                            this.#routed(object, true),
                            (_method, name) => (input: JsonObject) => invoke(object, name, input),
                        ),
                    invoke,
                });

                // remember for undo, composing into the latest mutation when every call is predicted
                const isPredicted = calls.every((call) => this.method(call).method.isPredicted);
                const composable = isPredicted ? latest?.id : undefined;
                const composed = await this.undoStack.remember(database, id, steps, composable);

                // queue the calls, or replace the latest mutation's with the composed ones
                if (composed === undefined || latest === undefined) {
                    return { kind: "add", calls, result };
                }
                const replaced = latest.calls.map((call, position) => ({
                    ...call,
                    input: nth(composed, position).input,
                }));

                return { kind: "compose", calls: replaced, result };
            },
            { isMainLine },
        );

        // confirm with the server's outcome of the mutation holding the calls
        const predicted = queued.then((entry) => entry.result);
        const confirmed = queued.then((entry) => this.outcome(entry.id));

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
            throw new TypeError(`local object ${object.name} can only be read through its queries`);
        }

        return mapValues<Readonly<Record<string, Method>>, Reader<Object>>(
            this.#routed(object, false),
            (_method, name) => (input: JsonObject) => {
                // read in the client's scope
                const call = this.#recordCall(object, name, input);

                return this.#remote.call(this.scope, call);
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
            new Branch(types, this.database.snapshot(), branch).changes(this.objects);

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

    /** Report whether an object type has branches, whose calls stay on the main line. */
    #isBranchType(object: ObjectType): boolean {
        const types = this.branch;

        return types !== undefined && types.has(object);
    }

    /** Close the client and fail unsettled mutations. */
    async close(): Promise<void> {
        // stop the scope's queries, closing the outbox when the client owns the store
        this.#closing.abort();
        this.#stopRefusals();
        await Promise.all([
            ...(this.#base === undefined ? [this.#outbox.close()] : []),
            ...[...this.#subscriptions].map((subscribed) => subscribed.done),
            ...this.#unfollows,
        ]);
    }

    /** Send an unstored event to an object's readers. */
    async broadcast(object: ObjectType, id: string, event: JsonValue): Promise<void> {
        await this.#remote.broadcast(
            this.scope,
            { packageId: object.policy.definition.packageId, type: object.name, id },
            event,
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
        const stop = this.store.listen(topic, listener);
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
            stop();
        }
    }

    /** Check whether the caller has a permission on an object, over the client's replica. */
    async check<Object extends ObjectType>(
        object: Object,
        id: string,
        permission: PermissionOf<Object>,
    ): Promise<boolean> {
        // decide in the store over its replica of the caller's access rows, as their home decides
        const caller = await this.store.callerContext(this.scope, this.database);
        const authorization = await this.store.authorize(
            Transaction.reading(this.database),
            this.scope,
            caller,
        );
        const decision = await authorization.check(
            object.permission(permission),
            object.reference(this.scope, id),
        );

        return decision.isAllowed;
    }

    /** Push the queued mutations and follow the held queries until aborted. */
    async run(signal: AbortSignal): Promise<void> {
        // refuse running a scope client, whose base runs the shared store
        if (this.#base !== undefined) {
            throw new TypeError(`the client of scope '${this.scope}' runs through its base client`);
        }

        await Promise.all([this.push(signal), this.follow(signal)]);
    }

    /** Push the queued mutations in order until aborted. */
    push(signal: AbortSignal): Promise<void> {
        return this.#outbox.push(signal);
    }

    /** Follow the held queries and the caller's access rows through the store until aborted. */
    async follow(signal: AbortSignal): Promise<void> {
        await this.store.run(signal);
    }

    /** Query one of the client's object types, as its keyed queries do. */
    queryType<Object extends ObjectType>(
        object: Object,
    ): ObjectQueryBuilder<Object, Objects[keyof Objects]> {
        return new ObjectQueryBuilder(this.subscriber, object);
    }

    /** Follow a query's items live over the local replica with predictions until closed. */
    #live(
        object: ObjectType,
        query: Omit<ObjectQuery, "object" | "package"> = {},
        options: SubscribeOptions = {},
    ): LiveItems {
        const followed = this.#followed(object, query, options);
        const { compiled, named } = followed;

        return {
            query: compiled,
            ready: followed.ready,
            read: async () => {
                // read the items from the replica, recording how long the read took
                const started = this.environment.telemetry.monotonic.now();
                const items = this.#presenter.present(
                    await this.#items(await named, compiled),
                    compiled,
                    this.objects,
                );
                const seconds =
                    (this.environment.telemetry.monotonic.now() - started) / MILLISECONDS;
                this.#reads.record(seconds);

                return items;
            },
            watch: (signal) =>
                this.#refusal.watch(
                    (stop) => this.#watch(named, compiled, stop),
                    [signal, this.#closing.signal],
                ),
            close: () => followed.close(),
        };
    }

    /** Follow an aggregate query's groups live over the local replica until closed. */
    #measured(
        object: ObjectType,
        query: Omit<ObjectQuery, "object" | "package">,
        options: SubscribeOptions,
    ): LiveQuery<readonly sync.AggregateGroup[]> {
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
            watch: (signal) =>
                this.#refusal.watch(
                    (stop) => this.#watchResults(named, compiled, stop),
                    [signal, this.#closing.signal],
                ),
            close: () => followed.close(),
        };
    }

    /** Follow a query in its open form over the local replica until closed. */
    #followed(
        object: ObjectType,
        query: Omit<ObjectQuery, "object" | "package">,
        options: SubscribeOptions,
    ): Follow {
        // split the query by storage
        if (!this.objects.some((type) => type.same(object))) {
            throw new TypeError(`the client has no object ${object.name}`);
        }
        const split = this.#split(object, query);
        const whole = ObjectQuery.parse({
            object: object.name,
            package: object.package.id,
            ...query,
        });
        const compiled = this.#compile(whole);

        // follow the storage query and its lookups
        const closed = new AbortController();
        const subscribed = this.#subscribe(
            split,
            AbortSignal.any([closed.signal, this.#closing.signal]),
        );
        this.#subscriptions.add(subscribed);

        // settle once the replica has the query, and the caller's access for objects it follows, or refused
        const refusal = this.#refusal;
        const accessed = object.storage === "local" ? true : this.#access();
        const replicated = Promise.all([subscribed.ready, accessed]).then(() => {});
        const ready = Promise.race([replicated, refusal.failed]);
        ready.catch(() => {});

        return {
            compiled,
            named: subscribed.named,
            ready,
            close: async () => {
                // stop following
                closed.abort();
                await subscribed.done;
                this.#subscriptions.delete(subscribed);

                // unfollow its keys at once, after their time to live, or never, but leave a closed client's to lapse
                if (!this.#closing.signal.aborted) {
                    const unfollow = subscribed.unfollow(options.ttl);
                    this.#unfollows.add(unfollow);
                    await unfollow.finally(() => this.#unfollows.delete(unfollow));
                }
            },
        };
    }

    /** Search the objects of some types live, the best and newest first, all for an empty text. */
    search(text: string, options: SearchOptions): LiveQuery<SearchResult> {
        // search the text as it is
        const search = RelationalQuery.presented(this.#search(text, options), (result) => ({
            ...result,
            isCorrected: false,
        }));
        if (options.typos !== "correct") {
            return search;
        }

        // search as someone most likely meant while the text itself finds nothing
        return RelationalQuery.either(
            search,
            ({ hits }) => hits.length === 0,
            async () => {
                const meant = await this.#meant(text, options);

                return meant === undefined
                    ? undefined
                    : RelationalQuery.presented(this.#search(meant, options), (result) => ({
                          ...result,
                          isCorrected: true,
                      }));
            },
        );
    }

    /** Search the objects of some types by a text as it is. */
    #search(text: string, options: SearchOptions): LiveQuery<Omit<SearchResult, "isCorrected">> {
        // match the words of each type's objects, scored by the columns it searches
        const search = Search.of(text);
        const where = search.isEmpty ? {} : { SEARCH: search.words.join(" ") };
        const members = Object.fromEntries(
            options.types.map((type) => [
                type.key,
                this.queryType(type).findMany({
                    where: { AND: [options.where ?? {}, where] },
                    extras: { relevance: relevanceOf(type, search), changed: changedOf(type) },
                }),
            ]),
        );
        const byKey = new Map(options.types.map((type) => [type.key, type]));
        const union = this.union(members, {
            orderBy: { relevance: "desc", changed: "desc" },
            limit: options.limit,
        });

        return RelationalQuery.presented(union, (entries) => ({
            text,
            hits: entries.map(({ name, row: read }) => {
                // reference the object
                const type = found(byKey, name);
                const { relevance, changed: _changed, ...row } = JsonValue.object(read);
                const excerpt = excerptOf(type, search, row);

                return {
                    type,
                    reference: type.reference(this.scope, schema.string().parse(row["id"])),
                    row,
                    relevance: schema.number().parse(relevance),
                    ...(excerpt === undefined ? {} : { excerpt }),
                };
            }),
        }));
    }

    /** Read what someone most likely meant by a text that finds nothing on this device. */
    async #meant(text: string, options: SearchOptions): Promise<string | undefined> {
        // replace each word by the closest of the types' indexes, leaving out one like none
        const tables = options.types.flatMap((type) => [
            type.table,
            ...(type.text.length === 0 ? [] : [textChunkTable]),
        ]);
        const indexes = [...new Set(tables)].flatMap((table) => table[TABLE].fullText ?? []);
        const { words } = Search.of(text);
        const closest = await Promise.all(
            words.map(async (word, index) => {
                // keep a word with a digit, such as an identifier's or a date's
                if (DIGIT.test(word)) {
                    return word;
                }
                const terms = await Promise.all(
                    indexes.map((each) => each.terms(this.database, word)),
                );

                return Search.closest(word, terms.flat(), index === words.length - 1);
            }),
        );

        // leave out the last other words until the rest finds something on this device
        let kept = closest.filter((word) => word !== undefined);
        while (kept.length > 0) {
            const meant = kept.join(" ");
            const live = this.#search(meant, { ...options, limit: 1 });
            const { hits } = await live.read();
            await live.close();
            if (hits.length > 0) {
                return meant === words.join(" ") ? undefined : meant;
            }
            const last = kept.findLastIndex((word) => !DIGIT.test(word));
            kept = last === -1 ? [] : kept.toSpliced(last, 1);
        }

        return undefined;
    }

    /** Read a scope and the scopes above it, nearest first, from the replica's access rows. */
    async chain(scope: string): Promise<readonly string[]> {
        return (await sync.Scope.chain(this.database.snapshot(), scope)).map(
            (link) => link.object.id,
        );
    }

    /** Settle once the replica has the caller's access rows, or once the client closes. */
    get ready(): Promise<void> {
        return this.#access().then(() => {});
    }

    /** Describe the client for inspection. */
    async inspect(): Promise<ClientInspection> {
        // count rows per object type
        const rows: Record<string, number> = {};
        for (const object of this.objects) {
            const [counted] = await this.database.select({ rows: count() }).from(object.table);
            rows[object.name] = present(counted, `the count of ${object.name} rows`).rows;
        }

        // list the followed queries by name
        const keys = await this.store.followKeys();
        const followed = keys.flatMap((key) => {
            const shape = [...this.#follows.values()].find(
                (follow) => follow.shape.name === key.shape,
            )?.shape;

            return shape === undefined
                ? []
                : Object.keys(shape.keyed([key.parameters])).map((name) => ({
                      name,
                      follower: key.follower,
                      expiresAt: key.expiresAt,
                  }));
        });

        return {
            scope: this.scope,
            rows,
            ...(this.store.storage === undefined ? {} : { storage: this.store.storage }),
            followed,
            replica: await this.replica.inspect(this.database),
            mutations: await this.prediction.inspect(this.database),
            feed: this.#local.inspect(),
        };
    }

    /** Stop following a closed query, even one followed forever. */
    async release(
        object: ObjectType,
        query: Omit<ObjectQuery, "object" | "package"> = {},
    ): Promise<void> {
        // unfollow the query followed forever at once
        const split = this.#split(object, query);
        const follow = this.#follows.get(split.object.storage);
        if (follow !== undefined) {
            const at = { key: { scope: this.#readScope(split.object), query: split.query } };
            await this.store.unfollow(FOREVER_FOLLOWER, follow, [at]);
        }
    }

    /** Wait for a mutation's outcome, failing with the server's rejection. */
    async outcome(id: string): Promise<void> {
        await this.#outbox.outcome(id, this.party);
    }

    /** Follow a live union of several queries in one order and limit, with each entry's member. */
    union<const Members extends Readonly<Record<string, RelationalQuery<readonly object[]>>>>(
        members: Members,
        options: UnionOptions<MemberItem<Members>>,
    ): LiveQuery<readonly UnionEntry<Members>[]> {
        // refuse members ordering or limiting themselves, and follow each in the union's order and limit
        for (const [name, member] of Object.entries(members)) {
            if (member.options.orderBy !== undefined || member.options.limit !== undefined) {
                throw new TypeError(`union member '${name}' cannot order or limit itself`);
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
                options.ttl === undefined ? {} : { ttl: options.ttl },
            ),
        }));
        const byName = new Map(followed.map((member) => [member.name, member]));
        const merge = (lists: readonly (readonly Item[])[]) => {
            // order the members' rows and select each row's item
            const items = new Map(lists.flatMap((list) => list.map((item) => [item.row, item])));
            const merged = Order.merge(
                Order.of(orderBy),
                zip(followed, lists).map(([{ name, table }, list]) => ({
                    name,
                    table,
                    rows: list.map((item) => item.row),
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
            // wait for the members only once asked, leaving no derived promise unobserved
            get ready() {
                return Promise.all(followed.map(({ live }) => live.ready)).then(() => {});
            },
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

    /** Wait until the replica has the caller's access rows, once for every query. */
    #access(): Promise<boolean> {
        const isAccessed = async () => (await this.access.position(this.database)) !== undefined;
        this.#accessed ??= this.database.log.waitUntil(isAccessed, this.#closing.signal);

        return this.#accessed;
    }

    /** Watch an aggregate query's groups through the local feed until aborted. */
    #watchResults(
        named: Promise<string>,
        query: sync.Query,
        signal: AbortSignal,
    ): AsyncGenerator<readonly sync.AggregateGroup[]> {
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
            // read the scopes the replica reads now
            const scopes = canonicalize(await this.replica.chain(this.database));
            const read = await this.#read(query);

            // watch over them until the replica reads others
            const rescoping = new AbortController();
            const stopped = AbortSignal.any([signal, rescoping.signal]);
            const isRescoped = async () =>
                canonicalize(await this.replica.chain(this.database)) !== scopes;
            const watching = new sync.ReplicaSet(this.database)
                .waitMarked(isRescoped, stopped)
                .then(
                    (isChanged) => isChanged && rescoping.abort(),
                    () => rescoping.abort(),
                );
            try {
                yield* watch(name, read, stopped);
            } finally {
                rescoping.abort();
                await watching;
            }
        }
    }

    /** Widen a durable query to every scope of its type the replica last read. */
    async #read(query: sync.Query): Promise<sync.Query> {
        // run a query of no durable type, whose replica reads no chain
        const object = this.objects.find((type) => type.table === query.table);
        if (object?.storage !== "durable") {
            return query;
        }

        // read the scopes with the type's rows, which the source chose to send
        const held = object.table[TABLE].column("scope").definition.schema;
        const read = await this.replica.chain(this.database);

        return { ...query, scopes: read.filter((scope) => held.safeParse(scope).success) };
    }

    /** Read a query's items from the local replica. */
    async #items(name: string, query: sync.Query): Promise<readonly Item[]> {
        return (await this.#dataflow(name, query)).read(name);
    }

    /** Load a query over the local database through the local feed's upstream. */
    async #dataflow(name: string, query: sync.Query): Promise<syncDataflow.Dataflow> {
        // read through the local feed's upstream
        const upstream = present(this.#local.upstream, "the local feed's upstream");
        const dataflow = new syncDataflow.Dataflow(
            { [name]: await this.#read(query) },
            {
                audience: syncDataflow.EVERYONE,
                database: this.database,
                upstream,
                changesThrough: syncDataflow.Dataflow.changesThrough(this.database),
                isMaterialized: true,
            },
        );
        await dataflow.load(await syncDataflow.View.latest(this.database));

        return dataflow;
    }

    /** Split a query by storage: its storage's query, and key lookups of the other's rows. */
    #split(object: ObjectType, options: Omit<ObjectQuery, "object" | "package">): StorageQuery {
        // require conditions relating one storage
        const { with: included, extras, columns: _columns, ...query } = options;
        this.#requireOneStorage(object, query.where ?? {}, extras ?? {});
        const lookups: Lookup[] = [];

        // take same-storage extras, and follow the rows other-storage extras measure by key
        const sameStorage: Record<string, Expression> = {};
        const measured = new Set<string>();
        for (const [name, expression] of Object.entries(extras ?? {})) {
            const vias = [...Expression.lookups(expression), ...Expression.rollups(expression)].map(
                ({ via }) => via,
            );
            const foreign = vias.filter(
                (via) => this.relations.related(object, via).object.storage !== object.storage,
            );
            if (foreign.length === 0) {
                sameStorage[name] = expression;
            }
            for (const via of foreign.filter((entry) => !measured.has(entry))) {
                measured.add(via);
                lookups.push(this.#lookup(object, via, {}));
            }
        }

        // take same-storage relations, and look up other-storage ones by key
        const nested: Record<string, FindOptions> = {};
        const expanded = this.relations.expand(object, included) ?? {};
        for (const [name, relation] of FindOptions.includes({ with: expanded })) {
            const joined = this.relations.related(object, name);
            if (joined.object.storage === object.storage) {
                const child = this.#split(joined.object, relation);
                const { object: _object, package: _package, ...childOptions } = child.query;
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
                package: object.package.id,
                ...query,
                ...(Object.keys(sameStorage).length === 0 ? {} : { extras: sameStorage }),
                ...(Object.keys(nested).length === 0 ? {} : { with: nested }),
            }),
            lookups,
        };
    }

    /** Look up the rows of another storage a relation joins by key. */
    #lookup(object: ObjectType, relation: string, options: FindOptions): Lookup {
        const { object: target, relation: joined } = this.relations.related(object, relation);
        if (joined.on.kind !== "key") {
            throw ObjectError.of(
                "RELATION_BY_KEY_INVALID",
                `relation '${relation}' of object ${object.name} is invalid: it can only relate another storage by key`,
            );
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
            if (this.relations.related(object, via).object.storage !== object.storage) {
                throw ObjectError.of(
                    "RELATION_ACROSS_STORAGE_INVALID",
                    `relation '${via}' of object ${object.name} is invalid: it has another storage`,
                );
            }
        }
    }

    /** Follow a key for the client's live queries, counting the queries following it. */
    async #follow(
        follow: OnDemandFollow<{ readonly query: ObjectQuery }>,
        at: FollowKey<{ readonly query: ObjectQuery }>,
    ): Promise<void> {
        // follow the key once its first live query follows it
        const key = canonicalize([follow.shape.name, at.key]);
        const following = (this.#followCounts.get(key) ?? 0) + 1;
        this.#followCounts.set(key, following);
        if (following === 1) {
            await this.store.follow(CLIENT_FOLLOWER, follow, [at]);
        }
    }

    /** Unfollow a key once no live query follows it, unless followed forever. */
    async #unfollow(
        follow: OnDemandFollow<{ readonly query: ObjectQuery }>,
        at: FollowKey<{ readonly query: ObjectQuery }>,
        ttl: Duration | "forever" | undefined,
    ): Promise<void> {
        // count the query off, leaving a key other live queries follow
        const key = canonicalize([follow.shape.name, at.key]);
        const following = (this.#followCounts.get(key) ?? 0) - 1;
        if (following > 0) {
            this.#followCounts.set(key, following);

            return;
        }
        this.#followCounts.delete(key);

        // leave a forever followed key until released, else unfollow it
        if (ttl === "forever") {
            await this.store.follow(FOREVER_FOLLOWER, follow, [at]);
            await this.store.unfollow(CLIENT_FOLLOWER, follow, [at]);
        } else {
            await this.store.unfollow(CLIENT_FOLLOWER, follow, [at], ttl);
        }
    }

    /** Follow a storage query and the lookups its rows key. */
    #subscribe(split: StorageQuery, signal: AbortSignal): Subscription {
        // name a local query, or follow a followed one
        const follow = this.#follows.get(split.object.storage);
        const at = { key: { scope: this.#readScope(split.object), query: split.query } };
        const named =
            follow === undefined
                ? ObjectClient.#queryName(split.query, this.environment.crypto)
                : this.#follow(follow, at).then(() =>
                      nth(Object.keys(follow.shape.keyed([{ query: split.query }])), 0),
                  );

        // read a local query at once, or wait until the store's replica follows it
        const replicated = named.then(async () => {
            if (follow !== undefined) {
                await this.database.log.waitUntil(
                    () => follow.shape.isSynced(this.database, at.key),
                    signal,
                );
            }
        });
        const lookups = new Map<Lookup, LookupSubscription>();
        if (split.lookups.length === 0) {
            const unfollow = async (ttl: Duration | "forever" | undefined) => {
                if (follow !== undefined) {
                    await this.#unfollow(follow, at, ttl);
                }
            };

            return { named, ready: replicated, done: named.then(() => {}), unfollow };
        }

        // key lookups from the query's rows, ready once the replica and the first lookups are
        const compiled = this.#compile(split.query);
        const { promise: ready, resolve: isReady } = Promise.withResolvers<void>();
        const signalReady = async () => {
            await replicated;
            await Promise.all([...lookups.values()].map((entry) => entry.subscribed.ready));
            isReady();
        };
        const done = this.#watchLookups(split, compiled, named, lookups, signal, signalReady);
        done.catch((error: unknown) => {
            // capture a failed lookup
            this.#instruments.captureException(error, { isEscaped: true });
        });

        return {
            named,
            ready: Promise.race([ready, done.then(() => replicated)]),
            done,
            unfollow: async (ttl) => {
                // unfollow the query's key and its lookups'
                if (follow !== undefined) {
                    await this.#unfollow(follow, at, ttl);
                }
                for (const { subscribed } of lookups.values()) {
                    await subscribed.unfollow(ttl);
                }
            },
        };
    }

    /** Watch a query's rows and replace each lookup whose keys changed, until stopped. */
    async #watchLookups(
        split: StorageQuery,
        compiled: sync.Query,
        named: Promise<string>,
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
        lookups: Map<Lookup, LookupSubscription>,
        signal: AbortSignal,
    ): Promise<void> {
        // skip unchanged lookups
        const key = canonicalize(keys);
        const current = lookups.get(lookup);
        if (current?.keys === key) {
            return;
        }

        // stop the current lookup and drop its key
        if (current !== undefined) {
            current.stop.abort();
            await current.subscribed.done;
            await current.subscribed.unfollow(undefined);
            lookups.delete(lookup);
        }

        // subscribe the new keys
        if (keys.length > 0) {
            const stop = new AbortController();
            const subscribed = this.#subscribe(
                ObjectClient.#keyedTarget(lookup, keys),
                AbortSignal.any([signal, stop.signal]),
            );
            lookups.set(lookup, { keys: key, subscribed, stop });
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
        // read the target around the prediction: the existing row a creation's conflict lands on, else the created row
        const method = object.method(name);
        const given = typeof input["id"] === "string" ? input["id"] : undefined;
        const id =
            method.onConflict === undefined
                ? given
                : await this.#landing(database, object, name, input);
        const before = id === undefined ? undefined : await this.row(database, object, id);
        const hasTarget = !Method.loads(method) || before !== undefined;
        const result =
            isInverse && !hasTarget
                ? undefined
                : await this.#predictCall(database, object, name, input);
        const created = Method.resultId(result) ?? id;
        const after = created === undefined ? undefined : await this.row(database, object, created);

        // store the result and the rows in the JSON form the server answers
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

    /** Find the row a creation lands on, predicting it once in a savepoint it rolls back. */
    async #landing(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: sync.Call["input"],
    ): Promise<string | undefined> {
        // predict the creation, then roll it back, keeping the identifier it answered
        let landed: string | undefined;
        const rolled = new Error("roll back the landing prediction");
        await database
            .transaction(async (savepoint) => {
                landed = Method.resultId(await this.#predictCall(savepoint, object, name, input));
                throw rolled;
            })
            .catch((error: unknown) => {
                if (error !== rolled) {
                    throw error;
                }
            });

        return landed;
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

    /** Send one ephemeral call as this client. */
    async #send(call: sync.Call, home: string): Promise<unknown> {
        // push only the call to its home
        const { outcomes } = await this.#remote.push(home, {
            mutations: [{ id: RequestId.create(this.environment), calls: [call] }],
            writerId: this.writerId,
            chain: [],
        });

        // throw a failure or return the result
        const { outcome } = present(outcomes[0], "the pushed call's outcome");
        if (!("value" in outcome)) {
            throw Failure.error(outcome.error);
        }
        const [result] = CALL_RESULTS.parse(outcome.value);

        return result;
    }

    /** Write one local call in the client's database, returning its result. */
    #write(object: ObjectType, name: string, call: sync.Call): Promise<unknown> {
        return this.database.transaction((transaction) =>
            this.#predictCall(transaction, object, name, call.input),
        );
    }

    /** Predict one call in the node's store, as the caller, answering its result. */
    async #predictCall(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: JsonObject,
    ): Promise<unknown> {
        const call = object.call(name, input);
        const [result] = await this.store.predict(
            database,
            { id: RequestId.create(this.environment), calls: [call] },
            await this.store.callerContext(this.#readScope(object), database),
        );

        return result;
    }

    /** Read an object type's scope here: the client's scope, or a local type's partition. */
    #readScope(object: ObjectType): string {
        // store replicated objects in the client's scope
        if (object.storage !== "local") {
            return this.scope;
        }

        // store local objects in the partition of the principal their scope type stands for
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
                ? { id: object.generateId(this.environment) }
                : kind === "createMany"
                  ? {
                        items: BatchItem.array()
                            .parse(input["items"])
                            .map((item) => ({
                                id: object.generateId(this.environment),
                                ...item,
                            })),
                    }
                  : {};

        // place the scope in the route field
        const field = object.route.field;
        const scoped = field === undefined ? {} : { [field]: scope };

        return object.call(name, { ...input, ...generated, ...scoped });
    }

    /** Read the scope a call acts in: the call's, its target's, else the client's. */
    async #actingScope(
        database: DatabaseConnection,
        object: ObjectType,
        name: string,
        input: JsonObject,
    ): Promise<string> {
        // take the call's scope
        const field = object.route.field;
        const named = field === undefined ? undefined : input[field];
        if (typeof named === "string") {
            return named;
        }

        // take the scope of the target the replica has, such as an enclosing scope's propagated row
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
            throw ObjectError.of(
                "HOMES_INVALID",
                `mutation is invalid: a mutation must change rows of one home, but its calls change rows of '${before}' and '${home}'`,
            );
        }
    }

    /** Read the scope whose home runs a recorded call. */
    #readHome(call: sync.Call): string {
        const { object, name } = this.method(call);

        return object.readScope(name, call.input);
    }

    /** Find the object type and method of a recorded call. */
    method(call: Pick<sync.Call, "type" | "method">): {
        readonly object: ObjectType;
        readonly name: string;
        readonly method: Method;
    } {
        const object = ObjectType.find(this.objects, call);
        if (!object) {
            throw new TypeError(`the client has no object ${call.type.type}`);
        }

        return { object, name: call.method, method: object.method(call.method) };
    }

    /** Pick a storage's replica, events beside durable rows, refusing one the client lacks. */
    #pickReplica(storage: ObjectStorage): syncDataflow.Replica {
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
        const object = this.objects.find(
            (type) => type.name === query.object && type.package.id === query.package,
        );
        const chain = [this.#readScope(present(object, `the client's object ${query.object}`))];
        const [compiled] = Object.values(this.relations.queries({ query }, chain));

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
                ([, method]) => method.mutates === mutates && Method.isRouted(method),
            ),
        );
    }

    /** List the tables of the object types stored in one storage. */
    static #storedTables(objects: readonly ObjectType[], storage: ObjectStorage): Table[] {
        return objects.filter((object) => object.storage === storage).map((object) => object.table);
    }

    /** Replicate some tables in a separate replica, absent for no tables. */
    static #separateReplica(
        name: string,
        scope: string,
        tables: readonly Table[],
    ): syncDataflow.Replica | undefined {
        return tables.length === 0 ? undefined : new syncDataflow.Replica({ name, scope, tables });
    }

    /** Read the distinct keys of a lookup's parent rows, in their JSON form, sorted. */
    static #keysOf(rows: readonly Item[], lookup: Lookup): Exclude<Scalar, null>[] {
        // follow the include path
        let items: readonly Item[] = rows;
        for (const name of lookup.path) {
            items = items.flatMap((item) => present(item.with[name], `the ${name} include`));
        }

        // collect the distinct keys of the parent rows, in their JSON form
        const column = lookup.target.object.table[TABLE].column(lookup.column).definition;
        const keys = new Set<Exclude<Scalar, null>>();
        for (const item of items) {
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

    /** Require a lookup key value to be a scalar, as key columns store them. */
    static #scalarOf(value: unknown): Exclude<Scalar, null> {
        if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
            return value;
        }

        throw new TypeError(`a lookup key is '${typeof value}', not a scalar`);
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

    /** Name a storage query by a prefix of its content's digest. */
    static async #queryName(query: ObjectQuery, crypto: DigestCrypto): Promise<string> {
        return (await Digest.json(query, crypto)).slice(0, QUERY_NAME_LENGTH);
    }
}

/** A refusal of a client's follow, which fails the queries open until the source refuses. */
class FollowRefusal {
    /** Rejects with the refusal once the source refuses the follow for good. */
    readonly failed: Promise<never>;
    /** Aborts the watches open until the refusal. */
    readonly #stopping = new AbortController();
    /** Reject the queries waiting until the refusal. */
    readonly #reject: (error: unknown) => void;
    /** The refusal, absent until the source refuses. */
    #refused: { readonly error: unknown } | undefined;

    /** Wait for a refusal, marked handled for queries nobody waits on. */
    constructor() {
        // reject once refused, marked handled
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => {});
        this.failed = failed.promise;
        this.#reject = failed.reject;
    }

    /** Fail the queries open until now with an error, and return the refusal later ones wait on. */
    refuse(error: unknown): FollowRefusal {
        // fail the waiting queries and stop the open watches
        this.#refused = { error };
        this.#reject(error);
        this.#stopping.abort();

        return new FollowRefusal();
    }

    /** Yield a watch until a signal or the refusal stops it, then throw the refusal. */
    async *watch<Value>(
        watch: (signal: AbortSignal) => AsyncGenerator<Value>,
        signals: readonly AbortSignal[],
    ): AsyncGenerator<Value> {
        yield* watch(AbortSignal.any([...signals, this.#stopping.signal]));
        if (this.#refused !== undefined) {
            throw this.#refused.error;
        }
    }
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
    /** Unfollow the query's and its lookups' keys, unless followed forever. */
    readonly unfollow: (ttl: Duration | "forever" | undefined) => Promise<void>;
}

/** A lookup's subscription to the keys its rows had last. */
interface LookupSubscription {
    /** The canonical keys subscribed. */
    readonly keys: string;
    /** The subscription to the keyed objects. */
    readonly subscribed: Subscription;
    /** Stops the subscription. */
    readonly stop: AbortController;
}

/** Score a type's rows by the text columns a search reads, nothing without any. */
function relevanceOf(type: ObjectType, search: Search): Expression {
    const searched = type.table[TABLE].fullText?.properties ?? [];
    const columns = type.search?.text.filter((name) => searched.includes(name)) ?? [];

    return columns.length === 0 || search.isEmpty
        ? Expression.literal(0)
        : Expression.relevance(columns, search.words.join(" "));
}

/** Read when a type's rows last changed, never for those that do not tell. */
function changedOf(type: ObjectType): Expression {
    return Object.hasOwn(type.table[TABLE].columns, "updatedAt")
        ? Expression.column("updatedAt")
        : Expression.literal(0);
}

/** Cut the passage of an object's searched long texts a search matches, absent without one. */
function excerptOf(type: ObjectType, search: Search, row: JsonObject): string | undefined {
    return type.text
        .filter((name) => type.search?.text.includes(name) === true)
        .map((name) =>
            search.excerpt(TextSpan.plain(schema.array(TextSpan).parse(row[name] ?? []))),
        )
        .find((excerpt) => excerpt !== undefined);
}
