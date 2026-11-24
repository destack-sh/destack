import { AccessAudience } from "./audience.ts";
import { FormerKey } from "../object/former.ts";
import type { Health } from "./health.ts";
import { Casing } from "../object/name.ts";
import { RelationGraph } from "../query/query.ts";
import { CLIENT_HEAD_HEADER, ROUTE_HEADER } from "../workload/start.ts";
import type { BlobStorage } from "@destack/resource";
import { ObjectError, refuseInput } from "../error/error.ts";
import { CallError } from "../error/call.ts";
import { PackageVersion, type PackageId } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import type { EventStore } from "@destack/event/store";
import {
    AccessContext,
    Policy,
    Caller,
    Grant,
    CONTAINED,
    principal,
    universe,
    decisionTables,
    type TableMapping,
    Manager,
} from "@destack/access";
import { Authorizer, type Reader, Access, type ChainOptions } from "@destack/access/authorizer";
import {
    type ObjectReference,
    ObjectTypeReference,
    Scope,
    type ScopeLink,
    Subject,
    RequestId,
    type Watermark,
} from "@destack/sync";
import {
    and,
    DatabaseError,
    eq,
    Key,
    type Select,
    type SQL,
    Row,
    TABLE,
    type Table,
    Change,
    LogPosition,
} from "@destack/db";
import {
    type Alarm,
    BlobStore,
    ControlLoop,
    type ControllerSet,
    type Controller,
    type DatabaseConnection,
    ReadSet,
    type Snapshot,
    type BatchWriter,
    Transaction,
} from "@destack/db/database";
import { objectUnmask } from "../audit/unmask.ts";
import { Replay } from "../audit/replay.ts";
import {
    AuditRecorder,
    type AuditOrigin,
    type AuditRecorderOptions,
    type CallRequest,
} from "../audit/recorder.ts";
import { Journal, type JournalRecord, type RequestIdentity } from "../audit/journal.ts";
import type { CallKey } from "../audit/key.ts";
import {
    canonicalize,
    Digest,
    Duration,
    Identifier,
    type JsonObject,
    JsonValue,
    nth,
    present,
    Edition,
    schema,
    Uuid,
    type UuidEnvironment,
    zip,
} from "@destack/schema";
import type { ServerContext } from "../workload/workload.ts";
import { ACTION_KIND } from "../declare/action.ts";
import { copyRequest, type Clock, type Environment } from "@destack/environment";
import { implement, type Router } from "./handler.ts";
import type { ServerAccess, RequestContext } from "./context.ts";
import type { PackageHandler } from "./http.ts";
import type { ProcedureContext } from "./procedure.ts";
import { Egress } from "../declare/egress.ts";
import { type MountDeclaration, Package } from "../declare/package.ts";
import type { ProcedureRouter } from "../procedure/index.ts";
import { traceCalls } from "../procedure/telemetry.ts";
import * as sync from "@destack/sync";
import * as syncDataflow from "@destack/sync/dataflow";
import {
    defineShape,
    type OnDemandFollow,
    type Follow,
    type FollowKey,
    type RowFollow,
    Shape,
} from "../replica/shape.ts";
import { followKey, STORE_FOLLOWER } from "../replica/key.ts";
import {
    ACCESS_PUBLICATION,
    QUERIES_PUBLICATION,
    QueriesParameters,
    WriterId,
    storeProcedures,
    type PushResult,
    type StoreProcedures,
    type AskedKind,
    type PackageImplementationDescription,
    ObjectTypeEntry,
} from "../replica/replica.ts";
import { CONNECTION_PATH } from "../replica/connection.ts";
import { Branch, type BranchEntry, type BranchType } from "../branch/index.ts";
import { MethodContext, type ResultsOf, type Run } from "../method/context.ts";
import { CallSite } from "../method/site.ts";
import { Limits } from "../method/limits.ts";
import { Method } from "../method/method.ts";
import { callableProcedures } from "../method/procedure.ts";
import { ACCESS_KINDS, type ObjectStorage, ObjectType } from "../object/object.ts";
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
import {
    Catalog,
    type CatalogOptions,
    type CatalogExclusion,
    type ObjectTypeImplementation,
} from "./catalog.ts";
import {
    type ResourceApply,
    ResourceController,
    type ResourceControllerOptions,
} from "./provisioned.ts";
import type { Resource, ResourceType } from "../trait/provisioned.ts";
import { ObjectRemote } from "../replica/remote.ts";
import { Outbox, type OutboxOptions } from "./outbox.ts";
import { TrackedHistory } from "./tracked.ts";
import { Settlement } from "./settlement.ts";
import { TelemetryScope } from "@destack/telemetry";
import type { Execution, StoreExecution } from "../method/execution.ts";
import type { Isolate } from "../hook/isolate.ts";
import type { MountFetch, PackageImplementation } from "./implementation.ts";
import { RecordExecution } from "./record.ts";

/** How long a scope's access rows stay followed after their last follower: five minutes. */
const ACCESS_TTL = { minutes: 5 } as const;

/** The RPC system of an in-process follower's calls, as `rpc.system.name` reports it. */
const PROCESS_SYSTEM = "destack.process";

/** The prefix of the positions of the store loop's sets in the log. */
const POSITION_PREFIX = "control:";

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

/** How long a request waits for its caller's watermarks: 5 s, above sub-second replica lag. */
const BOOKMARK_TIMEOUT_MILLISECONDS = 5000;

/** When a caller asks again after a replica missed its watermark: 1 s, about a follow's round. */
const REPLICA_RETRY_MILLISECONDS = 1000;

/** How long a store waits for a key it acquires to replicate: 5 s, above sub-second replica lag. */
const ACQUIRE_MILLISECONDS = 5000;

/** A database's replica: its subscription, and the scope whose publisher streams it. */
export interface SubscriberEntry {
    /** The subscription following the replica. */
    readonly subscription: sync.SubscriptionRecord;
    /** The scope whose publisher streams the replica's pages, one step closer to its rows' home. */
    readonly home: string;
}

/** The subscriptions a database follows for its replicas, each through its home's remote. */
export interface Subscriber {
    /** List the subscriptions of the store's chains and follows, reusing keys no change touched. */
    readonly subscriptions: (
        view: Snapshot,
        changes?: readonly Change[],
    ) => Promise<readonly SubscriberEntry[]>;
    /** Open the publisher streaming a home's replicas. */
    readonly publisher: (home: string) => sync.Publisher;
    /** The remote taking the store's changes to replicated rows, absent for none. */
    readonly remote: ObjectRemote | undefined;
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

/** The journal's progress through a mutation: the running call. */
type MutationProgress = {
    /** The position of the call running now, absent before and after the calls. */
    running: number | undefined;
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
    "objects" | "replicas" | "installed" | "external" | "provisioned" | "follows" | "principal"
> &
    CallLog & {
        /** The package servers, the first one running the store's controllers. */
        readonly servers: readonly PackageImplementation[];
        /** The isolates of the installations whose packages run here without a workload. */
        readonly isolates?: readonly Isolate[];
        /** Open a package's release a scope runs, or a given version, absent for none. */
        readonly release?: ObjectStore["release"];
        /** The machine running the servers' provisioned resources, which providers require. */
        readonly machine?: Identifier<"machine">;
    } & (
        | {
              /** The principal the store's work runs as. */
              readonly principal: Subject;
              /** No installation: the store runs as the given principal. */
              readonly server?: never;
          }
        | {
              /** The installation the store serves as, running its work and replicas. */
              readonly server: ServerContext;
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
    /** The built-in types the host knows beside those it serves, which installations refer to. */
    readonly known?: readonly ObjectType[];
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
    /** What the process's jobs read beyond the database, such as its linked releases. */
    readonly configuration?: JsonValue;
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimPrimary;
    /** The installations serving objects and their implementing packages, by type key. */
    readonly installed?: ReadonlyMap<string, ObjectTypeImplementation>;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** Open a principal's live link for one of its streams, answering its close. */
    readonly connect?: (subject: Subject) => Promise<() => Promise<void>>;
    /** The files with the served external objects, each storing some of their types. */
    readonly external?: readonly ExternalStorage[];
    /** Read how the store has its scope's chain: home, follower, pending, or absent with none. */
    readonly chain: (view: Snapshot) => Promise<ChainOptions | PendingChain | undefined>;
    /** The shapes the served code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
    /** Open the publisher toward a followed scope's home. */
    readonly publisher?: (home: string) => sync.Publisher;
    /** The followed installation's remote, taking changes to replicated rows. */
    readonly remote?: ObjectRemote;
    /** The outbox of the node's writes of rows homed elsewhere, absent for none. */
    readonly outbox?: OutboxOptions;
    /** The device the store runs on, partitioning device-scoped local objects. */
    readonly device?: Subject;
    /** The most replicated object rows in the store, absent for no limit. */
    readonly storage?: { readonly rows: number };
    /** How often merged changes arrive, such as for a background tab. */
    readonly refresh?: { readonly every: Duration };
    /** How long the log retains followed changes, absent for the database's own. */
    readonly log?: { readonly retention: Duration };
    /** The publications served and followed beside the objects' and policies' ones. */
    readonly publications?: readonly sync.Publication[];
    /** Whether reads of the objects record access events, as the space's audit setting asks. */
    readonly isAccessAudited?: boolean;
    /** The environment calls run and controllers reconcile in: its clock and randomness. */
    readonly environment: Environment;
    /** The providers reconciling the served kinds' resources, if the store reconciles any. */
    readonly provisioned?: ResourceControllerOptions;
}

/** A store's routes: each served object's procedures by its key. */
export type ObjectRouter = Readonly<Record<string, Router<ProcedureRouter, RequestContext>>>;

/** A store serving object types' methods over one database. */
export class ObjectStore {
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The record verbs the store runs for the built-in method kinds. */
    readonly record = RecordExecution;
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
    /** What the process's jobs read beyond the database, which names the loop's position. */
    readonly #configuration: JsonValue;
    /** The feed serving every sync. */
    readonly feed: syncDataflow.Feed;
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
    /** The store as an in-process follower's remote, running calls as their signer. */
    readonly remote: ObjectRemote;
    /** The store's writer of ephemeral rows at the homes it follows, one per process. */
    readonly writerId: string;
    /** The device the store runs on, partitioning device-scoped local objects. */
    readonly device: Subject | undefined;
    /** The most replicated object rows in the store, absent for no limit. */
    readonly storage: { readonly rows: number } | undefined;
    /** How often the followed replicas' merged changes arrive, absent for each change. */
    readonly refresh: { readonly every: Duration } | undefined;
    /** How long the log retains followed changes in milliseconds, absent for none. */
    readonly #retention: number | undefined;
    /** The moment after which compacting the log is due, at once on start. */
    #compactionDue = 0;
    /** The listeners of the events the followed replicas bring, by topic. */
    readonly #listeners = new Map<string, Set<(event: JsonValue) => void>>();
    /** The listeners of the followed replicas' refusals. */
    readonly #refusals = new Set<(error: unknown) => void>();
    /** The relays of every event the followed replicas bring. */
    readonly #relays = new Set<(broadcasts: readonly sync.Broadcast[]) => void>();
    /** The node's writes of rows homed elsewhere, absent for a store writing none. */
    readonly outbox: Outbox | undefined;
    /** Open an audit recorder for a scope and caller, with its calls' request and component. */
    readonly audit: (
        scope: string,
        caller: AccessContext,
        options: AuditRecorderOptions,
    ) => AuditRecorder;
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
    readonly #readers = new Map<string, Reader>();
    /** Each row follow's keys by source row as last listed, with what each key read. */
    readonly #rowKeys = new Map<Follow, Map<string, RowKey>>();
    /** The options each catalog compiles with, apart from the servers and isolates. */
    readonly #compiling: Omit<CatalogOptions, "servers" | "isolates">;
    /** The policies of described types no server here serves yet. */
    #established: readonly Policy[] = [];
    /** The packages, types, routes, tables and access rules the store serves. */
    #catalog: Catalog;
    /** The handler of each served package, by its catalog entry. */
    #handlers = new Map<
        Catalog["served"][number],
        PackageHandler & { readonly objects: ObjectStore }
    >();
    /** The served jobs by name and the tables they watch, which a change restarts the loop over. */
    #controlled = "";
    /** The next or running loop, its reload and stop signals, and its run until replaced. */
    #loop = Promise.withResolvers<{
        readonly loop: ControlLoop;
        readonly reloading: AbortSignal;
        readonly stopping: AbortSignal;
        readonly stopped: Promise<void>;
    }>();
    /** Aborts the running loop once the served jobs change. */
    #reloading = new AbortController();
    /** Aborts once the store serves other types, which streams that left some out recompile at. */
    #serving = new AbortController();
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

        // run as the installation serving the store, else as the given principal, refusing a second remote
        const { server } = given;
        if (server !== undefined && given.remote !== undefined) {
            throw new TypeError("an installation's replicated rows must change through its remote");
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
        this.origin = { ...given.origin, package: PackageVersion.of(given.origin.package) };
        this.#configuration = given.configuration ?? null;
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
            ...(partition === undefined ? {} : { partition }),
        };
        this.#catalog = Catalog.compile({
            ...this.#compiling,
            servers: given.servers,
            isolates: given.isolates ?? [],
        });
        this.feed = new syncDataflow.Feed(given.database, this.#catalog.tables, {});
        this.#declare();

        // place the follows' replicas, write ephemeral rows as one writer, and bound the replicas by the budget
        this.writerId = RequestId.create(this.environment);
        this.device = given.device;
        this.storage = given.storage;
        this.refresh = given.refresh;
        this.#retention =
            given.log === undefined ? undefined : Duration.milliseconds(given.log.retention);

        // follow the replicas, publishing a partition through its installation's root
        const isPartition = server?.partition !== undefined && server.partition !== server.scope;
        const publisher =
            server !== undefined && isPartition
                ? ObjectStore.#partitionPublisher(server, () => this, given.publisher)
                : given.publisher;
        this.subscriber = this.#following(
            server === undefined ? given.remote : server.remote,
            publisher,
        );
        this.remote = this.#remote();
        this.outbox = given.outbox === undefined ? undefined : new Outbox(this, given.outbox);

        // record the calls, the audit and the replicas
        this.#journal = ObjectStore.#journalOf(this.#catalog, given);
        this.audit = (scope, caller, recording) =>
            AuditRecorder.scoped(this.journal, this.origin)(scope, caller, recording);
        this.source = new ObjectSource(this);
        this.sink = new ObjectSink(this);

        // implement each served package, the first one also running the store's controllers
        this.#implement();
    }

    /** Authorize the policies of described types before a server here serves their packages. */
    establish(policies: readonly Policy[]): void {
        // leave the policies the catalog decides already
        const { authorizer } = this.#catalog;
        const added = policies.filter(
            (policy) => authorizer.find(policy.typeReference) === undefined,
        );
        if (added.length === 0) {
            return;
        }

        // compile the catalog again with them, reusing its reads of the servers
        const compiled = this.#compile(this.servers, this.isolates, [
            ...this.#established,
            ...added,
        ]);
        this.#established = compiled.established;
        this.#catalog = compiled.catalog;
    }

    /** Compile a catalog of some servers and isolates, with the established policies they leave. */
    #compile(
        servers: readonly PackageImplementation[],
        isolates: readonly Isolate[],
        established: readonly Policy[],
    ): { readonly catalog: Catalog; readonly established: readonly Policy[] } {
        // compile what the servers serve, then keep the established policies none of them decides
        const options = { ...this.#compiling, servers, isolates };
        const served = Catalog.compile(options, this.#catalog);
        const left = established.filter(
            (policy) => served.authorizer.find(policy.typeReference) === undefined,
        );
        const policies = [...(options.policies ?? []), ...left];

        return {
            catalog: left.length === 0 ? served : Catalog.compile({ ...options, policies }, served),
            established: left,
        };
    }

    /** Serve other servers and isolates, keeping the handlers of the packages read before. */
    apply(
        servers: readonly PackageImplementation[],
        isolates: readonly Isolate[],
    ): readonly (PackageHandler & { readonly objects: ObjectStore })[] {
        // compile the new catalog, reusing the current one's reads of unchanged servers
        const compiled = this.#compile(servers, isolates, this.#established);
        const { catalog } = compiled;

        // refuse methods in a store started without the journal recording them
        if (catalog.isServing && this.#journal === undefined) {
            throw new TypeError("a store started without methods has no journal for new ones");
        }
        this.#established = compiled.established;
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

    /** The isolates the store serves the installations of. */
    get isolates(): readonly Isolate[] {
        return this.#catalog.isolates;
    }

    /** The installations the store refuses, with why. */
    get refused(): readonly CatalogExclusion[] {
        return this.#catalog.refused;
    }

    /** The installations the store leaves out until types they refer to are served, with why. */
    get waiting(): readonly CatalogExclusion[] {
        return this.#catalog.waiting;
    }

    /** The tables whose changes relist the replicas: the chains', the follows' and their keys. */
    get watched(): readonly Table[] {
        // relist when chained rows or declared follows' rows change, which a host's chain may read, and when the store acquires or evicts a key
        const { follows, chained } = this.#catalog;
        const declared = follows.filter((each) => !("onDemand" in each));

        return this.subscriber === undefined
            ? []
            : [
                  ...chained,
                  ...declared.flatMap((each) => each.shape.objects.map((type) => type.table)),
                  ...follows.flatMap((each) => ("from" in each ? [each.from.table] : [])),
                  ...(this.#isFollowingOnDemand ? [followKey] : []),
              ];
    }

    /** Whether the store follows some shapes on demand, by keys it acquires and lets lapse. */
    get #isFollowingOnDemand(): boolean {
        return this.#catalog.follows.some((each) => "onDemand" in each);
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

    /** Implement each newly served package, and restart the loop once the served jobs change. */
    #implement(): void {
        // keep the handlers of packages still served
        const { served } = this.#catalog;
        this.#handlers = new Map(
            served.map((entry) => [
                entry,
                this.#handlers.get(entry) ?? this.#packageHandler(entry),
            ]),
        );

        // restart the loop over changed jobs
        const types = [...this.objects, ...this.#catalog.provisioned].map((object) =>
            ObjectTypeReference.key(object.typeReference),
        );
        const watched = [...this.watched, ...this.replicated.map((each) => each.table)].map(
            (table) => table[TABLE].sqlName,
        );
        const packages = served.map((entry) => entry.package.id);
        const controlled = canonicalize([
            [...new Set(types)].toSorted(),
            [...new Set(watched)].toSorted(),
            packages,
        ]);
        if (controlled !== this.#controlled) {
            this.#controlled = controlled;
            this.#reloading.abort();
            this.#serving.abort();
            this.#serving = new AbortController();
        }
    }

    /** The signal aborting once the store serves other types. */
    get serving(): AbortSignal {
        return this.#serving.signal;
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
    get standing(): readonly OnDemandFollow[] {
        return this.#catalog.standing;
    }

    /** The on-demand follow of a scope's access rows, keyed by scope alone. */
    static readonly ACCESS_FOLLOW: OnDemandFollow<Record<string, never>> = defineShape({
        name: ACCESS_PUBLICATION,
        parameters: schema.object({}),
        objects: [],
        queries: () => [],
    }).onDemand(ACCESS_TTL);

    /** The on-demand follow of followers' queries, absent at their home. */
    get queriesFollow(): OnDemandFollow | undefined {
        return this.#catalog.follows.find(
            (follow): follow is OnDemandFollow =>
                "onDemand" in follow && follow.shape.name === QUERIES_PUBLICATION,
        );
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

    /** Report whether the provider a resource requests creates nothing outside the store. */
    isPure(object: ResourceType<"space">, row: Pick<Resource, "provider" | "placement">): boolean {
        const provided = this.#catalog.providers.filter((each) => each.object.same(object));

        return ResourceController.select(provided, row)?.provision?.isPure === true;
    }

    /** Reconcile a provisioned resource inside a method call, as its kind's controller does. */
    async reconcileResource(
        call: MethodContext,
        object: ResourceType<"space">,
        row: Resource,
        apply?: ResourceApply,
    ): Promise<void> {
        // require the machine provisioned resources are placed on
        const { providers, machine } = this.#catalog;
        if (machine === undefined) {
            throw new TypeError("provisioned resources need a machine");
        }
        await new ResourceController(object, { providers, machine }).reconcileIn(call, row, apply);
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
    package(handle: Package): PackageHandler & { readonly objects: ObjectStore } {
        const found = this.packages.find((each) => each.package.id === handle.id);
        if (found === undefined) {
            throw new TypeError(`the store does not serve package '${handle.name}'`);
        }

        return found;
    }

    /** Follow the catalog's follows and the chains, absent for a root store following nothing. */
    #following(
        remote: ObjectRemote | undefined,
        publisher: ((home: string) => sync.Publisher) | undefined,
    ): Subscriber | undefined {
        // leave a root store, which follows nothing, without a subscriber
        if (this.#catalog.follows.length === 0 && remote === undefined && publisher === undefined) {
            return undefined;
        }

        // place the replicas in the store's scope, toward each scope's home
        const target = this.scope;

        // follow the current catalog's follows' keys and the chains
        return {
            remote,
            publisher: (home) => publisher?.(home) ?? present(remote, `the publisher of ${home}`),
            subscriptions: async (view, changes) => {
                // follow the follows' keys, then the chains
                const followed = await ObjectStore.#followSubscriptions(
                    this,
                    this.#catalog.follows,
                    { target, view, ...(changes === undefined ? {} : { changes }) },
                );
                const chainedEntries = (await this.sink.chains(target, view)).map(
                    ({ subscription, home }) => ({ subscription, home }),
                );

                // follow the principal's access rows in each scope a predicting store follows anything at
                const access = this.outbox === undefined ? [] : await this.#accessEntries(view);

                return [...chainedEntries, ...access, ...followed];
            },
        };
    }

    /** Describe the packages the store serves, with their listed types and declarations. */
    async describePackages(
        kinds: readonly schema.Infer<typeof AskedKind>[] = [],
    ): Promise<PackageImplementationDescription[]> {
        // describe each served package by its installation or as its scope's own, with its listed types' entries
        const isAsked = (declared: { readonly kind: string; readonly kindPackageId: string }) =>
            kinds.some(
                (asked) =>
                    asked.kind === declared.kind && asked.kindPackageId === declared.kindPackageId,
            );
        const served = await Promise.all(
            this.#catalog.served.map(async ({ package: packaged, objects, server }) => {
                // read an installed build's declarations of the asked kinds, else an own package's actions
                const declared =
                    server !== undefined
                        ? await ObjectStore.#declarations(server)
                        : Object.values(packaged.actions).map((action) => ({
                              ...ACTION_KIND.identity,
                              name: action.name,
                              description: action.describe(),
                          }));

                return {
                    packageId: packaged.id,
                    scope: server?.scope ?? this.principal.id,
                    ...(server !== undefined
                        ? { installation: { id: server.installation, name: server.name } }
                        : {}),
                    types: objects
                        .filter((type) => type.listing !== undefined)
                        .map((type) => ObjectTypeEntry.of(type)),
                    declarations: declared.filter((each) => isAsked(each)),
                };
            }),
        );

        // describe each isolated installation serving no package here, such as one shipping views alone, without types
        const described = new Set(served.map((each) => each.installation?.id));
        const unserved = await Promise.all(
            this.#catalog.isolates
                .filter((isolate) => !described.has(isolate.server.installation))
                .map(async ({ server }) => ({
                    packageId: PackageManifest.package(server.build.manifest).id,
                    scope: server.scope,
                    installation: { id: server.installation, name: server.name },
                    types: [],
                    declarations: (await ObjectStore.#declarations(server)).filter((each) =>
                        isAsked(each),
                    ),
                })),
        );

        return [...served, ...unserved];
    }

    /** Read an installed build's declarations as a description lists them. */
    static async #declarations(
        installation: ServerContext,
    ): Promise<PackageImplementationDescription["declarations"]> {
        return (await installation.build.declarations()).map(
            ({ kind, kindPackageId, name, description }) => ({
                kind,
                kindPackageId,
                name,
                description,
            }),
        );
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
                    "TYPE_NOT_FOUND",
                    `object type ${reference.type} of '${reference.packageId}' not found`,
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

    /** The replica of the principal's access rows in a scope, which a predicting store follows. */
    accessReplica(scope: string): syncDataflow.Replica {
        return new syncDataflow.Replica({
            name: `${ACCESS_PUBLICATION}:${scope}`,
            scope,
            tables: decisionTables,
        });
    }

    /** Report whether a scope's rows live here: the principal's, the device's or the store's. */
    isOwn(scope: string): boolean {
        return scope === this.principal.id || scope === this.device?.id || scope === this.scope;
    }

    /** List the access replicas a predicting store follows: one per scope of replicated rows. */
    async #accessEntries(view: Snapshot): Promise<SubscriberEntry[]> {
        // read the scopes of the keys followed open or not yet lapsed in one read, but local objects' keys
        const rows = await view.read(followKey).findMany({
            where: {
                OR: [{ expiresAt: { isNull: true } }, { expiresAt: { gt: this.clock.now() } }],
            },
        });
        const scopes = [
            ...new Set(rows.filter((row) => this.#isReplicatedKey(row)).map((row) => row.scope)),
        ].toSorted();

        // follow each scope's access rows at its home, placed in the store's scope
        return scopes.map((scope) => ({
            subscription: this.source.accessPublication.subscription({
                name: this.accessReplica(scope).name,
                scope,
                target: this.scope,
                parameters: {},
            }),
            home: scope,
        }));
    }

    /** Report whether a followed key's queries read replicated rows, not only local objects. */
    #isReplicatedKey(row: { readonly shape: string; readonly parameters: string }): boolean {
        // count a key of a shape the catalog does not follow on demand, such as access rows, as replicated
        const follow = this.#catalog.follows.find(
            (each) => "onDemand" in each && each.shape.name === row.shape,
        );
        if (follow === undefined) {
            return true;
        }

        // read the types the key's queries read, replicated when any is not local
        const { shape } = follow;
        const queries = shape.queries(shape.parameters.parse(JSON.parse(row.parameters)));

        return queries.some((query) =>
            shape.objects.some(
                (type) =>
                    type.name === query.object &&
                    type.package.id === query.package &&
                    type.storage !== "local",
            ),
        );
    }

    /** Read the principal's access in a scope with the groups its access replica vouches for. */
    async callerContext(
        scope: string,
        database: DatabaseConnection = this.database,
    ): Promise<AccessContext> {
        const subjects = await this.accessReplica(scope).subjects(database);
        const groups = subjects.filter((subject) => !Subject.same(subject, this.principal));

        return AccessContext.of({ ...Caller.of(this.principal), groups }, this.clock.now());
    }

    /** Stream the pages of a stream that opens later. */
    static async *#flatten(opening: Promise<AsyncIterable<sync.Page>>): AsyncGenerator<sync.Page> {
        yield* await opening;
    }

    /** Serve in-process followers: stream its replicas, and run their calls as the signer. */
    #remote(): ObjectRemote {
        const callerOf = (signer: Subject | undefined) =>
            AccessContext.of(
                Caller.of(present(signer, "the signer of an in-process follower's call")),
                this.clock.now(),
            );

        // trace each verb as a server call of the process, as a channel traces its own
        const traced = traceCalls("server", PROCESS_SYSTEM, this.environment);

        return {
            stream: (subscription, signal) =>
                ObjectStore.#flatten(
                    traced(["stream"], async () => this.source.publish(subscription, signal)),
                ),
            request: (subscription) => traced(["request"], () => this.source.request(subscription)),
            push: (home, push) =>
                traced(["push"], () =>
                    this.#pushAs(home, push.mutations, callerOf(push.signer), {
                        ...(push.writerId === undefined ? {} : { writerId: push.writerId }),
                        ...(push.origin === undefined ? {} : { origin: push.origin }),
                    }),
                ),
            call: (scope, call, signer) =>
                traced(["call"], async () =>
                    schema
                        .json()
                        .parse(await this.query(callerOf(signer), this.#place(call, scope))),
                ),
            broadcast: (scope, object, event, signer) =>
                traced(["broadcast"], async () => {
                    // send an event of a type served here to its readers, else through the remote toward its home
                    const served = this.objects.some(
                        (type) =>
                            type.name === object.type &&
                            type.policy.definition.packageId === object.packageId,
                    );
                    if (served) {
                        await this.source.broadcast(scope, object, event, callerOf(signer));
                    } else {
                        const publisher = present(
                            this.subscriber,
                            `the home of '${object.type}'`,
                        ).publisher(scope);
                        const remote = present(
                            ObjectRemote.is(publisher) ? publisher : undefined,
                            `the remote toward the home of '${object.type}'`,
                        );
                        await remote.broadcast(scope, object, event);
                    }
                }),
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
        catalog: Pick<Catalog, "isServing" | "hasMethods">,
        options: Pick<ObjectStoreOptions, "database" | "callKey" | "environment" | "events">,
    ): Journal | undefined {
        // require the call key and the event log recording the calls of a store serving methods, refusing them without any
        const { callKey, events } = options;
        const { isServing } = catalog;
        if (isServing && callKey === undefined) {
            throw new TypeError("a store with methods needs the call key of its journal");
        } else if (isServing && (events === undefined || !events.has(Journal.kind))) {
            throw new TypeError("a store with methods needs an event log for its calls");
        } else if (!catalog.hasMethods && callKey !== undefined) {
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
        // a space's store serves the space, an isolated installation's store its installation's space
        const { principal: self } = this;
        if (principal.space.is(self)) {
            return self.id;
        } else if (principal.installation.is(self)) {
            return self.scope;
        }

        return undefined;
    }

    /** Act as what serves a type: its installation, else the scope whose own package it is. */
    as(object: ObjectType, now = this.clock.now()): AccessContext {
        // require the type served here
        const { server } = this.implementation(object);
        const subject =
            server !== undefined
                ? principal.installation.reference(server.scope, server.installation)
                : this.principal;

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
        handle: Package,
        routed: readonly ObjectType[],
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
                    { package: PackageVersion.of(handle), requestId: call.request.requestId },
                ),
            ),
            ...(subscriber === undefined
                ? {}
                : { ready: (signal, health) => this.#waitReady(signal, health) }),
            router: this.router(routed),
            connections: { store: this.procedures() },
        };
    }

    /** Follow a shape's key on demand until it lapses, waiting a bounded time for the replica. */
    async acquire<Parameters extends object>(
        follow: OnDemandFollow<Parameters>,
        at: FollowKey<Parameters>,
        signal?: AbortSignal,
    ): Promise<void> {
        // read when the store's own follow of the key lapses
        const { onDemand } = follow;
        const key = this.#followKey(STORE_FOLLOWER, follow, at);
        const [current] = await this.database
            .select({ expiresAt: followKey.expiresAt })
            .from(followKey)
            .where(ObjectStore.#isFollowKey(key));

        // renew it past half its time
        const now = this.clock.now();
        const ttl = Duration.milliseconds(onDemand.ttl);
        const lapses = current?.expiresAt ?? undefined;
        if (lapses === undefined || lapses - now < ttl / 2) {
            await this.#writeFollowKeys([key], now + ttl);
        }

        // wait for the replica to follow it
        await this.waitSynced(follow, at, signal);
    }

    /** Wait until the replica follows a key's queries, refusing a replica still syncing. */
    async waitSynced<Parameters extends object>(
        follow: OnDemandFollow<Parameters>,
        at: FollowKey<Parameters>,
        signal?: AbortSignal,
    ): Promise<void> {
        // wait on commits until the synced replica follows the key's queries, refusing a late one
        const { shape } = follow;
        const { scope } = at.key;
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
                "REPLICA_UNAVAILABLE",
                `replica '${shape.name}' of '${scope}' is unavailable: it is still syncing`,
                { data: { retryAfter: REPLICA_RETRY_MILLISECONDS } },
            );
        }
    }

    /** Follow some keys on demand for a follower until it unfollows them, in one transaction. */
    async follow<Parameters extends object>(
        follower: string,
        follow: OnDemandFollow<Parameters>,
        keys: readonly FollowKey<Parameters>[],
    ): Promise<void> {
        const rows = keys.map((at) => this.#followKey(follower, follow, at));
        await this.#writeFollowKeys(rows, null);
    }

    /** Stop following some keys for a follower, now or after a time to live, in one transaction. */
    async unfollow<Parameters extends object>(
        follower: string,
        follow: OnDemandFollow<Parameters>,
        keys: readonly FollowKey<Parameters>[],
        ttl?: Duration,
    ): Promise<void> {
        // drop the keys at once, or let them lapse after their time to live
        const rows = keys.map((at) => this.#followKey(follower, follow, at));
        if (rows.length === 0) {
            return;
        } else if (ttl === undefined) {
            await this.database.transaction(async (transaction) => {
                for (const row of rows) {
                    await transaction.delete(followKey).where(ObjectStore.#isFollowKey(row));
                }
            });
        } else {
            await this.#writeFollowKeys(rows, this.clock.now() + Duration.milliseconds(ttl));
        }
    }

    /** List the keys the store follows on demand, with their followers and lapses. */
    async followKeys(): Promise<
        {
            readonly shape: string;
            readonly scope: string;
            readonly parameters: JsonObject;
            readonly follower: string;
            readonly expiresAt: number | null;
        }[]
    > {
        const rows = await this.database
            .snapshot()
            .read(followKey)
            .findMany({ orderBy: { follower: "asc" } });

        return rows.map((row) => ({
            shape: row.shape,
            scope: row.scope,
            parameters: schema.jsonObject().parse(JSON.parse(row.parameters)),
            follower: row.follower,
            expiresAt: row.expiresAt,
        }));
    }

    /** Read a follower's row key of a follow's key. */
    #followKey<Parameters extends object>(
        follower: string,
        follow: OnDemandFollow<Parameters>,
        at: FollowKey<Parameters>,
    ): Omit<Select<typeof followKey>, "expiresAt"> {
        const { scope, ...rest } = at.key;

        return {
            shape: follow.shape.name,
            scope,
            parameters: canonicalize(follow.shape.parameters.parse(rest)),
            subject: canonicalize(at.subject ?? this.principal),
            follower,
        };
    }

    /** Write followed keys' lapse or open them in one transaction, evicting lapsed keys. */
    async #writeFollowKeys(
        keys: readonly Omit<Select<typeof followKey>, "expiresAt">[],
        expiresAt: number | null,
    ): Promise<void> {
        if (keys.length === 0) {
            return;
        }
        await this.database
            .insert(followKey)
            .values(keys.map((key) => ({ ...key, expiresAt })))
            .onConflictDoUpdate({
                target: [
                    followKey.shape,
                    followKey.scope,
                    followKey.parameters,
                    followKey.subject,
                    followKey.follower,
                ],
                set: { expiresAt },
            });
    }

    /** Match a follower's row of a key. */
    static #isFollowKey(key: Omit<Select<typeof followKey>, "expiresAt">): SQL<boolean> {
        return and(
            eq(followKey.shape, key.shape),
            eq(followKey.scope, key.scope),
            eq(followKey.parameters, key.parameters),
            eq(followKey.subject, key.subject),
            eq(followKey.follower, key.follower),
        );
    }

    /** Listen to the refusals of the followed replicas' sources, until the returned stop. */
    listenRefusals(listener: (error: unknown) => void): () => void {
        this.#refusals.add(listener);

        return () => this.#refusals.delete(listener);
    }

    /** Hand a followed replica's refusal to the listeners. */
    refuse(error: unknown): void {
        for (const listener of this.#refusals) {
            listener(error);
        }
    }

    /** Relay every event the followed replicas bring to a listener until stopped. */
    relay(listener: (broadcasts: readonly sync.Broadcast[]) => void): () => void {
        this.#relays.add(listener);

        return () => this.#relays.delete(listener);
    }

    /** Listen to the events a followed replica brings for a topic, until the returned stop. */
    listen(topic: string, listener: (event: JsonValue) => void): () => void {
        // add the listener to the topic's, removing the topic with its last one
        const listeners = this.#listeners.get(topic) ?? new Set();
        listeners.add(listener);
        this.#listeners.set(topic, listeners);

        return () => {
            listeners.delete(listener);
            if (listeners.size === 0) {
                this.#listeners.delete(topic);
            }
        };
    }

    /** Take a followed page: its mutation outcomes and events, compacting the log when due. */
    async receive(page: sync.Page): Promise<void> {
        // end this node's mutations its source queued and its homes ran
        if (page.outcomes !== undefined && page.outcomes.length > 0) {
            await this.outbox?.receive(page.outcomes);
        }

        // deliver the events to their topics' listeners and the relays
        const broadcasts = page.broadcasts ?? [];
        for (const { topic, event } of broadcasts) {
            for (const listener of this.#listeners.get(topic) ?? []) {
                listener(event);
            }
        }
        if (broadcasts.length > 0) {
            for (const relay of this.#relays) {
                relay(broadcasts);
            }
        }

        // compact the log by its retention
        if (this.#retention !== undefined) {
            this.#compactionDue = await this.database.log.compactIfDue(
                this.#retention,
                this.#compactionDue,
                this.clock.now(),
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
            const chain = (await Scope.chain(this.database.snapshot(), this.scope)).map(
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
            const chain = await this.chain(database.snapshot());
            if (chain?.kind === "pending") {
                return `the chain of ${this.scope} is pending`;
            }

            // require each gating replica live and current, naming the first that is not
            for (const subscription of await this.sink.gates()) {
                const { name, scope } = subscription;
                if (!new sync.ReplicaSet(database).isLive(name, scope, this.authorizer.lag)) {
                    return `the replica ${name} of ${scope} is not live`;
                } else if (!(await new sync.ReplicaSet(database).isCurrent(subscription))) {
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

    /** Route the served objects' methods, or some of them. */
    router(types: readonly ObjectType[] = this.#catalog.declared): ObjectRouter {
        // route each object under its key, which its package's handle makes unique
        const routers: Record<string, Router<ProcedureRouter, RequestContext>> = {};
        for (const object of types) {
            routers[object.key] = this.#route(object);
        }

        return routers;
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

    /** Route the store procedures a connection calls. */
    procedures(): Router<StoreProcedures, RequestContext> {
        const store = implement(storeProcedures).$context<RequestContext>();

        return store.router({
            push: store.push.handler(async ({ input, context }) => {
                // refuse a called type outside the protocol the caller connected through
                for (const call of input.mutations.flatMap((mutation) => mutation.calls)) {
                    this.requireProtocolType(context, call.type);
                }

                // enter the pushed scope as the request's caller, and run the mutations as it
                await this.enter(context, input.scope, true);

                return this.#pushAs(input.scope, input.mutations, context.access(input.scope), {
                    ...(input.writerId === undefined ? {} : { writerId: input.writerId }),
                    ...(input.origin === undefined ? {} : { origin: input.origin }),
                });
            }),
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
                this.requireProtocolType(context, object.typeReference);
                if (object.readScope(name, called) !== input.scope) {
                    throw ObjectError.of(
                        "SCOPE_INVALID",
                        `scope '${input.scope}' is invalid: '${object.name}.${name}' reads another scope`,
                    );
                }

                return await this.#answer(object, name, called, context);
            }),
            broadcast: store.broadcast.handler(async ({ input, context }) => {
                // send as the request's caller, in the scope it entered, refusing a type outside its protocol
                this.requireProtocolType(context, input.object);
                await this.enter(context, input.scope, false);
                await this.source.broadcast(
                    input.scope,
                    input.object,
                    input.event,
                    context.access(input.scope),
                );

                return {};
            }),
            stream: store.stream.handler(({ input, context }) =>
                this.source.relayed(input, context),
            ),
            request: store.request.handler(async ({ input, context }) => {
                // record the caller's request of its own stream
                await this.source.requestFrom(input.subscription, context);

                return {};
            }),
            describe: store.describe.handler(async ({ input, context }) => {
                // describe every type, or a protocol's alone to a caller connected through it
                const protocol = this.#protocol(context);
                const packages = await this.describePackages(input.kinds);
                if (protocol === undefined) {
                    return { objects: this.describe(input.types), packages };
                }
                const reached = (
                    input.types ?? this.#catalog.declared.map((type) => type.typeReference)
                ).filter((type) => type.packageId === protocol);

                return {
                    objects: this.describe(reached),
                    packages: packages.flatMap((described) => {
                        const types = described.types.filter((type) => type.packageId === protocol);

                        return types.length === 0
                            ? []
                            : [{ ...described, types, declarations: [] }];
                    }),
                };
            }),
        });
    }

    /** Read the protocol a request connected through, absent for a served package's own. */
    #protocol(context: RequestContext): string | undefined {
        const isProtocol = this.#catalog.served.some((entry) =>
            entry.package.protocols.some((protocol) => protocol.id === context.audience),
        );

        return isProtocol ? context.audience : undefined;
    }

    /** Refuse a type outside the protocol a request connected through. */
    requireProtocolType(
        context: RequestContext,
        type: Pick<ObjectTypeReference, "packageId" | "type">,
    ): void {
        const protocol = this.#protocol(context);
        if (protocol !== undefined && type.packageId !== protocol) {
            throw ObjectError.of(
                "TYPE_NOT_FOUND",
                `type '${type.packageId}/${type.type}' not found`,
            );
        }
    }

    /** Refuse a subscription outside the protocol a request connected through. */
    requireProtocolSubscription(context: RequestContext, subscription: sync.Subscription): void {
        // follow a protocol's types through named queries of them alone
        if (this.#protocol(context) === undefined) {
            return;
        }
        const queries =
            subscription.publication === QUERIES_PUBLICATION
                ? QueriesParameters.parse(subscription.parameters).queries
                : undefined;
        if (queries === undefined) {
            throw ObjectError.of(
                "PUBLICATION_NOT_FOUND",
                `publication '${subscription.publication}' not found`,
            );
        }
        for (const query of Object.values(queries)) {
            this.requireProtocolType(context, { packageId: query.package, type: query.object });
        }
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
        // read untyped in a span, then type the answer
        const callable = this.#callable(call, false, this.replicas);
        const attributes = {
            "destack.object.type": call.type.type,
            "destack.object.method": call.method,
        };
        const answer = await this.#instruments.span(
            "destack.object.query",
            attributes,
            () => this.#query(caller, call, site, options),
            { duration: { attributes } },
        );
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

        // write a change of the store's own local objects without a journal, as their device writes them for its tabs
        if (this.#journal === undefined && mutation.calls.every((call) => this.#isLocal(call))) {
            return this.database.transaction((transaction) =>
                this.predict(transaction, mutation, caller),
            );
        }

        // write a change of rows homed elsewhere through its route within the caller's grant: queued and
        //  predicted at a node answering before its home, else run at the home as the caller's subject
        const home = this.#isForwarded(mutation) ? await this.#homeOf(mutation) : undefined;
        if (home !== undefined) {
            for (const call of mutation.calls) {
                await this.#requireForwardable(caller, call);
            }
            const sent = sync.Mutation.parse(mutation);

            if (this.outbox === undefined) {
                return ObjectRemote.mutate(home.remote, home.home, sent, caller.subject);
            }
            const origin = Outbox.origin(caller.subject);
            await this.outbox.queue(sent, origin);

            return this.outbox.results(sent.id, origin);
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
                "STORAGE_INVALID",
                "mutation is invalid: it cannot write both durable and ephemeral objects",
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
        const progress: MutationProgress = { running: undefined };
        const results = await this.#executeDurable(planned, progress);

        // settle prepared work
        await this.#settle(ObjectStore.#settlementsOf(prepared));

        // answer the submitted calls without their expansions
        return zip(results, calls)
            .filter(([, call]) => call.as === undefined)
            .map(([result]) => result);
    }

    /** Predict a mutation of rows homed elsewhere, writing its calls' changes. */
    async predict(
        database: DatabaseConnection,
        mutation: { readonly id: string; readonly calls: readonly sync.Call[] },
        caller: AccessContext,
    ): Promise<unknown[]> {
        // run each predicted call in one transaction over the replica, as the caller
        const transaction = await Transaction.open(database);
        const results: unknown[] = [];
        for (const entry of mutation.calls) {
            const { object, name, input } = ObjectType.resolve(
                [...this.objects, ...this.replicas],
                entry,
                true,
                CallSite.store,
            );
            const method = object.method(name);

            // leave an unpredicted call to its home
            if (!method.isPredicted) {
                results.push(undefined);
            }
            // execute a predicted one over the replica, leaving the caller's access to its home
            else {
                const { context, execution } = await this.#predictedContext(
                    transaction,
                    this.served(object),
                    name,
                    input,
                    caller,
                    mutation.id,
                );
                results.push(await method.execute(context, execution));
            }
        }

        // write the predicted changes into the database
        await transaction.changes.apply(database);

        return results;
    }

    /** Report whether a call changes a local object this store serves. */
    #isLocal(call: sync.Call): boolean {
        return ObjectType.find(this.objects, call)?.storage === "local";
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
        // keep former keys and claim the written names before the transaction's commit, whoever opened it
        this.#enlist(transaction);

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
                `'${object.name}.${name}' does external work, so it cannot run inside a transaction`,
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
            throw ObjectError.of(
                "SCOPES_INVALID",
                "mutation is invalid: its calls must act in one scope",
            );
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
            throw ObjectError.of(
                "CALLER_UNAUTHENTICATED",
                "authentication required: a mutation needs an authenticated caller",
            );
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
                call.object.redaction(call.name).redactFields(call.input),
            ),
        };
    }

    /** Execute a planned mutation once: commit it, else answer the outcome its journal recorded. */
    async #executeDurable(planned: MutationRun, progress: MutationProgress): Promise<unknown[]> {
        // reject a key expired or in the future
        RequestId.expiry(planned.request.requestId, this.clock.now());
        try {
            // run the authorized calls over a transaction committed with their records, again while it loses a race
            return await this.database.commit(async (transaction) => {
                const authorization = await this.#authorizeRun(transaction, planned);

                return this.#run(transaction, authorization, planned, progress);
            });
        } catch (error) {
            // answer the outcome an earlier or concurrent run recorded
            const committed = await this.journal.replay(planned.request, planned.fingerprint);
            if (committed !== undefined) {
                return committed;
            }

            // roll back prepared work
            await this.#settle(ObjectStore.#settlementsOf(planned.prepared));

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
        const calls = zip(mutation.calls, originals).map(([entry, { object, name, input }]) => ({
            type: entry.type,
            method: entry.method,
            input: object.redaction(name).redactFields(input, (found) => sensitive.push(found)),
        }));

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

        // keep former keys and claim the written names before the commit, and commit the prepared work with it
        progress.running = undefined;
        this.#enlist(transaction);
        transaction.record((committing) => this.#commit(committing, run.prepared));

        return executed;
    }

    /** Enlist the written objects' former keys and names in a transaction's commit. */
    #enlist(transaction: Transaction): void {
        FormerKey.enlist(transaction, this);
        Reservation.enlist(transaction, this);
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
    async #pushAs(
        scope: string,
        mutations: readonly sync.Mutation[],
        caller: AccessContext,
        pusher: { readonly writerId?: string; readonly origin?: string } = {},
    ): Promise<PushResult> {
        // run each mutation in a separate transaction, or queue it, answering this store's watermark
        const { writerId, origin } = pusher;
        const outcomes: PushResult["outcomes"][number][] = [];
        const queued: string[] = [];
        for (const pushed of mutations) {
            const mutation = {
                ...pushed,
                calls: pushed.calls.map((call) => this.#place(call, scope)),
            };
            try {
                // require the pushed scope to be the home running the mutation, a replicated type's for a forwarded one
                const leading = nth(mutation.calls, 0);
                const site = CallSite.of(caller);
                const first = this.#isUplinked(leading)
                    ? ObjectType.resolve(this.replicated, leading, true, site)
                    : this.#resolve(leading, true, site);
                if (first.object.readScope(first.name, first.input) !== scope) {
                    throw ObjectError.of(
                        "SCOPE_INVALID",
                        `mutation pushed to '${scope}' is invalid: it acts in another scope`,
                    );
                }
                // queue a change of rows homed elsewhere for the pushing party, answering at once, else run it here
                const home =
                    this.outbox !== undefined && this.#isForwarded(mutation)
                        ? await this.#homeOf(mutation)
                        : undefined;
                if (this.outbox !== undefined && home !== undefined) {
                    await this.outbox.queue(mutation, origin ?? Outbox.origin(caller.subject));
                    queued.push(mutation.id);
                } else {
                    const written = await this.mutate(
                        caller,
                        mutation,
                        writerId === undefined ? {} : { writerId },
                    );
                    outcomes.push({
                        id: mutation.id,
                        outcome: { value: schema.json().parse(written) },
                    });
                }
            } catch (error) {
                // record final failures, rethrow transient ones
                const failure = Replay.failure(error);
                if (failure === undefined) {
                    throw error;
                }
                outcomes.push({ id: mutation.id, outcome: { error: failure.error } });
            }
        }

        return { outcomes, queued, watermark: await this.watermark(scope) };
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
        // originate the propagated rows of a scope the store serves, such as a home's account's
        const isOriginated =
            object.propagated !== undefined && this.partitions.slice(1).includes(scope);
        if (isOriginated || object.storage !== "durable") {
            return false;
        }

        // replicate the propagated rows of a scope enclosing the store's, whether their replica arrived yet
        const isEnclosing =
            object.propagated !== undefined &&
            scope !== this.scope &&
            (await Scope.chain(snapshot, this.scope)).some((link) => link.object.id === scope);

        return (
            isEnclosing ||
            this.replicates(object) ||
            (await sync.ReplicaSet.isReplicated(snapshot, scope, object.table))
        );
    }

    /** Report whether every call of a mutation changes rows replicated here from their home. */
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
            edition: Edition.of(object.conversions(name)),
        };
        const home = await this.remoteOf(call);
        if (home === undefined) {
            throw ObjectError.of(
                "HOME_HANDLER_REQUIRED",
                `handler of '${object.name}.${name}' at the rows' home is required`,
            );
        }

        return home.call(scope, call, caller.subject);
    }

    /** Report whether a call's type's rows replicate from their home instead of living here. */
    #isUplinked(call: sync.Call): boolean {
        const object = ObjectType.find(this.replicated, call);

        return object !== undefined && !this.objects.some((served) => served.same(object));
    }

    /** Find the remote taking a change of replicated rows, else the store's remote. */
    async remoteOf(call: sync.Call): Promise<ObjectRemote | undefined> {
        // find the replica of the call's table in its scope
        const object = ObjectType.find(this.replicated, call);
        const scope = object?.readScope(call.method, call.input);
        const entries = await this.sink.followed();
        const home = entries.find(({ subscription, publisher }) => {
            const replica = this.source.buildReplica(subscription);

            return (
                ObjectRemote.is(publisher) &&
                replica.scope === scope &&
                object !== undefined &&
                replica.tables.includes(object.table)
            );
        });

        // fall back to the store's remote
        const publisher = home?.publisher;

        return publisher !== undefined && ObjectRemote.is(publisher)
            ? publisher
            : this.subscriber?.remote;
    }

    /** Find a mutation's one home and the remote reaching it, absent for rows stored here. */
    async #homeOf(mutation: {
        readonly calls: readonly sync.Call[];
    }): Promise<{ readonly home: string; readonly remote: ObjectRemote } | undefined> {
        // read each call's replicated type and scope, leaving a mutation of rows the database stores itself
        const view = this.database.snapshot();
        const routed = await Promise.all(
            mutation.calls.map(async (call) => {
                // read the call's replicated type and scope, a home only for rows replicated here
                const object = ObjectType.find(this.replicated, call);
                const scope = object?.readScope(call.method, call.input);
                const isReplica =
                    object !== undefined &&
                    scope !== undefined &&
                    (await this.isReplica(object, view, scope));

                return isReplica ? scope : undefined;
            }),
        );
        const homes = new Set(routed);
        if (homes.size === 1 && homes.has(undefined)) {
            return undefined;
        }

        // refuse a mutation mixing rows stored here, or rows of several homes
        const [home, other] = homes;
        if (homes.has(undefined)) {
            throw ObjectError.of(
                "HOMES_INVALID",
                "mutation is invalid: it must change rows of one home, local or replicated",
            );
        } else if (home === undefined || other !== undefined) {
            throw ObjectError.of(
                "HOMES_INVALID",
                `mutation is invalid: a mutation must change rows of one home, but its calls change rows of '${String(home)}' and '${String(other)}'`,
            );
        }

        // reach the home through the remote of its replica
        const remote = await this.remoteOf(nth(mutation.calls, 0));
        if (remote === undefined) {
            throw ObjectError.of(
                "REPLICA_RECEIVER_REQUIRED",
                "receiver of replicated row changes is required",
            );
        }

        return { home, remote };
    }

    /** List the store's job sets: the replicas' sync, then the store's and each package's. */
    control(): readonly ControllerSet[] {
        // sync the replicas at once
        const { subscriber } = this;
        const syncing = this.syncing();
        const synced = (signal: AbortSignal) =>
            subscriber === undefined ? Promise.resolve() : this.#waitReady(signal);

        // run the store's own jobs once synced
        const { served } = this.#catalog;
        const servedIds = new Set(served.map((entry) => entry.package.id));
        const tables = [
            ...this.#catalog.tables,
            ...served.flatMap((entry) => entry.package.tables),
        ];
        const expiring = [...new Set(tables)].filter((table) => table[TABLE].expiry !== undefined);
        const store: ControllerSet = {
            controllers: [
                ...this.#storeControllers(),
                ...expiring.map((table) => ControlLoop.expiry(table, this.database)),
            ],
            projections: [],
            cursors: [],
            activation: synced,
        };

        // run each unserved package's types' jobs once synced, as one set a package
        const unserved = new Map(
            [...this.objects, ...this.#catalog.provisioned]
                .filter((object) => !servedIds.has(object.package.id))
                .map((object) => [object.package.id, object.package]),
        );
        const own = [...unserved.values()].map((each): ControllerSet => ({
            ...this.#objectControl((packageId) => packageId === each.id),
            activation: synced,
            position: `${POSITION_PREFIX}${each.id}@${each.version}`,
        }));

        // run each package's jobs once synced and its installation activated
        const packages = served.map((entry): ControllerSet => {
            // read the package's types' jobs
            const objects = this.#objectControl((packageId) => packageId === entry.package.id);
            const activation = entry.server?.activation;

            return {
                ...objects,
                activation: async (signal) => {
                    await synced(signal);
                    await activation?.(signal);
                },
                position: `${POSITION_PREFIX}${entry.package.id}@${entry.package.version}`,
            };
        });

        return [syncing, store, ...own, ...packages];
    }

    /** Run the store's jobs in one loop until stopped, restarting it as the jobs change. */
    async run(
        signal: AbortSignal,
        options: {
            /** Set a wake-up for the earliest due key, so an evicted instance runs it. */
            readonly alarm?: Alarm;
            /** Read the process's own job sets, run beside the store's, again at each restart. */
            readonly sets?: () => readonly ControllerSet[];
        } = {},
    ): Promise<void> {
        // complete each set's position name by what its jobs read beyond the log
        const read = {
            store: import.meta.destack.package.version,
            configuration: this.#configuration,
        };
        const digest = await Digest.json(read, this.environment.crypto);
        let previous: ControlLoop | undefined;
        while (!signal.aborted) {
            // take the reload signal before reading the jobs, so a reload meanwhile restarts them
            const reloading = new AbortController();
            this.#reloading = reloading;

            // read the store's job sets and the process's
            const sets = [
                ...this.control().map((set) =>
                    set.position === undefined
                        ? set
                        : { ...set, position: `${set.position}:${digest}` },
                ),
                ...(options.sets?.() ?? []).map((set) => this.#synced(set)),
            ];

            // run the loop until stopped or reloaded
            const loop = new ControlLoop(this.database, sets, {
                ...(previous === undefined ? {} : { previous }),
                ...(options.alarm === undefined ? {} : { alarm: options.alarm }),
            });
            previous = loop;
            const stopped = loop.run(AbortSignal.any([signal, reloading.signal])).finally(() => {
                this.#loop = Promise.withResolvers();
            });
            this.#loop.resolve({ loop, reloading: reloading.signal, stopping: signal, stopped });
            await stopped;
        }
    }

    /** Settle once the running loop has no key due now or running, or at a deadline. */
    async idle(deadline?: number): Promise<void> {
        // wait for a running loop, up to the deadline
        const passed = new AbortController();
        const timeout =
            deadline === undefined
                ? new Promise<undefined>(() => {})
                : this.clock
                      .sleep(deadline - this.clock.now(), passed.signal)
                      .then(() => undefined);
        const running = await Promise.race([this.#loop.promise, timeout]);
        passed.abort();
        if (running === undefined) {
            return;
        }

        // wait for it, then for the next loop once a reload, not the store's stop, ended it
        await running.loop.idle(deadline);
        if (running.reloading.aborted && !running.stopping.aborted) {
            await running.stopped;
            await this.idle(deadline);
        }
    }

    /** Run a process's job set once the replicas are synced and its own activation settles. */
    #synced(set: ControllerSet): ControllerSet {
        // run it at once without replicas
        const { subscriber } = this;
        if (subscriber === undefined) {
            return set;
        }
        const { activation } = set;

        return {
            ...set,
            activation: async (signal) => {
                await this.#waitReady(signal);
                await activation?.(signal);
            },
        };
    }

    /** List the cursor and controller syncing the store's replicas. */
    syncing(): ControllerSet {
        const { subscriber } = this;

        return subscriber === undefined
            ? { controllers: [], projections: [], cursors: [] }
            : {
                  controllers: [
                      this.sink.controller(subscriber),
                      ...(this.#isFollowingOnDemand ? [this.sink.lapse()] : []),
                  ],
                  projections: [],
                  cursors: [
                      this.sink.cursor(subscriber),
                      ...(this.storage === undefined ? [] : [this.sink.budget(this.storage.rows)]),
                  ],
              };
    }

    /** List the controllers of the types a filter selects by their package's id. */
    #objectControl(isSelected: (packageId: PackageId) => boolean): ControllerSet {
        const selected = [...this.objects, ...this.#catalog.provisioned].filter((object) =>
            isSelected(object.package.id),
        );

        // work the controllers with a condition by their rows, the ones without by key
        const declared = selected.flatMap((object) =>
            [...object.controllers, ...object.traitControllers()].map((each) => ({
                object,
                controller: each,
            })),
        );

        return {
            controllers: declared.flatMap(({ object, controller }) =>
                "when" in controller ? [ObjectController.control(this, object, controller)] : [],
            ),
            projections: declared.flatMap(({ object, controller }) =>
                "when" in controller ? [] : [ObjectController.project(this, object, controller)],
            ),
            cursors: selected.flatMap((object) => object.cursors.map((cursor) => cursor(this))),
        };
    }

    /** List the store's controllers over its own tables, from blobs to its branch. */
    #storeControllers(): Controller[] {
        const isSettled = this.objects.some((object) =>
            Object.values(object.methods).some((method) => Method.settles(method)),
        );

        return [
            ...this.blobs.controllers(),
            ...(this.#journal === undefined ? [] : this.#journal.events.controllers()),
            ...(this.claims === undefined
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
    reader(access: Access, position: LogPosition): Reader {
        // reuse a known reader
        const key = JSON.stringify([access.scope, position.epoch, position.sequence]);
        const known = this.#readers.get(key);
        if (known) {
            return known;
        }

        // start one reading the grants as of the position, and drop the oldest
        const reader = this.authorizer.reader(this.database.log.at(position), access.scopes);
        this.#readers.set(key, reader);
        if (this.#readers.size > SHARED_READERS) {
            const [oldest] = this.#readers.keys();
            if (oldest !== undefined) {
                this.#readers.delete(oldest);
            }
        }

        return reader;
    }

    /** Open the audience of a scope's reader or of a fixed authorization. */
    async audience(
        scope: string,
        reader: AccessContext | Authorization,
        storage: ObjectStorage = "durable",
    ): Promise<AccessAudience> {
        return reader instanceof Authorization
            ? await AccessAudience.bound(this, scope, reader, storage)
            : await AccessAudience.of(this, scope, reader, { storage });
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
            const rows = await Scope.read(this.database.snapshot(), [scope, requested]);
            const row = rows.find((each) => each.scope === scope);
            const outer = rows.find((each) => each.scope === requested);
            const isInside = row !== undefined && Scope.within(row, requested);
            const isReplicated = !mutates && outer !== undefined && Scope.within(outer, scope);
            if (!isInside && !isReplicated) {
                throw ObjectError.of(
                    "SCOPE_OUTSIDE_REQUEST_NOT_FOUND",
                    `scope '${scope}' in the request's scope not found`,
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
                "SCOPE_OUTSIDE_CHAIN_INVALID",
                `scope '${outside.scope}' is invalid: it is outside the scope chain of '${scope}', so no replica here follows it`,
            );
        }

        // wait for each home's commit
        await this.#waitWatermarks(context, homes);
    }

    /** Read a scope and the scopes enclosing it, by identifier. */
    async #chainIds(scope: string): Promise<Set<string>> {
        const chain = await Scope.chain(this.database.snapshot(), scope);

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
                // refuse a scope still without a home or replica here
                const history = await this.history(watermark);
                if (history.kind === "unknown") {
                    throw ObjectError.of(
                        "HOME_OR_REPLICA_NOT_FOUND",
                        `home or replica of '${watermark.scope}' here not found`,
                    );
                }

                // refuse the late position, with where the log or the first late replica is
                throw ObjectError.of("REPLICA_UNAVAILABLE", await this.#late(watermark, history), {
                    data: { retryAfter: REPLICA_RETRY_MILLISECONDS },
                });
            }
        }
    }

    /** Write why a watermark is late here: where the store's log or its first late replica is. */
    async #late(watermark: Watermark, history: WatermarkHistory): Promise<string> {
        // read the store's own log for a watermark it wrote
        const { scope, sequence } = watermark;
        if (history.kind !== "replicated") {
            const head = await this.database.log.position();

            return `log of '${scope}' is unavailable: it is at sequence ${head.sequence} and the call needs sequence ${sequence}`;
        }

        // find the first late replica among those the wait reads
        const replicas = history.followed.length > 0 ? history.followed : history.including;
        const [late] = sync.ReplicaSet.late(
            await new sync.ReplicaSet(this.database).homes(replicas),
            watermark,
        );

        // report a replica without a position yet
        if (late?.home === undefined) {
            return late === undefined
                ? `replica of '${scope}' is unavailable: it follows another log than the call's`
                : `replica '${late.name}' of '${scope}' is unavailable: it has no position yet and the call needs sequence ${sequence}`;
        }

        // report where the late replica is
        return `replica '${late.name}' of '${scope}' is unavailable: it is at sequence ${late.home.sequence} and the call needs sequence ${sequence}`;
    }

    /** Read the position of the latest change in a scope's chain, which its readers see. */
    async watermark(scope: string): Promise<Watermark> {
        const chain = await Scope.chain(this.database.snapshot(), scope);
        const scopes = chain.map((link) => link.object.id);

        return { scope, ...(await this.database.log.changed(this.feed.tables, scopes)) };
    }

    /** Decide where a watermark's history is here: local log, an older one, replicas, or none. */
    async history(watermark: Watermark): Promise<WatermarkHistory> {
        // take the local log's current epoch for a scope the store serves, refusing an earlier one
        const { database } = this;
        const head = await database.log.position();
        const isHome = await this.authorizer.isHome(database.snapshot(), watermark.scope);
        const isServed = isHome || watermark.scope === this.scope;
        if (isServed && watermark.log === head.log) {
            return { kind: watermark.epoch === head.epoch ? "local" : "stale" };
        }
        // refuse another log for a scope homed here, which a move's lineage would continue
        else if (isHome) {
            return { kind: "stale" };
        }

        // take the replicas including the scope and the followed ones reading it
        const including = await new sync.ReplicaSet(database).including(watermark.scope);
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
                "EPOCH_CONFLICT",
                `cannot read '${watermark.scope}' at the watermark: it no longer has the watermark's history`,
            );
        }
        // wait until the followed replicas reading the scope have it, as only a followed replica moves
        else if (history.kind === "replicated" && history.followed.length > 0) {
            return await this.sink.wait(watermark, history.followed, signal);
        }
        // wait until the replicas including the scope reach it, for a store following none yet
        else {
            return await new sync.ReplicaSet(this.database).wait(
                watermark.scope,
                watermark,
                signal,
            );
        }
    }

    /** Find how this store implements a type, absent for a type it does not serve. */
    findImplementation(object: ObjectType): ObjectTypeImplementation | undefined {
        return this.#catalog.installed.get(ObjectTypeReference.key(object.typeReference));
    }

    /** Read how this store implements a type, refusing one it does not serve. */
    implementation(object: ObjectType): ObjectTypeImplementation {
        const served = this.findImplementation(object);
        if (served === undefined) {
            throw new TypeError(`nothing serves ${object.name} in this store`);
        }

        return served;
    }

    /** Read the package whose code implements a served type, refusing an unserved one. */
    implementer(object: ObjectType): PackageId {
        return this.implementation(object).package.id;
    }

    /** Read the package implementing a type here, as its calls' audit records it. */
    #implementing(object: ObjectType): PackageVersion {
        // take the installed implementer, or the declaring package in a store serving no installations
        const served = this.#catalog.installed.get(ObjectTypeReference.key(object.typeReference));

        return served?.package ?? object.package;
    }

    /** Read the memory store with the ephemeral objects, failing without one. */
    requireEphemeral(): EphemeralStorage {
        if (this.ephemeral === undefined) {
            throw ObjectError.of(
                "EPHEMERAL_STORAGE_NOT_FOUND",
                "ephemeral storage in this store not found",
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
            throw ObjectError.of(
                "WRITER_REQUIRED",
                "writer is required: only a writer can write ephemeral objects",
            );
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
            const limits = object.methods[name]?.limits;
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
                        "RATE_EXHAUSTED",
                        `'${object.name}.${name}' exceeds its limit of ${rate.count} calls per ${Duration.milliseconds(rate.every)} ms`,
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
                "REPLICA_BLOCKED",
                `cannot change '${replicated.name}' rows: they are replicas here, so send changes to their home`,
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
        // refuse executing row changes to replicas, which their home writes and this store predicts
        const method = object.method(name);
        const scope = object.readScope(name, input);
        const isRowChange = method.mutates && !ACCESS_KINDS.has(method.kind);
        if (isRowChange && (await this.isReplica(object, transaction.view(), scope))) {
            throw ObjectError.of(
                "REPLICA_BLOCKED",
                `cannot change ${object.name} rows of '${scope}': they are replicas here, so send changes to their home`,
            );
        }

        // parse the input and build the call in its scope as the installation serving its type
        const { database } = transaction;
        const { fields, id, revision, at, branch } = ObjectStore.#fields(object, name, input);
        authorization.requireScopeOf(object);
        const snapshot = await this.#view(database, authorization, scope, at, branch);
        const view = snapshot ?? transaction.view();
        const resolved = await object.resolve(fields, scope, view);
        const targetId =
            id === undefined
                ? undefined
                : await ObjectStore.#identifier(object, schema.string().parse(id), scope, view);
        const execution: StoreExecution = {
            kind: "store",
            store: this,
            authorization,
            transaction,
            ...(snapshot === undefined ? {} : { view: snapshot }),
            callId: Identifier.create("call", this.environment),
            ...(options.parentId === undefined ? {} : { parentId: options.parentId }),
            isPredicted: false,
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

        // load the permitted target at the given revision
        const target = await ObjectStore.#loaded(
            context,
            execution,
            { id, targetId, revision },
            snapshot,
        );

        return {
            context: target === undefined ? context : context.with({ target }),
            execution,
            method,
            scope,
            targetId,
        };
    }

    /** Build a predicted call's context over the replica. */
    async #predictedContext(
        transaction: Transaction,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: AccessContext,
        requestId: string,
    ): Promise<{ readonly context: MethodContext; readonly execution: Execution }> {
        // parse the input, resolving the target's key, as the home does
        const method = object.method(name);
        const scope = object.readScope(name, input);
        const { fields, id, revision } = ObjectStore.#fields(object, name, input);
        const view = transaction.view();
        const resolved = await object.resolve(fields, scope, view);
        const targetId =
            id === undefined
                ? undefined
                : await ObjectStore.#identifier(object, schema.string().parse(id), scope, view);

        // build the call in the scope's chain the store follows, writing through the transaction
        const execution: Execution = {
            kind: "store",
            store: this,
            transaction,
            callId: Identifier.create("call", this.environment),
            isPredicted: true,
        };
        const context: MethodContext = new MethodContext({
            object,
            name,
            method,
            scope,
            chain: [...(await this.sink.ancestry(scope, view))],
            input: resolved,
            id: targetId,
            target: undefined,
            prepared: undefined,
            caller,
            requestId,
            location: execution,
            relations: this.relations,
            replicas: this.replicas,
        });

        // load the target at the given revision, as the transaction has it
        const target = await ObjectStore.#loaded(context, execution, { id, targetId, revision });

        return { context: target === undefined ? context : context.with({ target }), execution };
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
                  message: `${object.name} '${addressed}' not found`,
                  cause: missing.cause,
              })
            : error;
    }

    /** Read the identifier of the object a call names by its identifier or its key in a scope. */
    static async #identifier(
        object: ObjectType,
        value: string,
        scope: string,
        view: Pick<Snapshot, "read">,
    ): Promise<string> {
        // read the identifier, the key's holder or its former holder, refusing a key nobody held
        const id = await object.holder(view, scope, value);
        if (id === undefined) {
            throw CallError.http("NOT_FOUND", { message: `${object.name} '${value}' not found` });
        }

        return id;
    }

    /** Load a call's permitted target through its requested view, absent without a target. */
    static async #target(
        context: MethodContext,
        execution: Execution,
        targetId: string | undefined,
        snapshot: Snapshot | undefined,
    ): Promise<Row | undefined> {
        // skip a call without a target
        const { method, object } = context;
        if (!Method.loads(method) || targetId === undefined) {
            return undefined;
        }

        // read a prediction's target as its transaction has it, refusing a missing one
        if (execution.isPredicted) {
            const row = await execution.transaction
                .read(object.table)
                .findFirst({ where: { id: targetId } });
            if (row === undefined) {
                throw CallError.http("NOT_FOUND", {
                    message: `${object.name} '${targetId}' not found`,
                });
            }

            return row;
        }
        // read the live permitted target, or the target in the view
        else if (snapshot === undefined) {
            return execution.authorization.read(context, targetId, execution);
        } else {
            return execution.authorization.readIn(context, targetId, snapshot);
        }
    }

    /** Load a call's target, refusing predefined, changed and blocked targets. */
    static async #loaded(
        context: MethodContext,
        execution: Execution,
        address: {
            readonly id: unknown;
            readonly targetId: string | undefined;
            readonly revision: unknown;
        },
        snapshot?: Snapshot,
    ): Promise<Row | undefined> {
        // refuse writing a predefined row, then load the target
        const { object, method } = context;
        const { id, targetId, revision } = address;
        if (method.mutates && targetId !== undefined) {
            object.refusePredefined(targetId);
        }
        const target = await ObjectStore.#target(context, execution, targetId, snapshot).catch(
            (error: unknown) => {
                throw ObjectStore.#addressed(object, id, targetId, error);
            },
        );

        // refuse a changed revision, and a method a trashed or archived target no longer takes
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw ObjectError.of(
                "REVISION_CONFLICT",
                `cannot update ${object.name}: its revision changed`,
            );
        }
        if (target !== undefined) {
            object.refuseBlocked(method, target);
        }

        return target;
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
            at === undefined ? database.snapshot() : database.log.at(LogPosition.parse(at));
        if (branch === undefined) {
            return base;
        }

        // require the served branch types and the caller's read of the branch
        const types = this.branch;
        if (types === undefined) {
            throw ObjectError.of(
                "BRANCHES_REQUIRED",
                "branch support is required: this store does not serve branches",
            );
        }
        const id = schema.string().parse(branch);
        const row = await database.snapshot().read(types.object.table).findFirst({ where: { id } });
        const permission = types.object.permission("read");
        const allowed =
            row === undefined
                ? []
                : (await authorization.filter(types.object, permission, [row], { scope })).rows;
        if (allowed.length === 0) {
            throw ObjectError.of("BRANCH_NOT_FOUND", `branch '${id}' not found`);
        }

        return base.layer(await new Branch(types, base, id).overlay(base, this.objects));
    }

    /** Read a method about to run, refusing one that declares prepared work it never prepares. */
    static #preparing(object: ObjectType, name: string): Method {
        const method = object.method(name);
        if (method.prepared !== undefined && method.prepare === undefined) {
            throw new TypeError(
                `'${object.name}.${name}' declares prepared work but has no prepare phase`,
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
            throw new TypeError(
                `'${context.object.name}.${context.name}' declares no prepared work`,
            );
        }
        const keyed = Method.requireKeyed(context.with({ run: this.#alone(context) }));
        const value: unknown = await method.prepare(keyed);

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
                // capture a failure, leaving the settlement to its next claim
                this.#instruments.captureException(error, { isEscaped: true });
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

        return this.#instruments.span(
            "destack.object.call",
            attributes,
            () =>
                this.#perform(
                    transaction,
                    authorization,
                    object,
                    name,
                    input,
                    caller,
                    options,
                ).catch((error: unknown) => {
                    throw object.conflict(error);
                }),
            { duration: { attributes } },
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

    /** Record a principal's call: audit durable changes, and journal requests for retries. */
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
        // record only a principal's calls, a nested call taking effect within its parent
        if (execution.parentId !== undefined) {
            return;
        }

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
        });
        transaction.batch(this.#calls, (stamp: string | undefined) => {
            const stamped =
                journaled === undefined || stamp === undefined
                    ? journaled
                    : { ...journaled, transaction: stamp };

            return recorder.ended(object.audit(name), recorded, "activity", isAudited, stamped);
        });
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

    /** Publish a partition at its installation's root, and other scopes as given. */
    static #partitionPublisher(
        server: ServerContext,
        store: () => ObjectStore,
        publisher: ((scope: string) => sync.Publisher) | undefined,
    ): (scope: string) => sync.Publisher {
        // open the root's publisher once, every link of the partition's chain going through it
        let root: sync.Publisher | undefined;

        return (scope) => {
            // publish other scopes as given
            if (scope !== server.partition) {
                return publisher?.(scope) ?? server.remote;
            }

            // publish the partition through its installation's root
            root ??= server.connect(
                Egress.address(server.name, server.scope),
                store().origin.package.id,
            );

            return root;
        };
    }

    /** Implement a served package: its routes, mounts, controllers, access and headers. */
    #packageHandler(
        entry: Catalog["served"][number],
    ): PackageHandler & { readonly objects: ObjectStore } {
        // implement the package's types
        const { mounts = {}, access, responseHeaders } = entry.definition;
        const implemented = this.implement(entry.package, entry.objects);

        // answer the package's mounts below its path, and the routes of each protocol it implements
        const fetch = ObjectStore.#mountFetch(entry.package, mounts, this);
        const protocols = new Map(
            entry.package.protocols.map((protocol) => [
                protocol.id,
                ObjectStore.#protocolRoutes(protocol, entry),
            ]),
        );

        return {
            ...implemented,
            ...(access === undefined ? {} : { access }),
            ...(responseHeaders === undefined ? {} : { responseHeaders }),
            ...(fetch === undefined ? {} : { fetch }),
            protocols,
        };
    }

    /** List the routes of a protocol a package implements, refusing a type or mount it skips. */
    static #protocolRoutes(
        protocol: Package,
        entry: Catalog["served"][number],
    ): ReadonlySet<string> {
        // require each of the protocol's types handled and each of its mounts answered
        const types: readonly ObjectType[] = Object.values(protocol.objects).filter(
            (type) => type instanceof ObjectType,
        );
        const unhandled = types.find((type) => !entry.objects.some((each) => each.same(type)));
        const mounts: readonly MountDeclaration[] = Object.values(protocol.mounts);
        const unanswered = mounts.find(
            (mount) => entry.definition.mounts?.[mount.name] === undefined,
        );
        if (unhandled !== undefined) {
            throw new TypeError(
                `'${entry.package.name}' implements '${protocol.name}' without handling '${unhandled.name}'`,
            );
        } else if (unanswered !== undefined) {
            throw new TypeError(
                `'${entry.package.name}' implements '${protocol.name}' without answering mount '${unanswered.name}'`,
            );
        }

        // route its types' collections, its mounts' paths and the store's connection
        return new Set([
            ...types.map((type) => `${type.route.prefix}/${Casing.kebab(type.plural)}`),
            ...mounts.map((mount) => mount.path),
            CONNECTION_PATH,
        ]);
    }

    /** Route requests below a package's path to the mounts answering their paths. */
    static #mountFetch(
        handle: Package,
        answers: Readonly<Record<string, MountFetch>>,
        store: ObjectStore,
    ): PackageHandler["fetch"] {
        // pair each declared mount with its answer, refusing an answer of none declared
        const mounts = Object.values(handle.mounts).map((mount) => {
            const fetch = answers[mount.name];
            if (fetch === undefined) {
                throw new TypeError(
                    `package '${handle.name}' does not implement mount '${mount.name}'`,
                );
            }

            return { name: mount.name, path: mount.path, callers: mount.callers, fetch };
        });
        const stray = Object.keys(answers).find((name) => handle.mounts[name] === undefined);
        if (stray !== undefined) {
            throw new TypeError(`package '${handle.name}' does not declare mount '${stray}'`);
        } else if (mounts.length === 0) {
            return undefined;
        }

        return async (request, context) => {
            // find the mount whose path the request is at or below
            const url = new URL(request.url);
            const mount = mounts.find(
                ({ path }) => url.pathname === path || url.pathname.startsWith(`${path}/`),
            );
            if (mount === undefined) {
                return undefined;
            }

            // allow the installation itself or identified callers of the declared types, leaving a mount declaring none its protocol
            const { callers } = mount;
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
                throw ObjectError.of(
                    "CREDENTIAL_UNAUTHENTICATED",
                    "authentication required: caller credential is missing",
                );
            } else if (!isDeclared) {
                throw ObjectError.of(
                    "MOUNT_REFUSES_CALLER_DENIED",
                    `caller ${caller.subject.type} '${caller.subject.id}' is forbidden: mount '${mount.name}' does not accept it`,
                );
            }

            // answer below the mount's path, trusting a client's head from a machine's route or the installation alone
            url.pathname = url.pathname.slice(mount.path.length) || "/";
            const headers = new Headers(request.headers);
            if (!request.headers.has(ROUTE_HEADER) && callers !== "self") {
                headers.delete(CLIENT_HEAD_HEADER);
            }

            return mount.fetch(copyRequest(request, { headers }, url.href), caller, context, store);
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
                    }));
                }
                // read a follow's keys from the store's rows
                else {
                    const keys = await store.#readKeys(follow, options.view, options.changes);

                    return keys.map((key) => ({ shape: follow.shape, key }));
                }
            }),
        );

        // group the keys by scope and the principal decided for, the store's own as none, then by shape
        const groups = new Map<
            string,
            {
                readonly publication: string;
                readonly scope: string;
                readonly subject: Subject | undefined;
                readonly shapes: Map<Shape, Map<string, object>>;
            }
        >();
        for (const { shape, key } of keyed.flat()) {
            const { scope, ...parameters } = key.key;
            const subject = store.#follower(key.subject);
            const { publication } = shape;
            const name = Shape.replicaName(publication, scope, subject);
            const group = groups.get(name) ?? {
                publication,
                scope,
                subject,
                shapes: new Map<Shape, Map<string, object>>(),
            };
            const keys = group.shapes.get(shape) ?? new Map<string, object>();
            keys.set(canonicalize(parameters), parameters);
            group.shapes.set(shape, keys);
            groups.set(name, group);
        }

        // subscribe to each group toward its scope's home, as one replica
        return [...groups.values()].map(({ scope, subject, shapes }) => ({
            subscription: Shape.subscription(
                scope,
                [...shapes].map(([shape, keys]) => ({ shape, keys: [...keys.values()] })),
                {
                    target: options.target,
                    writerId: store.writerId,
                    ...(subject === undefined ? {} : { subject }),
                },
            ),
            home: scope,
        }));
    }

    /** List an on-demand follow's acquired keys that have not lapsed. */
    static async #acquiredKeys(
        store: ObjectStore,
        follow: OnDemandFollow,
        view: Snapshot,
    ): Promise<{ readonly shape: Shape; readonly key: FollowKey }[]> {
        const rows = await view.read(followKey).findMany({
            where: {
                shape: follow.shape.name,
                OR: [{ expiresAt: { isNull: true } }, { expiresAt: { gt: store.clock.now() } }],
            },
        });
        const keys = new Map(
            rows.map((row) => [canonicalize([row.scope, row.parameters, row.subject]), row]),
        );

        return [...keys.values()].map((row) => ({
            shape: follow.shape,
            key: {
                key: { ...schema.jsonObject().parse(JSON.parse(row.parameters)), scope: row.scope },
                subject: Subject.parse(JSON.parse(row.subject)),
            },
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
