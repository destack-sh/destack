import { PackageManifest } from "@destack/package/manifest";
import { ObjectError } from "../error/error.ts";
import { graph, Package, type PackageId } from "@destack/package";
import type { EventStore } from "@destack/event";
import {
    Authorizer,
    AccessContext,
    decisionTables,
    type TupleReader,
    Policy,
    type Access,
    type ChainOptions,
    type TableMapping,
    Caller,
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
    Change,
    Key,
    LogPosition,
    ReadSet,
    Snapshot,
    TABLE,
    type DatabaseConnection,
    Row,
    type Table,
    DatabaseError,
    Transaction,
    and,
    eq,
    lte,
    CompactionController,
    type ReconcileController,
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
    type Identifier,
    type JsonObject,
    JsonValue,
    present,
    schema,
    zip,
    Uuid,
    type UuidEnvironment,
} from "@destack/schema";
import type { InstalledServerContext, ServerContext } from "../workload/workload.ts";
import type { Watermark } from "@destack/sync";
import { refuseInput } from "../error/error.ts";
import { copyRequest } from "@destack/environment";
import type { RequestIdentity } from "../audit/journal.ts";
import { implement, type Router } from "./handler.ts";
import { type ServerAccess, type RequestContext } from "./context.ts";
import { type PackageHandler } from "./http.ts";
import { type ProcedureContext } from "./procedure.ts";
import { Egress } from "../declare/egress.ts";
import { PACKAGE_DECLARATION, PackageDescription } from "../declare/description.ts";
import { PackageHandle } from "../declare/package.ts";
import { type ProcedureRouter } from "../procedure/index.ts";
import * as sync from "@destack/sync";
import { Follow, type FollowKey, type RowFollow } from "../replica/shape.ts";
import { followKey } from "../replica/key.ts";
import {
    QueriesParameters,
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
import { ObjectPolicy } from "../object/access.ts";
import { ObjectTypePool, ObjectDescription, type KnownDeclaration } from "../object/description.ts";
import { ObjectController } from "../object/controller.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { Authorization } from "./authorization.ts";
import { ClaimController } from "../claim/controller.ts";
import { type ClaimPrimary, Reservation } from "../claim/reservation.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import { ExternalStorage } from "./external.ts";
import { ObjectSource } from "./source.ts";
import { ObjectSink } from "./sink.ts";
import { Provisioned } from "../trait/provisioned.ts";
import { ResourceController, type ResourceControllerOptions } from "./provisioned.ts";
import { ExpiringController } from "./expiring.ts";
import { RecoverableController } from "./recoverable.ts";
import { TrackedHistory } from "./tracked.ts";
import type { Controller, Delta } from "@destack/db";
import { Settlement } from "./settlement.ts";
import { TelemetryScope } from "@destack/telemetry";
import { Claimant } from "../claim/claim.ts";
import type { StoreExecution } from "../method/execution.ts";
import type { HookSandbox } from "../hook/sandbox.ts";
import type {
    AdapterFetch,
    PackageImplementation,
    ObjectPackage,
    ImplementationInput,
} from "./implementation.ts";
import type { Clock, Environment } from "@destack/environment";

import { installation } from "../installation/installation.ts";
import { mapInstalledPackages } from "../installation/package.ts";
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

/** A database's replica: the subscription following it, and the publisher streaming its pages. */
export interface SubscriberEntry {
    /** The subscription following the replica. */
    readonly subscription: sync.Subscription;
    /** The publisher streaming the replica's pages, one step closer to its rows' home. */
    readonly publisher: sync.Publisher;
    /** Whether a use acquired the replica's key, as on-demand follows do, not a declaration. */
    readonly isAcquired: boolean;
}

/** How a store follows its replicas: the tables relisting them, and the uplink taking changes. */
export interface SubscriberOptions {
    /** The tables whose changes relist subscriptions, beside scopes and residents' homes. */
    readonly watches?: readonly Table[];
    /** The link to the replicas' store, receiving this store's changes to them, if it sends any. */
    readonly uplink?: sync.Uplink;
}

/** The subscriptions a database follows for its replicas, each from its publisher. */
export interface Subscriber extends SubscriberOptions {
    /** List the subscriptions of the store's chains and follows, reading through a view. */
    readonly subscriptions: (view: Snapshot, delta?: Delta) => Promise<readonly SubscriberEntry[]>;
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
          readonly followed: readonly sync.Subscription[];
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
    /** The principal the call records as its caller, the caller's when absent. */
    readonly as?: Subject;
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
    /** The principal the call records as its caller, the authorization's when absent. */
    readonly as?: Subject;
    /** The calls the method's expansion ran before it. */
    readonly expansion?: readonly BranchEntry[];
    /** The request the call belongs to. */
    readonly request?: CallRequest;
    /** The code making the call for a principal's code, such as a controller's name. */
    readonly component?: string;
    /** The declaration the call writes for, if any. */
    readonly manager?: Manager;
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

/** What a store runs with once composed from its servers. */
type ComposedOptions = StoreComposition & CallLog;

/** A package a store serves: its handle, definition, types and serving installation. */
interface StorePackage {
    /** The package. */
    readonly package: ObjectPackage;
    /** The package's code. */
    readonly definition: ImplementationInput;
    /** The package's types, its handled ones in place of their declared defaults. */
    readonly objects: readonly ObjectType[];
    /** The installation serving the package, absent for the platform's store. */
    readonly server: ServerContext | undefined;
}

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
    /** The principal of the store's work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The partitions the store serves, its own first: its installation's, else its scope. */
    readonly partitions?: readonly string[];
    /** The package recording the calls. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimPrimary;
    /** The installations serving objects and their projections, by type reference key. */
    readonly installed?: ReadonlyMap<string, ServerContext>;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
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
    /** The scope's branch types, among the objects, whose branches reads can see. */
    readonly branch?: BranchType;
    /** The providers reconciling the served kinds' resources, if the store reconciles any. */
    readonly provisioned?: ResourceControllerOptions;
}

/** A store's routes: each served object's procedures by its key, and the store procedures. */
export type ObjectRouter = Readonly<Record<string, Router<ProcedureRouter, RequestContext>>> & {
    readonly store: Router<StoreProcedures, RequestContext>;
};

/** A store serving object types' methods over one database. */
export class ObjectStore {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The types whose queries run over rows replicated here, never followed nor changed here. */
    readonly replicas: readonly ObjectType[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The principal of the store's work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The partitions the store serves, its own first, none for its principal's scope alone. */
    readonly partitions: readonly string[];
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
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
    /** The files with the served external objects. */
    readonly external: readonly ExternalStorage[];
    /** The subscriptions the database follows to maintain its replicas. */
    readonly subscriber: Subscriber | undefined;
    /** Read how the store has its scope's chain: home, follower, pending, or absent with none. */
    readonly chain: (view: Snapshot) => Promise<ChainOptions | PendingChain | undefined>;
    /** The durable types the store serves or decides, whose rows stand for their principals. */
    readonly decided: readonly ObjectType[];
    /** The on-demand follows of the rows standing for callers, acquired where they are missing. */
    readonly standing: readonly Extract<Follow, { readonly onDemand: unknown }>[];
    /** The types follower replicas compile against: the followed shapes' and the decided. */
    readonly followed: readonly ObjectType[];
    /** The served durable object types. */
    readonly durable: readonly ObjectType[];
    /** The publications the store serves and follows beside its objects' and policies' ones. */
    readonly publications: readonly sync.Publication[];
    /** The installations serving objects and their projections, by type reference key. */
    readonly #installed: ReadonlyMap<string, ServerContext>;
    /** The served or authorized durable types whose rows the database replicates from home. */
    readonly replicated: readonly ObjectType[];
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
    /** The scope's branch types, absent for a store without branches. */
    readonly branch: BranchType | undefined;
    /** The served kinds' resources, each controlled by the loop its trait builds. */
    readonly #provisioned: readonly ObjectType[];
    /** The providers reconciling the served kinds' resources, none for a store reconciling none. */
    readonly providers: ResourceControllerOptions["providers"];
    /** The declarations installed object types resolve against: the types, policies and kinds. */
    readonly known: readonly KnownDeclaration[];
    /** Whether reads of the objects record access events. */
    readonly isAccessAudited: boolean;
    /** The replicas the objects stream to clients and databases below. */
    readonly source: ObjectSource;
    /** The replicas the store follows into its database. */
    readonly sink: ObjectSink;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, TupleReader>();
    /** The types the store's packages declare, without the chunk types serving their text. */
    readonly #declared: readonly ObjectType[];
    /** Each row follow's keys by source row as last listed, with what each key read. */
    readonly #rowKeys = new Map<Follow, Map<string, RowKey>>();
    /** The served packages' implementations, in the order of their servers. */
    readonly packages: readonly (PackageHandler & { readonly objects: ObjectStore })[];
    /** The wait for the replicas gating readiness, shared by every implemented package. */
    #readiness: Promise<boolean> | undefined;

    /** Serve object types over one database. */
    constructor(given: ObjectStoreOptions) {
        // compose the servers' packages, with the principal and partition the follows place replicas below
        const { composed, served } = ObjectStore.#compose(given, () => this);
        this.principal = composed.principal;
        this.partitions = composed.partitions ?? [];
        this.standing = served.flatMap((each) => each.definition.standing ?? []);

        // add the referenced and followed types and the follows
        const options = this.#following(ObjectStore.#referencing(composed));
        const shapes = [...new Set((composed.follows ?? []).flatMap((each) => Follow.types(each)))];

        // require each object type once apart from the shared procedures, and the stores its storage needs
        this.#declared = options.objects;
        this.objects = ObjectType.served(this.#declared);
        this.durable = this.objects.filter((object) => object.storage === "durable");
        this.replicas = composed.replicas ?? [];
        // validate each served type once, as describing it at build would
        for (const type of options.objects) {
            type.requireValid();
        }
        ObjectStore.#requireRoutes(options.objects);
        ObjectStore.#requireStores(this.objects, options);

        // store the optional stores and machines
        this.claims = options.claims;
        this.subscriber = options.subscriber;
        this.chain = options.chain;
        this.publications = options.publications ?? [];
        this.#installed = options.installed ?? new Map();
        this.uplink = this.#uplink();
        this.ephemeral = options.ephemeral;
        this.external = options.external ?? [];

        // build the authorizer over the objects and the scope types enclosing them
        this.database = options.database;

        // authorize the replicas the policies do not already decide
        const policed = new Set(
            (options.policies ?? []).map((each) => ObjectStore.#policyKey(each)),
        );
        const unpoliced = this.replicas.filter(
            (type) => !policed.has(ObjectStore.#policyKey(type)),
        );
        const partition = this.home;
        this.authorizer = ObjectStore.#authorizer([...this.objects, ...unpoliced], {
            ...options,
            ...(partition === undefined ? {} : { partition }),
        });
        TrackedHistory.require(this.objects, this.authorizer);

        // store the journal, the audit and the request context
        this.#journal = ObjectStore.#journalOf(this.#declared, options);
        this.origin = { ...options.origin, package: Package.of(options.origin.package) };
        this.audit = (scope, caller, recording) =>
            AuditRecorder.scoped(this.journal, this.origin)(scope, caller, recording);
        this.isAccessAudited = options.isAccessAudited ?? false;
        this.source = new ObjectSource(this);
        this.sink = new ObjectSink(this);
        this.environment = options.environment;
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            options.environment.telemetry,
        );
        this.branch = options.branch;
        this.#report = options.report ?? ((error) => ObjectStore.#rethrow(error));
        this.#provisioned = ObjectStore.#control(this.objects, options.provisioned);
        this.providers = options.provisioned?.providers ?? [];

        // follow the durable tables in one feed
        const followed = ObjectStore.#follow(this.objects, options);
        this.decided = followed.decided;
        this.followed = [...shapes, ...followed.decided];
        this.replicated = followed.replicated;
        this.feed = followed.feed;
        this.known = ObjectStore.known(options.objects, options.policies ?? []);

        // implement each served package, the first one also running the store's controllers
        const servedIds = new Set(served.map((entry) => entry.package.id));
        this.packages = served.map((entry, index) =>
            this.#packageHandler(entry, index === 0, servedIds),
        );
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

    /** Add the types the served objects refer to, transitively, that the database replicates. */
    static #referencing(given: ComposedOptions): ComposedOptions {
        // add each referenced type of a replicated table
        const referenced = ObjectType.referenced(given.objects).filter((type) =>
            given.database.replicates(type.table),
        );

        return { ...given, policies: ObjectStore.#policies(given, referenced) };
    }

    /** List a store's policies with the types whose policy it maps no other way. */
    static #policies(
        given: ComposedOptions,
        types: readonly ObjectType[],
    ): NonNullable<ComposedOptions["policies"]> {
        // add each type once, unless an object or policy already maps its policy
        const mapped = new Set(
            [...given.objects, ...(given.policies ?? [])].map((each) =>
                ObjectStore.#policyKey(each),
            ),
        );
        const added = types.filter((type) => {
            // add a type whose policy nothing maps yet, once
            const key = ObjectStore.#policyKey(type);
            const isAdded = !mapped.has(key);
            mapped.add(key);

            return isAdded;
        });

        return [...(given.policies ?? []), ...added];
    }

    /** Add the follows' types to the policies, and the chains and follows to the subscriber. */
    #following(given: ComposedOptions): ComposedOptions & { readonly subscriber?: Subscriber } {
        // leave a root store, which follows nothing, without a subscriber
        const { follows = [], subscriber, publisher, ...rest } = given;
        if (follows.length === 0 && subscriber?.uplink === undefined && publisher === undefined) {
            return { ...rest, follows };
        }

        // place the replicas below the store's scope or partition, toward each scope's home
        const types = [...given.objects, ...(given.policies ?? [])].filter(
            (each) => each instanceof ObjectType,
        );
        const target = this.scope;
        const publisherOf = (scope: string) =>
            publisher?.(scope) ?? present(subscriber?.uplink, `the publisher of ${scope}`);

        // add the followed types and the subscriptions
        return {
            ...given,
            policies: ObjectStore.#policies(
                given,
                follows.flatMap((each) => Follow.types(each)),
            ),
            subscriber: {
                ...(subscriber?.uplink === undefined ? {} : { uplink: subscriber.uplink }),
                watches: [
                    ...(subscriber?.watches ?? []),
                    ...ObjectStore.#chainedTables(given, types),
                    // relist when followed rows change, which the host's chain reads
                    ...follows.flatMap((each) => Follow.types(each).map((type) => type.table)),
                    ...follows.flatMap((each) => ("from" in each ? [each.from.table] : [])),
                    // relist when the store acquires or evicts an on-demand key
                    ...(follows.some((each) => "onDemand" in each) ? [followKey] : []),
                ],
                subscriptions: async (view, delta) => {
                    // follow the follows' keys, then the chains
                    const followed = await ObjectStore.#followSubscriptions(this, follows, {
                        target,
                        publisher: publisherOf,
                        view,
                        ...(delta === undefined ? {} : { delta }),
                    });
                    const chained = (await this.sink.chains(target, view)).map(
                        ({ subscription, home }) => ({
                            subscription,
                            publisher: publisherOf(home),
                            isAcquired: false,
                        }),
                    );

                    return [...chained, ...followed];
                },
            },
        };
    }

    /** List the declarations installed types resolve against: served types, scopes and policies. */
    static known(
        objects: readonly ObjectType[],
        policies: readonly (Policy | ObjectType | TableMapping)[],
    ): KnownDeclaration[] {
        // add the scope types enclosing the served types, up to the universe
        const served = ObjectType.served(objects);
        const scopes = new Set(served.flatMap((type) => type.scopes));
        for (const scope of scopes) {
            scope.scopes.forEach((enclosing) => scopes.add(enclosing));
        }

        return [
            ...served,
            ...[...scopes].filter((scope) => !served.some((type) => type.same(scope))),
            ...policies.filter(
                (policy) => policy instanceof Policy || policy instanceof ObjectType,
            ),
        ];
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
        const pending = [...(types ?? this.#declared.map((type) => type.typeReference))];
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

    /** Follow durable tables and decision tables in one feed. */
    static #follow(
        objects: readonly ObjectType[],
        options: Pick<
            ComposedOptions,
            "database" | "policies" | "subscriber" | "environment" | "installed"
        >,
    ): {
        readonly decided: ObjectType[];
        readonly replicated: ObjectType[];
        readonly feed: sync.Feed;
    } {
        // find the durable types, and the replicated ones no installation here serves
        const decided = ObjectStore.#durable(objects, options.policies ?? []);
        const installed = options.installed ?? new Map();
        const replicated = decided.filter(
            (object) =>
                options.database.replicates(object.table) &&
                !installed.has(ObjectTypeReference.key(object.typeReference)),
        );
        const followed = decided.flatMap((object) => object.tables);
        const feed = new sync.Feed(
            options.database,
            [...new Set([...followed, ...decisionTables])],
            {},
        );

        return { decided, replicated, feed };
    }

    /** Stream served row replicas to in-process followers, and run the changes they send. */
    #uplink(): sync.Uplink {
        return {
            stream: (subscription, signal) => this.source.publish(subscription, signal),
            confirm: (subscription, watermark) => this.source.confirm(subscription, watermark),
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

    /** Control each served kind's resources with its trait's loop, if it has providers. */
    static #control(
        objects: readonly ObjectType[],
        provisioned: ResourceControllerOptions | undefined,
    ): ObjectType[] {
        return provisioned === undefined
            ? []
            : objects
                  .filter((object) => Provisioned.is(object))
                  .map((object) =>
                      object.implement({
                          controller: new ResourceController(object, provisioned),
                      }),
                  );
    }

    /** List the durable object types a store serves or decides. */
    static #durable(
        objects: readonly ObjectType[],
        policies: readonly (Policy | ObjectType | TableMapping)[],
    ): ObjectType[] {
        // list each durable type once, though several extensions name it
        const durable = [...objects, ...policies.filter((other) => other instanceof ObjectType)];

        return durable.filter(
            (object, index) =>
                object.storage === "durable" &&
                durable.findIndex((each) => each.same(object)) === index,
        );
    }

    /** Compose a store's partitions: its installation's own first, then the others it serves. */
    static #composedPartitions(
        own: string | undefined,
        served: readonly string[] | undefined,
    ): { readonly partitions?: readonly string[] } {
        // name none for a store serving its principal's scope alone
        if (own === undefined && served === undefined) {
            return {};
        }

        return {
            partitions: [...new Set([...(own === undefined ? [] : [own]), ...(served ?? [])])],
        };
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

    /** Build the authorizer over the objects, their scope types and other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        options: Pick<ComposedOptions, "database" | "policies" | "environment"> & {
            readonly partition?: string;
        },
    ): Authorizer {
        // map the types attached to the objects whose tables the database has beside them
        const stored = new Set(options.database.tables.map((table) => table[TABLE].sqlName));
        const attached: ObjectType[] = [];
        const pending = objects.flatMap((object) => object.attachments.map((each) => each.object));
        for (let type = pending.shift(); type !== undefined; type = pending.shift()) {
            const isKnown = [...objects, ...attached].some((each) => each.same(type));
            if (!isKnown && stored.has(type.table[TABLE].sqlName)) {
                attached.push(type);
                pending.push(...type.attachments.map((each) => each.object));
            }
        }

        // map packages onto their installations where the database stores them
        const sets = stored.has(installation.table[TABLE].sqlName)
            ? [mapInstalledPackages(installation.table)]
            : [];

        return ObjectPolicy.authorizer(
            [...objects, ...attached],
            options.policies ?? [],
            new Set(options.database.tables.filter((table) => options.database.replicates(table))),
            { partition: options.partition, sets },
        );
    }

    /** Open a method store's journal with its call key, refusing a call key without methods. */
    static #journalOf(
        routed: readonly ObjectType[],
        options: Pick<ComposedOptions, "database" | "callKey" | "environment" | "events">,
    ): Journal | undefined {
        // require the call key and the event log recording the calls of a store serving methods
        const isServing = routed.length > 0;
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

    /** Require each object type once, apart from the shared procedures. */
    static #requireRoutes(objects: readonly ObjectType[]): void {
        const routed = new Set<string>();
        for (const object of objects) {
            // refuse a type served twice, and one under the shared replica procedures' name
            const key = ObjectTypeReference.key(object.typeReference);
            if (routed.has(key)) {
                throw new TypeError(
                    `object ${object.name} of ${object.package.id} is served twice`,
                );
            } else if (Object.hasOwn(object.shared, object.key)) {
                throw new TypeError(
                    `object ${object.name} has the same name as the shared ${object.key} procedures`,
                );
            }
            routed.add(key);
        }
    }

    /** Refuse another principal's local types, and require claims, memory stores and files. */
    static #requireStores(
        objects: readonly ObjectType[],
        options: Pick<
            ComposedOptions,
            "database" | "claims" | "ephemeral" | "events" | "external" | "principal"
        >,
    ): void {
        // refuse local objects outside the scope of the store's principal
        const local = objects.find(
            (object) =>
                object.storage === "local" &&
                !object.scopes.some((scope) => scope.policy.is(options.principal)),
        );
        if (local !== undefined) {
            throw new TypeError(`local object ${local.name} only exists in its scope's store`);
        }

        // require claims for indexed objects
        const indexed = objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.claims === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but has no claim type for their names`,
            );
        }

        // require a store for ephemeral objects
        const unstored = objects.find(
            (object) =>
                object.storage === "ephemeral" &&
                options.ephemeral?.objects.some((stored) => stored.same(object)) !== true,
        );
        if (unstored !== undefined) {
            throw new TypeError(`ephemeral object ${unstored.name} needs a store`);
        }

        // require the event log over the store's database for events types
        const unlogged = objects.find(
            (object) =>
                object.storage === "events" &&
                (options.events === undefined ||
                    !options.events.has(object.eventKind()) ||
                    options.events.database !== options.database),
        );
        if (unlogged !== undefined) {
            throw new TypeError(`events object ${unlogged.name} needs the store's event log`);
        }

        // require files for external objects
        const unfiled = objects.find(
            (object) =>
                object.storage === "external" &&
                !(options.external ?? []).some((storage) =>
                    storage.objects.some((stored) => stored.same(object)),
                ),
        );
        if (unfiled !== undefined) {
            throw new TypeError(`external object ${unfiled.name} needs files`);
        }
    }

    /** Implement a package's served objects. */
    implement(
        handle: PackageHandle,
        routed: readonly ObjectType[],
        controllers: readonly Controller[],
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
            controllers,
            ...(subscriber === undefined
                ? {}
                : {
                      ready: (signal) => this.#awaitReady(signal),
                      synced: (signal) => this.#awaitReady(signal),
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
        const wanted = shape.onDemandSubscription(scope, [parameters], this.scope, at.subject);
        const timeout = this.clock.timeout(ACQUIRE_MILLISECONDS);
        try {
            await this.database.log.until(
                () => this.#covers(wanted),
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

    /** Report whether a synced replica follows every query of a subscription of its keys. */
    async #covers(wanted: sync.Subscription): Promise<boolean> {
        // require a synced replica
        const recorded = await this.source.buildReplica(wanted).subscribed(this.database);
        const isSynced = await sync.Replica.isSynced(this.database, wanted.name, wanted.scope);
        if (recorded === undefined || !isSynced) {
            return false;
        }

        // require each wanted query among the recorded ones
        const has = QueriesParameters.parse(recorded.parameters).queries ?? {};
        const queries = QueriesParameters.parse(wanted.parameters).queries ?? {};

        return Object.entries(queries).every(
            ([name, query]) =>
                has[name] !== undefined && canonicalize(has[name]) === canonicalize(query),
        );
    }

    /** Wait for the replicas gating readiness, checked once for all packages, or until stopped. */
    async #awaitReady(signal: AbortSignal): Promise<void> {
        // share one check of the replicas, checking again after the checking server stopped first
        const stopped = new Promise<boolean>((resolve) => {
            signal.addEventListener("abort", () => resolve(false), { once: true });
        });
        while (!signal.aborted) {
            const readiness = (this.#readiness ??= this.#ready(signal));
            if (await Promise.race([readiness, stopped])) {
                return;
            }
            if (this.#readiness === readiness) {
                this.#readiness = undefined;
            }
        }
    }

    /** Wait until the chain's replicas are confirmed within the lag, checked on record changes. */
    async #ready(signal: AbortSignal): Promise<boolean> {
        // check the listed gates now and after each replica record commit
        const { database } = this;
        const records = new Set(sync.replicaRecordTables.map((table) => table[TABLE].sqlName));

        return database.log.until(async (committed) => {
            // leave commits that change no replica's record
            if (![...committed].some((table) => records.has(table))) {
                return false;
            }

            // wait for a pending chain, whose replicas the store lists once it is known
            const chain = await this.chain(Snapshot.live(database));
            if (chain?.kind === "pending") {
                return false;
            }

            // require each gating replica confirmed by its home within the lag
            const gates = await this.sink.gates();
            const since = this.clock.now() - this.authorizer.lag;
            const confirmed = await Promise.all(
                gates.map(({ subscription }) => sync.Replica.confirmedAt(database, subscription)),
            );

            return confirmed.every((at) => at !== undefined && at >= since);
        }, signal);
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
    router(types: readonly ObjectType[] = this.#declared): ObjectRouter {
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
                await this.enter(context, input.scope);
                await this.#waitForHomes(context, input.scope, input.homes);

                // confirm the caller's streams of the scope at the watermark
                const watermark = await this.watermark(input.scope);
                this.source.confirmCaller(input.scope, context.authentication, watermark);

                return watermark;
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
            confirm: store.confirm.handler(async ({ input, context }) => {
                // confirm the caller's subscription, as allowed to follow it
                await this.source.confirmRequest(input.subscription, input.watermark, context);

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
        await this.enter(context, scope);
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
        const recorder = this.audit(scope, caller, { package: object.package });
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
            return this.requireEphemeral().database;
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

    /** Run one platform call inside a running mutation, remote code's without internal methods. */
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
            (await Reservation.reserve(this.claims, claimants, this.objects, {
                id: run.id,
                environment: this.environment,
            }));
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
                for (const { author, ...entry } of listed) {
                    expanded.push({
                        ...this.#resolve(this.#place(entry, scope), true, CallSite.of(caller)),
                        as: author,
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
        await this.enter(context, scope);
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

        // confirm the caller's streams of the scope past its writes
        const watermark = await this.watermark(scope);
        this.source.confirmCaller(scope, context.authentication, watermark);

        return { outcomes, watermark };
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

    /** Decide whether a scope's rows of a type are replicas here, written only by their home. */
    async isReplica(object: ObjectType, snapshot: Snapshot, scope: string): Promise<boolean> {
        // serve the rows of a type an installation serves here, even with a replicated table
        const isInstalled = this.#installed.has(ObjectTypeReference.key(object.typeReference));

        // originate the inherited rows of a scope the store serves, such as a home's account's
        const isOriginated =
            object.inherited !== undefined && this.partitions.slice(1).includes(scope);

        return (
            !isOriginated &&
            object.storage === "durable" &&
            ((snapshot.database.replicates(object.table) && !isInstalled) ||
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

    /** List the controllers the served objects need: the store's, then each type's. */
    controllers(): readonly Controller[] {
        return [
            ...this.#storeControllers(),
            ...this.#syncing(),
            ...this.#objectControllers(() => true),
        ];
    }

    /** List the controllers syncing the store's replicas. */
    #syncing(): Controller[] {
        return this.subscriber === undefined ? [] : [this.sink.controller(this.subscriber)];
    }

    /** List the controllers of the types a filter selects by their package's id. */
    #objectControllers(isSelected: (packageId: PackageId) => boolean): ReconcileController[] {
        return [...this.objects, ...this.#provisioned].flatMap((object) =>
            object.controller === undefined || !isSelected(object.package.id)
                ? []
                : [ObjectController.control(this, object, object.controller)],
        );
    }

    /** List the store's controllers, from compaction to its branch. */
    #storeControllers(): Controller[] {
        // compact the log, and pick the controllers the objects need
        const isRecoverable = this.objects.some(
            (object) => object.lifecycle.recoverable !== undefined,
        );
        const isExpiring = this.objects.some((object) => object.lifecycle.expiring !== undefined);
        const isSettled = this.objects.some((object) =>
            Object.values(object.methods).some((method) => Method.settles(method)),
        );

        return [
            new CompactionController(this.database, this.clock),
            ...(this.#journal === undefined
                ? []
                : [this.#journal.controller(), ...this.#journal.events.controllers]),
            ...(this.claims === undefined
                ? []
                : [
                      new ClaimController(
                          this.claims,
                          this.database,
                          this.objects,
                          this.environment,
                      ),
                  ]),
            ...(isRecoverable ? [new RecoverableController(this)] : []),
            ...(isExpiring ? [new ExpiringController(this)] : []),
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
        const key = `${scope} ${Caller.key(caller)}`;
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

    /** Enter a scope: follow the caller's row, require the scope, and wait for seen writes. */
    async enter(context: RequestContext, scope: string): Promise<void> {
        // follow the row standing for the caller where it is missing
        await this.source.stand(context.access(scope).subject, context.signal);

        // enter the requested scope, or any without one, at once without watermarks to wait for
        const requested = context.scope;
        const { watermarks } = context.bookmark;
        const isRequested = requested === undefined || requested === scope;
        if (isRequested && watermarks.length === 0) {
            return;
        }

        // read the scope's row, refusing one outside the request's scope or not here yet
        const [row] = await Scope.read(Snapshot.live(this.database), [scope]);
        if (!isRequested && (row === undefined || !Scope.within(row, requested))) {
            throw ObjectError.of(
                "SCOPE_OUTSIDE_REQUEST",
                `scope ${scope} is outside the request's scope`,
            );
        }

        // wait for the caller's watermarks along the scope's chain
        const chain = new Set([scope, ...(row?.ancestors ?? [])]);
        await this.#waitForAll(
            context,
            watermarks.filter((entry) => chain.has(entry.scope)),
        );
    }

    /** Wait until a scope's chain replicas have their homes' commits, refusing other homes. */
    async #waitForHomes(
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
        await this.#waitForAll(context, homes);
    }

    /** Read a scope and the scopes enclosing it, by identifier. */
    async #chainIds(scope: string): Promise<Set<string>> {
        const chain = await Scope.chain(Snapshot.live(this.database), scope);

        return new Set([scope, ...chain.map((link) => link.object.id)]);
    }

    /** Wait until the scope's log or its replicas have each watermark, in a bounded wait each. */
    async #waitForAll(context: RequestContext, watermarks: readonly Watermark[]): Promise<void> {
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

    /** Read the position the store's log has now, under a scope its commits wrote. */
    async watermark(scope: string): Promise<Watermark> {
        return { scope, ...(await this.database.log.position()) };
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
            .filter(
                ({ subscription, replica }) =>
                    subscription.scope === watermark.scope ||
                    replica.reads.includes(watermark.scope),
            )
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
        // wait through the replicas reading the scope, asking upstream for a position they miss
        else {
            const { including, followed } =
                history.kind === "replicated" ? history : { including: [], followed: [] };
            if (!(await sync.Replica.isAt(this.database, watermark.scope, watermark, followed))) {
                await this.sink.request(watermark, [...including, ...followed]);
            }

            return await sync.Replica.wait(
                this.database,
                watermark.scope,
                watermark,
                signal,
                followed,
            );
        }
    }

    /** Read the server context serving a type inside the store, absent for a type it serves. */
    readServer(object: ObjectType): ServerContext | undefined {
        return this.#installed.get(ObjectTypeReference.key(object.typeReference));
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
        options: Pick<CallOptions, "as" | "expansion" | "requestId"> = {},
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
        const targetId = id === undefined ? undefined : schema.string().parse(id);
        const snapshot = await this.#view(database, authorization, scope, at, branch);
        const execution: StoreExecution = {
            kind: "store",
            store: this,
            authorization,
            transaction,
            view: snapshot ?? transaction,
        };
        const { now } = authorization.access.context;
        const context: MethodContext = new MethodContext({
            object,
            name,
            method,
            scope,
            chain: authorization.chain,
            input: fields,
            id: targetId,
            target: undefined,
            prepared: undefined,
            caller:
                options.as === undefined
                    ? { ...caller, now }
                    : AccessContext.of(Caller.of(options.as), now),
            ...(options.expansion === undefined ? {} : { expansion: options.expansion }),
            ...(options.requestId === undefined ? {} : { requestId: options.requestId }),
            location: execution,
            objects: this.objects,
            replicas: this.replicas,
        });

        // refuse writing a predefined row, then load the permitted target at the given revision
        if (method.mutates && targetId !== undefined) {
            object.refusePredefined(targetId);
        }
        const target = await ObjectStore.#target(context, execution, targetId, snapshot);
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw ObjectError.of("REVISION_STALE", `${object.name} revision has changed`);
        }

        // refuse callers' other methods on a trashed object
        const isTrashed =
            object.lifecycle.recoverable !== undefined &&
            target !== undefined &&
            target["deletedAt"] !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.isInternal !== true) {
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
            run: this.#invoker(transaction, authorization, context, options),
        });

        // authorize the call unless prepared, then execute it, hiding a denial the caller cannot see
        let executed: unknown;
        try {
            if (prepared === undefined) {
                await method.authorize?.(called, execution);
            }
            executed = this.#isAskedAtHome(object, method)
                ? await this.#askHome(object, name, input, scope, caller)
                : await method.execute(called, execution);
        } catch (error) {
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
            id,
            presented,
            caller,
            options.request,
            options.component,
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
        options: CallOptions,
    ): Run {
        // pass the call's options down, with the transaction's authorizations of nested calls
        const { from, writerId, requestId, component } = options;
        const { authorizations } = options;
        const recorded: CallOptions = {
            ...(from === undefined ? {} : { from }),
            ...(writerId === undefined ? {} : { writerId }),
            ...(requestId === undefined ? {} : { requestId }),
            ...(component === undefined ? {} : { component }),
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
            package: object.package,
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
        id: string | undefined,
        presented: unknown,
        caller: AccessContext,
        request: CallRequest | undefined,
        component: string | undefined,
    ): void {
        // describe the call's target and outcome
        const { method, scope, name } = context;
        const isAudited = method.mutates && object.storage === "durable";
        const target = id === undefined ? { type: "scope", id: scope } : { type: object.name, id };
        const recorded: {
            target: typeof target;
            details: {};
            outcome: sync.Outcome;
        } = {
            target,
            details: {},
            outcome: {
                kind: "success",
                ...(request === undefined ? {} : { value: schema.json().parse(presented ?? null) }),
            },
        };

        // audit a durable change, or journal a request's other call for retries, once the mutation commits under its transaction
        const recorder = this.audit(scope, caller, {
            package: object.package,
            ...(component === undefined ? {} : { component }),
        });
        transaction.record(async (committing) => {
            const stamp = await committing.log.transaction();
            const stamped =
                request === undefined || stamp === undefined
                    ? request
                    : { ...request, transaction: stamp };
            if (isAudited) {
                await recorder.record(
                    committing,
                    object.audit(name),
                    recorded,
                    "activity",
                    stamped,
                );
            } else if (stamped !== undefined) {
                await recorder.recordRead(committing, object.audit(name), recorded, stamped);
            }
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

    /** Compose a store's servers and sandboxes, and list the packages it serves. */
    static #compose(
        given: ObjectStoreOptions,
        store: () => ObjectStore,
    ): { readonly composed: ComposedOptions; readonly served: readonly StorePackage[] } {
        // require the machine running any provisioned resources, and one uplink
        const { servers, sandboxes = [], uplink, machine, publisher, server, ...rest } = given;
        const { policies = [], branch } = rest;
        const implementations = servers.map((each) =>
            ObjectStore.#packageOf(each, each.installation ?? server),
        );
        const providers = implementations.flatMap((each) => each.definition.providers ?? []);
        if (providers.length > 0 && machine === undefined) {
            throw new TypeError("provisioned resources need a machine");
        } else if (server !== undefined && uplink !== undefined) {
            throw new TypeError("an installation's replicas must leave through its publisher");
        }

        // serve the sandboxed installations' packages, declared against what the servers make known
        const objects = implementations.flatMap((each) => each.objects);
        const followed = implementations
            .flatMap((each) => ObjectStore.#follows(each.definition))
            .flatMap((each) => Follow.types(each));
        const known = ObjectStore.known(objects, [
            ...policies,
            ...ObjectType.referenced(objects),
            ...followed,
        ]);
        const served = [
            ...implementations,
            ...sandboxes.flatMap((sandbox) => ObjectStore.#sandboxPackages(sandbox, known)),
        ];

        // route every package's types, answering queries of replicated types nothing here serves
        const routes = ObjectStore.#routes(served);

        // refuse a package served twice, whose servers would share one package's routes
        const servedIds = new Set<string>();
        for (const each of served) {
            if (servedIds.has(each.package.id)) {
                throw new TypeError(`package ${each.package.name} is served twice`);
            }
            servedIds.add(each.package.id);
        }
        const replicas = known
            .filter((type) => type instanceof ObjectType)
            .filter(
                (type) =>
                    !routes.has(ObjectTypeReference.key(type.typeReference)) &&
                    given.database.replicates(type.table),
            );
        const routed = [...routes.values()].map((route) => route.type);
        if (branch !== undefined && !routed.some((object) => object.same(branch.object))) {
            throw new TypeError(`no server handles ${branch.object.name}, the branch object`);
        }
        const follows = served.flatMap((each) => ObjectStore.#follows(each.definition));
        const external = served.flatMap((each) =>
            each.definition.external === undefined ? [] : [each.definition.external],
        );

        // serve each type at its package's installation, else at the store's, refusing one served at none
        const installed = new Map(
            [...routes].map(([key, { type, entry }]) => {
                const context = entry.server ?? server;
                if (context === undefined) {
                    throw new TypeError(`${type.name} is served at no installation`);
                }

                return [key, context] as const;
            }),
        );

        // follow a partition's chain at its installation's root
        const isPartition = server?.partition !== undefined && server.partition !== server.scope;
        const partitionPublisher =
            server !== undefined && isPartition
                ? ObjectStore.#partitionPublisher(server, store, publisher)
                : publisher;
        const composed: ComposedOptions = {
            ...rest,
            installed,
            ...(replicas.length === 0 ? {} : { replicas }),
            principal:
                server === undefined
                    ? present(given.principal, "the store's principal")
                    : principal.installation.reference(server.scope, server.installation),
            ...ObjectStore.#composedPartitions(server?.partition, given.partitions),
            objects: routed,
            policies,
            ...(external.length === 0 ? {} : { external }),
            ...(follows.length === 0 ? {} : { follows }),
            ...(partitionPublisher === undefined ? {} : { publisher: partitionPublisher }),
            ...(providers.length === 0 || machine === undefined
                ? {}
                : { provisioned: { providers, machine } }),
            subscriber:
                server === undefined
                    ? uplink === undefined
                        ? {}
                        : { uplink }
                    : { uplink: server.publisher },
        };

        return { composed, served };
    }

    /** List a package's follows, its standing ones among them. */
    static #follows(definition: ImplementationInput): readonly Follow[] {
        return [...(definition.follows ?? []), ...(definition.standing ?? [])];
    }

    /** Read a server's package for an installation, its handled types over their defaults. */
    static #packageOf(
        server: PackageImplementation,
        context: ServerContext | undefined,
    ): StorePackage {
        // refuse a handled type of the package that its handle neither declares nor its providers provision
        const definition = server.definition(context);
        const handled = definition.objects ?? [];
        const served = Object.values(server.package.objects);
        const provisioned = (definition.providers ?? []).map((provider) => provider.object);
        const others = handled.filter((type) => !served.some((each) => each.same(type)));
        const stray = others.find(
            (type) =>
                type.package.id === server.package.id &&
                !provisioned.some((each) => each.same(type)),
        );
        if (stray !== undefined) {
            throw new TypeError(`${server.package.name} does not declare object ${stray.name}`);
        }

        // serve the declared types, external ones only with their files, beside other packages' handled types
        const filed = definition.external?.objects ?? [];
        const declared = served
            .filter((type) => type.storage !== "external" || filed.some((each) => each.same(type)))
            .map((type) => handled.find((each) => each.same(type)) ?? type);

        return {
            package: server.package,
            definition,
            objects: [...declared, ...others],
            server: context,
        };
    }

    /** Route each package's declared types to it, refusing two packages implementing one. */
    static #routes(
        served: readonly StorePackage[],
    ): ReadonlyMap<string, { readonly type: ObjectType; readonly entry: StorePackage }> {
        // implement each package's declared types, once
        const routes = new Map<
            string,
            { readonly type: ObjectType; readonly entry: StorePackage }
        >();
        for (const entry of served) {
            for (const type of entry.objects) {
                const key = ObjectTypeReference.key(type.typeReference);
                const earlier = routes.get(key);
                if (earlier !== undefined) {
                    throw new TypeError(
                        `${ObjectStore.#servedBy(earlier.entry)} and ${ObjectStore.#servedBy(entry)} both implement ${type.name} objects of ${type.package.id}`,
                    );
                }
                routes.set(key, { type, entry });
            }
        }

        return routes;
    }

    /** Describe who serves a package: its installation, else the package itself. */
    static #servedBy(entry: StorePackage): string {
        return entry.server === undefined
            ? `package ${entry.package.name}`
            : `installation ${entry.server.installation}`;
    }

    /** Serve an installation's package, running the code of its types in its sandbox. */
    static #sandboxPackages(
        sandbox: HookSandbox,
        known: Iterable<KnownDeclaration>,
    ): StorePackage[] {
        // declare the installation's types and index them beside the known ones by reference
        const { declarations, server } = sandbox;
        const platform = [...known];
        const types = ObjectTypePool.of(platform).declare(
            ObjectDescription.installed(declarations),
        );
        const byReference = new Map(
            [...platform, ...types]
                .filter((declared) => declared instanceof ObjectType)
                .map((type) => [ObjectTypeReference.key(type.typeReference), type]),
        );
        const owner = PackageManifest.package(server.build.manifest);
        const local = declarations.filter(
            (entry) => graph.Moniker.packageId(entry.symbol) === owner.id,
        );

        // route each described type, implementing the package's types in the sandbox
        const route = (routes: PackageDescription["objects"]): Record<string, ObjectType> =>
            Object.fromEntries(
                Object.entries(routes).map(([name, routed]) => {
                    // refuse a route of a type nothing here declares
                    const type = byReference.get(ObjectTypeReference.key(routed.type));
                    if (type === undefined) {
                        throw new TypeError(
                            `installation ${server.installation} routes ${routed.type.type} objects of ${routed.type.packageId}, but nothing here implements or replicates them`,
                        );
                    }

                    // implement the package's types in the sandbox and route other types as declared
                    if (routed.type.packageId === owner.id) {
                        return [name, sandbox.implement(type, routed.hooks)];
                    } else if (Object.keys(routed.hooks).length > 0) {
                        throw new TypeError(
                            `installation ${server.installation} cannot hook ${type.name} of ${type.package.id}: the type's package implements it`,
                        );
                    }

                    return [name, type];
                }),
            );

        // serve the package's objects, refusing adapters a sandbox has no code to answer
        return local
            .filter((entry) => entry.kind === PACKAGE_DECLARATION)
            .map((entry) => PackageDescription.parse(entry.description))
            .flatMap((described): StorePackage[] => {
                // refuse adapters, and leave a package of no objects unserved
                const adapters = Object.keys(described.adapters);
                if (adapters.length > 0) {
                    throw new TypeError(
                        `installation ${server.installation} declares adapters ${adapters.join(", ")}, but a sandbox cannot run them`,
                    );
                } else if (Object.keys(described.objects).length === 0) {
                    return [];
                }

                // implement the package's objects as the installation
                const objects = route(described.objects);
                const since = described.since === undefined ? {} : { since: described.since };
                const handle = PackageHandle.of(owner, { objects, ...since });

                return [
                    { package: handle, definition: {}, objects: Object.values(objects), server },
                ];
            });
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
        entry: StorePackage,
        isFirst: boolean,
        servedIds: ReadonlySet<PackageId>,
    ): PackageHandler & { readonly objects: ObjectStore } {
        // run the package's types' controllers, the first package also the store's and unserved types'
        const { controllers = [], adapters = {}, access, responseHeaders } = entry.definition;
        const listed = [
            ...(isFirst ? this.#storeControllers() : []),
            ...this.#objectControllers(
                (packageId) =>
                    packageId === entry.package.id || (isFirst && !servedIds.has(packageId)),
            ),
        ];
        const built = controllers.map((controller) => controller(this));
        const implemented = this.implement(entry.package, entry.objects, [...listed, ...built]);

        // answer the package's adapters below its mount
        const fetch = ObjectStore.#adapterFetch(entry.package, adapters, this);

        // run the controllers of a package served at an installed installation once its host activates it, a base one's at once
        const { server } = entry;
        const activation =
            server === undefined || server.origin === "base" ? undefined : server.activation;

        return {
            ...implemented,
            ...(isFirst ? { syncing: this.#syncing() } : {}),
            ...(access === undefined ? {} : { access }),
            ...(responseHeaders === undefined ? {} : { responseHeaders }),
            ...(fetch === undefined ? {} : { fetch }),
            ...(activation === undefined ? {} : { activation }),
        };
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
            // find the adapter whose path the request is at or below
            const url = new URL(request.url);
            const adapter = adapters.find(
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

            // answer the request below the adapter's path
            url.pathname = url.pathname.slice(adapter.path.length) || "/";

            return adapter.fetch(copyRequest(request, {}, url.href), caller, context, store);
        };
    }

    /** Key a type's policy by its package and name, the identity the authorizer maps once. */
    static #policyKey(declared: Policy | ObjectType | TableMapping): string {
        const { policy } = declared instanceof Policy ? { policy: declared } : declared;

        return `${policy.definition.packageId} ${policy.name}`;
    }

    /** List the replicated scope tables served types live in or relate to, which relist chains. */
    static #chainedTables(given: ComposedOptions, types: readonly ObjectType[]): Table[] {
        const served = given.objects.filter((each) => each instanceof ObjectType);

        return types
            .filter(
                (type) =>
                    given.database.replicates(type.table) &&
                    served.some((object) =>
                        [...object.scopes, ...object.related].some((each) => each.same(type)),
                    ),
            )
            .map((type) => type.table);
    }

    /** List the subscriptions of the follows' keys, from the publisher toward each scope's home. */
    static async #followSubscriptions(
        store: ObjectStore,
        follows: readonly Follow[],
        options: {
            readonly target: string;
            readonly publisher: (scope: string) => sync.Publisher;
            readonly view: Snapshot;
            readonly delta?: Delta;
        },
    ): Promise<SubscriberEntry[]> {
        // subscribe to the acquired keys of each on-demand follow, one replica per scope and subject
        const onDemand = await Promise.all(
            follows
                .filter((follow) => "onDemand" in follow)
                .map((follow) => ObjectStore.#onDemandSubscriptions(store, follow, options)),
        );

        // read each shape follow's keys, leaving the chain to the sink and on-demand keys to their uses
        const keyed = (
            await Promise.all(
                follows.map(async (follow) => {
                    // leave a chain or an on-demand follow, which has no keys
                    if ("chain" in follow || "onDemand" in follow) {
                        return [];
                    }
                    // take a follow's fixed keys
                    else if ("keys" in follow) {
                        return [{ follow, keys: follow.keys }];
                    }
                    // read a follow's keys from the store's rows
                    else {
                        return [
                            {
                                follow,
                                keys: await store.#readKeys(follow, options.view, options.delta),
                            },
                        ];
                    }
                }),
            )
        ).flat();

        // subscribe to each key of each shape
        const subscribed = await Promise.all(
            keyed.map(async ({ follow, keys }) => {
                // subscribe to each key toward its scope's home, decided for its principal
                const { shape } = follow;
                const derived = keys.map(({ key, subject }) => ({
                    subscription: shape.subscription(key, options.target, subject),
                    home: key.scope,
                }));

                return derived.map(({ subscription, home }) => ({
                    subscription,
                    publisher: options.publisher(home),
                    isAcquired: false,
                }));
            }),
        );
        const entries = [...onDemand.flat(), ...subscribed.flat()];

        return [
            ...new Map(
                entries.map((entry) => [
                    `${entry.subscription.name} ${entry.subscription.scope}`,
                    entry,
                ]),
            ).values(),
        ];
    }

    /** List an on-demand follow's live keys as one subscription per scope and subject. */
    static async #onDemandSubscriptions(
        store: ObjectStore,
        follow: Extract<Follow, { readonly onDemand: unknown }>,
        options: {
            readonly target: string;
            readonly publisher: (scope: string) => sync.Publisher;
            readonly view: Snapshot;
        },
    ): Promise<SubscriberEntry[]> {
        // read the keys that have not lapsed
        const rows = await options.view.read(followKey).findMany({
            where: { shape: follow.shape.name, expiresAt: { gt: store.clock.now() } },
        });

        // group their parameters by scope and subject
        const groups = new Map<string, { scope: string; subject: string; keys: object[] }>();
        for (const key of rows) {
            const group = `${key.scope} ${key.subject}`;
            const entry = groups.get(group) ?? {
                scope: key.scope,
                subject: key.subject,
                keys: [],
            };
            entry.keys.push(follow.shape.parameters.parse(JSON.parse(key.parameters)));
            groups.set(group, entry);
        }

        // subscribe to each group toward its scope's home
        return [...groups.values()].map(({ scope, subject, keys }) => ({
            subscription: follow.shape.onDemandSubscription(
                scope,
                keys,
                options.target,
                ObjectStore.#onDemandSubject(store, subject),
            ),
            publisher: options.publisher(scope),
            isAcquired: true,
        }));
    }

    /** Read a followed key's subject, none for the store's principal. */
    static #onDemandSubject(store: ObjectStore, subject: string): Subject | undefined {
        const parsed = Subject.parse(JSON.parse(subject));

        return Subject.key(parsed) === Subject.key(store.principal) ? undefined : parsed;
    }

    /** Read a follow's keys over the store's rows, again only for rows a change touched. */
    async #readKeys(
        follow: RowFollow<object, ObjectType>,
        view: Snapshot,
        delta: Delta | undefined,
    ): Promise<FollowKey[]> {
        // read the rows of the store's database the keys come from, in identifier order
        const table = follow.from.table;
        const rows = await view
            .read(table)
            .findMany({ where: follow.where ?? {}, orderBy: { id: "asc" } });

        // list the source rows the commit changed, whose keys read again
        const changed = new Set(
            (delta?.changes ?? [])
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
                    delta !== undefined &&
                    known !== undefined &&
                    !changed.has(name) &&
                    !delta.changes.some((change) => known.reads.touches(change)) &&
                    !known.reads.tables.some((read) => delta.unlogged.has(read[TABLE].sqlName));
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
