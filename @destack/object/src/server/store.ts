import { graph, Package } from "@destack/package";
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
    accessRelationship,
} from "@destack/access";
import {
    type ObjectReference,
    ObjectTypeReference,
    Scope,
    type ScopeLink,
    Subject,
} from "@destack/sync";
import {
    LogPosition,
    Snapshot,
    TABLE,
    type DatabaseConnection,
    Row,
    type Table,
    DatabaseError,
    Transaction,
    and,
    eq,
    gt,
    lte,
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
import type { CallKey } from "@destack/runtime/request";

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
} from "@destack/schema";
import type { ServerContext } from "@destack/runtime/workload";
import type { Watermark } from "@destack/runtime/bookmark";
import { CompactionController } from "@destack/runtime/database";
import { refuseInput, CallError } from "@destack/runtime/error";
import { copyRequest, RequestId, type RequestIdentity } from "@destack/runtime/request";
import { v7 } from "uuid";
import {
    implement,
    type Router,
    type ServerAccess,
    type RequestContext,
    type PackageImplementation,
    type ProcedureContext,
} from "@destack/runtime/server";
import {
    Egress,
    PACKAGE_DECLARATION,
    PackageDescription,
    PackageHandle,
    type ProcedureRouter,
} from "@destack/runtime";
import * as sync from "@destack/sync";
import { type Follow, type FollowKey, type RowFollow } from "../replica/shape.ts";
import { followKey } from "../replica/key.ts";
import { QueriesParameters } from "../replica/replica.ts";
import { Branch, type BranchEntry, type BranchType } from "../branch/index.ts";
import { MethodContext, type ResultsOf, type Run } from "../method/context.ts";
import { CallSite } from "../method/site.ts";
import { Limits } from "../method/limits.ts";
import { Method, type MethodKind } from "../method/method.ts";
import { callableProcedures } from "../method/procedure.ts";
import { InputSchema } from "../method/input.ts";
import {
    WriterId,
    storeProcedures,
    type PushResult,
    type StoreProcedures,
} from "../replica/replica.ts";
import { ACCESS_KINDS, ObjectType } from "../object/object.ts";
import { INTRINSIC } from "../object/intrinsic.ts";
import { ObjectPolicy } from "../object/access.ts";
import { ObjectTypePool, ObjectDescription, type KnownDeclaration } from "../object/description.ts";
import { ObjectController } from "../object/controller.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { Authorization } from "./authorization.ts";
import { ClaimController } from "../claim/controller.ts";
import { type ClaimHome, Reservation } from "../claim/reservation.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import { ExternalStorage } from "./external.ts";
import { ObjectSource } from "./source.ts";

import { Provisioned } from "../trait/provisioned.ts";
import { ResourceController, type ResourceControllerOptions } from "./provisioned.ts";
import { ExpiringController } from "./expiring.ts";
import { RecoverableController } from "./recoverable.ts";
import { TrackedHistory } from "./tracked.ts";
import type { Controller } from "@destack/runtime/control";

import { Settlement } from "./settlement.ts";
import { telemetry } from "@destack/telemetry";
import { Claimant } from "../claim/claim.ts";
import type { StoreExecution } from "../method/execution.ts";
import type { HookSandbox } from "../hook/sandbox.ts";
import type { AdapterFetch, PackageServer, ServedPackage, ServerDefinition } from "./server.ts";
import { Clock } from "@destack/environment";

/** The object call spans. */
const { span } = telemetry.scope(import.meta.destack.package);

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

/** A database's replica: the subscription following it, and the publisher streaming its pages. */
export interface SubscriberEntry {
    /** The subscription following the replica. */
    readonly subscription: sync.Subscription;
    /** Whether the store waits for the replica before serving, else as its shape says. */
    readonly isReadinessGate?: boolean;
    /** The publisher streaming the replica's pages, one step closer to its rows' home. */
    readonly publisher: sync.Publisher;
}

/** How a store follows its replicas: the tables relisting them, and the uplink taking changes. */
export interface SubscriberOptions {
    /** The tables whose changes relist subscriptions, beside scopes and residents' homes. */
    readonly watches?: readonly Table[];
    /** The link to the replicas' store, receiving this store's changes to them, if it sends any. */
    readonly uplink?: sync.Uplink;
}

/** The subscriptions a database follows for its replicas, each from its own publisher. */
export interface Subscriber extends SubscriberOptions {
    /** List the subscriptions of the store's chains and follows. */
    readonly subscriptions: () => Promise<readonly SubscriberEntry[]>;
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
    /** The code making the calls for a principal's own code, such as a controller's name. */
    readonly component?: string;
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
    /** The authorized calls by type and method, whose owned types need no visible scope. */
    readonly calls?: readonly { readonly object: ObjectType; readonly name: string }[];
}

/** How a mutation runs beyond its caller. */
export interface MutateOptions {
    /** The running mutation to join without external work, which its owner commits, if any. */
    readonly transaction?: Transaction;
    /** The client instance writing ephemeral objects. */
    readonly writerId?: string;
    /** The code making the calls for a principal's own code, such as a controller's name. */
    readonly component?: string;
    /** The scope of the calls without one. */
    readonly scope?: string;
    /** Where the calls are made, the caller's site when absent, such as the store's own steps. */
    readonly site?: CallSite;
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
    /** The code making the call for a principal's own code, such as a controller's name. */
    readonly component?: string;
};

/** The options of an object store: its package servers over one database, and its call log. */
export type ObjectStoreOptions = Omit<
    StoreParts,
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
        /** The package servers, the first one running the store's own controllers. */
        readonly servers: readonly PackageServer[];
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
              /** The principal the store's own work runs as. */
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

/** What a store runs with once composed from its servers. */
type ComposedOptions = StoreParts & CallLog;

/** A package a store serves: its handle, definition, types and serving installation. */
interface StorePackage {
    /** The package. */
    readonly package: ServedPackage;
    /** The package's code. */
    readonly definition: ServerDefinition;
    /** The package's types, its handled ones in place of their declared defaults. */
    readonly objects: readonly ObjectType[];
    /** The installation serving the package, absent for the platform's own store. */
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
interface StoreParts {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The types whose queries run over rows replicated here, never followed nor changed here. */
    readonly replicas?: readonly ObjectType[];
    /** Further policies and table mappings to authorize. */
    readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The principal of the store's own work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The one partition the store serves, its own scope's when absent, never one inside it. */
    readonly partition?: string;
    /** The package recording the calls. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimHome;
    /** The installations serving objects and their projections, by type reference key. */
    readonly installed?: ReadonlyMap<string, ServerContext>;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** The files with the served external objects, each storing some of their types. */
    readonly external?: readonly ExternalStorage[];
    /** The subscriptions the database follows to maintain its replicas. */
    readonly subscriber?: SubscriberOptions;
    /** Read how the store holds its scope's chain, home or follower, absent while it has none. */
    readonly chain: (view: Snapshot) => Promise<ChainOptions | undefined>;
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
    /** The clock calls run and controllers reconcile at. */
    readonly clock: Clock;
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
    /** The principal of the store's own work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The one partition the store serves, its own scope's when absent, never one inside it. */
    readonly #partition: string | undefined;
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
    /** The journal of executed mutations, recorded only by a store serving methods. */
    readonly #journal: Journal | undefined;
    /** The package the store serves, as the audit records it. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The feed serving every sync. */
    readonly feed: sync.Feed;
    /** The claim type recording the names of the served objects' unique indexes. */
    readonly claims: ClaimHome | undefined;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral: EphemeralStorage | undefined;
    /** The files with the served external objects. */
    readonly external: readonly ExternalStorage[];
    /** The subscriptions the database follows to maintain its replicas. */
    readonly subscriber: Subscriber | undefined;
    /** Read how the store holds its scope's chain, home or follower, absent while it has none. */
    readonly chain: (view: Snapshot) => Promise<ChainOptions | undefined>;
    /** The durable types the store serves or decides, whose rows stand for their principals. */
    readonly decided: readonly ObjectType[];
    /** The types of the shapes the store follows, which its follower replicas compile against. */
    readonly followed: readonly ObjectType[];
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
        options?: AuditRecorderOptions,
    ) => AuditRecorder;
    /** Report failed settlements. */
    readonly #report: (error: unknown) => void;
    /** The clock calls run and controllers reconcile at. */
    readonly clock: Clock;
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
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, TupleReader>();
    /** The scopes each caller lives inside, read once per caller and database. */
    readonly #inside = new WeakMap<Caller, Map<DatabaseConnection, Promise<readonly Subject[]>>>();
    /** The types the store's packages declare, without the chunk types serving their text. */
    readonly #declared: readonly ObjectType[];
    /** The served packages' implementations, in the order of their servers. */
    readonly packages: readonly (PackageImplementation & { readonly objects: ObjectStore })[];
    /** The wait for the replicas gating readiness, shared by every implemented package. */
    #readiness: Promise<boolean> | undefined;

    /** Serve object types over one database. */
    constructor(given: ObjectStoreOptions) {
        // compose the servers' packages, then add the referenced and followed types and the follows
        const { parts, served } = ObjectStore.#compose(given, () => this);
        const options = this.#following(ObjectStore.#referencing(parts));
        this.followed = [...new Set((parts.follows ?? []).flatMap((each) => each.shape.objects))];

        // require each object type once apart from the shared procedures, and the stores its storage needs
        this.#declared = options.objects;
        this.objects = ObjectType.served(this.#declared);
        this.replicas = parts.replicas ?? [];
        // validate each served type once, as describing it at build would
        for (const type of options.objects) {
            type.requireValid();
        }
        ObjectStore.#requireRoutes(options.objects);
        ObjectStore.#requireStores(this.objects, options);

        // hold the optional stores and machines
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
        this.principal = options.principal;
        this.#partition = options.partition;

        // authorize the replicas the policies do not already decide
        const policed = new Set(
            (options.policies ?? []).map((each) => ObjectStore.#policyKey(each)),
        );
        const unpoliced = this.replicas.filter(
            (type) => !policed.has(ObjectStore.#policyKey(type)),
        );
        this.authorizer = ObjectStore.#authorizer([...this.objects, ...unpoliced], options);
        TrackedHistory.require(this.objects, this.authorizer);

        // hold the journal, the audit and the request context
        this.#journal = ObjectStore.#journalOf(this.#declared, options);
        this.origin = { ...options.origin, package: Package.of(options.origin.package) };
        this.audit = (scope, caller, recording) =>
            AuditRecorder.scoped(this.journal, this.origin)(scope, caller, recording);
        this.isAccessAudited = options.isAccessAudited ?? false;
        this.source = new ObjectSource(this);
        this.clock = options.clock;
        this.branch = options.branch;
        this.#report = options.report ?? ((error) => ObjectStore.#rethrow(error));
        this.#provisioned = ObjectStore.#control(this.objects, options.provisioned);
        this.providers = options.provisioned?.providers ?? [];

        // follow the durable tables in one feed
        const followed = ObjectStore.#follow(this.objects, options);
        this.decided = followed.decided;
        this.replicated = followed.replicated;
        this.feed = followed.feed;
        this.known = ObjectStore.known(options.objects, options.policies ?? []);

        // implement each served package, the first one running the store's own controllers
        this.packages = served.map((entry, index) => this.#implementPackage(entry, index === 0));
    }

    /** Read the implementation of a package the store serves, refusing one it serves not. */
    package(handle: PackageHandle): PackageImplementation & { readonly objects: ObjectStore } {
        const found = this.packages.find((each) => each.package.id === handle.id);
        if (found === undefined) {
            throw new TypeError(`the store serves no package ${handle.name}`);
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

        // place the replicas below the store's own scope or partition, toward each scope's home
        const types = [...given.objects, ...(given.policies ?? [])].filter(
            (each) => each instanceof ObjectType,
        );
        const below = ObjectStore.#below(given.partition, given.principal);
        const reach = (scope: string) =>
            publisher?.(scope) ?? present(subscriber?.uplink, `the publisher of ${scope}`);

        // add the followed types and the subscriptions
        return {
            ...given,
            policies: ObjectStore.#policies(
                given,
                follows.flatMap((each) => each.shape.objects),
            ),
            subscriber: {
                ...(subscriber?.uplink === undefined ? {} : { uplink: subscriber.uplink }),
                watches: [
                    ...(subscriber?.watches ?? []),
                    ...ObjectStore.#chainedTables(given, types),
                    // relist when followed rows change, which the host's chain fact reads
                    ...follows.flatMap((each) => each.shape.objects.map((type) => type.table)),
                    ...follows.flatMap((each) => ("from" in each ? [each.from.table] : [])),
                    // relist when the store acquires or evicts an on-demand key
                    ...(follows.some((each) => "onDemand" in each) ? [followKey] : []),
                ],
                subscriptions: async () => {
                    // follow the follows' keys, then the chains, gating like the follows that brought them
                    const followed = await ObjectStore.#followSubscriptions(this, follows, {
                        below,
                        publisher: reach,
                    });
                    const gating = (name: string) =>
                        followed.find((entry) => entry.subscription.name === name)
                            ?.isReadinessGate !== false;
                    const chained = (await this.source.chains(below)).map(
                        ({ subscription, home, brought }) => ({
                            subscription,
                            publisher: reach(home),
                            ...(brought === undefined
                                ? {}
                                : { isReadinessGate: brought.some(gating) }),
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
        const intrinsic = ObjectTypePool.of([]);

        // describe each asked type, then each type a description refers to beyond the intrinsic policies
        const served = this.#declared.filter((type) => !(INTRINSIC in type.source));
        const pending = [...(types ?? served.map((type) => type.typeReference))];
        const described = new Map<string, ObjectDescription>();
        let reference = pending.pop();
        while (reference !== undefined) {
            const key = ObjectTypeReference.key(reference);
            const type = known.get(key);
            if (type === undefined) {
                throw new CallError("NOT_FOUND", {
                    message: `no object type ${reference.type} of ${reference.packageId} is known here`,
                });
            } else if (!described.has(key)) {
                const description = type.describe();
                described.set(key, description);
                pending.push(...intrinsic.references(description));
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
            "database" | "policies" | "subscriber" | "clock" | "installed"
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
            {
                clock: options.clock,
            },
        );

        return { decided, replicated, feed };
    }

    /** Stream served row replicas to in-process followers, and run the changes they send. */
    #uplink(): sync.Uplink {
        return {
            stream: (subscription, signal) => this.source.publish(subscription, signal),
            receive: (mutation, signer) => {
                // run the changes as the follower's principal
                if (signer === undefined) {
                    throw new TypeError("a change sent in process needs its sending principal");
                }

                return this.receive(mutation, signer);
            },
            call: (call, scope, signer) => {
                // answer the query as the follower's principal in the scope it reads
                if (signer === undefined) {
                    throw new TypeError("a query sent in process needs its sending principal");
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

    /** Build the authorizer over the objects, their scope types and other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        options: Pick<ComposedOptions, "database" | "policies" | "clock" | "partition">,
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

        return ObjectPolicy.authorizer(
            objects,
            [...(options.policies ?? []), ...attached],
            (table) => options.database.replicates(table),
            options.partition,
        );
    }

    /** Open a method store's journal with its call key, refusing a call key without methods. */
    static #journalOf(
        routed: readonly ObjectType[],
        options: Pick<ComposedOptions, "database" | "callKey" | "clock" | "events">,
    ): Journal | undefined {
        // require the call key and the event log recording the calls of a store serving methods
        const isServing = routed.length > 0;
        const { callKey, events } = options;
        if (isServing && callKey === undefined) {
            throw new TypeError("a store serving methods needs the call key of their journal");
        } else if (isServing && (events === undefined || !events.has(Journal.kind))) {
            throw new TypeError("a store serving methods needs an event log for its calls");
        } else if (!isServing && callKey !== undefined) {
            throw new TypeError("a store without methods has no journal and no call key");
        }

        return callKey === undefined || events === undefined
            ? undefined
            : new Journal(options.database, callKey, { events, clock: options.clock });
    }

    /** The space the store serves, absent for a machine's or the universe's store. */
    get space(): string | undefined {
        // a space's store serves the space, a sandboxed installation's store its installation's space
        const { principal: own } = this;
        if (principal.space.is(own)) {
            return own.id;
        } else if (principal.installation.is(own)) {
            return own.scope;
        }

        return undefined;
    }

    /** Read the caller serving a type: the installation serving it, else the store's principal. */
    serving(object: ObjectType, now = this.clock.now()): AccessContext {
        const served = this.readServer(object);
        const subject =
            served === undefined
                ? this.principal
                : principal.installation.reference(served.space, served.installation);

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
                    `object ${object.name} has the name of the shared ${object.key} procedures`,
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
            throw new TypeError(`local object ${local.name} lives only in its scope's own store`);
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
    ): PackageImplementation & { readonly objects: ObjectStore } {
        const { subscriber } = this;

        return {
            objects: this,
            package: handle,
            access: this.access,
            audit: AuditRecorder.procedure((call) =>
                this.audit(
                    this.#procedureScope(call, routed) ?? Scope.universe.id,
                    ObjectStore.#requestCaller(call.request),
                    { requestId: call.request.requestId },
                ),
            ),
            controllers,
            ...(subscriber === undefined
                ? {}
                : { ready: (signal) => this.#awaitReady(subscriber, signal) }),
            router: this.router(routed),
        };
    }

    /** Follow a shape's key on demand until it lapses, waiting until the replica has it. */
    async acquire<Parameters extends object>(
        follow: Extract<Follow<Parameters>, { readonly onDemand: unknown }>,
        at: FollowKey<Parameters>,
        signal: AbortSignal,
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

        // wait on commits until the synced replica follows the key's queries
        const wanted = shape.onDemandSubscription(
            scope,
            [parameters],
            ObjectStore.#below(this.#partition, this.principal),
            at.subject,
        );
        await this.database.log.until(() => this.#covers(wanted), signal);
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
    async #awaitReady(subscriber: Subscriber, signal: AbortSignal): Promise<void> {
        // share one check of the replicas, checking again after the checking server stopped first
        const stopped = new Promise<boolean>((resolve) => {
            signal.addEventListener("abort", () => resolve(false), { once: true });
        });
        while (!signal.aborted) {
            const readiness = (this.#readiness ??= this.#ready(subscriber, signal));
            if (await Promise.race([readiness, stopped])) {
                return;
            }
            if (this.#readiness === readiness) {
                this.#readiness = undefined;
            }
        }
    }

    /** Wait until every readiness replica is a complete snapshot, reporting sync until then. */
    async #ready(subscriber: Subscriber, signal: AbortSignal): Promise<boolean> {
        // wait on commits until every gating replica synced
        const { database } = this;

        return database.log.until(async () => {
            // check each gating replica
            const entries = await subscriber.subscriptions();
            const gates = entries.filter(
                ({ subscription, isReadinessGate }) =>
                    isReadinessGate ?? this.source.isReadinessGate(subscription),
            );
            const synced = await Promise.all(
                gates.map(({ subscription }) => sync.Replica.isCurrent(database, subscription)),
            );

            return synced.every(Boolean);
        }, signal);
    }

    /** The server access deciding the served objects. */
    get access(): ServerAccess {
        return {
            authorizer: this.authorizer,
            database: this.database,
        };
    }

    /** Route the served objects' methods, or some of them, and the store procedures. */
    router(types: readonly ObjectType[] = this.#declared): ObjectRouter {
        // route each object under its key, which its package's handle holds unique
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
                await this.#reachHomes(context, input.scope, input.homes);

                return this.watermark(input.scope);
            }),
            call: store.call.handler(async ({ input, context }) => {
                // read in the given scope
                const {
                    object,
                    name,
                    input: called,
                } = this.#resolve(input.call, false, CallSite.of(context.access(input.scope)));
                if (object.readScope(name, called) !== input.scope) {
                    throw new CallError("BAD_REQUEST", {
                        message: `${object.name}.${name} reads a scope other than ${input.scope}`,
                    });
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

        // write through a single-call mutation and report its watermark
        const [written] = isEphemeral
            ? await this.mutate(
                  caller,
                  { id: RequestId.create(this.clock.now()), calls: [call] },
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
    ): Promise<Result> {
        // read untyped, then type the answer by the method's output
        const callable = this.#callable(call, false, this.replicas);
        const answer = await this.#query(caller, call, site);
        const [result] = MethodContext.results(callable, [call], [answer]);

        return result;
    }

    /** Read through a method as a caller, answering its untyped result. */
    async #query(caller: AccessContext, call: sync.Call, site: CallSite): Promise<unknown> {
        // resolve the call, reading only a method the caller's grant allows
        const { object, name, input } = this.#resolve(call, false, site);
        const method = object.method(name);
        const scope = object.readScope(name, input);
        await this.#limit([{ object, name, input }], scope, caller);
        const database = await this.#databaseOf(object, scope);
        const read = async () => {
            // prepare on the served type, then read, commit and settle
            const prepared = await this.#prepare(
                [{ object: this.served(object), name, input }],
                scope,
                caller,
                RequestId.create(this.clock.now()),
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
            // read over a mutation that never commits, allowing the caller against the durable database
            const reading = Transaction.reading(database);
            const authorizing =
                object.storage === "durable" ? reading : Transaction.reading(this.database);
            const authorization = await this.authorizeHere(authorizing, scope, caller, {
                calls: [{ object, name }],
            });

            return this.#execute(reading, authorization, object, name, input, caller, { prepared });
        };

        // read unaudited objects directly
        if (!object.isReadAudited && method.isAudited !== true && !this.isAccessAudited) {
            return read();
        }

        // audit one object, or the scope's collection
        const recorder = this.audit(scope, caller);
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
            throw new CallError("BAD_REQUEST", {
                message: "a mutation writes durable or ephemeral objects, not both",
            });
        }

        // expand calls into the calls they run after, and identify prepared creations
        const calls = (await this.#expand(originals, scope, caller)).map((call) =>
            ObjectStore.#identified(call),
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
            ...(options.component === undefined ? {} : { component: options.component }),
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
                `${object.name}.${name} does external work: a call inside a transaction cannot`,
            );
        } else if (object.storage !== storage && !isBeside) {
            throw new TypeError(`${object.name} is stored outside the transaction`);
        }

        // run it under the calling method's authorization, else allow the caller in its scope
        const scope = object.readScope(name, input);
        const authorization =
            parent !== undefined &&
            parent.context.caller === caller &&
            parent.context.scope === scope
                ? parent.authorization
                : await this.authorizeHere(
                      transaction,
                      scope,
                      caller,
                      parent === undefined
                          ? {
                                links: await Scope.chain(transaction.view(), scope),
                                calls: [{ object, name }],
                            }
                          : { now: parent.context.now, calls: [{ object, name }] },
                  );

        return this.#execute(transaction, authorization, object, name, input, caller, options);
    }

    /** Read the one scope a mutation's calls act in. */
    #mutationScope(originals: readonly ReturnType<typeof ObjectType.resolve>[]): string {
        // require every call to act in the first call's scope
        const first = nth(originals, 0);
        const scope = first.object.readScope(first.name, first.input);
        if (originals.some((call) => call.object.readScope(call.name, call.input) !== scope)) {
            throw new CallError("BAD_REQUEST", {
                message: "a mutation's calls must act in one scope",
            });
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
            throw new CallError("UNAUTHORIZED", {
                message: "a mutation needs an authenticated caller",
            });
        }

        return {
            id: mutation.id,
            scope,
            caller,
            ...(options.component === undefined ? {} : { component: options.component }),
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

        return authorization;
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
        for (const [index, call] of run.calls.entries()) {
            progress.running = index;
            const options: CallOptions = {
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
                    authorization,
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
                now: this.clock.now(),
            }));
        progress.reservation = reservation;

        // commit prepared work and claims with the mutation
        transaction.record(async (committing) => {
            // commit the prepared work
            await this.#commit(committing, run.prepared);

            // release the relationships bound to the request, when it wrote any
            const written = transaction.changes.writtenRows(accessRelationship).values();
            if ([...written].some((row) => row?.["requestId"] === run.id)) {
                await this.authorizer.release(committing, run.id);
            }

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
        await this.audit(
            run.scope,
            run.caller,
            run.component === undefined ? {} : { component: run.component },
        ).record(
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

        // run each mutation in its own transaction
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
                    throw new CallError("BAD_REQUEST", {
                        message: `a mutation pushed to ${scope} acts in another scope`,
                    });
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

    /** Decide whether a scope's rows of a type are replicas here, written only by their home. */
    async isReplica(object: ObjectType, snapshot: Snapshot, scope: string): Promise<boolean> {
        // serve the rows of a type an installation serves here, even with a replicated table
        const isInstalled = this.#installed.has(ObjectTypeReference.key(object.typeReference));

        return (
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
            throw new CallError("BAD_REQUEST", {
                message: `nothing here answers ${object.name}.${name} at the rows' home`,
            });
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
            throw new CallError("BAD_REQUEST", {
                message: "nothing here receives changes of the replicated rows",
            });
        } else if (home === undefined || homes.some((each) => each !== home)) {
            throw new CallError("BAD_REQUEST", {
                message: "a mutation of replicated rows must change rows of one home",
            });
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

        throw new CallError("BAD_REQUEST", {
            message: `the rows ${[...new Set(replicas)].join(", ")} are replicas here: change them at their home`,
        });
    }

    /** Find the publisher receiving a change of replicated rows. */
    async #readHome(call: sync.Call): Promise<sync.Uplink | undefined> {
        // find the replica of the call's table in its scope whose publisher receives changes
        const object = ObjectType.find(this.replicated, call);
        const scope = object?.readScope(call.method, call.input);
        const entries = (await this.subscriber?.subscriptions()) ?? [];
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
            throw new CallError("BAD_REQUEST", {
                message: "a mutation must change rows of one home, local or replicated",
            });
        }

        // pass it to the publisher the replicas follow, signed as the given principal
        const sent = sync.Mutation.parse(mutation);

        return (await this.#readHomeAll(sent)).receive(sent, signer);
    }

    /** List the controllers the served objects need. */
    controllers(): readonly Controller[] {
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
                : [new ClaimController(this.claims, this.database, this.objects, this.clock)]),
            ...(isRecoverable ? [new RecoverableController(this)] : []),
            ...(isExpiring ? [new ExpiringController(this)] : []),
            ...(isSettled ? [Settlement.controller(this)] : []),
            ...(this.subscriber === undefined ? [] : [this.source.controller(this.subscriber)]),
            ...[...this.objects, ...this.#provisioned].flatMap((object) =>
                object.controller === undefined
                    ? []
                    : [ObjectController.control(this, object, object.controller)],
            ),
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
        // place the caller inside the scopes given, else those its principal's row places it, read once per caller and database
        const { request, links } = options;
        const database = transaction.database;
        const read =
            this.#inside.get(caller) ?? new Map<DatabaseConnection, Promise<readonly Subject[]>>();
        this.#inside.set(caller, read);
        const known =
            options.within === undefined
                ? (read.get(database) ?? this.source.within(caller.subject, database))
                : Promise.resolve(
                      options.within.map((inside) => ({ ...inside, relation: CONTAINED })),
                  );
        if (options.within === undefined) {
            read.set(database, known);
        }
        const within = await known.catch((error: unknown) => {
            // forget a failed read, so the caller's next call reads again
            read.delete(database);
            throw error;
        });

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

        return new Authorization(this.authorizer, transaction, bind, access);
    }

    /** Authorize a caller for work this store serves: an unmoved scope, in its partition. */
    async authorizeHere(
        transaction: Transaction,
        scope: string,
        caller: AccessContext,
        options: AuthorizeHereOptions = {},
    ): Promise<Authorization> {
        // resolve its access, then require a scope this store serves, moved only for moving calls
        const { calls = [], ...authorizing } = options;
        const authorization = await this.authorize(transaction, scope, caller, authorizing);
        authorization.requireHere(
            calls.map(({ object, name }) => object.permission(object.method(name).permission)),
            this.#partition,
            calls.length === 0 ? undefined : ObjectStore.#below(undefined, this.principal),
        );
        authorization.requireMethods(calls);

        return authorization;
    }

    /** Enter a scope: require the pinned scope or one inside it, and wait for observed writes. */
    async enter(context: RequestContext, scope: string): Promise<void> {
        // require the pinned scope or a scope inside it
        const pinned = context.scope;
        if (pinned !== undefined && pinned !== scope) {
            const [row] = await Scope.read(Snapshot.live(this.database), [scope]);
            if (row?.ancestors.includes(pinned) !== true) {
                throw new CallError("NOT_FOUND", {
                    message: `scope ${scope} is outside the pinned scope`,
                });
            }
        }

        // wait for observed writes
        await this.reach(context, scope);
    }

    /** Wait until the database has the caller's watermarks in the scope chain. */
    async reach(context: RequestContext, scope: string): Promise<void> {
        // wait for the watermarks of the scope chain
        const watermarks = context.bookmark.watermarks;
        if (watermarks.length === 0) {
            return;
        }
        const chain = await this.#chainIds(scope);
        await this.#reachAll(
            context,
            scope,
            watermarks.filter((entry) => chain.has(entry.scope)),
        );
    }

    /** Wait until a scope's chain replicas have their homes' commits, refusing other homes. */
    async #reachHomes(
        context: RequestContext,
        scope: string,
        homes: readonly Watermark[],
    ): Promise<void> {
        // require homes along the scope's chain
        const chain = await this.#chainIds(scope);
        const outside = homes.find((home) => !chain.has(home.scope));
        if (outside !== undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${outside.scope} is outside the chain of ${scope}, so no replica here follows it`,
            });
        }

        // wait for each home's commit
        await this.#reachAll(context, scope, homes);
    }

    /** Read a scope and the scopes enclosing it, by identifier. */
    async #chainIds(scope: string): Promise<Set<string>> {
        const chain = await Scope.chain(Snapshot.live(this.database), scope);

        return new Set([scope, ...chain.map((link) => link.object.id)]);
    }

    /** Wait until the scope's log or its replicas reach each watermark, in a bounded wait each. */
    async #reachAll(
        context: RequestContext,
        scope: string,
        watermarks: readonly Watermark[],
    ): Promise<void> {
        for (const watermark of watermarks) {
            const signal = AbortSignal.any([
                context.signal,
                AbortSignal.timeout(BOOKMARK_TIMEOUT_MILLISECONDS),
            ]);
            const isReached = await this.#reach(scope, watermark, signal);
            if (!isReached) {
                throw new CallError("SERVICE_UNAVAILABLE", {
                    message: `${watermark.scope} has not reached the required sequence`,
                });
            }
        }
    }

    /** Read the watermark the scope's log has now. */
    async watermark(scope: string): Promise<Watermark> {
        return { scope, ...(await this.database.log.position()) };
    }

    /** Wait until the scope's log or a replica has the watermark. */
    async #reach(scope: string, watermark: Watermark, signal: AbortSignal): Promise<boolean> {
        // wait on the scope's own log
        if (watermark.scope === scope) {
            if (watermark.epoch !== (await this.database.log.epoch())) {
                throw new DatabaseError(
                    "STALE_EPOCH",
                    `${scope} no longer has the watermark's history`,
                );
            }

            return await this.database.log.wait(watermark.sequence, signal);
        }

        // wait on the replica of an enclosing scope
        return await sync.Replica.wait(this.database, watermark.scope, watermark, signal);
    }

    /** The served durable object types. */
    get durable(): ObjectType[] {
        return this.objects.filter((object) => object.storage === "durable");
    }

    /** Read the server context serving a type inside the store, absent for a type it serves. */
    readServer(object: ObjectType): ServerContext | undefined {
        return this.#installed.get(ObjectTypeReference.key(object.typeReference));
    }

    /** Read the memory store with the ephemeral objects, failing without one. */
    requireEphemeral(): EphemeralStorage {
        if (this.ephemeral === undefined) {
            throw new CallError("NOT_FOUND", {
                message: "no ephemeral objects are served here",
            });
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
        // require the writer owning the rows
        if (writerId === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: "only a writer can write ephemeral objects",
            });
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
                            { writerId, requestId },
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
                    throw new CallError("TOO_MANY_REQUESTS", {
                        message: `${object.name}.${name} is limited to ${rate.count} calls per ${Duration.milliseconds(rate.every)} ms`,
                        data: { retryAfter },
                    });
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
            throw new CallError("CONFLICT", {
                message: `${replicated.name} rows are replicas here, so send their changes to their home`,
            });
        }

        return mutates ? this.objects : [...this.objects, ...this.replicas];
    }

    /** Read a procedure call's scope: a method's own, else the decided target's or context's. */
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
            throw new CallError("CONFLICT", {
                message: `${object.name} rows of ${scope} are replicas here, so send their changes to their home`,
            });
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
            throw new CallError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        // refuse callers' other methods on a trashed object
        const isTrashed =
            object.lifecycle.recoverable !== undefined &&
            target !== undefined &&
            target["deletedAt"] !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.isInternal !== true) {
            throw new CallError("CONFLICT", { message: `${object.name} is in the trash` });
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
            throw new CallError("BAD_REQUEST", { message: "this store does not serve branches" });
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
            throw new CallError("NOT_FOUND", { message: `no branch ${id}` });
        }

        return base.layer(await new Branch(types, base, id).overlay(base, this.objects));
    }

    /** Read a method about to run, refusing one that declares prepared work it never prepares. */
    static #preparing(object: ObjectType, name: string): Method {
        const method = object.method(name);
        if (method.prepared !== undefined && method.prepare === undefined) {
            throw new TypeError(`${object.name}.${name} declares prepared work it never prepares`);
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
        // prepare in order, holding the reserved settlements
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
                `${context.object.name}.${context.name} has no declared prepared work`,
            );
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

    /** Execute, redact and audit one call in its own span. */
    #execute(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: AccessContext,
        options: CallOptions = {},
    ): Promise<unknown> {
        const attributes = { "destack.object.type": object.name, "destack.object.method": name };

        return span("object.call", attributes, () =>
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
        span.current()?.setAttributes({
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

    /** Run a phase's calls outside its mutation, each a request of its own under a derived key. */
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
            const id = await RequestId.derive(parent.now, `${key}/${runs}`);

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
        const { from, writerId, requestId, component } = options;
        const recorded: CallOptions = {
            ...(from === undefined ? {} : { from }),
            ...(writerId === undefined ? {} : { writerId }),
            ...(requestId === undefined ? {} : { requestId }),
            ...(component === undefined ? {} : { component }),
        };

        return async (entry, caller, mutates, code) => {
            const result = await this.#runInside(
                transaction,
                entry,
                caller,
                mutates,
                code,
                { context: parent, authorization },
                recorded,
            );

            // decide the rest of the parent call on the access a nested mutation changed
            if (mutates) {
                authorization.renew();
            }

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
        await this.audit(scope, caller, requestId === undefined ? {} : { requestId }).record(
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
        const recorder = this.audit(scope, caller, component === undefined ? {} : { component });
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
            const [epoch, sequence] = lastEventId.split("/");
            const position = LogPosition.safeParse({ epoch, sequence: Number(sequence) });
            if (!position.success) {
                throw new CallError("BAD_REQUEST", {
                    message: `no page has event ${lastEventId}`,
                });
            }

            return position.data;
        }

        return after;
    }

    /** Give a prepared creation without an identifier a fresh one, for its prepared work. */
    static #identified(call: Expansion): Expansion {
        const method = call.object.method(call.name);
        const isUnnamed =
            method.kind === "create" &&
            method.prepare !== undefined &&
            call.input["id"] === undefined;

        return isUnnamed
            ? { ...call, input: { ...call.input, id: `${call.object.identity}-${v7()}` } }
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

    /** Compose a store's parts from its servers and sandboxes, and list the packages it serves. */
    static #compose(
        given: ObjectStoreOptions,
        store: () => ObjectStore,
    ): { readonly parts: ComposedOptions; readonly served: readonly StorePackage[] } {
        // require the machine running any provisioned resources, and one uplink
        const { servers, sandboxes = [], uplink, machine, publisher, server, ...rest } = given;
        const { policies = [], branch } = rest;
        const own = servers.map((each) =>
            ObjectStore.#packageOf(each, each.installation ?? server),
        );
        const providers = own.flatMap((each) => each.definition.providers ?? []);
        if (providers.length > 0 && machine === undefined) {
            throw new TypeError("provisioned resources need their machine");
        } else if (server !== undefined && uplink !== undefined) {
            throw new TypeError("an installation's replicas must leave through its publisher");
        }

        // serve the sandboxed installations' packages, declared against what the servers make known
        const objects = own.flatMap((each) => each.objects);
        const followed = own
            .flatMap((each) => each.definition.follows ?? [])
            .flatMap((each) => each.shape.objects);
        const known = ObjectStore.known(objects, [
            ...policies,
            ...ObjectType.referenced(objects),
            ...followed,
        ]);
        const served = [
            ...own,
            ...sandboxes.flatMap((sandbox) => ObjectStore.#sandboxPackages(sandbox, known)),
        ];

        // route every package's types, answering queries of replicated types nothing here serves
        const routes = ObjectStore.#routes(served);
        const replicas = known
            .filter((type) => type instanceof ObjectType)
            .filter(
                (type) =>
                    !routes.has(ObjectTypeReference.key(type.typeReference)) &&
                    given.database.replicates(type.table),
            );
        const routed = [...routes.values()].map((route) => route.type);
        if (branch !== undefined && !routed.some((object) => object.same(branch.object))) {
            throw new TypeError(
                `no server serves ${branch.object.name}, the object of the branches`,
            );
        }
        const follows = served.flatMap((each) => each.definition.follows ?? []);
        const external = served.flatMap((each) =>
            each.definition.external === undefined ? [] : [each.definition.external],
        );

        // serve each type at its package's installation, else at the store's installation
        const installed = new Map(
            [...routes].flatMap(([key, { entry }]) => {
                const serving = entry.server ?? server;

                return serving === undefined ? [] : [[key, serving] as const];
            }),
        );

        // reach a partition's chain at its installation's root
        const isPartition = server?.partition !== undefined && server.partition !== server.space;
        const reach =
            server !== undefined && isPartition
                ? ObjectStore.#partitionPublisher(server, store, publisher)
                : publisher;
        const parts: ComposedOptions = {
            ...rest,
            installed,
            ...(replicas.length === 0 ? {} : { replicas }),
            principal:
                server === undefined
                    ? present(given.principal, "the store's principal")
                    : principal.installation.reference(server.space, server.installation),
            ...(server?.partition === undefined ? {} : { partition: server.partition }),
            objects: routed,
            policies,
            ...(external.length === 0 ? {} : { external }),
            ...(follows.length === 0 ? {} : { follows }),
            ...(reach === undefined ? {} : { publisher: reach }),
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

        return { parts, served };
    }

    /** Read a server's package for an installation, its handled types over their defaults. */
    static #packageOf(server: PackageServer, context: ServerContext | undefined): StorePackage {
        // refuse a handled type of the package that its handle neither declares nor its providers provision
        const definition = server.definition(context);
        const handled = definition.objects ?? [];
        const own = Object.values(server.package.objects);
        const provisioned = (definition.providers ?? []).map((provider) => provider.object);
        const others = handled.filter((type) => !own.some((each) => each.same(type)));
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
        const declared = own
            .filter((type) => type.storage !== "external" || filed.some((each) => each.same(type)))
            .map((type) => handled.find((each) => each.same(type)) ?? type);

        return {
            package: server.package,
            definition,
            objects: [...declared, ...others],
            server: context,
        };
    }

    /** Route each package's own types to it, refusing two packages implementing one. */
    static #routes(
        served: readonly StorePackage[],
    ): ReadonlyMap<string, { readonly type: ObjectType; readonly entry: StorePackage }> {
        // implement each package's own types, once
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
        const owner = server.build.manifest.package;
        const own = declarations.filter(
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
        return own
            .filter((entry) => entry.kind === PACKAGE_DECLARATION)
            .map((entry) => PackageDescription.parse(entry.description))
            .flatMap((described): StorePackage[] => {
                // refuse adapters, and leave a package of no objects unserved
                const adapters = Object.keys(described.adapters);
                if (adapters.length > 0) {
                    throw new TypeError(
                        `installation ${server.installation} declares adapters ${adapters.join(", ")}, but a sandbox cannot answer them`,
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

    /** Reach a partition at its installation's root, and other scopes as given. */
    static #partitionPublisher(
        server: ServerContext,
        store: () => ObjectStore,
        publisher: ((scope: string) => sync.Publisher) | undefined,
    ): (scope: string) => sync.Publisher {
        // open the root's publisher once, every link of the partition's chain going through it
        let root: sync.Publisher | undefined;

        return (scope) => {
            // reach other scopes as given
            if (scope !== server.partition) {
                return publisher?.(scope) ?? server.publisher;
            }

            // reach the partition through its installation's root
            root ??= server.publisherAt(
                Egress.address(server.name, server.space),
                store().origin.package.id,
            );

            return root;
        };
    }

    /** Implement a served package: its routes, adapters, controllers, access and headers. */
    #implementPackage(
        entry: StorePackage,
        isFirst: boolean,
    ): PackageImplementation & { readonly objects: ObjectStore } {
        // implement the package with its controllers, the first one also running the store's own
        const { controllers = [], adapters = {}, access, responseHeaders } = entry.definition;
        const own = isFirst ? this.controllers() : [];
        const built = controllers.map((controller) => controller(this));
        const implemented = this.implement(entry.package, entry.objects, [...own, ...built]);

        // answer the package's adapters below its mount
        const fetch = ObjectStore.#adapterFetch(entry.package, adapters, this);

        return {
            ...implemented,
            ...(access === undefined ? {} : { access }),
            ...(responseHeaders === undefined ? {} : { responseHeaders }),
            ...(fetch === undefined ? {} : { fetch }),
        };
    }

    /** Route requests below a package's mount to the adapters answering their paths. */
    static #adapterFetch(
        handle: PackageHandle,
        answers: Readonly<Record<string, AdapterFetch>>,
        store: ObjectStore,
    ): PackageImplementation["fetch"] {
        // pair each declared adapter with its answer, refusing an answer of none declared
        const adapters = Object.values(handle.adapters).map((adapter) => {
            const fetch = answers[adapter.name];
            if (fetch === undefined) {
                throw new TypeError(
                    `package ${handle.name} does not answer adapter ${adapter.name}`,
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

            // allow the installation itself or identified callers of the declared types, leaving an adapter declaring none its own protocol
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
                throw new CallError("UNAUTHORIZED", { message: "missing caller credential" });
            } else if (!isDeclared) {
                throw new CallError("FORBIDDEN", {
                    message: `adapter ${adapter.name} answers no ${caller.subject.type} ${caller.subject.id}`,
                });
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
            readonly below: string;
            readonly publisher: (scope: string) => sync.Publisher;
        },
    ): Promise<SubscriberEntry[]> {
        // subscribe to the acquired keys of each on-demand follow, one replica per scope and subject
        const onDemand = await Promise.all(
            follows
                .filter((follow) => "onDemand" in follow)
                .map((follow) => ObjectStore.#onDemandSubscriptions(store, follow, options)),
        );

        // read each other follow's keys, fixed or from the store's own rows
        const keyed = await Promise.all(
            follows.map(async (follow) => ({
                follow,
                keys:
                    "keys" in follow
                        ? follow.keys
                        : "from" in follow
                          ? await ObjectStore.#readKeys(store, follow)
                          : [],
            })),
        );

        // subscribe to each key of each shape
        const subscribed = await Promise.all(
            keyed.map(async ({ follow, keys }) => {
                // subscribe to each key toward its scope's home, decided for its principal
                const { shape } = follow;
                const isReadinessGate = follow.isReadinessGate ?? true;
                const derived = keys.map(({ key, subject }) => ({
                    subscription: shape.subscription(key, options.below, subject),
                    home: key.scope,
                }));

                return derived.map(({ subscription, home }) => ({
                    subscription,
                    publisher: options.publisher(home),
                    isReadinessGate,
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
        options: { readonly below: string; readonly publisher: (scope: string) => sync.Publisher },
    ): Promise<SubscriberEntry[]> {
        // read the keys that have not lapsed
        const rows = await store.database
            .select()
            .from(followKey)
            .where(
                and(
                    eq(followKey.shape, follow.shape.name),
                    gt(followKey.expiresAt, store.clock.now()),
                ),
            );

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

        // subscribe to each group toward its scope's home, never gating the store's readiness
        return [...groups.values()].map(({ scope, subject, keys }) => ({
            subscription: follow.shape.onDemandSubscription(
                scope,
                keys,
                options.below,
                ObjectStore.#onDemandSubject(store, subject),
            ),
            publisher: options.publisher(scope),
            isReadinessGate: false,
        }));
    }

    /** Read a followed key's subject, none for the store's own principal. */
    static #onDemandSubject(store: ObjectStore, subject: string): Subject | undefined {
        const parsed = Subject.parse(JSON.parse(subject));

        return Subject.key(parsed) === Subject.key(store.principal) ? undefined : parsed;
    }

    /** Read the scope or partition below which a store places its replicas. */
    static #below(partition: string | undefined, self: Subject): string {
        return partition ?? (principal.installation.is(self) ? self.scope : self.id);
    }

    /** Read the keys of a follow over the store's own rows: one per row it reads. */
    static async #readKeys(
        store: ObjectStore,
        follow: RowFollow<object, ObjectType>,
    ): Promise<FollowKey[]> {
        // read the rows of the store's own database the keys come from
        const view = Snapshot.live(store.database);
        const rows = await view.read(follow.from.table).findMany({ where: follow.where ?? {} });

        // read each row's key
        const found = await Promise.all(rows.map((row) => follow.key(row, view, store.space)));

        return found.filter((key) => key !== undefined);
    }
}
