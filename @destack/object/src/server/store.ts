import type { Health } from "./health.ts";
import { Casing } from "../object/name.ts";
import { RelationGraph } from "../query/query.ts";
import { CLIENT_HEAD_HEADER, ROUTE_PATH } from "../workload/start.ts";
import type { BlobStorage } from "@destack/resource";
import { ObjectError } from "../error/error.ts";
import { CallError } from "../error/call.ts";
import { Package, type PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { EventStore } from "@destack/event";
import {
    Authorizer,
    AccessContext,
    type TupleReader,
    Policy,
    Access,
    type ChainOptions,
    type TableMapping,
    Caller,
    Grant,
    CONTAINED,
    principal,
    Manager,
    universe,
} from "@destack/access";
import {
    type ObjectReference,
    ObjectTypeReference,
    Scope,
    type ScopeLink,
    Subject,
    RequestId,
} from "@destack/sync";
import {
    and,
    BlobStore,
    Change,
    type Control,
    type Controller,
    type DatabaseConnection,
    DatabaseError,
    eq,
    Key,
    LogPosition,
    lte,
    ReadSet,
    Row,
    Snapshot,
    TABLE,
    type BatchWriter,
    type Table,
    Transaction,
} from "@destack/db";
import { objectUnmask } from "../audit/unmask.ts";
import { Replay } from "../audit/replay.ts";
import {
    AuditRecorder,
    type AuditOrigin,
    type AuditRecorderOptions,
    type CallRequest,
} from "../audit/recorder.ts";
import { Journal } from "../audit/journal.ts";
import type { CallKey } from "../audit/key.ts";
import {
    nth,
    canonicalize,
    Duration,
    Identifier,
    type JsonObject,
    JsonValue,
    present,
    schema,
    zip,
    Uuid,
    type UuidEnvironment,
} from "@destack/schema";
import { type InstalledServerContext, ServerContext } from "../workload/workload.ts";
import type { Watermark } from "@destack/sync";
import { refuseInput } from "../error/error.ts";
import { copyRequest } from "@destack/environment";
import type { JournalRecord, RequestIdentity } from "../audit/journal.ts";
import { implement, type Router } from "./handler.ts";
import { type ServerAccess, type RequestContext } from "./context.ts";
import { type PackageHandler } from "./http.ts";
import { type ProcedureContext } from "./procedure.ts";
import { Egress } from "../declare/egress.ts";
import { type AdapterDeclaration, PackageHandle } from "../declare/package.ts";
import { type ProcedureRouter } from "../procedure/index.ts";
import * as sync from "@destack/sync";
import { type Follow, type FollowKey, type RowFollow, Shape } from "../replica/shape.ts";
import { followKey } from "../replica/key.ts";
import {
    WriterId,
    storeProcedures,
    type PushResult,
    type StoreProcedures,
} from "../replica/replica.ts";
import { Branch, type BranchEntry, type BranchType } from "../branch/index.ts";
import { MethodContext, type ResultsOf, type Run } from "../method/context.ts";
import { CallSite } from "../method/site.ts";
import { Limits } from "../method/limits.ts";
import { Method, type MethodKind } from "../method/method.ts";
import { callableProcedures } from "../method/procedure.ts";
import { InputSchema } from "../method/input.ts";
import { ACCESS_KINDS, ObjectType } from "../object/object.ts";
import { ObjectTypePool, ObjectDescription, type KnownDeclaration } from "../object/description.ts";
import { ObjectController } from "../object/controller.ts";
import { Chunk } from "../text/chunk.ts";
import { CHUNKS } from "../text/table.ts";
import { Authorization } from "./authorization.ts";
import { type ClaimPrimary, Reservation } from "../claim/reservation.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import { ExternalStorage } from "./external.ts";
import { ObjectSource } from "./source.ts";
import { ObjectSink } from "./sink.ts";
import { Catalog, type CatalogOptions, type ServedType } from "./catalog.ts";
import type { ResourceControllerOptions } from "./provisioned.ts";
import { TrackedHistory } from "./tracked.ts";
import { Settlement } from "./settlement.ts";
import { TelemetryScope } from "@destack/telemetry";
import { Claimant } from "../claim/claim.ts";
import type { StoreExecution } from "../method/execution.ts";
import type { HookSandbox } from "../hook/sandbox.ts";
import type { AdapterFetch, PackageImplementation } from "./implementation.ts";
import type { Clock, Environment } from "@destack/environment";

import type {} from "@destack/package/import-meta";

/** The recent pages sharing grant readers: 64 readers of a few KiB, well under a megabyte. */
const SHARED_READERS = 64;

/** The input of a procedure call, by field. */
const PROCEDURE_INPUT = schema.record(schema.string(), schema.unknown());

/** The identifier of a row a method returns. */
const IDENTIFIER = schema.string();

/** The page of a list method: its rows and their includes, by include name and row id. */
const LISTED_PAGE = schema.looseObject({
    items: schema.array(Row),
    included: schema
        .record(schema.string(), schema.record(schema.string(), schema.unknown()))
        .exactOptional(),
});

/** The methods an object in the trash still takes. */
const TRASH_METHODS: ReadonlySet<MethodKind> = new Set([
    "get",
    "restore",
    "purge",
    "relationships",
    "invitations",
]);

/** How long a request waits for its caller's watermarks: 5 s, above sub-second replica lag. */
const BOOKMARK_TIMEOUT_MILLISECONDS = 5000;

/** How long a store waits for a key it acquires to replicate: 5 s, above sub-second replica lag. */
const ACQUIRE_MILLISECONDS = 5000;

/** A database's replica: its subscription, and the scope whose publisher streams it. */
export interface SubscriberEntry {
    /** The subscription following the replica. */
    readonly subscription: sync.SubscriptionRecord;
    /** The scope whose publisher streams the replica's pages, one step closer to its rows' home. */
    readonly home: string;
    /** Whether a use acquired the replica's key, as on-demand follows do, not a declaration. */
    readonly isAcquired: boolean;
}

/** How a store follows its replicas: the tables relisting them, and the uplink taking changes. */
export interface SubscriberOptions {
    /** The link to the replicas' store, receiving this store's changes to them, if it sends any. */
    readonly uplink?: sync.Uplink;
}

/** The subscriptions a database follows for its replicas, each from its publisher. */
export interface Subscriber extends SubscriberOptions {
    /** List the subscriptions of the store's chains and follows, reusing keys no change touched. */
    readonly subscriptions: (
        view: Snapshot,
        changes?: readonly Change[],
    ) => Promise<readonly SubscriberEntry[]>;
    /** Open the publisher streaming a home's replicas. */
    readonly publisher: (home: string) => sync.Publisher;
}

/** Where a watermark's history is in a store: the local log, a lost one, its replicas, or none. */
export type WatermarkHistory =
    | {
          /** The store's log in its current epoch. */
          readonly kind: "local";
      }
    | {
          /** An earlier epoch of the store's log, or another log of a scope homed here. */
          readonly kind: "stale";
      }
    | {
          /** Replicas including or reading the scope. */
          readonly kind: "replicated";
          /** The replicas including the scope's rows. */
          readonly including: readonly { readonly name: string; readonly scope: string }[];
          /** The followed replicas of the scope or reading it. */
          readonly followed: readonly sync.SubscriptionRecord[];
      }
    | {
          /** No home or replica of the scope here. */
          readonly kind: "unknown";
      };

/** One source row's key of a row follow, with what reading the key read. */
interface RowKey {
    /** The key, absent for a row the follow asks nothing for. */
    readonly key: FollowKey | undefined;
    /** What reading the key read. */
    readonly reads: ReadSet;
}

/** A call's prepared external work: the keyed call, parsed value and settlement, if any. */
type Prepared =
    | {
          readonly context: MethodContext;
          readonly value: unknown;
          readonly settlement: string | undefined;
      }
    | undefined;

/** A resolved call with the author it runs as and the calls its expansion ran before it. */
type Expansion = ReturnType<typeof ObjectType.resolve> & {
    /** The principal and actors the call records as its caller, the caller's when absent. */
    readonly as?: Pick<Caller, "subject" | "actors">;
    /** The calls the method's expansion ran before it. */
    readonly expansion?: readonly BranchEntry[];
};

/** A durable mutation as the journal executes it. */
type MutationRun = {
    /** The mutation's identifier. */
    readonly id: string;
    /** The scope the calls act in. */
    readonly scope: string;
    /** The caller the mutation runs as. */
    readonly caller: AccessContext;
    /** The code making the calls for a principal's code, such as a controller's name. */
    readonly component?: string;
    /** The declaration the calls write for, if any. */
    readonly manager?: Manager;
    /** The expanded calls, in order. */
    readonly calls: readonly Expansion[];
    /** The prepared external work, by call. */
    readonly prepared: readonly Prepared[];
    /** The caller, scope and request identifier the journal keys the mutation by. */
    readonly request: RequestIdentity & { readonly caller: string };
    /** The digest of the mutation's redacted calls. */
    readonly fingerprint: string;
    /** Each call's input as its record stores it. */
    readonly redacted: readonly JsonObject[];
};

/** How an authorization binds its caller beyond its scope. */
export interface AuthorizeOptions {
    /** The request grants bound to one request match. */
    readonly request?: string;
    /** The scope chain read in the same snapshot. */
    readonly links?: readonly ScopeLink[];
    /** The scopes the caller lives inside, those its principal's row places it in when absent. */
    readonly within?: readonly ObjectReference[];
    /** The time of the work the access is decided for, the store's clock when absent. */
    readonly now?: number;
}

/** How a store authorizes a caller for work it serves, beyond its authorization. */
export interface AuthorizeHereOptions extends Omit<AuthorizeOptions, "within"> {
    /** The authorized calls by type and method, whose types with an owner need no visible scope. */
    readonly calls?: readonly { readonly object: ObjectType; readonly name: string }[];
}

/** How a query runs beyond its caller. */
export interface QueryOptions {
    /** The running mutation to read within, seeing its writes, if any. */
    readonly transaction?: Transaction;
}

/** How a mutation runs beyond its caller. */
export interface MutateOptions {
    /** The running mutation to join without external work, which its owner commits, if any. */
    readonly transaction?: Transaction;
    /** The client instance writing ephemeral objects. */
    readonly writerId?: string;
    /** The code making the calls for a principal's code, such as a controller's name. */
    readonly component?: string;
    /** The scope of the calls without one. */
    readonly scope?: string;
    /** Where the calls are made, the caller's site when absent, such as the store's steps. */
    readonly site?: CallSite;
    /** The declaration the calls write for, which manages the records they create, if any. */
    readonly manager?: Manager;
}

/** The journal's progress through a mutation: the running call and its claim reservation. */
type MutationProgress = {
    /** The position of the call running now, absent before and after the calls. */
    running: number | undefined;
    /** The reservation of the names the written objects claim, once the calls ran. */
    reservation: Reservation | undefined;
};

/** How one call runs within its mutation. */
type CallOptions = {
    /** The call's prepared external work. */
    readonly prepared?: Prepared;
    /** The log position before the mutation, for tracked objects. */
    readonly from?: LogPosition;
    /** The client instance writing ephemeral objects. */
    readonly writerId?: string;
    /** The request a durable mutation replays under. */
    readonly requestId?: string;
    /** The principal and actors the call records as its caller, the authorization's when absent. */
    readonly as?: Pick<Caller, "subject" | "actors">;
    /** The calls the method's expansion ran before it. */
    readonly expansion?: readonly BranchEntry[];
    /** The request the call belongs to. */
    readonly request?: CallRequest;
    /** The code making the call for a principal's code, such as a controller's name. */
    readonly component?: string;
    /** The declaration the call writes for, if any. */
    readonly manager?: Manager;
    /** The call running this one inside its transaction, absent for a call sent to the store. */
    readonly parentId?: Identifier<"call">;
    /** The authorizations the run resolved for its calls, by scope and caller. */
    readonly authorizations: Map<string, Authorization>;
};

/** A store's chain not known yet, such as a placed space's before its placement arrives. */
export interface PendingChain {
    /** The chain's place, which a later view decides. */
    readonly kind: "pending";
}

/** The options of an object store: its package servers over one database, and its call log. */
export type ObjectStoreOptions = Omit<
    StoreComposition,
    | "objects"
    | "replicas"
    | "installed"
    | "external"
    | "subscriber"
    | "provisioned"
    | "follows"
    | "principal"
    | "publisher"
> &
    CallLog & {
        /** The package servers, the first one running the store's controllers. */
        readonly servers: readonly PackageImplementation[];
        /** The sandboxes of the installations whose packages run here without a workload. */
        readonly sandboxes?: readonly HookSandbox[];
        /** The followed installation's link for replicated rows' changes, else the publisher. */
        readonly uplink?: sync.Uplink;
        /** Open a package's release a scope runs, or a given version, absent for none. */
        readonly release?: ObjectStore["release"];
        /** The machine running the servers' provisioned resources, which providers require. */
        readonly machine?: Identifier<"machine">;
        /** Open the publisher toward a followed scope's home, else the store's uplink. */
        readonly publisher?: (scope: string) => sync.Publisher;
    } & (
        | {
              /** The principal the store's work runs as. */
              readonly principal: Subject;
              /** No installation: the store runs as the given principal. */
              readonly server?: never;
          }
        | {
              /** The installation the store serves as, running its work and replicas. */
              readonly server: InstalledServerContext;
              /** No principal: the store runs as the installation. */
              readonly principal?: never;
          }
    );

/** How a store records calls: journaled in its event log when serving methods, else not at all. */
export type CallLog =
    | {
          /** The journal's key for fingerprinting sensitive call inputs. */
          readonly callKey: CallKey;
          /** The event log of the store's database, with its calls and served events types. */
          readonly events: EventStore;
      }
    | {
          /** No journal: the store serves no methods. */
          readonly callKey?: never;
          /** No event log: the store records no calls and no events. */
          readonly events?: never;
      };

/** An object store's objects over one database and its adjacent stores, as composed. */
interface StoreComposition {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The types whose queries run over rows replicated here, never followed nor changed here. */
    readonly replicas?: readonly ObjectType[];
    /** Further policies and table mappings to authorize. */
    readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The storage of the partition's blobs, which its rows' blob columns keep alive. */
    readonly blobs: BlobStorage;
    /** The principal of the store's work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The partitions the store serves, its own first: its installation's, else its scope. */
    readonly partitions?: readonly string[];
    /** The package recording the calls. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimPrimary;
    /** The installations serving objects and their implementing packages, by type key. */
    readonly installed?: ReadonlyMap<string, ServedType>;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** Open a principal's live link for one of its streams, answering its close. */
    readonly connect?: (subject: Subject) => Promise<() => Promise<void>>;
    /** The files with the served external objects, each storing some of their types. */
    readonly external?: readonly ExternalStorage[];
    /** The subscriptions the database follows to maintain its replicas. */
    readonly subscriber?: SubscriberOptions;
    /** Read how the store has its scope's chain: home, follower, pending, or absent with none. */
    readonly chain: (view: Snapshot) => Promise<ChainOptions | PendingChain | undefined>;
    /** The shapes the served code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
    /** Open the publisher toward the home of a scope a follow reads. */
    readonly publisher?: (scope: string) => sync.Publisher;
    /** The publications served and followed beside the objects' and policies' ones. */
    readonly publications?: readonly sync.Publication[];
    /** Report failed settlements, thrown when absent. */
    readonly report?: (error: unknown) => void;
    /** Whether reads of the objects record access events, as the space's audit setting asks. */
    readonly isAccessAudited?: boolean;
    /** The environment calls run and controllers reconcile in: its clock and randomness. */
    readonly environment: Environment;
    /** The providers reconciling the served kinds' resources, if the store reconciles any. */
    readonly provisioned?: ResourceControllerOptions;
}

/** A store's routes: each served object's procedures by its key, and the store procedures. */
export type ObjectRouter = Readonly<Record<string, Router<ProcedureRouter, RequestContext>>> & {
    readonly store: Router<StoreProcedures, RequestContext>;
};

/** A store serving object types' methods over one database. */
export class ObjectStore {
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** Open a package's release a scope runs, or a given version, absent for none. */
    readonly release:
        | ((
              view: Snapshot,
              scope: string,
              packageId: PackageId,
              version?: string,
          ) => Promise<BuildReader | undefined>)
        | undefined;
    /** The partition's blobs, kept alive by its rows' blob columns. */
    readonly blobs: BlobStore;
    /** The principal of the store's work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The partitions the store serves, its own first, none for its principal's scope alone. */
    readonly partitions: readonly string[];
    /** The journal of executed mutations, recorded only by a store serving methods. */
    readonly #journal: Journal | undefined;
    /** The package the store serves, as the audit records it. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The feed serving every sync. */
    readonly feed: sync.Feed;
    /** The claim type recording the names of the served objects' unique indexes. */
    readonly claims: ClaimPrimary | undefined;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral: EphemeralStorage | undefined;
    /** Open a principal's live link for one of its streams, absent without links. */
    readonly connect: ((subject: Subject) => Promise<() => Promise<void>>) | undefined;
    /** The subscriptions the database follows to maintain its replicas. */
    readonly subscriber: Subscriber | undefined;
    /** Read how the store has its scope's chain: home, follower, pending, or absent with none. */
    readonly chain: (view: Snapshot) => Promise<ChainOptions | PendingChain | undefined>;
    /** The publications the store serves and follows beside its objects' and policies' ones. */
    readonly publications: readonly sync.Publication[];
    /** Stream served row replicas to an in-process follower, running its changes as principal. */
    readonly uplink: sync.Uplink;
    /** Open an audit recorder for a scope and caller, with its calls' request and component. */
    readonly audit: (
        scope: string,
        caller: AccessContext,
        options: AuditRecorderOptions,
    ) => AuditRecorder;
    /** Report failed settlements. */
    readonly #report: (error: unknown) => void;
    /** The object call spans, on the environment's telemetry. */
    readonly #instruments: TelemetryScope;
    /** The environment calls run and controllers reconcile in: its clock and randomness. */
    readonly environment: Environment;
    /** Whether reads of the objects record access events. */
    readonly isAccessAudited: boolean;
    /** The replicas the objects stream to clients and databases below. */
    readonly source: ObjectSource;
    /** The replicas the store follows into its database. */
    readonly sink: ObjectSink;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, TupleReader>();
    /** Each row follow's keys by source row as last listed, with what each key read. */
    readonly #rowKeys = new Map<Follow, Map<string, RowKey>>();
    /** The options each catalog compiles with, apart from the servers and sandboxes. */
    readonly #compiling: Omit<CatalogOptions, "servers" | "sandboxes">;
    /** The packages, types, routes, tables and access rules the store serves. */
    #catalog: Catalog;
    /** The handler of each served package, by its catalog entry. */
    #handlers = new Map<
        Catalog["served"][number],
        PackageHandler & { readonly objects: ObjectStore }
    >();
    /** What the first handler's store controllers were built over: the served types and tables. */
    #controlled = "";
    /** The wait for the replicas gating readiness, shared by every implemented package. */
    #readiness: Promise<boolean> | undefined;
    /** The health of each server waiting on the replicas gating readiness. */
    readonly #waiting = new Set<Health>();
    /** Write a transaction's recorded calls at its commit, stamped with its log identity once. */
    readonly #calls: BatchWriter<(stamp: string | undefined) => JournalRecord> = async (
        items,
        committing,
    ) => {
        const stamp = await committing.log.transaction();
        await this.journal.write(
            items.map((item) => item(stamp)),
            committing,
        );
    };

    /** Serve object types over one database. */
    constructor(given: ObjectStoreOptions) {
        // keep the partition's blobs over the store's database
        this.blobs = new BlobStore({
            storage: given.blobs,
            database: given.database,
            environment: given.environment,
        });

        // run as the installation serving the store, else as the given principal, refusing a second uplink
        const { server, uplink } = given;
        if (server !== undefined && uplink !== undefined) {
            throw new TypeError("an installation's replicas must leave through its publisher");
        }
        this.principal =
            server === undefined
                ? present(given.principal, "the store's principal")
                : principal.installation.reference(server.scope, server.installation);
        this.partitions = ObjectStore.#composedPartitions(server?.partition, given.partitions);

        // store the stores, machines, releases and settings the store runs with
        this.release = given.release;
        this.database = given.database;
        this.environment = given.environment;
        this.claims = given.claims;
        this.chain = given.chain;
        this.publications = given.publications ?? [];
        this.ephemeral = given.ephemeral;
        this.connect = given.connect;
        this.isAccessAudited = given.isAccessAudited ?? false;
        this.origin = { ...given.origin, package: Package.of(given.origin.package) };
        this.#report = given.report ?? ((error) => ObjectStore.#rethrow(error));
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            given.environment.telemetry,
        );

        // compile what the servers serve, then follow its tables in one feed
        const partition = this.home;
        this.#compiling = {
            ...given,
            blobs: this.blobs,
            principal: this.principal,
            report: this.#report,
            ...(partition === undefined ? {} : { partition }),
        };
        this.#catalog = Catalog.compile({
            ...this.#compiling,
            servers: given.servers,
            sandboxes: given.sandboxes ?? [],
        });
        this.feed = new sync.Feed(given.database, this.#catalog.tables, {});
        this.#declare();

        // follow the replicas, publishing a partition through its installation's root
        const isPartition = server?.partition !== undefined && server.partition !== server.scope;
        const publisher =
            server !== undefined && isPartition
                ? ObjectStore.#partitionPublisher(server, () => this, given.publisher)
                : given.publisher;
        this.subscriber = this.#following(
            server === undefined ? uplink : server.publisher,
            publisher,
        );
        this.uplink = this.#uplink();

        // record the calls, the audit and the replicas
        this.#journal = ObjectStore.#journalOf(this.#catalog.isServing, given);
        this.audit = (scope, caller, recording) =>
            AuditRecorder.scoped(this.journal, this.origin)(scope, caller, recording);
        this.source = new ObjectSource(this);
        this.sink = new ObjectSink(this);

        // implement each served package, the first one also running the store's controllers
        this.#implement();
    }

    /** Serve other servers and sandboxes, keeping the handlers of the packages read before. */
    apply(
        servers: readonly PackageImplementation[],
        sandboxes: readonly HookSandbox[],
    ): readonly (PackageHandler & { readonly objects: ObjectStore })[] {
        // compile the new catalog, reusing the current one's reads of unchanged servers
        const catalog = Catalog.compile({ ...this.#compiling, servers, sandboxes }, this.#catalog);

        // refuse methods in a store started without the journal recording them
        if (catalog.isServing && this.#journal === undefined) {
            throw new TypeError("a store started without methods has no journal for new ones");
        }
        this.#catalog = catalog;

        // declare its tables to the database and the feed, then implement its packages
        this.#declare();
        this.feed.declare(this.#catalog.tables);
        this.#implement();

        return this.packages;
    }

    /** The package servers the store serves, in order. */
    get servers(): readonly PackageImplementation[] {
        return this.#catalog.servers;
    }

    /** The sandboxes the store serves the installations of. */
    get sandboxes(): readonly HookSandbox[] {
        return this.#catalog.sandboxes;
    }

    /** The tables whose changes relist the replicas: the chains', the follows' and their keys. */
    get watched(): readonly Table[] {
        // relist when chained or followed rows change, and when the store acquires or evicts a key
        const { follows, chained } = this.#catalog;

        return this.subscriber === undefined
            ? []
            : [
                  ...chained,
                  ...follows.flatMap((each) => each.shape.objects.map((type) => type.table)),
                  ...follows.flatMap((each) => ("from" in each ? [each.from.table] : [])),
                  ...(follows.some((each) => "onDemand" in each) ? [followKey] : []),
              ];
    }

    /** The served packages' implementations, in the order of their servers. */
    get packages(): readonly (PackageHandler & { readonly objects: ObjectStore })[] {
        return this.#catalog.served.map((entry) =>
            present(this.#handlers.get(entry), `the handler of ${entry.package.name}`),
        );
    }

    /** Declare the tables of the objects and replicas stored in the database. */
    #declare(): void {
        this.database.state.declare(
            [...this.objects, ...this.replicas]
                .filter((object) => object.storage === "durable" || object.storage === "events")
                .flatMap((object) => object.tables),
        );
    }

    /** Implement each served package once, the first again once its store controllers change. */
    #implement(): void {
        // keep the handlers of packages still served, the first while its store controllers read the same types and tables
        const { served } = this.#catalog;
        const servedIds = new Set(served.map((entry) => entry.package.id));
        const types = [...this.objects, ...this.#catalog.provisioned].map((object) =>
            ObjectTypeReference.key(object.typeReference),
        );
        const watched = [...this.watched, ...this.replicated.map((each) => each.table)].map(
            (table) => table[TABLE].sqlName,
        );
        const controlled = canonicalize([
            [...new Set(types)].toSorted(),
            [...new Set(watched)].toSorted(),
        ]);
        const handlers = new Map<
            Catalog["served"][number],
            PackageHandler & { readonly objects: ObjectStore }
        >();
        for (const [index, entry] of served.entries()) {
            const isFirst = index === 0;
            const kept = this.#handlers.get(entry);
            const isKept = kept !== undefined && (!isFirst || controlled === this.#controlled);
            handlers.set(entry, isKept ? kept : this.#packageHandler(entry, isFirst, servedIds));
        }
        this.#handlers = handlers;
        this.#controlled = controlled;
    }

    /** The object types served. */
    get objects(): readonly ObjectType[] {
        return this.#catalog.objects;
    }

    /** The types whose queries run over rows replicated here, never followed nor changed here. */
    get replicas(): readonly ObjectType[] {
        return this.#catalog.replicas;
    }

    /** The evaluator of the served objects' policies. */
    get authorizer(): Authorizer {
        return this.#catalog.authorizer;
    }

    /** The files with the served external objects. */
    get external(): readonly ExternalStorage[] {
        return this.#catalog.external;
    }

    /** The durable types the store serves or decides, whose rows stand for their principals. */
    get decided(): readonly ObjectType[] {
        return this.#catalog.decided;
    }

    /** The on-demand follows of the rows standing for callers, acquired where they are missing. */
    get standing(): readonly Extract<Follow, { readonly onDemand: unknown }>[] {
        return this.#catalog.standing;
    }

    /** The types follower replicas compile against: the followed shapes' and the decided. */
    get followed(): RelationGraph {
        return this.#catalog.followed;
    }

    /** The relations between the object types served, which the served queries compile against. */
    get relations(): RelationGraph {
        return this.#catalog.relations;
    }

    /** The served durable object types. */
    get durable(): readonly ObjectType[] {
        return this.#catalog.durable;
    }

    /** The served or authorized durable types whose rows the database replicates from home. */
    get replicated(): readonly ObjectType[] {
        return this.#catalog.replicated;
    }

    /** The scope's branch types, absent for a store without branches. */
    get branch(): BranchType | undefined {
        return this.#catalog.branch;
    }

    /** The providers reconciling the served kinds' resources, none for a store reconciling none. */
    get providers(): ResourceControllerOptions["providers"] {
        return this.#catalog.providers;
    }

    /** The declarations installed object types resolve against: the types, policies and kinds. */
    get known(): readonly KnownDeclaration[] {
        return this.#catalog.known;
    }

    /** The clock calls run and controllers reconcile at, its environment's. */
    get clock(): Clock {
        return this.environment.clock;
    }

    /** Read the implementation of a package the store serves, refusing one it serves not. */
    package(handle: PackageHandle): PackageHandler & { readonly objects: ObjectStore } {
        const found = this.packages.find((each) => each.package.id === handle.id);
        if (found === undefined) {
            throw new TypeError(`the store does not serve package ${handle.name}`);
        }

        return found;
    }

    /** Follow the catalog's follows and the chains, absent for a root store following nothing. */
    #following(
        uplink: sync.Uplink | undefined,
        publisher: ((scope: string) => sync.Publisher) | undefined,
    ): Subscriber | undefined {
        // leave a root store, which follows nothing, without a subscriber
        if (this.#catalog.follows.length === 0 && uplink === undefined && publisher === undefined) {
            return undefined;
        }

        // place the replicas below the store's scope or partition, toward each scope's home
        const target = this.scope;
        const publisherOf = (scope: string) =>
            publisher?.(scope) ?? present(uplink, `the publisher of ${scope}`);

        // follow the current catalog's follows' keys and the chains
        return {
            ...(uplink === undefined ? {} : { uplink }),
            publisher: publisherOf,
            subscriptions: async (view, changes) => {
                // follow the follows' keys, then the chains
                const followed = await ObjectStore.#followSubscriptions(
                    this,
                    this.#catalog.follows,
                    { target, view, ...(changes === undefined ? {} : { changes }) },
                );
                const chainedEntries = (await this.sink.chains(target, view)).map(
                    ({ subscription, home }) => ({ subscription, home, isAcquired: false }),
                );

                return [...chainedEntries, ...followed];
            },
        };
    }

    /** Describe known types and every type they refer to, every served one when given none. */
    describe(types?: readonly ObjectTypeReference[]): ObjectDescription[] {
        // index the known types and every type their descriptions mention
        const known = new Map<string, ObjectType>();
        const pendingTypes = this.known.filter((declaration) => declaration instanceof ObjectType);
        for (let type = pendingTypes.pop(); type !== undefined; type = pendingTypes.pop()) {
            const key = ObjectTypeReference.key(type.typeReference);
            if (!known.has(key)) {
                known.set(key, type);
                pendingTypes.push(
                    ...type.referenced,
                    ...type.ancestors,
                    ...type.attachments.map((attachment) => attachment.object),
                );
            }
        }
        const loaded = ObjectTypePool.of([]);

        // describe each asked type, then each type a description refers to, leaving the exported ones every pool knows
        const pending = [...(types ?? this.#catalog.declared.map((type) => type.typeReference))];
        const described = new Map<string, ObjectDescription>();
        let reference = pending.pop();
        while (reference !== undefined) {
            const key = ObjectTypeReference.key(reference);
            const type = known.get(key);
            if (type === undefined) {
                throw ObjectError.of(
                    "TYPE_UNKNOWN",
                    `unknown object type ${reference.type} of ${reference.packageId}`,
                );
            } else if (!described.has(key) && !loaded.objects.has(key)) {
                const description = ObjectDescription.of(type);
                described.set(key, description);
                pending.push(...loaded.references(description));
            }
            reference = pending.pop();
        }

        return [...described.values()];
    }

    /** Stream served row replicas to in-process followers, and run the changes they send. */
    #uplink(): sync.Uplink {
        return {
            stream: (subscription, signal) => this.source.publish(subscription, signal),
            request: (subscription) => this.source.request(subscription),
            receive: (mutation, signer) => {
                // run the changes as the follower's principal
                if (signer === undefined) {
                    throw new TypeError("an in-process change has no sending principal");
                }

                return this.receive(mutation, signer);
            },
            call: (call, scope, signer) => {
                // answer the query as the follower's principal in the scope it reads
                if (signer === undefined) {
                    throw new TypeError("an in-process query has no sending principal");
                }
                const caller = AccessContext.of(Caller.of(signer), this.clock.now());

                return this.query(caller, this.#place(call, scope));
            },
        };
    }

    /** Compose a store's partitions: its installation's own first, then the others it serves. */
    static #composedPartitions(
        own: string | undefined,
        served: readonly string[] | undefined,
    ): readonly string[] {
        return [...new Set([...(own === undefined ? [] : [own]), ...(served ?? [])])];
    }

    /** Read the scope a principal's store serves: an installation's scope, else the principal. */
    static #scopeOf(subject: Subject): string {
        return principal.installation.is(subject) ? subject.scope : subject.id;
    }

    /** Read the partition a store serves inside its space, absent for the space itself or none. */
    #partitionOf(): string | undefined {
        const [own] = this.partitions;

        return own === this.space ? undefined : own;
    }

    /** Open a method store's journal with its call key, refusing a call key without methods. */
    static #journalOf(
        isServing: boolean,
        options: Pick<ObjectStoreOptions, "database" | "callKey" | "environment" | "events">,
    ): Journal | undefined {
        // require the call key and the event log recording the calls of a store serving methods
        const { callKey, events } = options;
        if (isServing && callKey === undefined) {
            throw new TypeError("a store with methods needs the call key of its journal");
        } else if (isServing && (events === undefined || !events.has(Journal.kind))) {
            throw new TypeError("a store with methods needs an event log for its calls");
        } else if (!isServing && callKey !== undefined) {
            throw new TypeError("a store without methods has no journal and no call key");
        }

        return callKey === undefined || events === undefined
            ? undefined
            : new Journal(options.database, callKey, { events, environment: options.environment });
    }

    /** The scope the store's database is home to: its partition, its space, else its principal. */
    get scope(): string {
        return this.partitions[0] ?? ObjectStore.#scopeOf(this.principal);
    }

    /** The partition the store's database is home to: its partition, its space, or the universe. */
    get home(): string | undefined {
        return (
            this.#partitionOf() ??
            this.space ??
            (universe.is(this.principal) ? Scope.universe.id : undefined)
        );
    }

    /** The space the store serves, absent for a machine's or the universe's store. */
    get space(): string | undefined {
        // a space's store serves the space, a sandboxed installation's store its installation's space
        const { principal: self } = this;
        if (principal.space.is(self)) {
            return self.id;
        } else if (principal.installation.is(self)) {
            return self.scope;
        }

        return undefined;
    }

    /** Act as the installation serving a type, refusing a type none serves. */
    as(object: ObjectType, now = this.clock.now()): AccessContext {
        // require the installation serving the type, which every served type has
        const served = this.readServer(object);
        if (served === undefined) {
            throw new TypeError(`no installation serves ${object.name} in this store`);
        }
        const subject = principal.installation.reference(served.scope, served.installation);

        return AccessContext.of(Caller.of(subject), now);
    }

    /** The event log with the store's calls and events, failing for a store without methods. */
    get events(): EventStore {
        return this.journal.events;
    }

    /** Read the journal of executed mutations, failing for a store serving no methods. */
    get journal(): Journal {
        if (this.#journal === undefined) {
            throw new TypeError("a store without methods has no journal");
        }

        return this.#journal;
    }

    /** Implement a package's served objects. */
    implement(
        handle: PackageHandle,
        routed: readonly ObjectType[],
        control: Control,
    ): PackageHandler & { readonly objects: ObjectStore } {
        const { subscriber } = this;

        return {
            objects: this,
            package: handle,
            access: this.access,
            audit: AuditRecorder.procedure((call) =>
                this.audit(
                    this.#procedureScope(call, routed) ?? Scope.universe.id,
                    ObjectStore.#requestCaller(call.request),
                    { package: Package.of(handle), requestId: call.request.requestId },
                ),
            ),
            control,
            ...(subscriber === undefined
                ? {}
                : {
                      ready: (signal, health) => this.#waitReady(signal, health),
                      synced: (signal) => this.#waitReady(signal),
                  }),
            router: this.router(routed),
        };
    }

    /** Follow a shape's key on demand until it lapses, waiting a bounded time for the replica. */
    async acquire<Parameters extends object>(
        follow: Extract<Follow<Parameters>, { readonly onDemand: unknown }>,
        at: FollowKey<Parameters>,
        signal?: AbortSignal,
    ): Promise<void> {
        // read the key's parameters and the principal its replica is decided for
        const { shape, onDemand } = follow;
        const { scope, ...rest } = at.key;
        const parameters = shape.parameters.parse(rest);
        const followed = {
            shape: shape.name,
            scope,
            parameters: canonicalize(parameters),
            subject: canonicalize(at.subject ?? this.principal),
        };

        // renew a key past half its time, evicting the lapsed ones as the store writes
        const now = this.clock.now();
        const ttl = Duration.milliseconds(onDemand.ttl);
        const [current] = await this.database
            .select({ expiresAt: followKey.expiresAt })
            .from(followKey)
            .where(
                and(
                    eq(followKey.shape, followed.shape),
                    eq(followKey.scope, followed.scope),
                    eq(followKey.parameters, followed.parameters),
                    eq(followKey.subject, followed.subject),
                ),
            );
        if (current === undefined || current.expiresAt - now < ttl / 2) {
            await this.database.transaction(async (transaction) => {
                await transaction.delete(followKey).where(lte(followKey.expiresAt, now));
                await transaction
                    .insert(followKey)
                    .values({ ...followed, expiresAt: now + ttl })
                    .onConflictDoUpdate({
                        target: [
                            followKey.shape,
                            followKey.scope,
                            followKey.parameters,
                            followKey.subject,
                        ],
                        set: { expiresAt: now + ttl },
                    });
            });
        }

        // wait on commits until the synced replica follows the key's queries, refusing a late one
        const timeout = this.clock.timeout(ACQUIRE_MILLISECONDS);
        try {
            await this.database.log.waitUntil(
                () => shape.isSynced(this.database, at.key, this.#follower(at.subject)),
                signal === undefined ? timeout : AbortSignal.any([signal, timeout]),
            );
        } catch (error) {
            if (!timeout.aborted) {
                throw error;
            }

            throw ObjectError.of(
                "REPLICA_SYNCING",
                `the ${shape.name} replica of ${scope} is still syncing`,
            );
        }
    }

    /** Wait for the replicas gating readiness, checked once for all packages, or until stopped. */
    async #waitReady(signal: AbortSignal, health?: Health): Promise<void> {
        // report what the shared check waits for on the server's health
        if (health !== undefined) {
            this.#waiting.add(health);
        }

        // share one check of the replicas, checking again after the checking server stopped first
        const stopped = new Promise<boolean>((resolve) => {
            signal.addEventListener("abort", () => resolve(false), { once: true });
        });
        try {
            while (!signal.aborted) {
                const readiness = (this.#readiness ??= this.#ready(signal));
                if (await Promise.race([readiness, stopped])) {
                    return;
                }
                if (this.#readiness === readiness) {
                    this.#readiness = undefined;
                }
            }
        } finally {
            if (health !== undefined) {
                this.#waiting.delete(health);
            }
        }
    }

    /** Wait until each access replica of the store's chain is live. */
    async waitLive(signal: AbortSignal): Promise<void> {
        await this.#waitLive(async () => {
            const chain = (await Scope.chain(Snapshot.live(this.database), this.scope)).map(
                (link) => link.object.id,
            );
            const stale = await Access.stale(this.database, chain, this.authorizer.lag);

            return stale === undefined ? undefined : `the access replica of ${stale} is not live`;
        }, signal);
    }

    /** Wait until each current gating replica is live. */
    async #ready(signal: AbortSignal): Promise<boolean> {
        return await this.#waitLive(async () => {
            // wait for a pending chain, whose replicas the store lists once it is known
            const { database } = this;
            const chain = await this.chain(Snapshot.live(database));
            if (chain?.kind === "pending") {
                return `the chain of ${this.scope} is pending`;
            }

            // require each gating replica live and current, naming the first that is not
            for (const subscription of await this.sink.gates()) {
                const { name, scope } = subscription;
                if (!sync.Replica.isLive(database, name, scope, this.authorizer.lag)) {
                    return `the replica ${name} of ${scope} is not live`;
                } else if (!(await sync.Replica.isCurrent(database, subscription))) {
                    return `the replica ${name} of ${scope} is not current`;
                }
            }

            return undefined;
        }, signal);
    }

    /** Check now and after each replica record or liveness change, until it passes. */
    async #waitLive(
        check: () => Promise<string | undefined>,
        signal: AbortSignal,
    ): Promise<boolean> {
        // check until passed or aborted
        const { database } = this;
        const records = new Set(sync.replicaRecordTables.map((table) => table[TABLE].sqlName));
        while (!signal.aborted) {
            // watch liveness from before the check, so no change passes unseen
            const changed = new AbortController();
            const unwatch = database.tracker.watchMarked(() => changed.abort());
            const { generation } = database.state.commits;
            try {
                const reason = await check();
                if (reason === undefined) {
                    return true;
                }
                for (const health of this.#waiting) {
                    health.set("starting", reason);
                }

                // wait for a replica record commit or a liveness change
                const waking = AbortSignal.any([signal, changed.signal]);
                await database.log
                    .waitUntil(
                        async (committed) => [...committed].some((table) => records.has(table)),
                        waking,
                        generation,
                    )
                    .catch((error: unknown) => {
                        if (!waking.aborted) {
                            throw error;
                        }
                    });
            } finally {
                unwatch();
            }
        }

        return false;
    }

    /** The server access deciding the served objects. */
    get access(): ServerAccess {
        return {
            authorizer: this.authorizer,
            database: this.database,
            environment: this.environment,
        };
    }

    /** Route the served objects' methods, or some of them, and the store procedures. */
    router(types: readonly ObjectType[] = this.#catalog.declared): ObjectRouter {
        // route each object under its key, which its package's handle makes unique
        const routers: Record<string, Router<ProcedureRouter, RequestContext>> = {};
        for (const object of types) {
            routers[object.key] = this.#route(object);
        }

        return { store: this.#procedures(), ...routers };
    }

    /** Route an object's methods to the call pipeline. */
    #route(object: ObjectType): Record<string, Router<ProcedureRouter, RequestContext>> {
        // bind each method's procedure to the call pipeline
        const routes: Record<string, Router<ProcedureRouter, RequestContext>> = {};
        for (const [name, procedure] of Object.entries(callableProcedures(object))) {
            routes[name] = implement(procedure)
                .$context<RequestContext>()
                .handler(({ input, context }) => this.#answer(object, name, input, context));
        }

        return routes;
    }

    /** Route the store procedures. */
    #procedures(): Router<StoreProcedures, RequestContext> {
        const store = implement(storeProcedures).$context<RequestContext>();

        return store.router({
            push: store.push.handler(({ input, context }) =>
                this.#push(input.scope, input.mutations, context, input.writerId),
            ),
            watermark: store.watermark.handler(async ({ input, context }) => {
                // read the scope's watermark once its replicas have the homes' commits
                await this.enter(context, input.scope, false);
                await this.#waitHomes(context, input.scope, input.homes);

                return await this.watermark(input.scope);
            }),
            call: store.call.handler(async ({ input, context }) => {
                // read in the given scope
                const {
                    object,
                    name,
                    input: called,
                } = this.#resolve(input.call, false, CallSite.of(context.access(input.scope)));
                if (object.readScope(name, called) !== input.scope) {
                    throw ObjectError.of(
                        "SCOPE_MISMATCH",
                        `${object.name}.${name} reads a scope other than ${input.scope}`,
                    );
                }

                return await this.#answer(object, name, called, context);
            }),
            receive: store.receive.handler(async ({ input, context }) => {
                // run the follower's mutation as the caller the route's identity verified
                const results = await this.receiveFrom(input.mutation, context.access());

                return { results: results.map((result) => schema.json().parse(result ?? null)) };
            }),
            broadcast: store.broadcast.handler(async ({ input, context }) => {
                await this.source.broadcast(input.scope, input.object, input.event, context);

                return {};
            }),
            stream: store.stream.handler(({ input, context, lastEventId }) => {
                const { after: _after, ...subscription } = input;
                const after = ObjectStore.#resumed(input.after, lastEventId);

                return this.source.relayed(
                    after === undefined ? subscription : { ...subscription, after },
                    context,
                );
            }),
            request: store.request.handler(async ({ input, context }) => {
                // record the caller's request of its own stream
                await this.source.requestFrom(input.subscription, context);

                return {};
            }),
            describe: store.describe.handler(({ input }) => ({
                objects: this.describe(input.types),
            })),
        });
    }

    /** Read a request's caller for its audit: anonymous when its credential failed. */
    static #requestCaller(context: RequestContext): AccessContext {
        return context.authenticationError === undefined
            ? context.access()
            : AccessContext.of(Caller.anonymous(), context.clock.now());
    }

    /** Answer one routed procedure as its request's caller. */
    async #answer(
        object: ObjectType,
        name: string,
        input: JsonObject,
        context: RequestContext,
    ): Promise<unknown> {
        // split off the request's identifier or the writer, and enter the call's scope as the request's caller
        const method = object.method(name);
        const isEphemeral = object.storage === "ephemeral";
        const { [isEphemeral ? "writerId" : "requestId"]: named, ...rest } = input;
        const scope = object.readScope(name, rest);
        await this.enter(context, scope, method.mutates);
        const caller = context.access(scope);

        // read through a query
        const call = object.call(name, rest);
        if (!method.mutates) {
            return this.query(caller, call);
        }

        // write through a single-call mutation and report its watermark in the written scope
        const [written] = isEphemeral
            ? await this.mutate(
                  caller,
                  { id: RequestId.create(this.environment), calls: [call] },
                  { writerId: WriterId.parse(named) },
              )
            : await this.mutate(caller, { id: schema.string().parse(named), calls: [call] });
        context.observed.observe(await this.watermark(scope));

        return written;
    }

    /** Read through a method as a caller in a read-only transaction, recording audited reads. */
    async query<Result>(
        caller: AccessContext,
        call: sync.Call<Result>,
        site: CallSite = CallSite.of(caller),
        options: QueryOptions = {},
    ): Promise<Result> {
        // read untyped, then type the answer by the method's output
        const callable = this.#callable(call, false, this.replicas);
        const answer = await this.#query(caller, call, site, options);
        const [result] = MethodContext.results(callable, [call], [answer]);

        return result;
    }

    /** Read through a method as a caller, answering its untyped result. */
    async #query(
        caller: AccessContext,
        call: sync.Call,
        site: CallSite,
        options: QueryOptions,
    ): Promise<unknown> {
        // resolve the call, reading only a method the caller's grant allows
        const { object, name, input } = this.#resolve(call, false, site);
        const method = object.method(name);
        const scope = object.readScope(name, input);
        await this.#limit([{ object, name, input }], scope, caller);

        // refuse joining a durable transaction for objects living elsewhere
        if (options.transaction !== undefined && object.storage !== "durable") {
            throw new TypeError(
                `${object.storage} ${object.name} objects cannot join a durable transaction`,
            );
        }
        const database = await this.#databaseOf(object, scope);
        const read = async () => {
            // prepare on the served type, then read, commit and settle
            const prepared = await this.#prepare(
                [{ object: this.served(object), name, input }],
                scope,
                caller,
                RequestId.create(this.environment),
            );
            try {
                const value = await readIn(prepared[0]);
                await this.#commit(this.database, prepared);

                return value;
            } finally {
                await this.#settle(ObjectStore.#settlementsOf(prepared));
            }
        };
        const readIn = async (prepared: Prepared) => {
            // read over the joined transaction or one that never commits, allowing the caller against the durable database
            const reading = options.transaction ?? Transaction.reading(database);
            const authorizing =
                object.storage === "durable" ? reading : Transaction.reading(this.database);
            const authorization = await this.authorizeHere(authorizing, scope, caller, {
                calls: [{ object, name }],
            });

            return this.#execute(reading, authorization, object, name, input, caller, {
                prepared,
                authorizations: new Map(),
            });
        };

        // read unaudited objects directly
        if (!object.isReadAudited && method.isAudited !== true && !this.isAccessAudited) {
            return read();
        }

        // audit one object, or the scope's collection
        const recorder = this.audit(scope, caller, { package: this.#implementing(object) });
        const { action, values } = object.auditCall(name, input, scope);
        const details = method.audit?.details;

        // record external work as an attempt and its result
        if (method.isAudited === true) {
            return recorder.attempt(action, values, read);
        }
        // record a read as one access event, with the details of its result
        else {
            return recorder.read(action, values, read, details && ObjectStore.#detailsOf(details));
        }
    }

    /** Open the database with an object type's rows in a scope. */
    async #databaseOf(object: ObjectType, scope: string): Promise<DatabaseConnection> {
        // read ephemeral objects from memory
        if (object.storage === "ephemeral") {
            const ephemeral = this.requireEphemeral();
            await ephemeral.ready();

            return ephemeral.database;
        }
        // read external objects from the scope's files
        else if (object.storage === "external") {
            return ExternalStorage.of(this.external, [object.name]).open(scope);
        }
        // read durable objects from the database
        else {
            return this.database;
        }
    }

    /** Run a mutation's calls as a caller in one transaction, once per caller and identifier. */
    async mutate<const Calls extends readonly sync.Call[]>(
        caller: AccessContext,
        transaction: { readonly id: string; readonly calls: Calls },
        options: MutateOptions = {},
    ): Promise<ResultsOf<Calls>> {
        // parse each answer by its served type, or by the replicated one whose home answered it
        const answers = await this.#mutate(caller, transaction, options);

        return MethodContext.results(
            [...this.objects, ...this.replicated],
            transaction.calls,
            answers,
        );
    }

    /** Run a mutation's calls as a caller in one transaction, answering untyped results. */
    async #mutate(
        caller: AccessContext,
        submitted: { readonly id: string; readonly calls: readonly sync.Call[] },
        options: MutateOptions,
    ): Promise<readonly unknown[]> {
        // place the calls without a scope in the mutation's scope
        const { scope: within } = options;
        const mutation =
            within === undefined
                ? submitted
                : {
                      ...submitted,
                      calls: submitted.calls.map((entry) => this.#place(entry, within)),
                  };

        // answer an empty mutation with no results, else join an open transaction
        if (mutation.calls.length === 0) {
            return [];
        } else if (options.transaction !== undefined) {
            return this.#mutateInside(options.transaction, mutation, caller, options);
        }

        // run a change of rows replicated here at their home as the caller's subject, within its grant
        if (this.#isForwarded(mutation)) {
            for (const call of mutation.calls) {
                await this.#requireForwardable(caller, call);
            }
            const forwarded = await this.#forward(mutation, caller.subject);
            if (forwarded !== undefined) {
                return forwarded;
            }
        }

        // resolve each call in one scope, of methods the caller's grant allows
        const site = options.site ?? CallSite.of(caller);
        const originals = mutation.calls.map((entry) => this.#resolve(entry, true, site));
        const scope = this.#mutationScope(originals);
        await this.#limit(originals, scope, caller);

        // write ephemeral objects apart
        const ephemeral = originals.filter((call) => call.object.storage === "ephemeral").length;
        if (ephemeral === originals.length) {
            return this.#mutateEphemeral(originals, scope, caller, mutation.id, options.writerId);
        } else if (ephemeral > 0) {
            throw ObjectError.of(
                "STORAGE_MIXED",
                "a mutation cannot write both durable and ephemeral objects",
            );
        }

        // expand calls into the calls they run after, and identify prepared creations
        const calls = (await this.#expand(originals, scope, caller)).map((call) =>
            ObjectStore.#identified(call, this.environment),
        );

        // identify the mutation and prepare external work
        const run = await this.#identify(mutation, originals, calls, scope, caller, options);
        const prepared = await this.#prepare(
            calls,
            scope,
            caller,
            run.request.requestId,
            run.request,
        );
        const planned = { ...run, prepared };

        // execute through the journal, tracking the running call
        const progress: MutationProgress = { running: undefined, reservation: undefined };
        const results = await this.#executeDurable(planned, progress);

        // confirm keys and settle prepared work
        try {
            await progress.reservation?.confirm();
        } finally {
            await this.#settle(ObjectStore.#settlementsOf(prepared));
        }

        // answer the submitted calls without their expansions
        return zip(results, calls)
            .filter(([, call]) => call.as === undefined)
            .map(([result]) => result);
    }

    /** Place a call without a scope in one, by its served or replicated type's route field. */
    #place(entry: sync.Call, scope: string): sync.Call {
        // find the route field of the call's method, which resolution refuses when unknown
        const type = ObjectType.find([...this.objects, ...this.replicated], entry);
        const field =
            type !== undefined && Object.hasOwn(type.methods, entry.method)
                ? type.route.field
                : undefined;

        return field === undefined || Object.hasOwn(entry.input, field)
            ? entry
            : { ...entry, input: { ...entry.input, [field]: scope } };
    }

    /** Run calls inside a running mutation without external work. */
    async #mutateInside(
        transaction: Transaction,
        mutation: { readonly id: string; readonly calls: readonly sync.Call[] },
        caller: AccessContext,
        options: MutateOptions,
    ): Promise<unknown[]> {
        // run each call in order as the caller under the mutation's request, recorded under the component making it
        const recorded: CallOptions = {
            requestId: mutation.id,
            authorizations: new Map(),
            ...(options.component === undefined ? {} : { component: options.component }),
            ...(options.manager === undefined ? {} : { manager: options.manager }),
        };
        const results: unknown[] = [];
        for (const entry of mutation.calls) {
            results.push(
                await this.#runInside(
                    transaction,
                    entry,
                    caller,
                    true,
                    CallSite.store,
                    undefined,
                    recorded,
                ),
            );
        }

        return results;
    }

    /** Run one platform call inside a running mutation, a client's only on client methods. */
    async #runInside(
        transaction: Transaction,
        entry: sync.Call,
        caller: AccessContext,
        mutates: boolean,
        site: CallSite,
        parent:
            | { readonly context: MethodContext; readonly authorization: Authorization }
            | undefined,
        options: CallOptions,
    ): Promise<unknown> {
        // place the call in the calling method's scope unless it has another
        const placed = parent === undefined ? entry : this.#place(entry, parent.context.scope);

        // resolve the call, refusing external work and other storages, except events beside durable rows
        const { object, name, input } = ObjectType.resolve(
            this.#callable(placed, mutates, [...this.replicated, ...this.replicas]),
            placed,
            mutates,
            site,
        );
        const storage = parent?.context.object.storage ?? "durable";
        const isBeside = object.storage === "events" && storage === "durable";

        // read external objects outside the transaction, consistent as their rows never change
        if (object.storage === "external" && !mutates && parent !== undefined) {
            return this.query(caller, placed);
        }

        // refuse other storages and external work
        if (this.served(object).method(name).prepare !== undefined) {
            throw new TypeError(
                `${object.name}.${name} does external work, so it cannot run inside a transaction`,
            );
        } else if (object.storage !== storage && !isBeside) {
            throw new TypeError(`${object.name} is stored outside the transaction's storage`);
        }

        // run it under the calling method's authorization while current, else the scope and caller's
        const scope = object.readScope(name, input);
        const authorization =
            parent !== undefined &&
            parent.context.caller === caller &&
            parent.context.scope === scope &&
            parent.authorization.isCurrent
                ? parent.authorization
                : await this.#authorizeCall(transaction, scope, caller, [{ object, name }], {
                      ...(parent === undefined ? {} : { now: parent.context.now }),
                      authorizations: options.authorizations,
                  });

        // write for the call's declaration, if any
        const writing =
            options.manager === undefined ? authorization : authorization.applying(options.manager);

        return this.#execute(transaction, writing, object, name, input, caller, options);
    }

    /** Read the one scope a mutation's calls act in. */
    #mutationScope(originals: readonly ReturnType<typeof ObjectType.resolve>[]): string {
        // require every call to act in the first call's scope
        const first = nth(originals, 0);
        const scope = first.object.readScope(first.name, first.input);
        if (originals.some((call) => call.object.readScope(call.name, call.input) !== scope)) {
            throw ObjectError.of("SCOPES_MIXED", "a mutation's calls must act in one scope");
        }

        return scope;
    }

    /** Identify a durable mutation by caller, scope and request id, with its redacted calls. */
    async #identify(
        mutation: { readonly id: string; readonly calls: readonly sync.Call[] },
        originals: readonly ReturnType<typeof ObjectType.resolve>[],
        calls: readonly Expansion[],
        scope: string,
        caller: AccessContext,
        options: MutateOptions,
    ): Promise<MutationRun> {
        // require an authenticated caller the journal keys the mutation by
        if (caller.subject === undefined) {
            throw ObjectError.of("UNAUTHENTICATED", "a mutation needs an authenticated caller");
        }

        return {
            id: mutation.id,
            scope,
            caller,
            ...(options.component === undefined ? {} : { component: options.component }),
            ...(options.manager === undefined ? {} : { manager: options.manager }),
            calls,
            prepared: [],
            request: { caller: Caller.key(caller), scope, requestId: mutation.id },
            fingerprint: await this.#fingerprint(mutation, originals),
            redacted: calls.map((call) =>
                schema.redactFields(InputSchema.fields(call.object.input(call.name)), call.input),
            ),
        };
    }

    /** Execute a planned mutation once: commit it, else answer the outcome its journal recorded. */
    async #executeDurable(planned: MutationRun, progress: MutationProgress): Promise<unknown[]> {
        // reject a key expired or in the future
        RequestId.expiry(planned.request.requestId, this.clock.now());
        try {
            // run the authorized calls over a transaction committed with their records, again while it loses a race
            return await Transaction.run(this.database, async (transaction) => {
                const authorization = await this.#authorizeRun(transaction, planned);

                return this.#run(transaction, authorization, planned, progress);
            });
        } catch (error) {
            // answer the outcome an earlier or concurrent run recorded
            const committed = await this.journal.replay(planned.request, planned.fingerprint);
            if (committed !== undefined) {
                return committed;
            }

            // roll back prepared work and release the reserved names
            await this.#settle(ObjectStore.#settlementsOf(planned.prepared));
            await progress.reservation?.release().catch((failure: unknown) => {
                throw new AggregateError(
                    [error, failure],
                    "transaction and its name release failed",
                    {
                        cause: error,
                    },
                );
            });

            // record the failed call for audit and retries
            if (progress.running !== undefined) {
                await this.#recordFailure(planned, progress.running, error);
            }
            throw error;
        }
    }

    /** Allow a mutation's caller in its chain, refusing installation writes to capped spaces. */
    async #authorizeRun(transaction: Transaction, planned: MutationRun): Promise<Authorization> {
        // allow the caller in the scope chain the mutation reads
        const chain = await Scope.chain(transaction.view(), planned.scope);
        const authorization = await this.authorizeHere(transaction, planned.scope, planned.caller, {
            request: planned.id,
            links: chain,
            calls: planned.calls,
        });

        // refuse an installation's writes to a space whose storage is capped, the submitted calls deciding
        if (principal.installation.is(this.principal)) {
            authorization.requireUncapped(planned.calls.filter((call) => call.as === undefined));
        }

        // write for the run's declaration, if any
        return planned.manager === undefined
            ? authorization
            : authorization.applying(planned.manager);
    }

    /** Digest a mutation's calls with their sensitive inputs redacted. */
    async #fingerprint(
        mutation: { readonly id: string; readonly calls: readonly sync.Call[] },
        originals: readonly ReturnType<typeof ObjectType.resolve>[],
    ): Promise<string> {
        const sensitive: unknown[] = [];
        const calls = zip(mutation.calls, originals).map(([entry, { object, name, input }]) => {
            const fields = InputSchema.fields(object.input(name));

            return {
                type: entry.type,
                method: entry.method,
                input: schema.redact(fields, input, (found) => sensitive.push(found)),
            };
        });

        return this.journal.fingerprint({ id: mutation.id, calls }, sensitive);
    }

    /** Execute a mutation's calls in order, recording each for retries after commit. */
    async #run(
        transaction: Transaction,
        authorization: Authorization,
        run: MutationRun,
        progress: MutationProgress,
    ): Promise<unknown[]> {
        // execute the calls in order, tracked objects from the mutation's base
        const from = run.calls.some((call) => call.object.lifecycle.tracked !== undefined)
            ? transaction.base
            : undefined;
        const executed: unknown[] = [];
        const authorizations = new Map<string, Authorization>();
        let current = authorization;
        for (const [index, call] of run.calls.entries()) {
            // authorize the remaining calls again once an earlier call changed the caller's access
            progress.running = index;
            current = current.isCurrent
                ? current
                : await this.#authorizeRun(transaction, { ...run, calls: run.calls.slice(index) });
            const options: CallOptions = {
                authorizations,
                prepared: run.prepared[index],
                ...(from === undefined ? {} : { from }),
                ...(call.as === undefined ? {} : { as: call.as }),
                ...(call.expansion === undefined ? {} : { expansion: call.expansion }),
                ...(run.component === undefined ? {} : { component: run.component }),
                requestId: run.request.requestId,
                request: ObjectStore.#callRequest(run, index),
            };
            executed.push(
                await this.#execute(
                    transaction,
                    current,
                    call.object,
                    call.name,
                    call.input,
                    run.caller,
                    options,
                ),
            );
        }

        // release the names a lost attempt reserved
        progress.running = undefined;
        await progress.reservation?.release();
        progress.reservation = undefined;

        // reserve the claimed names before the commit
        const claimants = await Claimant.written(transaction, this.objects);
        const reservation =
            this.claims &&
            (await Reservation.reserve(
                this.claims,
                claimants,
                this.objects,
                { id: run.id, environment: this.environment },
                this.database,
            ));
        progress.reservation = reservation;

        // commit prepared work and claims with the mutation
        transaction.record(async (committing) => {
            // commit the prepared work
            await this.#commit(committing, run.prepared);

            // commit the claimed names
            await reservation?.commit(committing);
        });

        return executed;
    }

    /** Record a mutation's failed call for audit and retries, leaving denials to procedures. */
    async #recordFailure(run: MutationRun, position: number, error: unknown): Promise<void> {
        // skip a denial
        const outcome = AuditRecorder.outcome(error);
        if (outcome.kind === "denied") {
            return;
        }

        // record the failed call with its request
        const failed = nth(run.calls, position);
        const { action, values } = failed.object.auditCall(failed.name, failed.input, run.scope);
        await this.audit(run.scope, run.caller, {
            package: failed.object.package,
            ...(run.component === undefined ? {} : { component: run.component }),
        }).record(
            undefined,
            action,
            { ...values, outcome },
            "activity",
            ObjectStore.#callRequest(run, position),
        );
    }

    /** Build the request record of a mutation's call at a position. */
    static #callRequest(run: MutationRun, position: number): CallRequest {
        return {
            requestId: run.request.requestId,
            caller: run.request.caller,
            position,
            digest: run.fingerprint,
            input: nth(run.redacted, position),
        };
    }

    /** Expand calls whose methods list the calls they run after, each run as its author. */
    async #expand(
        calls: readonly ReturnType<typeof ObjectType.resolve>[],
        scope: string,
        caller: AccessContext,
    ): Promise<Expansion[]> {
        // pass calls whose methods expand into nothing
        const methods = calls.map((call) => this.served(call.object).method(call.name));
        if (methods.every((method) => method.expand === undefined)) {
            return [...calls];
        }

        // list each expanding call's calls before it, as the caller reads them now over a mutation that never commits
        const reading = Transaction.reading(this.database);
        const authorization = await this.authorizeHere(reading, scope, caller, { calls });
        const expanded: Expansion[] = [];
        for (const [call, method] of zip(calls, methods)) {
            // pass a call that expands into nothing
            if (method.expand === undefined) {
                expanded.push(call);
            }
            // run each listed call as its author first, hiding a denial the caller cannot see
            else {
                const built = await this.#context(
                    reading,
                    authorization,
                    call.object,
                    call.name,
                    call.input,
                    caller,
                ).catch(async (error: unknown) => {
                    throw await this.#conceal(authorization, caller, undefined, error);
                });
                let listed: readonly BranchEntry[];
                try {
                    listed = await method.expand(built.context);
                } catch (error) {
                    throw await this.#conceal(authorization, caller, built.context.target, error);
                }
                for (const { author, actors, ...entry } of listed) {
                    expanded.push({
                        ...this.#resolve(this.#place(entry, scope), true, CallSite.of(caller)),
                        as: { subject: author, actors },
                    });
                }
                expanded.push({ ...call, expansion: listed });
            }
        }

        return expanded;
    }

    /** Run a client's pushed mutations in order as its caller, returning the scope's watermark. */
    async #push(
        scope: string,
        mutations: readonly sync.Mutation[],
        context: RequestContext,
        writerId?: string,
    ): Promise<PushResult> {
        // enter the pushed scope as the request's caller
        await this.enter(context, scope, true);
        const caller = context.access(scope);

        // run each mutation in a separate transaction
        const outcomes: PushResult["outcomes"][number][] = [];
        for (const pushed of mutations) {
            const mutation = {
                ...pushed,
                calls: pushed.calls.map((call) => this.#place(call, scope)),
            };
            try {
                // refuse a mutation of replicated rows, which only their home writes
                this.#requireServed(mutation);

                // require the pushed scope to be the home running the mutation
                const first = this.#resolve(nth(mutation.calls, 0), true, CallSite.of(caller));
                if (first.object.readScope(first.name, first.input) !== scope) {
                    throw ObjectError.of(
                        "SCOPE_MISMATCH",
                        `a mutation pushed to ${scope} acts in another scope`,
                    );
                }
                const written = await this.mutate(
                    caller,
                    mutation,
                    writerId === undefined ? {} : { writerId },
                );
                outcomes.push({
                    id: mutation.id,
                    outcome: { value: schema.json().parse(written) },
                });
            } catch (error) {
                // record final failures, rethrow transient ones
                const failure = Replay.failure(error);
                if (failure === undefined) {
                    throw error;
                }
                outcomes.push({ id: mutation.id, outcome: { error: failure.error } });
            }
        }

        return { outcomes, watermark: await this.watermark(scope) };
    }

    /** Find the served replica of an object type with the store's handlers. */
    served<Type extends ObjectType>(object: Type): Type {
        const found = [...this.objects, ...this.replicas].find((served): served is Type =>
            served.isCopyOf(object),
        );
        if (found === undefined) {
            throw new TypeError(`object store does not serve ${object.name}`);
        }

        return found;
    }

    /** Decide whether the store keeps a type's rows only as replicas, serving it nowhere here. */
    replicates(object: ObjectType): boolean {
        return (
            this.database.replicates(object.table) &&
            !this.#catalog.installed.has(ObjectTypeReference.key(object.typeReference))
        );
    }

    /** Decide whether a scope's rows of a type are replicas here, written only by their home. */
    async isReplica(object: ObjectType, snapshot: Snapshot, scope: string): Promise<boolean> {
        // originate the inherited rows of a scope the store serves, such as a home's account's
        const isOriginated =
            object.inherited !== undefined && this.partitions.slice(1).includes(scope);

        return (
            !isOriginated &&
            object.storage === "durable" &&
            (this.replicates(object) ||
                (await sync.Replica.isReplicated(snapshot, scope, object.table)))
        );
    }

    /** Run a mutation another in-process store sent once, as its principal, answering results. */
    async receive(mutation: sync.Mutation, as: Subject): Promise<readonly unknown[]> {
        // pass changes of replicated rows to their home, else run them here under the sent request
        const forwarded = await this.#forward(mutation, this.principal);

        return (
            forwarded ??
            this.#mutate(AccessContext.of(Caller.of(as), this.clock.now()), mutation, {})
        );
    }

    /** Run a follower's mutation of the served rows as the caller, answering its results. */
    async receiveFrom(
        transaction: sync.Mutation,
        caller: AccessContext,
    ): Promise<readonly unknown[]> {
        // pass an installation's change of replicated rows toward their home
        if (principal.installation.is(caller.subject) && this.#isForwarded(transaction)) {
            return (await this.#readHomeAll(transaction)).receive(transaction, caller.subject);
        }
        // run the served rows' change as the caller, refusing anyone else's change of replicated rows
        else {
            this.#requireServed(transaction);

            return this.#mutate(caller, transaction, {});
        }
    }

    /** Report whether every call of a mutation changes rows replicated over the uplink. */
    #isForwarded(transaction: { readonly calls: readonly sync.Call[] }): boolean {
        return (
            this.subscriber !== undefined &&
            transaction.calls.every((call) => this.#isUplinked(call))
        );
    }

    /** Refuse a forwarded call outside the caller's grants, as its home checks only the subject. */
    async #requireForwardable(caller: AccessContext, call: sync.Call): Promise<void> {
        // check a delegated caller alone, in the scope of the call's replicated rows
        if (caller.grants.length === 0) {
            return;
        }
        const object = present(
            ObjectType.find(this.replicated, call),
            `the type of ${call.method}`,
        );
        const scope = object.readScope(call.method, call.input);
        const authorization = await this.authorize(
            Transaction.reading(this.database),
            scope,
            caller,
        );

        // require the grants to give the call
        const id = typeof call.input["id"] === "string" ? call.input["id"] : undefined;
        authorization.requireGranted({
            object,
            name: call.method,
            ...(id === undefined ? {} : { id }),
        });
    }

    /** Report whether an unhandled custom query of a replicated type runs at its rows' home. */
    #isAskedAtHome(object: ObjectType, method: Method): boolean {
        return (
            method.handler === undefined &&
            !method.mutates &&
            this.replicas.some((replica) => replica.same(object)) &&
            !this.objects.some((served) => served.same(object))
        );
    }

    /** Answer a custom query at its rows' home, as the caller's subject. */
    async #askHome(
        object: ObjectType,
        name: string,
        input: JsonObject,
        scope: string,
        caller: AccessContext,
    ): Promise<unknown> {
        // find the home of the call's rows, refusing a call nothing here answers
        const call = {
            type: object.typeReference,
            method: name,
            input,
            release: object.package.version,
        };
        const home = await this.#readHome(call);
        if (home === undefined) {
            throw ObjectError.of(
                "HOME_HANDLER_MISSING",
                `nothing here handles ${object.name}.${name} at the rows' home`,
            );
        }

        return home.call(call, scope, caller.subject);
    }

    /** Report whether a call's type's rows replicate over the uplink instead of living here. */
    #isUplinked(call: sync.Call): boolean {
        const object = ObjectType.find(this.replicated, call);

        return object !== undefined && !this.objects.some((served) => served.same(object));
    }

    /** Find the one publisher receiving a replicated rows mutation, refusing one spanning homes. */
    async #readHomeAll(transaction: sync.Mutation): Promise<sync.Uplink> {
        // require one home for every call, refusing calls nothing here receives
        const homes = await Promise.all(transaction.calls.map((call) => this.#readHome(call)));
        const [home] = homes;
        if (homes.includes(undefined)) {
            throw ObjectError.of(
                "REPLICA_RECEIVER_MISSING",
                "nothing here receives changes of the replicated rows",
            );
        } else if (home === undefined || homes.some((each) => each !== home)) {
            throw ObjectError.of(
                "HOMES_MIXED",
                "a mutation of replicated rows must change rows of one home",
            );
        }

        return home;
    }

    /** Refuse a mutation changing rows this store only replicates, since their home writes them. */
    #requireServed(transaction: { readonly calls: readonly sync.Call[] }): void {
        // find each call's type that no served object has
        const replicas = transaction.calls
            .filter((call) => ObjectType.find(this.objects, call) === undefined)
            .map((call) => call.type.type);
        if (replicas.length === 0) {
            return;
        }

        throw ObjectError.of(
            "REPLICA_WRITE",
            `the rows ${[...new Set(replicas)].join(", ")} are replicas here: change them at their home`,
        );
    }

    /** Find the publisher receiving a change of replicated rows. */
    async #readHome(call: sync.Call): Promise<sync.Uplink | undefined> {
        // find the replica of the call's table in its scope whose publisher receives changes
        const object = ObjectType.find(this.replicated, call);
        const scope = object?.readScope(call.method, call.input);
        const entries = await this.sink.followed();
        const home = entries.find(({ subscription, publisher }) => {
            const replica = this.source.buildReplica(subscription);

            return (
                ObjectStore.#isUplink(publisher) &&
                replica.scope === scope &&
                object !== undefined &&
                replica.tables.includes(object.table)
            );
        });

        // fall back to the subscriber's uplink
        const publisher = home?.publisher;

        return publisher !== undefined && ObjectStore.#isUplink(publisher)
            ? publisher
            : this.subscriber?.uplink;
    }

    /** Pass a mutation of replicated rows to their one home once per request, answering results. */
    async #forward(
        mutation: { readonly id: string; readonly calls: readonly sync.Call[] },
        signer: Subject,
    ): Promise<readonly unknown[] | undefined> {
        // leave a mutation of rows the database stores itself, and refuse one mixing both
        const view = Snapshot.live(this.database);
        const replicated = await Promise.all(
            mutation.calls.map(async (call) => {
                const object = ObjectType.find(this.replicated, call);

                return (
                    object !== undefined &&
                    (await this.isReplica(object, view, object.readScope(call.method, call.input)))
                );
            }),
        );
        if (replicated.every((isReplicated) => !isReplicated)) {
            return undefined;
        } else if (replicated.some((isReplicated) => !isReplicated)) {
            throw ObjectError.of(
                "HOMES_MIXED",
                "a mutation must change rows of one home, local or replicated",
            );
        }

        // pass it to the publisher the replicas follow, signed as the given principal
        const sent = sync.Mutation.parse(mutation);

        return (await this.#readHomeAll(sent)).receive(sent, signer);
    }

    /** List what the served objects need: the store's controllers, then each type's. */
    control(): Control {
        const syncing = this.syncing();
        const objects = this.#objectControl(() => true);

        return {
            controllers: [
                ...this.#storeControllers(),
                ...syncing.controllers,
                ...objects.controllers,
            ],
            cursors: [...syncing.cursors, ...objects.cursors],
        };
    }

    /** List the cursor and controller syncing the store's replicas. */
    syncing(): Control {
        const { subscriber } = this;

        return subscriber === undefined
            ? { controllers: [], cursors: [] }
            : {
                  controllers: [this.sink.controller(subscriber)],
                  cursors: [this.sink.cursor(subscriber)],
              };
    }

    /** List the controllers and projections of the types a filter selects by their package's id. */
    #objectControl(isSelected: (packageId: PackageId) => boolean): Control {
        const selected = [...this.objects, ...this.#catalog.provisioned].filter((object) =>
            isSelected(object.package.id),
        );

        return {
            controllers: selected.flatMap((object) =>
                [...object.controllers, ...object.traitControllers()].map((declared) =>
                    ObjectController.control(this, object, declared),
                ),
            ),
            cursors: selected.flatMap((object) =>
                object.projections.map((declared) =>
                    ObjectController.project(this, object, declared),
                ),
            ),
        };
    }

    /** List the store's controllers over its own tables, from blobs to its branch. */
    #storeControllers(): Controller[] {
        const isSettled = this.objects.some((object) =>
            Object.values(object.methods).some((method) => Method.settles(method)),
        );

        return [
            ...this.blobs.controllers(),
            ...(this.#journal === undefined
                ? []
                : [this.#journal.controller(), ...this.#journal.events.controllers()]),
            ...(this.claims?.home === undefined
                ? []
                : [Reservation.controller(this, this.claims.object, this.claims.home)]),
            ...(isSettled ? [Settlement.controller(this)] : []),
            ...(this.branch === undefined
                ? []
                : [
                      ObjectController.control(
                          this,
                          this.branch.object,
                          this.branch.controller(this.objects),
                      ),
                  ]),
        ];
    }

    /** Share one grant reader per scope and page position. */
    reader(access: Access, position: LogPosition): TupleReader {
        // reuse a known reader
        const key = JSON.stringify([access.scope, position.epoch, position.sequence]);
        const known = this.#readers.get(key);
        if (known) {
            return known;
        }

        // start one and drop the oldest
        const reader = this.authorizer.reader(Snapshot.live(this.database), access.scopes);
        this.#readers.set(key, reader);
        if (this.#readers.size > SHARED_READERS) {
            const [oldest] = this.#readers.keys();
            if (oldest !== undefined) {
                this.#readers.delete(oldest);
            }
        }

        return reader;
    }

    /** Resolve a caller's access in a scope at the store's time, inside its principal's scopes. */
    async authorize(
        transaction: Transaction,
        scope: string,
        caller: AccessContext,
        options: AuthorizeOptions = {},
    ): Promise<Authorization> {
        // place the caller inside the scopes given, else those its principal's row places it
        const { request, links } = options;
        const within =
            options.within === undefined
                ? await this.source.within(caller.subject, transaction.database)
                : options.within.map((inside) => ({ ...inside, relation: CONTAINED }));

        // bind the caller with the scopes it sits inside, at the work's time
        const now = options.now ?? this.clock.now();
        const bind = (): AccessContext => ({
            ...caller,
            groups: [...caller.groups, ...within],
            now,
            ...(request === undefined ? {} : { request }),
        });

        // resolve its access
        const access = await this.authorizer.resolve(transaction.view(), scope, bind(), links);

        return new Authorization(this.authorizer, transaction, bind, this.environment, {
            resolved: access,
        });
    }

    /** Authorize a caller for work this store serves: an unmoved scope, in its partition. */
    async authorizeHere(
        transaction: Transaction,
        scope: string,
        caller: AccessContext,
        options: AuthorizeHereOptions = {},
    ): Promise<Authorization> {
        // resolve its access, then require a scope this store serves for the calls
        const { calls = [], ...authorizing } = options;
        const authorization = await this.authorize(transaction, scope, caller, authorizing);
        this.#requireHere(authorization, calls);

        return authorization;
    }

    /** Authorize a nested call once per run, scope and caller while current, requiring it here. */
    async #authorizeCall(
        transaction: Transaction,
        scope: string,
        caller: AccessContext,
        calls: readonly { readonly object: ObjectType; readonly name: string }[],
        options: {
            readonly now?: number;
            readonly authorizations: Map<string, Authorization>;
        },
    ): Promise<Authorization> {
        // reuse the scope and caller's authorization while nothing it was resolved from changed
        const { now, authorizations } = options;
        const key = `${scope} ${AccessContext.key(caller)}`;
        let authorization = authorizations.get(key);
        if (authorization?.isCurrent !== true) {
            authorization = await this.authorize(
                transaction,
                scope,
                caller,
                now === undefined
                    ? { links: await Scope.chain(transaction.view(), scope) }
                    : { now },
            );
            authorizations.set(key, authorization);
        }

        // require a scope this store serves for the call
        this.#requireHere(authorization, calls);

        return authorization;
    }

    /** Require a scope this store serves, moved only for moving calls, and the calls' methods. */
    #requireHere(
        authorization: Authorization,
        calls: readonly { readonly object: ObjectType; readonly name: string }[],
    ): void {
        authorization.requireHere(
            calls.map(({ object, name }) => object.permission(object.method(name).permission)),
            this.#partitionOf(),
            calls.length === 0 ? undefined : (this.space ?? this.principal.id),
        );
        authorization.requireMethods(calls);
    }

    /** Enter a scope to read or write, requiring it and waiting for the writes the caller saw. */
    async enter(context: RequestContext, scope: string, mutates: boolean): Promise<void> {
        // follow the row standing for the caller where it is missing
        await this.source.stand(context.access(scope).subject, context.signal);

        // enter the requested scope, or any without one, at once without watermarks to wait for
        const requested = context.scope;
        const { watermarks } = context.bookmark;
        const isRequested = requested === undefined || requested === scope;
        if (isRequested && watermarks.length === 0) {
            return;
        }

        // refuse a scope outside the request's scope, letting reads reach its ancestors
        if (!isRequested) {
            const rows = await Scope.read(Snapshot.live(this.database), [scope, requested]);
            const row = rows.find((each) => each.scope === scope);
            const outer = rows.find((each) => each.scope === requested);
            const isInside = row !== undefined && Scope.within(row, requested);
            const isReplicated = !mutates && outer !== undefined && Scope.within(outer, scope);
            if (!isInside && !isReplicated) {
                throw ObjectError.of(
                    "SCOPE_OUTSIDE_REQUEST",
                    `scope ${scope} is outside the request's scope`,
                );
            }
        }

        // wait for each of the caller's watermarks this store has a history of, read your writes
        const known = await Promise.all(
            watermarks.map(async (entry) =>
                (await this.history(entry)).kind === "unknown" ? [] : [entry],
            ),
        );
        await this.#waitWatermarks(context, known.flat());
    }

    /** Wait until a scope's chain replicas have their homes' commits, refusing other homes. */
    async #waitHomes(
        context: RequestContext,
        scope: string,
        homes: readonly Watermark[],
    ): Promise<void> {
        // require homes along the scope's chain
        const chain = await this.#chainIds(scope);
        const outside = homes.find((home) => !chain.has(home.scope));
        if (outside !== undefined) {
            throw ObjectError.of(
                "SCOPE_OUTSIDE_CHAIN",
                `${outside.scope} is outside the scope chain of ${scope}, so no replica here follows it`,
            );
        }

        // wait for each home's commit
        await this.#waitWatermarks(context, homes);
    }

    /** Read a scope and the scopes enclosing it, by identifier. */
    async #chainIds(scope: string): Promise<Set<string>> {
        const chain = await Scope.chain(Snapshot.live(this.database), scope);

        return new Set([scope, ...chain.map((link) => link.object.id)]);
    }

    /** Wait until the scope's log or its replicas have each watermark, in a bounded wait each. */
    async #waitWatermarks(
        context: RequestContext,
        watermarks: readonly Watermark[],
    ): Promise<void> {
        for (const watermark of watermarks) {
            const signal = AbortSignal.any([
                context.signal,
                this.clock.timeout(BOOKMARK_TIMEOUT_MILLISECONDS),
            ]);
            if (!(await this.wait(watermark, signal))) {
                // refuse a scope still without a home or replica here, else the late position
                const history = await this.history(watermark);
                throw history.kind === "unknown"
                    ? ObjectError.of(
                          "HOME_OR_REPLICA_MISSING",
                          `${watermark.scope} has no home or replica here`,
                      )
                    : ObjectError.of(
                          "REPLICA_BEHIND",
                          `${watermark.scope} has not reached the required sequence`,
                      );
            }
        }
    }

    /** Read the position of the latest change in a scope's chain, which its readers see. */
    async watermark(scope: string): Promise<Watermark> {
        const chain = await Scope.chain(Snapshot.live(this.database), scope);
        const scopes = chain.map((link) => link.object.id);

        return { scope, ...(await this.database.log.changed("every", scopes)) };
    }

    /** Decide where a watermark's history is here: local log, an older one, replicas, or none. */
    async history(watermark: Watermark): Promise<WatermarkHistory> {
        // take the local log's current epoch for a scope the store serves, refusing an earlier one
        const { database } = this;
        const head = await database.log.position();
        const isHome = await this.authorizer.isHome(Snapshot.live(database), watermark.scope);
        const isServed = isHome || watermark.scope === this.scope;
        if (isServed && watermark.log === head.log) {
            return { kind: watermark.epoch === head.epoch ? "local" : "stale" };
        }
        // refuse another log for a scope homed here, which a move's lineage would continue
        else if (isHome) {
            return { kind: "stale" };
        }

        // take the replicas including the scope and the followed ones reading it
        const including = await sync.Replica.including(database, watermark.scope);
        const followed = (await this.sink.followed())
            .filter(({ replica }) => replica.holds(watermark.scope))
            .map((entry) => entry.subscription);

        return including.length === 0 && followed.length === 0
            ? { kind: "unknown" }
            : { kind: "replicated", including, followed };
    }

    /** Wait until the store's log or its replicas of another scope's log have a watermark. */
    async wait(watermark: Watermark, signal: AbortSignal): Promise<boolean> {
        const history = await this.history(watermark);

        // wait on the store's log for a watermark it wrote in its current epoch
        if (history.kind === "local") {
            return await this.database.log.wait(watermark.sequence, signal);
        }
        // refuse a watermark of a history this store no longer has
        else if (history.kind === "stale") {
            throw DatabaseError.of(
                "EPOCH_STALE",
                `${watermark.scope} no longer has the watermark's history`,
            );
        }
        // wait until the followed replicas reading the scope reach it, as only a followed replica moves
        else if (history.kind === "replicated" && history.followed.length > 0) {
            return await sync.Replica.waitAt(this.database, history.followed, watermark, signal);
        }
        // wait until the replicas including the scope reach it, for a store following none yet
        else {
            return await sync.Replica.wait(this.database, watermark.scope, watermark, signal);
        }
    }

    /** Read the server context serving a type inside the store, absent for a type it serves. */
    readServer(object: ObjectType): ServerContext | undefined {
        return this.#catalog.installed.get(ObjectTypeReference.key(object.typeReference))?.server;
    }

    /** Read the package whose code implements a served type, refusing an unserved one. */
    implementer(object: ObjectType): PackageId {
        const served = this.#catalog.installed.get(ObjectTypeReference.key(object.typeReference));
        if (served === undefined) {
            throw new TypeError(`no installation serves ${object.name} in this store`);
        }

        return served.package.id;
    }

    /** Read the package implementing a type here, as its calls' audit records it. */
    #implementing(object: ObjectType): Package {
        // take the installed implementer, or the declaring package in a store serving no installations
        const served = this.#catalog.installed.get(ObjectTypeReference.key(object.typeReference));

        return served?.package ?? object.package;
    }

    /** Read the memory store with the ephemeral objects, failing without one. */
    requireEphemeral(): EphemeralStorage {
        if (this.ephemeral === undefined) {
            throw ObjectError.of(
                "EPHEMERAL_STORAGE_MISSING",
                "this store does not serve ephemeral objects",
            );
        }

        return this.ephemeral;
    }

    /** Execute ephemeral calls of a request in one memory transaction as their writer. */
    async #mutateEphemeral(
        calls: readonly { object: ObjectType; name: string; input: JsonObject }[],
        scope: string,
        caller: AccessContext,
        requestId: string,
        writerId: string | undefined,
    ): Promise<unknown[]> {
        // require the writer of the rows
        if (writerId === undefined) {
            throw ObjectError.of("WRITER_REQUIRED", "only a writer can write ephemeral objects");
        }

        // allow the caller, then execute the calls in order over a transaction of the writer's rows, committed in its write
        const authorization = await this.authorizeHere(
            Transaction.reading(this.database),
            scope,
            caller,
        );

        return this.requireEphemeral().write(
            EphemeralStorage.writerKey(caller, writerId),
            async (writing) => {
                // run the calls over a transaction of the writer's rows
                const transaction = await Transaction.open(writing);
                const executed: unknown[] = [];
                for (const call of calls) {
                    executed.push(
                        await this.#execute(
                            transaction,
                            authorization,
                            call.object,
                            call.name,
                            call.input,
                            caller,
                            { writerId, requestId, authorizations: new Map() },
                        ),
                    );
                }

                // commit them in the writer's write
                await transaction.commit();

                return executed;
            },
        );
    }

    /** Refuse calls past their methods' rates before their work, saying when one is due. */
    async #limit(
        calls: readonly {
            readonly object: ObjectType;
            readonly name: string;
            readonly input: unknown;
        }[],
        scope: string,
        caller: AccessContext,
    ): Promise<void> {
        // take a token of each limited method's rate for the call's key
        const now = this.clock.now();
        for (const { object, name, input } of calls) {
            const limits = object.limits[name];
            const rate = limits?.rate;
            if (rate !== undefined) {
                const key = Limits.key(limits?.key, {
                    scope,
                    caller: caller.subject,
                    input: schema.json().parse(input),
                });
                const label = `method:${object.package.name}/${object.name}.${name}`;
                const retryAfter = await Limits.take(this.database, label, key, rate, now);
                if (retryAfter !== undefined) {
                    throw ObjectError.of(
                        "RATE_LIMITED",
                        `${object.name}.${name} is limited to ${rate.count} calls per ${Duration.milliseconds(rate.every)} ms`,
                        { data: { retryAfter } },
                    );
                }
            }
        }
    }

    /** Resolve a recorded call against the served types. */
    #resolve(
        entry: sync.Call,
        mutates: boolean,
        site: CallSite,
    ): ReturnType<typeof ObjectType.resolve> {
        return ObjectType.resolve(
            this.#callable(entry, mutates, this.replicas),
            entry,
            mutates,
            site,
        );
    }

    /** List the types a call may resolve to: the served ones, and the replicas for a query. */
    #callable(
        entry: sync.Call,
        mutates: boolean,
        refused: readonly ObjectType[],
    ): readonly ObjectType[] {
        // refuse a change of a type only replicated here, which its home changes
        const isServed = ObjectType.find(this.objects, entry) !== undefined;
        const replicated = ObjectType.find(refused, entry);
        if (mutates && !isServed && replicated !== undefined) {
            throw ObjectError.of(
                "REPLICA_WRITE",
                `${replicated.name} rows are replicas here: send changes to their home`,
            );
        }

        return mutates ? this.objects : [...this.objects, ...this.replicas];
    }

    /** Read a procedure call's scope: a method's, else the decided target's or context's. */
    #procedureScope(
        context: ProcedureContext<RequestContext>,
        routed: readonly ObjectType[],
    ): string | undefined {
        // read the scope field of the object the package routes the call to
        const object = routed.find((each) => each.key === context.path[0]);
        const input =
            context.input === undefined ? undefined : PROCEDURE_INPUT.parse(context.input);
        const field = object?.route.field;
        const named = field === undefined ? undefined : input?.[field];

        return typeof named === "string"
            ? named
            : (context.request.target?.scope ?? context.request.scope);
    }

    /** Build one method context with its permitted target at the given revision. */
    async #context(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: AccessContext,
        options: Pick<CallOptions, "as" | "expansion" | "requestId" | "parentId"> = {},
    ): Promise<{
        context: MethodContext;
        execution: StoreExecution;
        method: Method;
        scope: string;
        targetId: string | undefined;
    }> {
        // refuse row changes to replicas from their home, leaving access writes to the authorization
        const method = object.method(name);
        const scope = object.readScope(name, input);
        const isRowChange = method.mutates && !ACCESS_KINDS.has(method.kind);
        if (isRowChange && (await this.isReplica(object, transaction.view(), scope))) {
            throw ObjectError.of(
                "REPLICA_WRITE",
                `${object.name} rows of ${scope} are replicas here: send changes to their home`,
            );
        }

        // parse the input and build the call in its scope as the installation serving its type
        const { database } = transaction;
        const { fields, id, revision, at, branch } = ObjectStore.#fields(object, name, input);
        authorization.requireScopeOf(object);
        const snapshot = await this.#view(database, authorization, scope, at, branch);
        const view = snapshot ?? transaction.view();
        const resolved = await object.resolve(fields, view);
        const targetId =
            id === undefined
                ? undefined
                : await ObjectStore.#identifier(object, schema.string().parse(id), view);
        const execution: StoreExecution = {
            kind: "store",
            store: this,
            authorization,
            transaction,
            view: snapshot ?? transaction,
            callId: Identifier.create("call", this.environment),
            ...(options.parentId === undefined ? {} : { parentId: options.parentId }),
        };
        const { now } = authorization.access.context;
        const acting =
            options.as === undefined
                ? { ...caller, now }
                : AccessContext.of(await Grant.chain(transaction.view(), options.as, now), now);
        const context: MethodContext = new MethodContext({
            object,
            name,
            method,
            scope,
            chain: authorization.chain,
            input: resolved,
            id: targetId,
            target: undefined,
            prepared: undefined,
            caller: acting,
            ...(options.expansion === undefined ? {} : { expansion: options.expansion }),
            ...(options.requestId === undefined ? {} : { requestId: options.requestId }),
            location: execution,
            relations: this.relations,
            replicas: this.replicas,
        });

        // refuse writing a predefined row, then load the permitted target at the given revision
        if (method.mutates && targetId !== undefined) {
            object.refusePredefined(targetId);
        }
        const target = await ObjectStore.#target(context, execution, targetId, snapshot).catch(
            (error: unknown) => {
                throw ObjectStore.#addressed(object, id, targetId, error);
            },
        );
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw ObjectError.of("REVISION_STALE", `${object.name} revision has changed`);
        }

        // refuse other methods on a trashed object, but the package's own
        const isTrashed =
            object.lifecycle.recoverable !== undefined &&
            target !== undefined &&
            target["deletedAt"] !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.visibility !== "package") {
            throw ObjectError.of("OBJECT_TRASHED", `${object.name} is in the trash`);
        }

        return {
            context: target === undefined ? context : context.with({ target }),
            execution,
            method,
            scope,
            targetId,
        };
    }

    /** Parse a call's input by schema, splitting off routing fields and the requested view. */
    static #fields(
        object: ObjectType,
        name: string,
        input: JsonObject,
    ): {
        readonly fields: JsonObject;
        readonly id: JsonValue | undefined;
        readonly revision: JsonValue | undefined;
        readonly at?: JsonValue | undefined;
        readonly branch?: JsonValue | undefined;
    } {
        // refuse input the method's schema rejects, as its procedure does
        const method = object.method(name);
        const parsed = object.input(name).safeParse(input);
        if (!parsed.success) {
            throw refuseInput(input, parsed.error.issues);
        }

        // split off the identifier, the revision, the scope and the view of a read
        const field = object.route.field;
        const isViewed = method.kind === "get" || method.kind === "list";
        const { id, revision, ...named } = parsed.data;
        const { at, branch, ...viewed } = named;
        const fields = Object.fromEntries(
            Object.entries(isViewed ? viewed : named).filter(([property]) => property !== field),
        );

        return isViewed ? { fields, id, revision, at, branch } : { fields, id, revision };
    }

    /** Name a hidden or missing target as the caller addressed it. */
    static #addressed(
        object: ObjectType,
        addressed: unknown,
        targetId: string | undefined,
        error: unknown,
    ): unknown {
        // keep the concealed denial as the cause, which callers read refusals from
        const isKeyed = typeof addressed === "string" && addressed !== targetId;
        const missing = CallError.of(error);

        return isKeyed && missing?.code === "NOT_FOUND"
            ? CallError.http("NOT_FOUND", {
                  message: `${object.name} ${addressed} not found`,
                  cause: missing.cause,
              })
            : error;
    }

    /** Read the identifier of the object a call names by its identifier or its key. */
    static async #identifier(
        object: ObjectType,
        value: string,
        view: Pick<Snapshot, "read">,
    ): Promise<string> {
        // take an identifier as it is
        const column = object.identify(value);
        if (column === "id") {
            return value;
        }

        // read the identifier of the object holding the key, refusing a key nobody holds
        const row = await view.read(object.table).findFirst({ where: { [column]: value } });
        if (row === undefined) {
            throw CallError.http("NOT_FOUND", { message: `${object.name} ${value} not found` });
        }

        return schema.string().parse(row["id"]);
    }

    /** Load a call's permitted target through its requested view, absent without a target. */
    static async #target(
        context: MethodContext,
        execution: StoreExecution,
        targetId: string | undefined,
        snapshot: Snapshot | undefined,
    ): Promise<Row | undefined> {
        // skip a call without a target
        const { method } = context;
        if (!Method.loads(method) || targetId === undefined) {
            return undefined;
        }

        // read the live target, or the target in the view
        if (snapshot === undefined) {
            return execution.authorization.read(context, targetId, execution);
        } else {
            return execution.authorization.readIn(context, targetId, snapshot);
        }
    }

    /** Build a read's view: the database at a position, a branch, or none for the main line. */
    async #view(
        database: DatabaseConnection,
        authorization: Authorization,
        scope: string,
        at: unknown,
        branch: unknown,
    ): Promise<Snapshot | undefined> {
        // read the live main line, or the database at the position
        if (at === undefined && branch === undefined) {
            return undefined;
        }
        const base =
            at === undefined ? Snapshot.live(database) : database.log.at(LogPosition.parse(at));
        if (branch === undefined) {
            return base;
        }

        // require the served branch types and the caller's read of the branch
        const types = this.branch;
        if (types === undefined) {
            throw ObjectError.of("BRANCHES_MISSING", "this store does not serve branches");
        }
        const id = schema.string().parse(branch);
        const row = await Snapshot.live(database)
            .read(types.object.table)
            .findFirst({ where: { id } });
        const permission = types.object.permission("read");
        const allowed =
            row === undefined
                ? []
                : (await authorization.filter(types.object, permission, [row], { scope })).rows;
        if (allowed.length === 0) {
            throw ObjectError.of("BRANCH_NOT_FOUND", `branch ${id} not found`);
        }

        return base.layer(await new Branch(types, base, id).overlay(base, this.objects));
    }

    /** Read a method about to run, refusing one that declares prepared work it never prepares. */
    static #preparing(object: ObjectType, name: string): Method {
        const method = object.method(name);
        if (method.prepared !== undefined && method.prepare === undefined) {
            throw new TypeError(
                `${object.name}.${name} declares prepared work but has no prepare phase`,
            );
        }

        return method;
    }

    /** Prepare a request's external work outside the transaction, skipping executed mutations. */
    async #prepare(
        calls: readonly { object: ObjectType; name: string; input: JsonObject }[],
        scope: string,
        caller: AccessContext,
        requestId: string,
        request?: RequestIdentity,
    ): Promise<Prepared[]> {
        // skip when nothing prepares or the request executed
        const methods = calls.map((call) => ObjectStore.#preparing(call.object, call.name));
        if (
            methods.every((method) => method.prepare === undefined) ||
            (request !== undefined && (await this.journal.outcome(request)) !== undefined)
        ) {
            return calls.map(() => undefined);
        }

        // build and authorize each preparing call as the caller over a mutation that never commits
        const reading = Transaction.reading(this.database);
        const authorization = await this.authorizeHere(
            reading,
            scope,
            caller,
            request === undefined ? { calls } : { request: request.requestId, calls },
        );

        return this.#prepareEach(
            requestId,
            zip(calls, methods).map(([entry, method]) => {
                if (method.prepare === undefined) {
                    return undefined;
                }

                return async () => {
                    // build and authorize the call, hiding a denial in a scope the caller cannot see
                    const { context, execution } = await this.#context(
                        reading,
                        authorization,
                        entry.object,
                        entry.name,
                        entry.input,
                        caller,
                        { requestId },
                    ).catch(async (error: unknown) => {
                        throw await this.#conceal(authorization, caller, undefined, error);
                    });
                    try {
                        await method.authorize?.(context, execution);
                    } catch (error) {
                        throw await this.#conceal(authorization, caller, context.target, error);
                    }

                    return context;
                };
            }),
        );
    }

    /** Prepare a request's calls in order, reserving settlements first, rolling back on failure. */
    async #prepareEach(
        requestId: string,
        builds: readonly ((() => Promise<MethodContext>) | undefined)[],
    ): Promise<Prepared[]> {
        // prepare in order, with the reserved settlements
        const prepared: Prepared[] = [];
        const settlements: (string | undefined)[] = [];
        try {
            for (const [position, build] of builds.entries()) {
                // skip a call without external work
                if (build === undefined) {
                    prepared.push(undefined);
                    continue;
                }

                // reserve under the key of the call's place in its request before preparing
                const reserved = await this.#reserve(await build(), `${requestId}/${position}`);
                settlements.push(reserved.settlement);
                prepared.push(await this.#prepareOne(reserved.context, reserved.settlement));
            }
        } catch (error) {
            // roll back prepared work
            await this.#settle(settlements);
            throw error;
        }

        return prepared;
    }

    /** Key a call's external work, stable across retries, and reserve a settlement if needed. */
    async #reserve(
        context: MethodContext,
        key: string,
    ): Promise<{ readonly context: MethodContext; readonly settlement: string | undefined }> {
        const keyed = context.with({ idempotencyKey: key });

        return {
            context: keyed,
            settlement: Method.settles(context.method)
                ? await Settlement.reserve(this.database, keyed, key)
                : undefined,
        };
    }

    /** Run a reserved call's prepare phase, parsing its value and recording it for settlement. */
    async #prepareOne(context: MethodContext, settlement: string | undefined): Promise<Prepared> {
        // run the prepare phase the method declares
        const { method } = context;
        const declared = method.prepared;
        if (method.prepare === undefined || declared === undefined) {
            throw new TypeError(`${context.object.name}.${context.name} declares no prepared work`);
        }
        const value: unknown = await method.prepare(context.with({ run: this.#alone(context) }));

        // store the value as a settlement reads it back
        if (settlement === undefined) {
            return { context: context, value: declared.parse(value), settlement };
        }
        const recorded = schema.json().parse(value);
        await Settlement.record(this.database, settlement, context, recorded);

        return { context: context, value: declared.parse(recorded), settlement };
    }

    /** Mark the settling calls of a transaction committed. */
    async #commit(transaction: DatabaseConnection, prepared: readonly Prepared[]): Promise<void> {
        for (const work of prepared) {
            if (work?.settlement !== undefined) {
                await Settlement.commit(
                    transaction,
                    work.settlement,
                    work.context,
                    this.clock.now(),
                );
            }
        }
    }

    /** Settle each reserved settlement as recorded, skipping ones claimed elsewhere. */
    async #settle(settlements: readonly (string | undefined)[]): Promise<void> {
        for (const id of settlements) {
            if (id === undefined) {
                continue;
            }
            try {
                const now = this.clock.now();
                const claimed = await Settlement.claim(this.database, id, now);
                if (claimed !== undefined) {
                    await Settlement.settle(this, claimed, now);
                }
            } catch (error) {
                // report a failure
                this.#report(error);
            }
        }
    }

    /** Execute, redact and audit one call in a separate span. */
    #execute(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: AccessContext,
        options: CallOptions,
    ): Promise<unknown> {
        const attributes = { "destack.object.type": object.name, "destack.object.method": name };

        return this.#instruments.span("object.call", attributes, () =>
            this.#perform(transaction, authorization, object, name, input, caller, options).catch(
                (error: unknown) => {
                    throw object.conflict(error);
                },
            ),
        );
    }

    /** Hide a remote caller's refusal in a scope it cannot see, unless it reads the target. */
    #conceal(
        authorization: Authorization,
        caller: AccessContext,
        target: unknown,
        error: unknown,
    ): Promise<unknown> {
        return AccessContext.isRemote(caller) && target === undefined
            ? authorization.concealDenial(error, this.objects)
            : Promise.resolve(error);
    }

    /** Execute, redact and audit one call. */
    async #perform(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: AccessContext,
        options: CallOptions,
    ): Promise<unknown> {
        // build the call on the served type, a creation identified, hiding a refused target the caller cannot see
        const { prepared, from, writerId, requestId } = options;
        const writes = transaction.changes.writes("every");
        const { context, execution, method, scope, targetId } = await this.#context(
            transaction,
            authorization,
            this.served(object),
            name,
            ObjectStore.#named(transaction, object, name, input, requestId),
            caller,
            options,
        ).catch(async (error: unknown) => {
            throw await this.#conceal(authorization, caller, undefined, error);
        });
        this.#instruments.span.current()?.setAttributes({
            "destack.scope": scope,
            ...(targetId === undefined ? {} : { "destack.object.id": targetId }),
        });

        // refuse a listing of a scope the caller cannot see, as the scope's absence
        if (method.kind === "list") {
            await authorization.requireSeen(this.objects);
        }

        // run invoked calls in the call's transaction and scope
        const called = context.with({
            ...ObjectStore.#preparedFields(prepared),
            ...(writerId === undefined ? {} : { writerId }),
            run: this.#invoker(transaction, authorization, context, execution.callId, options),
        });

        // authorize the call here unless prepared, or asked at its rows' home for a caller acting alone, which the home authorizes
        let executed: unknown;
        const isAskedAtHome = this.#isAskedAtHome(object, method);
        try {
            if (prepared === undefined && (!isAskedAtHome || caller.actors.length > 0)) {
                await method.authorize?.(called, execution);
            }

            // ask a query at its rows' home, else execute it here
            executed = isAskedAtHome
                ? await this.#askHome(object, name, input, scope, caller)
                : await method.execute(called, execution);
        }
        // hide a denial the caller cannot see
        catch (error) {
            throw await this.#conceal(authorization, caller, called.target, error);
        }

        // present the result, track changes and record the call
        const presented = await this.#present(
            transaction,
            authorization,
            object,
            context,
            executed,
        );
        const id = targetId ?? Method.resultId(executed);
        if (method.mutates && id !== undefined) {
            await ObjectStore.#track(execution, object, context.with({ id }), from);
        }
        this.#record(
            transaction,
            object,
            context,
            execution,
            id,
            presented,
            caller,
            options.request,
            options.component,
            transaction.changes.writes("every") > writes || prepared !== undefined,
        );

        return presented;
    }

    /** Run a phase's calls outside its mutation, each a separate request under a derived key. */
    #alone(parent: MethodContext): Run {
        // key each run by the phase's external work and its place among the phase's runs
        const key = present(
            parent.idempotencyKey,
            `the key of ${parent.object.name}.${parent.name}`,
        );
        let runs = 0;

        return async (entry, caller, mutates) => {
            // place the call in the phase's scope under the next key, then run or read it
            const placed = this.#place(entry, parent.scope);
            runs += 1;
            const id = await RequestId.derive(
                parent.now,
                `${key}/${runs}`,
                this.environment.crypto,
            );

            return mutates
                ? (await this.mutate(caller, { id, calls: [placed] }))[0]
                : this.query(caller, placed);
        };
    }

    /** Run a method's calls in its transaction and scope, authorized for its caller and request. */
    #invoker(
        transaction: Transaction,
        authorization: Authorization,
        parent: MethodContext,
        parentId: Identifier<"call">,
        options: CallOptions,
    ): Run {
        // pass the call's options down as the nested calls' parent, with the transaction's authorizations
        const { from, writerId, requestId, component } = options;
        const { authorizations } = options;
        const recorded: CallOptions = {
            ...(from === undefined ? {} : { from }),
            ...(writerId === undefined ? {} : { writerId }),
            ...(requestId === undefined ? {} : { requestId }),
            ...(component === undefined ? {} : { component }),
            parentId,
            authorizations,
        };

        return async (entry, caller, mutates, code) => {
            // run the nested call under the recorded options
            const result = await this.#runInside(
                transaction,
                entry,
                caller,
                mutates,
                code,
                { context: parent, authorization },
                recorded,
            );

            return result;
        };
    }

    /** Record a read that showed guarded personal values unmasked, of one object or a scope's. */
    async recordUnmask(read: {
        readonly scope: string;
        readonly caller: AccessContext;
        readonly object: ObjectType;
        readonly operation: string;
        readonly id?: string;
        readonly requestId?: string;
    }): Promise<void> {
        const { scope, caller, object, operation, id, requestId } = read;
        await this.audit(scope, caller, {
            package: this.#implementing(object),
            ...(requestId === undefined ? {} : { requestId }),
        }).record(
            undefined,
            objectUnmask,
            {
                target: { type: object.name, id: id ?? scope },
                details: { operation },
                outcome: { kind: "success" },
            },
            "access",
        );
    }

    /** Redact and encode a call's result: its value, its object page, or its object with texts. */
    async #present(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        context: MethodContext,
        executed: unknown,
    ): Promise<unknown> {
        const { method, scope } = context;
        const table = object.table;

        // answer a value as is
        if (method.result === "value") {
            return executed;
        }
        // answer a page's objects with the texts it includes
        else if (method.result === "page") {
            const { items, included, ...page } = LISTED_PAGE.parse(executed);
            const { [CHUNKS]: chunks, ...rest } = included ?? {};
            const redacted = await authorization.redact(object, items);
            if (redacted.isRevealing) {
                await this.recordUnmask({
                    scope,
                    caller: context.caller,
                    object,
                    operation: context.name,
                    ...(context.requestId === undefined ? {} : { requestId: context.requestId }),
                });
            }

            return {
                ...page,
                ...(included === undefined ? {} : { included: rest }),
                items: redacted.rows.map((row) => ({
                    ...table[TABLE].encode(row),
                    ...(chunks === undefined
                        ? {}
                        : Chunk.text(object, Chunk.included(chunks[IDENTIFIER.parse(row["id"])]))),
                })),
            };
        }
        // answer one object with its texts
        else {
            const redacted = await authorization.redact(object, [Row.parse(executed)]);
            const row = nth(redacted.rows, 0);
            const id = IDENTIFIER.parse(row["id"]);
            if (redacted.isRevealing) {
                await this.recordUnmask({
                    scope,
                    caller: context.caller,
                    object,
                    operation: context.name,
                    ...(context.requestId === undefined ? {} : { requestId: context.requestId }),
                    id,
                });
            }
            const texts =
                object.text.length === 0
                    ? undefined
                    : await Chunk.texts(transaction, object, scope, [id]);

            return { ...table[TABLE].encode(row), ...texts?.get(id) };
        }
    }

    /** Record a tracked object's change by a call, from the log position before its mutation. */
    static async #track(
        execution: StoreExecution,
        object: ObjectType,
        context: MethodContext,
        from: LogPosition | undefined,
    ): Promise<void> {
        // skip untracked objects
        const history = object.lifecycle.tracked;
        if (history === undefined) {
            return;
        }

        // record the row after the call against the target before it
        const id = present(context.id, "the tracked object");
        const after = await execution.transaction.read(object.table).findFirst({ where: { id } });
        if (after !== undefined) {
            const before = present(from, "the log position before a tracked call");
            await TrackedHistory.record(history, context, execution, context.target, after, before);
        }
    }

    /** Record a call: audit durable changes, and journal every call of a request for retries. */
    #record(
        transaction: Transaction,
        object: ObjectType,
        context: MethodContext,
        execution: StoreExecution,
        id: string | undefined,
        presented: unknown,
        caller: AccessContext,
        request: CallRequest | undefined,
        component: string | undefined,
        isEffective: boolean,
    ): void {
        // audit durable changes, a component's only once they take effect, and journal client requests
        const { method, scope, name } = context;
        const isAudited =
            method.mutates &&
            object.storage === "durable" &&
            (component === undefined || isEffective);
        const journaled = component === undefined ? request : undefined;
        const target = id === undefined ? { type: "scope", id: scope } : { type: object.name, id };
        const recorded: {
            target: typeof target;
            details: {};
            outcome: sync.Outcome;
            id: Identifier<"call">;
        } = {
            id: execution.callId,
            target,
            details: {},
            outcome: {
                kind: "success",
                ...(request === undefined ? {} : { value: schema.json().parse(presented ?? null) }),
            },
        };

        // add the call to the transaction's batch of recorded calls
        if (!isAudited && journaled === undefined) {
            return;
        }
        const recorder = this.audit(scope, caller, {
            package: this.#implementing(object),
            ...(component === undefined ? {} : { component }),
            ...(execution.parentId === undefined ? {} : { parentId: execution.parentId }),
        });
        transaction.batch(this.#calls, (stamp: string | undefined) => {
            const stamped =
                journaled === undefined || stamp === undefined
                    ? journaled
                    : { ...journaled, transaction: stamp };

            return recorder.ended(object.audit(name), recorded, "activity", isAudited, stamped);
        });
    }

    /** Throw a failed settlement. */
    static #rethrow(error: unknown): never {
        throw error;
    }

    /** Read where a resumed sync continues. */
    static #resumed(
        after: LogPosition | undefined,
        lastEventId: string | undefined,
    ): LogPosition | undefined {
        // parse the last event identifier
        if (lastEventId !== undefined) {
            const [log, epoch, sequence] = lastEventId.split("/");
            const position = LogPosition.safeParse({ log, epoch, sequence: Number(sequence) });
            if (!position.success) {
                throw ObjectError.of("EVENT_NOT_FOUND", `event ${lastEventId} not found`);
            }

            return position.data;
        }

        return after;
    }

    /** Give a prepared creation without an identifier a fresh one, for its prepared work. */
    static #identified(call: Expansion, environment: UuidEnvironment): Expansion {
        const method = call.object.method(call.name);
        const isUnnamed =
            method.kind === "create" &&
            method.prepare !== undefined &&
            call.input["id"] === undefined;

        return isUnnamed
            ? {
                  ...call,
                  input: { ...call.input, id: `${call.object.identity}-${Uuid.v7(environment)}` },
              }
            : call;
    }

    /** Identify a creation its caller leaves unnamed at the transaction's next position. */
    static #named(
        transaction: Transaction,
        object: ObjectType,
        name: string,
        input: JsonObject,
        requestId: string | undefined,
    ): JsonObject {
        const isUnnamed =
            object.method(name).kind === "create" &&
            input["id"] === undefined &&
            requestId !== undefined &&
            !transaction.isReading;

        return isUnnamed
            ? { ...input, id: object.deriveId(requestId, transaction.takePosition()) }
            : input;
    }

    /** Project a read's result onto the audit details its method declares. */
    static #detailsOf(
        details: schema.Object<Record<string, schema.Schema>>,
    ): (value: unknown) => Readonly<Record<string, unknown>> {
        return (value) => {
            const result = details.loose().parse(value);

            return Object.fromEntries(
                Object.keys(details.shape).map((field) => [field, result[field]]),
            );
        };
    }

    /** Build the fields a call takes from its prepared work: the value and the idempotency key. */
    static #preparedFields(
        work: Prepared,
    ): Partial<Pick<MethodContext, "prepared" | "idempotencyKey">> {
        if (work === undefined) {
            return {};
        }
        const key = work.context.idempotencyKey;

        return { prepared: work.value, ...(key === undefined ? {} : { idempotencyKey: key }) };
    }

    /** List the settlements of prepared work, in call order. */
    static #settlementsOf(prepared: readonly Prepared[]): (string | undefined)[] {
        return prepared.map((work) => work?.settlement);
    }

    /** Report whether a publisher also receives the changes sent to the rows it publishes. */
    static #isUplink(publisher: sync.Publisher): publisher is sync.Uplink {
        return "receive" in publisher;
    }

    /** Publish a partition at its installation's root, and other scopes as given. */
    static #partitionPublisher(
        server: InstalledServerContext,
        store: () => ObjectStore,
        publisher: ((scope: string) => sync.Publisher) | undefined,
    ): (scope: string) => sync.Publisher {
        // open the root's publisher once, every link of the partition's chain going through it
        let root: sync.Publisher | undefined;

        return (scope) => {
            // publish other scopes as given
            if (scope !== server.partition) {
                return publisher?.(scope) ?? server.publisher;
            }

            // publish the partition through its installation's root
            root ??= server.publisherAt(
                Egress.address(server.name, server.scope),
                store().origin.package.id,
            );

            return root;
        };
    }

    /** Implement a served package: its routes, adapters, controllers, access and headers. */
    #packageHandler(
        entry: Catalog["served"][number],
        isFirst: boolean,
        servedIds: ReadonlySet<PackageId>,
    ): PackageHandler & { readonly objects: ObjectStore } {
        // run the package's types' controllers, the first package also the store's and unserved types'
        const {
            controllers = [],
            cursors = [],
            adapters = {},
            access,
            responseHeaders,
        } = entry.definition;
        const objects = this.#objectControl(
            (packageId) => packageId === entry.package.id || (isFirst && !servedIds.has(packageId)),
        );
        const implemented = this.implement(entry.package, entry.objects, {
            controllers: [
                ...(isFirst ? this.#storeControllers() : []),
                ...objects.controllers,
                ...controllers.map((controller) => controller(this)),
            ],
            cursors: [...objects.cursors, ...cursors.map((cursor) => cursor(this))],
        });

        // answer the package's adapters below its mount, and the routes of each protocol it implements
        const fetch = ObjectStore.#adapterFetch(entry.package, adapters, this);
        const protocols = new Map(
            (entry.definition.protocols ?? []).map((protocol) => [
                protocol.id,
                ObjectStore.#protocolRoutes(protocol, entry),
            ]),
        );

        // run the controllers of a package served at an installed installation once its host activates it, a base one's at once
        const { server } = entry;
        const activation =
            server !== undefined && ServerContext.isInstalled(server)
                ? server.activation
                : undefined;

        return {
            ...implemented,
            ...(isFirst ? { syncing: this.syncing() } : {}),
            ...(access === undefined ? {} : { access }),
            ...(responseHeaders === undefined ? {} : { responseHeaders }),
            ...(fetch === undefined ? {} : { fetch }),
            ...(activation === undefined ? {} : { activation }),
            protocols,
        };
    }

    /** List the routes of a protocol a package implements, refusing a type or adapter it skips. */
    static #protocolRoutes(
        protocol: PackageHandle,
        entry: Catalog["served"][number],
    ): ReadonlySet<string> {
        // require each of the protocol's types handled and each of its adapters answered
        const types: readonly ObjectType[] = Object.values(protocol.objects).filter(
            (type) => type instanceof ObjectType,
        );
        const unhandled = types.find((type) => !entry.objects.some((each) => each.same(type)));
        const adapters: readonly AdapterDeclaration[] = Object.values(protocol.adapters);
        const unanswered = adapters.find(
            (adapter) => entry.definition.adapters?.[adapter.name] === undefined,
        );
        if (unhandled !== undefined) {
            throw new TypeError(
                `${entry.package.name} implements ${protocol.name} without handling ${unhandled.name}`,
            );
        } else if (unanswered !== undefined) {
            throw new TypeError(
                `${entry.package.name} implements ${protocol.name} without answering adapter ${unanswered.name}`,
            );
        }

        // route its types' collections and its adapters' paths
        return new Set([
            ...types.map((type) => `${type.route.prefix}/${Casing.kebab(type.plural)}`),
            ...adapters.map((adapter) => adapter.path),
        ]);
    }

    /** Route requests below a package's mount to the adapters answering their paths. */
    static #adapterFetch(
        handle: PackageHandle,
        answers: Readonly<Record<string, AdapterFetch>>,
        store: ObjectStore,
    ): PackageHandler["fetch"] {
        // pair each declared adapter with its answer, refusing an answer of none declared
        const adapters = Object.values(handle.adapters).map((adapter) => {
            const fetch = answers[adapter.name];
            if (fetch === undefined) {
                throw new TypeError(
                    `package ${handle.name} does not implement adapter ${adapter.name}`,
                );
            }

            return { name: adapter.name, path: adapter.path, callers: adapter.callers, fetch };
        });
        const stray = Object.keys(answers).find((name) => handle.adapters[name] === undefined);
        if (stray !== undefined) {
            throw new TypeError(`package ${handle.name} does not declare adapter ${stray}`);
        } else if (adapters.length === 0) {
            return undefined;
        }

        return async (request, context) => {
            // find the adapter a route names, else the one whose path the request is at or below
            const url = new URL(request.url);
            const routed = adapters.find(
                ({ name }) => url.pathname === `${ROUTE_PATH}/${encodeURIComponent(name)}`,
            );
            const adapter =
                routed ??
                adapters.find(
                    ({ path }) => url.pathname === path || url.pathname.startsWith(`${path}/`),
                );
            if (adapter === undefined) {
                return undefined;
            }

            // allow the installation itself or identified callers of the declared types, leaving an adapter declaring none its protocol
            const { callers } = adapter;
            const caller =
                callers === undefined
                    ? AccessContext.of(Caller.anonymous(), context.clock.now())
                    : context.access();
            const isDeclared =
                callers === undefined ||
                (callers === "self"
                    ? Subject.same(caller.subject, store.principal)
                    : callers.some(
                          (type) =>
                              type.packageId === caller.subject.packageId &&
                              type.type === caller.subject.type,
                      ));
            if (callers !== undefined && Caller.isAnonymous(caller)) {
                throw ObjectError.of("CREDENTIAL_MISSING", "missing caller credential");
            } else if (!isDeclared) {
                throw ObjectError.of(
                    "ADAPTER_REFUSES_CALLER",
                    `adapter ${adapter.name} does not accept ${caller.subject.type} ${caller.subject.id}`,
                );
            }

            // answer below the route or the adapter's path, trusting a client's head from a route or the installation alone
            const prefix =
                routed === undefined
                    ? adapter.path
                    : `${ROUTE_PATH}/${encodeURIComponent(adapter.name)}`;
            url.pathname = url.pathname.slice(prefix.length) || "/";
            const headers = new Headers(request.headers);
            if (routed === undefined && callers !== "self") {
                headers.delete(CLIENT_HEAD_HEADER);
            }

            return adapter.fetch(
                copyRequest(request, { headers }, url.href),
                caller,
                context,
                store,
            );
        };
    }

    /** List the subscriptions of the follows' keys, one per scope and principal. */
    static async #followSubscriptions(
        store: ObjectStore,
        follows: readonly Follow[],
        options: {
            readonly target: string;
            readonly view: Snapshot;
            readonly changes?: readonly Change[];
        },
    ): Promise<SubscriberEntry[]> {
        // read each follow's keys: fixed, from the store's rows, or acquired on demand
        const keyed = await Promise.all(
            follows.map(async (follow) => {
                // leave a chain follow to the sink
                if ("chain" in follow) {
                    return [];
                }
                // take the acquired keys that have not lapsed
                else if ("onDemand" in follow) {
                    return ObjectStore.#acquiredKeys(store, follow, options.view);
                }
                // take a follow's fixed keys
                else if ("keys" in follow) {
                    return follow.keys.map((key) => ({
                        shape: follow.shape,
                        key,
                        isAcquired: false,
                    }));
                }
                // read a follow's keys from the store's rows
                else {
                    const keys = await store.#readKeys(follow, options.view, options.changes);

                    return keys.map((key) => ({ shape: follow.shape, key, isAcquired: false }));
                }
            }),
        );

        // group the keys by scope and the principal decided for, the store's own as none, then by shape
        const groups = new Map<
            string,
            {
                readonly scope: string;
                readonly subject: Subject | undefined;
                readonly shapes: Map<Shape, Map<string, object>>;
                isAcquired: boolean;
            }
        >();
        for (const { shape, key, isAcquired } of keyed.flat()) {
            const { scope, ...parameters } = key.key;
            const subject = store.#follower(key.subject);
            const name = Shape.replicaName(scope, subject);
            const group = groups.get(name) ?? {
                scope,
                subject,
                shapes: new Map<Shape, Map<string, object>>(),
                isAcquired,
            };
            const keys = group.shapes.get(shape) ?? new Map<string, object>();
            keys.set(canonicalize(parameters), parameters);
            group.shapes.set(shape, keys);
            group.isAcquired &&= isAcquired;
            groups.set(name, group);
        }

        // subscribe to each group toward its scope's home, as one replica
        return [...groups.values()].map(({ scope, subject, shapes, isAcquired }) => ({
            subscription: Shape.subscription(
                scope,
                [...shapes].map(([shape, keys]) => ({ shape, keys: [...keys.values()] })),
                options.target,
                subject,
            ),
            home: scope,
            isAcquired,
        }));
    }

    /** List an on-demand follow's acquired keys that have not lapsed. */
    static async #acquiredKeys(
        store: ObjectStore,
        follow: Extract<Follow, { readonly onDemand: unknown }>,
        view: Snapshot,
    ): Promise<{ readonly shape: Shape; readonly key: FollowKey; readonly isAcquired: true }[]> {
        const rows = await view.read(followKey).findMany({
            where: { shape: follow.shape.name, expiresAt: { gt: store.clock.now() } },
        });

        return rows.map((row) => ({
            shape: follow.shape,
            key: {
                key: { ...schema.jsonObject().parse(JSON.parse(row.parameters)), scope: row.scope },
                subject: Subject.parse(JSON.parse(row.subject)),
            },
            isAcquired: true,
        }));
    }

    /** Read the principal a follow is decided for, none for the store's own. */
    #follower(subject: Subject | undefined): Subject | undefined {
        return subject === undefined || Subject.key(subject) === Subject.key(this.principal)
            ? undefined
            : subject;
    }

    /** Read a follow's keys over the store's rows, again only for rows a change touched. */
    async #readKeys(
        follow: RowFollow<object, ObjectType>,
        view: Snapshot,
        changes: readonly Change[] | undefined,
    ): Promise<FollowKey[]> {
        // read the rows of the store's database the keys come from, in identifier order
        const table = follow.from.table;
        const rows = await view
            .read(table)
            .findMany({ where: follow.where ?? {}, orderBy: { id: "asc" } });

        // list the source rows the commit changed, whose keys read again
        const changed = new Set(
            (changes ?? [])
                .filter((change) => Change.of(change, table))
                .map((change) => Key.name(table, change.key)),
        );

        // reuse a row's key while the row and what its key read stay unchanged, else read it again
        const previous = this.#rowKeys.get(follow);
        const listed = new Map<string, RowKey>();
        const keys = await Promise.all(
            rows.map(async (row) => {
                // reuse the key while the row and what its key read stay unchanged
                const name = Key.name(table, row);
                const known = previous?.get(name);
                const isReused =
                    changes !== undefined &&
                    known !== undefined &&
                    !changed.has(name) &&
                    !changes.some((change) => known.reads.touches(change));
                const entry = isReused ? known : await this.#readKey(follow, row, view);

                // list the row's entry, and its reads in the listing's
                listed.set(name, entry);
                view.reads?.add(entry.reads);

                return entry.key;
            }),
        );
        this.#rowKeys.set(follow, listed);

        return keys.filter((key) => key !== undefined);
    }

    /** Read one source row's key, recording what the key read. */
    async #readKey(
        follow: RowFollow<object, ObjectType>,
        row: Parameters<RowFollow<object, ObjectType>["key"]>[0],
        view: Snapshot,
    ): Promise<RowKey> {
        const reads = new ReadSet();
        const key = await follow.key(row, view.recording(reads), this.space);

        return { key, reads };
    }
}
