import {
    Authorizer,
    AccessContext,
    decisionTables,
    type GrantReader,
    Policy,
    type Access,
    type TableMapping,
    Caller,
    CONTAINED,
    principal,
} from "@destack/access";
import { type ObjectReference, Scope, type ScopeLink, Subject } from "@destack/sync";
import {
    LogPosition,
    Snapshot,
    TABLE,
    type DatabaseConnection,
    Row,
    type Table,
    DatabaseError,
    Transaction,
} from "@destack/db";
import { eventUnmask, Replay } from "@destack/audit";
import type { UnmaskedRead } from "@destack/event/server";
import {
    AuditRecorder,
    Journal,
    type AuditDestination,
    type AuditOrigin,
    type AuditCalled,
    type CallRequest,
} from "@destack/audit/server";
import { Loan } from "@destack/service/authentication";
import type { CallKey } from "@destack/service/request";

import { aligned, Clock, type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import type { InstallationContext } from "@destack/service/workload";
import type { Watermark } from "@destack/service/bookmark";
import { CompactionController } from "@destack/service/database";
import { refuseInput, ServiceError } from "@destack/service/error";
import { RequestId, type RequestIdentity } from "@destack/service/request";
import { v7 } from "uuid";
import {
    implement,
    type Router,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
    type ProcedureCall,
} from "@destack/service/server";
import { type Service, type ServiceRouter } from "@destack/service";
import * as sync from "@destack/sync";
import { Branch, type BranchCall, type BranchType } from "../branch/index.ts";
import { Call, OutboxCall, type ObjectCall, type ResultsOf, type Run } from "../method/call.ts";
import { Method, type MethodKind } from "../method/method.ts";
import { callableProcedures } from "../method/procedure.ts";
import { InputSchema } from "../method/input.ts";
import {
    WriterId,
    replicaProcedures,
    type PushResult,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ObjectType } from "../object/object.ts";
import { ObjectAccess } from "../object/access.ts";
import { ObjectController } from "../object/controller.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { Authorization } from "./authorization.ts";
import { ChangeController } from "./change.ts";
import { Outbox } from "@destack/service/outbox";
import { ClaimController } from "../claim/controller.ts";
import { type ClaimStore, Reservation } from "../claim/store.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import { ExternalStorage } from "./external.ts";
import { ObjectSource } from "./source.ts";

import type { Residence } from "../trait/projected.ts";
import { Provisioned } from "../trait/provisioned.ts";
import { ProvisionedController, type ProvisionedOptions } from "./provisioned.ts";
import { ExpiringController } from "./expiring.ts";
import { RecoverableController } from "./recoverable.ts";
import { TrackedHistory } from "./tracked.ts";
import type { Controller } from "@destack/service/control";
import { Trigger, type ChangeTrigger, type RunClient } from "@destack/service/trigger";

import { Settlement } from "./settlement.ts";
import { telemetry } from "@destack/telemetry";
import { Claimant } from "../claim/claim.ts";

/** The object call spans. */
const { span } = telemetry.scope(import.meta.destack.package);

/** The recent pages sharing grant readers: 64 readers of a few KiB, well under a megabyte. */
const SHARED_READERS = 64;

/** The input of a procedure call, by field. */
const PROCEDURE_INPUT = schema.record(schema.string(), schema.unknown());

/** The identifier of a row a method returns. */
const IDENTIFIER = schema.string();

/** The page a list method returns: its rows and what each includes, by include name and row identifier. */
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

/** How long a request waits for its caller's watermarks, in milliseconds: 5 s, above sub-second copy lag. */
const BOOKMARK_TIMEOUT_MILLISECONDS = 5000;

/** A copy a database keeps: the subscription following it, and the publisher streaming its pages. */
export interface SubscriberEntry {
    /** The subscription following the copy. */
    readonly subscription: sync.Subscription;
    /** The publisher streaming the copy's pages, one step closer to its rows' home. */
    readonly publisher: sync.Publisher;
}

/** The subscriptions a database follows to keep its copies, each streamed from its own publisher. */
export interface Subscriber {
    /** The tables whose changes list the subscriptions again, beside the scopes and the residents' homes and addresses. */
    readonly watches?: readonly Table[];
    /** The link to the store the copies follow, which receives the changes this store sends to their rows, absent where it sends none. */
    readonly uplink?: sync.Uplink;
    /** List the subscriptions of the copies the database keeps now, each with its publisher. */
    subscriptions(): Promise<readonly SubscriberEntry[]>;
}

/** Build the subscriber of a database's copies. */
export const Subscriber = {
    /** Keep the copies some subscriptions list, each streamed over one uplink receiving the changes sent to them. */
    of(
        uplink: sync.Uplink,
        subscriptions: () => Promise<readonly sync.Subscription[]>,
    ): Subscriber {
        return {
            uplink,
            subscriptions: async () =>
                (await subscriptions()).map((subscription) => ({
                    subscription,
                    publisher: uplink,
                })),
        };
    },
};

/** A call's prepared external work: the keyed call, its parsed value and its settlement, absent without any. */
type Prepared =
    | { readonly call: Call; readonly value: unknown; readonly settlement: string | undefined }
    | undefined;

/** A resolved call with the author it runs as and the calls its expansion ran before it. */
type Expansion = ReturnType<typeof Call.resolve> & {
    /** The principal the call records as its caller, the caller's when absent. */
    readonly as?: Subject;
    /** The calls the method's expansion ran before it. */
    readonly expansion?: readonly BranchCall[];
};

/** A durable mutation as the journal executes it. */
type MutationRun = {
    /** The mutation's identifier. */
    readonly id: string;
    /** The scope the calls act in. */
    readonly scope: string;
    /** The caller the mutation runs as. */
    readonly caller: Caller;
    /** The code making the calls for a principal running its own code, such as a controller's name. */
    readonly component?: string;
    /** The expanded calls, in order. */
    readonly calls: readonly Expansion[];
    /** The prepared external work, by call. */
    readonly prepared: readonly Prepared[];
    /** The caller, scope and request identifier the journal keys the mutation by. */
    readonly request: RequestIdentity & { readonly caller: string };
    /** The digest of the mutation's redacted calls. */
    readonly fingerprint: string;
    /** Each call's input as its record keeps it. */
    readonly redacted: readonly JsonObject[];
};

/** How an authorization binds its caller beyond its scope: the request a mutation's grants bind to, the scope chain read in the same snapshot, and the scopes the caller lives inside. */
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

/** How a mutation runs beyond its caller: the client instance writing ephemeral objects, the component of a principal's own code, or the open transaction it joins. */
export interface MutateOptions {
    /** The running mutation the calls join, without external work, its owner committing once; the store's own when absent. */
    readonly transaction?: Transaction;
    /** The client instance writing ephemeral objects. */
    readonly writerId?: string;
    /** The code making the calls for a principal running its own code, such as a controller's name. */
    readonly component?: string;
    /** The scope the calls carrying none run in. */
    readonly scope?: string;
}

/** How far the journal got through a mutation: the running call and the reservation of its claims. */
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
    readonly expansion?: readonly BranchCall[];
    /** The request the call belongs to. */
    readonly request?: CallRequest;
    /** The code making the call for a principal running its own code, such as a controller's name. */
    readonly component?: string;
};

/** The options of an object store: the objects it serves over one database, and the stores, machines and providers beside them. */
export interface ObjectStoreOptions {
    /** The object types served, by name. */
    readonly objects: Readonly<Record<string, ObjectType>>;
    /** Further policies and table mappings to authorize. */
    readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The principal the store's own work runs as: its controllers, settlements and the changes it receives in process. */
    readonly principal: Subject;
    /** The one partition the store serves, every partition of its database when absent. */
    readonly partition?: string;
    /** The key sensitive call inputs are fingerprinted under in the journal, given exactly when object types serve methods. */
    readonly callKey?: CallKey;
    /** The package and service recording the calls. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The audit history the journal delivers calls to, absent where another store delivers them. */
    readonly history?: AuditDestination;
    /** The claims of unique indexes, kept by the account service. */
    readonly claims?: ClaimStore;
    /** The installation serving the objects, which sends and receives projected rows, absent for a machine. */
    readonly installation?: InstallationContext;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** The files with the served external objects, each keeping some of their types. */
    readonly external?: readonly ExternalStorage[];
    /** The subscriptions the database follows to keep its copies. */
    readonly subscriber?: Subscriber;
    /** The shapes the store serves and follows beside its objects' and its policies' own, such as a package's own copies. */
    readonly shapes?: readonly sync.Shape[];
    /** The machine recording the objects' runs: the calls methods send, and the calls of their change triggers. */
    readonly runs?: RunClient;
    /** The triggers of the served objects' package, whose change triggers need a machine recording their runs. */
    readonly triggers?: readonly Trigger[];
    /** Report failed settlements, thrown when absent. */
    readonly report?: (error: unknown) => void;
    /** Whether reads of the objects record access events, as the space's audit setting asks. */
    readonly isAccessAudited?: boolean;
    /** The clock calls run and controllers reconcile at. */
    readonly clock: Clock;
    /** The scope's branch types, among the objects, whose branches reads can see. */
    readonly branch?: BranchType;
    /** The providers reconciling the served kinds' resources, absent for a store reconciling none. */
    readonly provisioned?: ProvisionedOptions;
}

/** A store's routes: each served object's procedures by its key, and the replica procedures. */
export type ObjectRouter = Readonly<Record<string, Router<ServiceRouter, ServiceContext>>> & {
    readonly replica: Router<ReplicaProcedures, ServiceContext>;
};

/** A store serving object types' methods over one database. */
export class ObjectStore {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The principal the store's own work runs as: its controllers, settlements and the changes it receives in process. */
    readonly principal: Subject;
    /** The one partition the store serves, every partition of its database when absent. */
    readonly #partition: string | undefined;
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
    /** The journal of executed mutations, kept only by a store serving methods. */
    readonly #journal: Journal | undefined;
    /** The package and service the store serves, as the audit records them. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The feed serving every sync. */
    readonly feed: sync.Feed;
    /** The claims of the served objects' unique indexes. */
    readonly claims: ClaimStore | undefined;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral: EphemeralStorage | undefined;
    /** The files with the served external objects. */
    readonly external: readonly ExternalStorage[];
    /** The subscriptions the database follows to keep its copies. */
    readonly subscriber: Subscriber | undefined;
    /** The durable object types the store serves or decides, home or copied, whose rows stand for their principals here. */
    readonly decided: readonly ObjectType[];
    /** The shapes the store serves and follows beside its objects' and its policies' own. */
    readonly shapes: readonly sync.Shape[];
    /** The installation serving the objects, which sends and receives projected rows, absent for a machine. */
    readonly installation: InstallationContext | undefined;
    /** The copied types identifying the residents whose rows the served types project, absent where nothing projects. */
    readonly residence: Residence | undefined;
    /** The durable object types, served or authorized, whose rows the database keeps as copies from their home. */
    readonly copied: readonly ObjectType[];
    /** The machine recording the objects' runs: the calls methods send, and the calls of their change triggers. */
    readonly runs: RunClient | undefined;
    /** Stream the copies of the served rows to a store following them in this process, and run the changes it sends to them as its principal. */
    readonly uplink: sync.Uplink;
    /** The change triggers of the served objects, each change they admit recorded as a run. */
    readonly triggers: readonly ChangeTrigger[];
    /** The audit history the journal delivers calls to, absent where another store delivers them. */
    readonly #history: AuditDestination | undefined;
    /** The outbox delivering the calls methods send and the changes of copied rows, absent where nothing receives them. */
    readonly #sends?: Outbox;
    /** Open an audit recorder for a scope and a caller, with the request and component its calls answer. */
    readonly audit: (
        scope: string,
        caller: Caller,
        called?: AuditCalled,
    ) => AuditRecorder<DatabaseConnection>;
    /** Record a read of a scope's events that showed their personal values, as the event service and its readers report one. */
    readonly unmasked = async (context: ServiceContext, read: UnmaskedRead): Promise<void> => {
        const { scope, object, kind, operation } = read;
        await this.audit(scope, ObjectStore.#requestCaller(context), {
            requestId: context.requestId,
        }).record(
            undefined,
            eventUnmask,
            {
                target: { type: object.type, id: object.id },
                details: { kind: kind.name, operation },
                outcome: { kind: "success" },
            },
            "access",
        );
    };
    /** Report failed settlements. */
    readonly #report: (error: unknown) => void;
    /** The clock calls run and controllers reconcile at. */
    readonly clock: Clock;
    /** The scope's branch types, absent for a store without branches. */
    readonly branch: BranchType | undefined;
    /** The served kinds' resources, each controlled by the loop its trait builds. */
    readonly #provisioned: readonly ObjectType[];
    /** The providers reconciling the served kinds' resources, none for a store reconciling none. */
    readonly providers: ProvisionedOptions["providers"];
    /** Whether reads of the objects record access events. */
    readonly isAccessAudited: boolean;
    /** The copies the objects stream to clients and databases below. */
    readonly source: ObjectSource;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, GrantReader>();
    /** The scopes each caller lives inside, read once per caller and database. */
    readonly #inside = new WeakMap<Caller, Map<DatabaseConnection, Promise<readonly Subject[]>>>();
    /** The object types routed by name, without the chunk types serving them. */
    readonly routed: readonly ObjectType[];
    /** The wait until the copies gating readiness synced, shared by every service the store implements. */
    #readiness: Promise<boolean> | undefined;

    /** Serve object types over one database. */
    constructor(options: ObjectStoreOptions) {
        // require each object type under its own name, and the stores its storage needs
        this.routed = Object.values(options.objects);
        this.objects = ObjectType.served(this.routed);
        ObjectStore.#requireNames(options.objects);
        ObjectStore.#requireStores(this.objects, options);

        // keep the optional stores and machines
        this.claims = options.claims;
        this.subscriber = options.subscriber;
        this.shapes = options.shapes ?? [];
        this.installation = options.installation;
        this.uplink = this.#uplink();
        this.runs = options.runs;
        this.triggers = ObjectStore.#changeTriggers(options);
        this.ephemeral = options.ephemeral;
        this.external = options.external ?? [];

        // build the authorizer over the objects, the scope types enclosing them and the residence
        this.database = options.database;
        this.principal = options.principal;
        this.#partition = options.partition;
        this.residence = ObjectStore.#residence(this.objects);
        this.authorizer = ObjectStore.#authorizer(this.objects, this.residence, options);
        TrackedHistory.require(this.objects, this.authorizer);

        // keep the journal, the audit and the request context
        this.#journal = ObjectStore.#journalOf(this.routed, options);
        this.origin = options.origin;
        this.audit = (scope, caller, called) =>
            AuditRecorder.service(this.journal, options.origin)(scope, caller, called);
        this.#history = options.history;
        this.isAccessAudited = options.isAccessAudited ?? false;
        this.source = new ObjectSource(this);
        this.clock = options.clock;
        this.branch = options.branch;
        this.#report = options.report ?? rethrow;
        this.#provisioned = ObjectStore.#control(this.objects, options.provisioned);
        this.providers = options.provisioned?.providers ?? [];

        // follow the durable objects' tables in one feed, with an outbox where calls and changes of copied rows leave
        const followed = ObjectStore.#follow(this.objects, options);
        this.decided = followed.decided;
        this.copied = followed.copied;
        this.feed = followed.feed;
        if (followed.sends !== undefined) {
            this.#sends = followed.sends;
        }
    }

    /** Follow the durable objects' tables and the decision tables in one feed, with an outbox where calls or changes of copied rows leave. */
    static #follow(
        objects: readonly ObjectType[],
        options: Pick<
            ObjectStoreOptions,
            "database" | "policies" | "runs" | "subscriber" | "clock"
        >,
    ): {
        readonly decided: ObjectType[];
        readonly copied: ObjectType[];
        readonly feed: sync.Feed;
        readonly sends?: Outbox;
    } {
        // find the durable and copied object types, and follow their tables
        const decided = ObjectStore.#durable(objects, options.policies ?? []);
        const copied = decided.filter((object) => options.database.copies(object.table));
        const followed = decided.flatMap((object) => object.tables);
        const feed = new sync.Feed(
            options.database,
            [...new Set([...followed, ...decisionTables])],
            {
                clock: options.clock,
            },
        );

        // keep an outbox for the machine recording runs and the copies' home
        const isUplinked = options.subscriber?.uplink !== undefined && copied.length > 0;
        const isSending = options.runs !== undefined || isUplinked;

        return isSending
            ? { decided, copied, feed, sends: new Outbox(options.database) }
            : { decided, copied, feed };
    }

    /** Stream the copies of the served rows to stores following them in this process, and run the changes they send. */
    #uplink(): sync.Uplink {
        return {
            stream: (subscription, signal) => this.source.publish(subscription, signal),
            receive: async (mutation, signer) => {
                // run each change as the follower's principal
                if (signer === undefined) {
                    throw new TypeError("a change sent in process names the principal sending it");
                }
                for (const call of mutation.calls) {
                    await this.receive(call, signer);
                }
            },
        };
    }

    /** Control each served kind's resources with the loop their trait builds, none without providers. */
    static #control(
        objects: readonly ObjectType[],
        provisioned: ProvisionedOptions | undefined,
    ): ObjectType[] {
        return provisioned === undefined
            ? []
            : objects
                  .filter((object) => Provisioned.is(object))
                  .map((object) =>
                      object.implement({
                          controller: new ProvisionedController(object, provisioned),
                      }),
                  );
    }

    /** List the durable object types a store serves or decides. */
    static #durable(
        objects: readonly ObjectType[],
        policies: readonly (Policy | ObjectType | TableMapping)[],
    ): ObjectType[] {
        return [...objects, ...policies.filter((other) => other instanceof ObjectType)].filter(
            (object) => object.storage === "durable",
        );
    }

    /** Keep the change triggers of the served package, which need a machine recording their runs. */
    static #changeTriggers(
        options: Pick<ObjectStoreOptions, "runs" | "triggers">,
    ): ChangeTrigger[] {
        const triggers = (options.triggers ?? []).flatMap(
            (trigger) => Trigger.of(trigger, "change") ?? [],
        );
        if (triggers.length > 0 && options.runs === undefined) {
            throw new TypeError("change triggers need a machine recording their runs");
        }

        return triggers;
    }

    /** Build the authorizer over the objects, every scope type enclosing them, the residence and the other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        residence: Residence | undefined,
        options: Pick<ObjectStoreOptions, "database" | "policies" | "clock">,
    ): Authorizer {
        // add the residence's types the store does not serve
        const residents = (
            residence === undefined ? [] : [residence.account, residence.residence]
        ).filter((type) => !objects.some((served) => served.same(type)));

        return ObjectAccess.authorizer(
            objects,
            [...(options.policies ?? []), ...residents],
            (table) => options.database.copies(table),
        );
    }

    /** Keep the journal of a store serving methods, which needs the call key, and refuse a call key without methods. */
    static #journalOf(
        routed: readonly ObjectType[],
        options: Pick<ObjectStoreOptions, "database" | "callKey" | "clock">,
    ): Journal | undefined {
        const isServing = routed.length > 0;
        if (isServing && options.callKey === undefined) {
            throw new TypeError("a store serving methods needs the call key of their journal");
        } else if (!isServing && options.callKey !== undefined) {
            throw new TypeError(
                "a store serving no methods keeps no journal and takes no call key",
            );
        }

        return options.callKey === undefined
            ? undefined
            : new Journal(options.database, options.callKey, { clock: options.clock });
    }

    /** Read the journal of executed mutations, failing for a store serving no methods. */
    get journal(): Journal {
        if (this.#journal === undefined) {
            throw new TypeError("a store serving no methods keeps no journal");
        }

        return this.#journal;
    }

    /** Require each object type under its own key, apart from the shared procedures. */
    static #requireNames(objects: Readonly<Record<string, ObjectType>>): void {
        for (const [key, object] of Object.entries(objects)) {
            // refuse a type under another key
            if (key !== object.key) {
                throw new TypeError(`object ${object.name} is served under ${key}`);
            }
            // refuse a type under a shared procedure's name
            else if (Object.hasOwn(object.shared, key)) {
                throw new TypeError(
                    `object ${object.name} takes the name of the shared ${key} procedures`,
                );
            }
        }
    }

    /** Refuse local types, and require claims for indexed types, a memory store for ephemeral types and files for external types. */
    static #requireStores(
        objects: readonly ObjectType[],
        options: Pick<ObjectStoreOptions, "database" | "claims" | "ephemeral" | "external">,
    ): void {
        // refuse local objects, which live on their client alone
        const local = objects.find((object) => object.storage === "local");
        if (local !== undefined) {
            throw new TypeError(
                `local object ${local.name} lives on its client, and no server serves it`,
            );
        }

        // require claims for indexed objects
        const indexed = objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.claims === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but no claim store keeps their names`,
            );
        }

        // require a store for ephemeral objects
        const unstored = objects.find(
            (object) =>
                object.storage === "ephemeral" &&
                options.ephemeral?.objects.some((stored) => stored.same(object)) !== true,
        );
        if (unstored !== undefined) {
            throw new TypeError(`ephemeral object ${unstored.name} needs a store keeping it`);
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
            throw new TypeError(`external object ${unfiled.name} needs files keeping it`);
        }
    }

    /** Read the residence the projecting types share, refusing types with different ones. */
    static #residence(objects: readonly ObjectType[]): Residence | undefined {
        // read each projecting type's residence, refusing a second one
        const [first, ...others] = objects.flatMap((object) =>
            object.projected === undefined ? [] : [object.projected.residence],
        );
        const differing = others.find(
            (other) =>
                !other.account.same(first?.account) || !other.residence.same(first?.residence),
        );
        if (differing !== undefined) {
            throw new TypeError("the projecting types name different residences");
        }

        return first;
    }

    /** Read the name of the object type a call's method belongs to, as `<object>.<method>` writes it. */
    static #typeOf(call: Pick<ObjectCall, "method">): string {
        return call.method.slice(0, call.method.lastIndexOf("."));
    }

    /** Implement a service routing some of the served objects beside its own procedures, with the store's access, audit and readiness and some controllers. */
    implement(
        service: Service,
        routed: readonly ObjectType[],
        controllers: readonly Controller[],
        procedures: Readonly<Record<string, Router<ServiceRouter, ServiceContext>>>,
    ): ServiceImplementation & { readonly objects: ObjectStore } {
        const { subscriber } = this;

        return {
            objects: this,
            service,
            access: this.access,
            audit: AuditRecorder.procedure((call) =>
                this.audit(
                    this.#procedureScope(call) ?? Scope.universe.id,
                    ObjectStore.#requestCaller(call.context),
                    { requestId: call.context.requestId },
                ),
            ),
            controllers,
            ...(subscriber === undefined
                ? {}
                : { ready: (signal) => this.#awaitReady(subscriber, signal) }),
            router: { ...this.router(routed), ...procedures },
        };
    }

    /** Wait until the copies gating the store's readiness synced, checking them once for every service it implements, or until the service stops. */
    async #awaitReady(subscriber: Subscriber, signal: AbortSignal): Promise<void> {
        // share one check of the copies, checking again after the checking service stopped first
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

    /** Wait until every copy gating this store's readiness holds a complete snapshot, reporting whether they synced before the signal. */
    async #ready(subscriber: Subscriber, signal: AbortSignal): Promise<boolean> {
        // wait on commits until every gating copy synced
        const { database } = this;

        return database.log.until(async () => {
            // check each gating copy
            const entries = await subscriber.subscriptions();
            const gates = entries.filter(({ subscription }) =>
                this.source.isReadinessGate(subscription),
            );
            const synced = await Promise.all(
                gates.map(({ subscription }) =>
                    sync.Replica.isSynced(database, subscription.name, subscription.scope),
                ),
            );

            return synced.every(Boolean);
        }, signal);
    }

    /** The service access deciding the served objects. */
    get access(): ServiceAccess {
        return {
            authorizer: this.authorizer,
            database: this.database,
        };
    }

    /** Route the served objects' methods, or some of them, and the replica procedures. */
    router(routed: readonly ObjectType[] = this.routed): ObjectRouter {
        const routers: Record<string, Router<ServiceRouter, ServiceContext>> = {};
        for (const object of routed) {
            routers[object.key] = this.#route(object);
        }

        return { replica: this.#replica(), ...routers };
    }

    /** Route an object's methods to the call pipeline. */
    #route(object: ObjectType): Record<string, Router<ServiceRouter, ServiceContext>> {
        // bind each method's procedure to the call pipeline
        const routes: Record<string, Router<ServiceRouter, ServiceContext>> = {};
        for (const [name, procedure] of Object.entries(callableProcedures(object))) {
            routes[name] = implement(procedure)
                .$context<ServiceContext>()
                .handler(({ input, context }) => this.#answer(object, name, input, context));
        }

        return routes;
    }

    /** Route the replica procedures. */
    #replica(): Router<ReplicaProcedures, ServiceContext> {
        const replica = implement(replicaProcedures).$context<ServiceContext>();

        return replica.router({
            push: replica.push.handler(({ input, context }) =>
                this.#push(input.scope, input.mutations, context, input.writerId),
            ),
            call: replica.call.handler(async ({ input, context }) => {
                // read in the given scope
                const {
                    object,
                    name,
                    input: called,
                } = this.#resolve(input.call, false, context.caller(input.scope));
                if (this.#scope(object, called) !== input.scope) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `${object.name}.${name} reads another scope than ${input.scope}`,
                    });
                }

                return await this.#answer(object, name, called, context);
            }),
            receive: replica.receive.handler(async ({ input, context }) => {
                await this.receiveFrom(input.mutation, context);

                return {};
            }),
            broadcast: replica.broadcast.handler(async ({ input, context }) => {
                await this.source.broadcast(input.scope, input.object, input.event, context);

                return {};
            }),
            stream: replica.stream.handler(({ input, context, lastEventId }) => {
                const { after: _after, ...subscription } = input;
                const after = resumed(input.after, lastEventId);

                return this.source.relayed(
                    after === undefined ? subscription : { ...subscription, after },
                    context,
                );
            }),
        });
    }

    /** Read a request's caller for its audit: anonymous when its credential failed. */
    static #requestCaller(context: ServiceContext): Caller {
        return context.authenticationError === undefined ? context.caller() : { subjects: [] };
    }

    /** Answer one routed procedure as its request's caller: enter its pinned scope, run the method, report the watermark of a write and answer the procedure's output. */
    async #answer(
        object: ObjectType,
        name: string,
        input: JsonObject,
        context: ServiceContext,
    ): Promise<unknown> {
        // split off the request's identifier or the writer, and enter the call's scope as the request's caller
        const method = object.method(name);
        const isEphemeral = object.storage === "ephemeral";
        const { [isEphemeral ? "writerId" : "requestId"]: named, ...rest } = input;
        const scope = this.#scope(object, rest);
        await this.enter(context, scope);
        const caller = context.caller(scope);

        // read through a query, or write through a single-call mutation and report its watermark
        const call = Call.record(object, name, rest);
        let result: unknown;
        if (!method.mutates) {
            result = await this.query(caller, call);
        } else {
            const [written] = isEphemeral
                ? await this.mutate(
                      caller,
                      { id: RequestId.create(this.clock.now()), calls: [call] },
                      {
                          writerId: WriterId.parse(named),
                      },
                  )
                : await this.mutate(caller, { id: schema.string().parse(named), calls: [call] });
            context.observed.observe(await this.watermark(scope));
            result = written;
        }

        // answer what the procedure answers remote callers
        const output = schema.standard().parse(object.procedure(name).output);
        const validated = await output["~standard"].validate(result);
        if (validated.issues !== undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `invalid output of ${object.name}.${name}`,
                data: { issues: validated.issues },
            });
        }

        return validated.value;
    }

    /** Read through a method as a caller, in a read-only transaction, recording the reads its method or the store audits. */
    async query<Result>(caller: Caller, call: ObjectCall<Result>): Promise<Result>;
    /**
     * Read through a method as a caller.
     *
     * @construct the result is the result the call's method answers, which ObjectCall carries from type.call.
     */
    async query(caller: Caller, call: ObjectCall): Promise<unknown> {
        // resolve the call, reading only the one method a Loan lends
        const { object, name, input } = this.#resolve(call, false, caller);
        const method = object.method(name);
        Loan.require(caller, Loan.method(object, name));
        const scope = this.#scope(object, input);
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
                await this.#settle(settlementsOf(prepared));
            }
        };
        const readIn = async (prepared: Prepared) => {
            // read over a mutation that never commits, admitting the caller against the durable database
            const reading = Transaction.reading(database);
            const admitting =
                object.storage === "durable" ? reading : Transaction.reading(this.database);
            const authorization = await this.admit(admitting, scope, caller);

            return this.#execute(reading, authorization, object, name, input, caller, { prepared });
        };

        // read unaudited objects directly
        if (!object.isReadAudited && method.audited !== true && !this.isAccessAudited) {
            return read();
        }

        // audit one object, or the scope's collection
        const recorder = this.audit(scope, caller);
        const { action, values } = object.auditCall(name, input, scope);
        const details = method.audit?.details;

        // record external work as an attempt and its result
        if (method.audited === true) {
            return recorder.attempt(action, values, read);
        }
        // record a read as one access event, with the details of its result
        else {
            return recorder.read(action, values, read, details && detailsOf(details));
        }
    }

    /** Open the database keeping an object type's rows in a scope. */
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

    /** Run a mutation's calls as a caller in one transaction, answering it once per caller and id, each submitted call's result in order. */
    async mutate<const Calls extends readonly ObjectCall[]>(
        caller: Caller,
        transaction: { readonly id: string; readonly calls: Calls },
        options?: MutateOptions,
    ): Promise<ResultsOf<Calls>>;
    /**
     * Run a mutation's calls as a caller in one transaction.
     *
     * @construct each result is the result its call's method answers, which ObjectCall carries from type.call.
     */
    async mutate(
        caller: Caller,
        submitted: { readonly id: string; readonly calls: readonly ObjectCall[] },
        options: MutateOptions = {},
    ): Promise<unknown[]> {
        // place the calls carrying no scope in the mutation's scope
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

        // resolve each call in one scope, as the one method a Loan lends when the caller came with one
        const originals = mutation.calls.map((entry) => this.#resolve(entry, true, caller));
        for (const { object, name } of originals) {
            Loan.require(caller, Loan.method(object, name));
        }
        const scope = this.#mutationScope(originals);

        // write ephemeral objects apart
        const ephemeral = originals.filter((call) => call.object.storage === "ephemeral").length;
        if (ephemeral === originals.length) {
            return this.#mutateEphemeral(originals, scope, caller, mutation.id, options.writerId);
        } else if (ephemeral > 0) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation writes durable or ephemeral objects, not both",
            });
        }

        // expand calls into the calls they run after, and identify prepared creations
        const calls = (await this.#expand(originals, scope, caller)).map(identified);

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

        // execute through the journal and keep the running call
        const progress: MutationProgress = { running: undefined, reservation: undefined };
        const results = await this.#executeDurable(planned, progress);

        // confirm keys and settle prepared work
        try {
            await progress.reservation?.confirm();
        } finally {
            await this.#settle(settlementsOf(prepared));
        }

        // answer the submitted calls without their expansions
        return results.filter((_result, index) => aligned(calls, index).as === undefined);
    }

    /** Place a call in a scope when it carries none, by the route field of its served or copied type. */
    #place(entry: ObjectCall, scope: string): ObjectCall {
        // find the route field of the call's served or copied type
        const typeName = ObjectStore.#typeOf(entry);
        const type = [...this.objects, ...this.copied].find((known) => known.name === typeName);
        const field = type?.route.field;

        return field === undefined || Object.hasOwn(entry.input, field)
            ? entry
            : { ...entry, input: { ...entry.input, [field]: scope } };
    }

    /** Run calls inside a running mutation without external work: a change of copied rows goes toward their home once it commits, the rest run here. */
    async #mutateInside(
        transaction: Transaction,
        mutation: { readonly id: string; readonly calls: readonly ObjectCall[] },
        caller: Caller,
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
                await this.#runInside(transaction, entry, caller, true, undefined, recorded),
            );
        }

        return results;
    }

    /** Run one call of the platform's own code inside a running mutation: a change of copied rows goes toward their home once it commits, the rest runs here as the caller, under the calling method's authorization or admitted in the call's scope. */
    async #runInside(
        transaction: Transaction,
        entry: ObjectCall,
        caller: Caller,
        mutates: boolean,
        parent: Call | undefined,
        options: CallOptions,
    ): Promise<unknown> {
        // place the call in the calling method's scope unless it carries another
        const placed = parent === undefined ? entry : this.#place(entry, parent.scope);

        // read whether the call's type is served here or only copied from its home
        const typeName = ObjectStore.#typeOf(placed);
        const copied = this.copied.find((type) => type.name === typeName);
        const isServed = this.objects.some((served) => served.name === typeName);
        const served = parent?.location.kind === "server" ? parent.location : undefined;
        const sending = served?.authorization.delegation;
        const send = (call: sync.Call) =>
            this.send(call, transaction, sending === undefined ? {} : { delegation: sending });

        // send a change of a type only copied here toward its home
        if (mutates && !isServed && copied !== undefined) {
            await send(Call.record(copied, placed.method.slice(typeName.length + 1), placed.input));

            return undefined;
        }

        // resolve the call, internal methods included, refusing external work and another storage
        const { object, name, input } = Call.resolve(this.objects, placed, mutates, false);
        if (this.served(object).method(name).prepare !== undefined) {
            throw new TypeError(
                `${object.name}.${name} does external work, which a call inside a transaction cannot`,
            );
        } else if (object.storage !== (parent?.object.storage ?? "durable")) {
            throw new TypeError(`${object.name} is stored apart from the transaction`);
        }

        // send a change of rows kept here as copies toward their home
        const scope = this.#scope(object, input);
        if (mutates && (await object.isCopy(transaction.view(), scope))) {
            await send(Call.record(object, name, input));

            return undefined;
        }

        // run it under the calling method's authorization for its own caller in its scope, else admit the caller there, guarding the chain once outside a method
        const authorization =
            served !== undefined && parent?.caller === caller && parent.scope === scope
                ? served.authorization
                : await this.admit(
                      transaction,
                      scope,
                      caller,
                      parent === undefined
                          ? { links: await Scope.chain(transaction.view(), scope) }
                          : { now: parent.now },
                  );

        return this.#execute(transaction, authorization, object, name, input, caller, options);
    }

    /** Read the one scope a mutation's calls act in. */
    #mutationScope(originals: readonly ReturnType<typeof Call.resolve>[]): string {
        // require every call to act in the first call's scope
        const first = aligned(originals, 0);
        const scope = this.#scope(first.object, first.input);
        if (originals.some((call) => this.#scope(call.object, call.input) !== scope)) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation's calls act in one scope",
            });
        }

        return scope;
    }

    /** Identify a durable mutation by caller, scope and request identifier, with its redacted calls. */
    async #identify(
        mutation: { readonly id: string; readonly calls: readonly ObjectCall[] },
        originals: readonly ReturnType<typeof Call.resolve>[],
        calls: readonly Expansion[],
        scope: string,
        caller: Caller,
        options: MutateOptions,
    ): Promise<MutationRun> {
        // require an authenticated caller the journal keys the mutation by
        if (caller.subject === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
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

    /** Execute a planned mutation once: answer a recorded outcome, else run its calls over a mutation and commit it, undoing its prepared work and keys once it fails. */
    async #executeDurable(planned: MutationRun, progress: MutationProgress): Promise<unknown[]> {
        // reject a key expired or in the future, and answer an outcome a previous copy recorded
        RequestId.expiry(planned.request.requestId, this.clock.now());
        const recorded = await this.journal.replay(planned.request, planned.fingerprint);
        if (recorded !== undefined) {
            return recorded;
        }
        try {
            // run the authorized calls over a transaction committed with their records, again while it loses a race
            return await Transaction.run(this.database, async (transaction) => {
                const authorization = await this.#authorizeRun(transaction, planned);

                return this.#run(transaction, authorization, planned, progress);
            });
        } catch (error) {
            // answer the outcome a concurrent copy committed, which made this copy fail
            const committed = await this.journal.replay(planned.request, planned.fingerprint);
            if (committed !== undefined) {
                return committed;
            }

            // roll back prepared work and release keys
            await this.#settle(settlementsOf(planned.prepared));
            await this.claims?.release(planned.id).catch((failure: unknown) => {
                throw new AggregateError(
                    [error, failure],
                    "transaction and its key release failed",
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

    /** Admit a mutation's caller in its scope chain, refusing an installation's writes to a space whose storage is capped. */
    async #authorizeRun(transaction: Transaction, planned: MutationRun): Promise<Authorization> {
        // admit the caller in the scope chain the mutation reads
        const chain = await Scope.chain(transaction.view(), planned.scope);
        const authorization = await this.admit(transaction, planned.scope, planned.caller, {
            request: planned.id,
            links: chain,
        });

        // refuse an installation's writes to a space whose storage is capped, the submitted calls deciding
        if (this.installation !== undefined) {
            authorization.requireUncapped(planned.calls.filter((call) => call.as === undefined));
        }

        return authorization;
    }

    /** Digest a mutation's calls with their sensitive inputs redacted. */
    async #fingerprint(
        mutation: { readonly id: string; readonly calls: readonly ObjectCall[] },
        originals: readonly ReturnType<typeof Call.resolve>[],
    ): Promise<string> {
        const sensitive: unknown[] = [];
        const calls = mutation.calls.map((entry, index) => {
            const { object, name, input } = aligned(originals, index);
            const fields = InputSchema.fields(object.input(name));

            return {
                method: entry.method,
                input: schema.redact(fields, input, (found) => sensitive.push(found)),
            };
        });

        return this.journal.fingerprint({ id: mutation.id, calls }, sensitive);
    }

    /** Execute a mutation's calls in order over the mutation, recording each for retries once it commits. */
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

        // commit prepared work, release request-bound relationships and reserve the claimed names with the mutation
        progress.running = undefined;
        const claimants = await Claimant.written(transaction, this.objects);
        transaction.record(async (committing) => {
            await this.#commit(committing, run.prepared);
            await this.authorizer.release(committing, run.id);
            progress.reservation =
                this.claims &&
                (await Reservation.reserve(
                    this.claims,
                    committing,
                    claimants,
                    this.objects,
                    run.id,
                ));
        });

        return executed;
    }

    /** Record a mutation's failed call for audit and retries, leaving denials to the procedure layer. */
    async #recordFailure(run: MutationRun, position: number, error: unknown): Promise<void> {
        // skip a denial
        const outcome = AuditRecorder.outcome(error);
        if (outcome.kind === "denied") {
            return;
        }

        // record the failed call with its request
        const failed = aligned(run.calls, position);
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
            input: aligned(run.redacted, position),
        };
    }

    /** Expand calls whose methods list the calls they run after, each run as its author. */
    async #expand(
        calls: readonly ReturnType<typeof Call.resolve>[],
        scope: string,
        caller: Caller,
    ): Promise<Expansion[]> {
        // keep calls whose methods expand into nothing
        const methods = calls.map((call) => this.served(call.object).method(call.name));
        if (methods.every((method) => method.expand === undefined)) {
            return [...calls];
        }

        // list each expanding call's calls before it, as the caller reads them now over a mutation that never commits
        const reading = Transaction.reading(this.database);
        const authorization = await this.admit(reading, scope, caller);
        const expanded: Expansion[] = [];
        for (const [index, call] of calls.entries()) {
            // keep a call that expands into nothing
            const method = aligned(methods, index);
            if (method.expand === undefined) {
                expanded.push(call);
            }
            // run each listed call as its author before the expanding call
            else {
                const built = await this.#call(
                    reading,
                    authorization,
                    call.object,
                    call.name,
                    call.input,
                    caller,
                );
                const listed = await method.expand(built.call);
                for (const { author, ...entry } of listed) {
                    expanded.push({
                        ...this.#resolve(this.#place(entry, scope), true, caller),
                        as: author,
                    });
                }
                expanded.push({ ...call, expansion: listed });
            }
        }

        return expanded;
    }

    /** Run a client's pushed mutations in order as its request's caller, stopping at a transient failure, and answer the scope's watermark. */
    async #push(
        scope: string,
        mutations: readonly sync.Mutation[],
        context: ServiceContext,
        writerId?: string,
    ): Promise<PushResult> {
        // enter the pushed scope as the request's caller
        await this.enter(context, scope);
        const caller = context.caller(scope);

        // run each mutation in its own transaction
        const outcomes: PushResult["outcomes"][number][] = [];
        for (const pushed of mutations) {
            const mutation = {
                ...pushed,
                calls: pushed.calls.map((call) => this.#place(call, scope)),
            };
            try {
                // refuse a mutation of copied rows, which only their home writes
                this.#requireServed(mutation);

                // require the pushed scope
                const first = this.#resolve(aligned(mutation.calls, 0), true, caller);
                if (this.#scope(first.object, first.input) !== scope) {
                    throw new ServiceError("BAD_REQUEST", {
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

    /** Find the served copy of an object type with the store's handlers. */
    served<Type extends ObjectType>(object: Type): Type;
    /**
     * Find the served copy of an object type.
     *
     * @construct the served copy is the declared type with this store's handlers, matched by package and name.
     */
    served(object: ObjectType): ObjectType {
        const found = this.objects.find((served) => served.same(object));
        if (found === undefined) {
            throw new TypeError(`object store serves no ${object.name}`);
        }

        return found;
    }

    /** Send a call to run once a transaction on this store's database commits, at the store keeping its rows or through the machine recording runs. */
    async send(
        call: sync.Call,
        transaction: Transaction,
        sending: { readonly at?: number; readonly delegation?: string } = {},
    ): Promise<void> {
        // require an outbox delivering the store's sends
        if (this.#sends === undefined) {
            throw new TypeError(`the store sends ${call.method}, but nothing receives its sends`);
        }

        // record the send once the mutation commits
        const requestId = RequestId.create();
        const sends = this.#sends;
        transaction.record(async (committing) => {
            await sends.append(
                OutboxCall,
                requestId,
                { requestId, request: { call, ...sending } },
                committing,
            );
        });
    }

    /** Run a call another store in this process sent to change the served rows as its principal, or pass a change of copied rows toward their home, once however often it is delivered. */
    async receive(call: sync.Call, as: Subject): Promise<void> {
        // pass a change of copied rows to the store keeping them
        if (await this.#forward(call, RequestId.create())) {
            return;
        }

        // resolve the served object and method, those not routed to people included
        const separator = call.method.lastIndexOf(".");
        const object = this.objects.find(
            (served) => served.name === call.method.slice(0, separator),
        );
        const name = call.method.slice(separator + 1);
        const method = object?.methods[name];
        if (object === undefined || method === undefined || !method.mutates) {
            throw new ServiceError("BAD_REQUEST", { message: `no mutating method ${call.method}` });
        }

        // read the target or the chosen identifier
        const id = typeof call.input["id"] === "string" ? call.input["id"] : undefined;
        const [target] =
            id === undefined
                ? []
                : await this.database
                      .select()
                      .from(object.table)
                      .where(object.withIds([id]));

        // pass a creation delivered before, and refuse a missing target
        if (!method.target && method.kind === "create" && target !== undefined) {
            return;
        } else if (method.target && target === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no ${object.name} ${id ?? ""}` });
        }

        // run it as the sending principal
        const now = this.clock.now();
        await this.mutate(Caller.of(as, now), { id: RequestId.create(now), calls: [call] });
    }

    /** Run a mutation a follower sent to the served rows as the caller, or pass one of copied rows from an installation of this space toward their home, signed as it. */
    async receiveFrom(transaction: sync.Mutation, context: ServiceContext): Promise<void> {
        // pass an installation's change of copied rows toward their home, which the space's key signs only for its own
        const caller = context.caller();
        const { subject } = context.requireCaller();
        if (principal.installation.is(subject) && this.#isForwarded(transaction)) {
            await (await this.#homeOfAll(transaction)).receive(transaction, subject);
        }
        // run the served rows' change as the caller, refusing anyone else's change of copied rows
        else {
            this.#requireServed(transaction);
            await this.mutate(caller, transaction);
        }
    }

    /** Report whether every call of a mutation changes rows copied over the uplink. */
    #isForwarded(transaction: sync.Mutation): boolean {
        return (
            this.subscriber !== undefined &&
            transaction.calls.every((call) => this.#isUplinked(ObjectStore.#typeOf(call)))
        );
    }

    /** Report whether an object type's rows are copied over the uplink rather than kept here. */
    #isUplinked(type: string): boolean {
        const object = this.copied.find((copied) => copied.name === type);

        return object !== undefined && !this.objects.some((served) => served.same(object));
    }

    /** Find the one publisher receiving every call of a mutation of copied rows, refusing a mutation spanning several homes. */
    async #homeOfAll(transaction: sync.Mutation): Promise<sync.Uplink> {
        // require one home for every call
        const homes = await Promise.all(transaction.calls.map((call) => this.#homeOf(call)));
        const [home] = homes;
        if (home === undefined || homes.some((each) => each !== home)) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation of copied rows changes rows of one home",
            });
        }

        return home;
    }

    /** Refuse a mutation changing rows this store only copies, which their home alone writes. */
    #requireServed(transaction: { readonly calls: readonly ObjectCall[] }): void {
        // find each call's type that no served object keeps
        const copies = transaction.calls
            .map((call) => ObjectStore.#typeOf(call))
            .filter((type) => !this.objects.some((object) => object.name === type));
        if (copies.length === 0) {
            return;
        }

        throw new ServiceError("BAD_REQUEST", {
            message: `the rows ${[...new Set(copies)].join(", ")} change are copies here, written only at their home`,
        });
    }

    /** Find the publisher receiving a change of copied rows: the one the copy keeping the rows' table in the call's scope follows, else the subscriber's uplink. */
    async #homeOf(call: sync.Call): Promise<sync.Uplink | undefined> {
        // find the copy of the call's table in its scope whose publisher receives changes
        const type = ObjectStore.#typeOf(call);
        const object = this.copied.find((copied) => copied.name === type);
        const scope = object === undefined ? undefined : this.#scope(object, call.input);
        const entries = (await this.subscriber?.subscriptions()) ?? [];
        const copy = entries.find(({ subscription, publisher }) => {
            const replica = this.source.replicaOf(subscription);

            return (
                isUplink(publisher) &&
                replica.scope === scope &&
                object !== undefined &&
                replica.tables.includes(object.table)
            );
        });

        // fall back to the subscriber's uplink
        const publisher = copy?.publisher;

        return publisher !== undefined && isUplink(publisher) ? publisher : this.subscriber?.uplink;
    }

    /** Pass a change of copied rows toward the store keeping them over the uplink the copies follow, answering whether it passed. */
    async #forward(call: sync.Call, requestId: string): Promise<boolean> {
        // leave a change of rows the database keeps itself
        const type = ObjectStore.#typeOf(call);
        const object = this.copied.find((copied) => copied.name === type);
        if (object === undefined || this.objects.some((served) => served.same(object))) {
            return false;
        }

        // pass it to the publisher the copy keeping its rows follows, once per request, signed as this store's principal
        const uplink = await this.#homeOf(call);
        if (uplink !== undefined) {
            await uplink.receive({ id: requestId, calls: [call] }, this.principal);
        }
        // refuse a change nothing here receives
        else {
            throw new ServiceError("BAD_REQUEST", {
                message: `nothing here receives changes of copied ${object.name} rows`,
            });
        }

        return true;
    }

    /** Deliver a sent call to the store keeping the rows it changes, or to the machine recording runs. */
    async #deliver(sent: OutboxCall, signal: AbortSignal): Promise<void> {
        // deliver a change of copied rows toward their home
        const { call } = sent.request;
        if (await this.#forward(call, sent.requestId)) {
            return;
        }
        // record any other sent call's run at the machine
        else if (this.runs !== undefined) {
            await this.runs.send(sent.request, { requestId: sent.requestId, signal });
        }
        // refuse a call nothing receives
        else {
            throw new ServiceError("BAD_REQUEST", {
                message: `nothing receives the sent call ${call.method}`,
            });
        }
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
            ...(this.#journal === undefined ? [] : [this.#journal.controller(this.#history)]),
            ...(this.claims === undefined
                ? []
                : [new ClaimController(this.claims, this.database, this.objects, this.clock)]),
            ...(isRecoverable ? [new RecoverableController(this)] : []),
            ...(isExpiring ? [new ExpiringController(this)] : []),
            ...(isSettled ? [Settlement.controller(this)] : []),
            ...(this.#sends === undefined
                ? []
                : [
                      this.#sends.controller(
                          OutboxCall.destination(
                              (sent, signal) => this.#deliver(sent, signal),
                              this.#report,
                          ),
                      ),
                  ]),
            ...(this.runs === undefined || this.triggers.length === 0
                ? []
                : [new ChangeController(this.database, this.triggers, this.runs, this.#report)]),
            ...(this.subscriber === undefined ? [] : [this.source.controller(this.subscriber)]),
            ...[this.source.addresses()].filter((controller) => controller !== undefined),
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
    reader(access: Access, position: LogPosition): GrantReader {
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

    /** Resolve a caller's access in a scope at the store's time, inside the scopes its principal lives in, bound to a request for mutations. */
    async authorize(
        transaction: Transaction,
        scope: string,
        caller: Caller,
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
        const bind = (): AccessContext =>
            AccessContext.of(
                { ...caller, subjects: [...caller.subjects, ...within] },
                now,
                request,
            );

        // resolve its access
        const access = await this.authorizer.resolve(transaction.view(), scope, bind(), links);

        return new Authorization(this.authorizer, transaction, bind, access, caller.delegation);
    }

    /** Admit a caller in a scope: resolve its access and require an unmoved scope, which a remote caller must also see. */
    async admit(
        transaction: Transaction,
        scope: string,
        caller: Caller,
        options: Omit<AuthorizeOptions, "within"> = {},
    ): Promise<Authorization> {
        // resolve its access, then require an unmoved scope a remote caller also sees
        const authorization = await this.authorize(transaction, scope, caller, options);
        authorization.requireUnmoved();
        authorization.requirePartition(this.#partition);
        if (Caller.isRemote(caller)) {
            await authorization.requireVisible(this.objects);
        }

        return authorization;
    }

    /** Enter a scope: require the pinned scope and wait for observed writes. */
    async enter(context: ServiceContext, scope: string): Promise<void> {
        // require the pinned scope
        if (context.scope !== undefined && context.scope !== scope) {
            throw new ServiceError("NOT_FOUND", {
                message: `scope ${scope} is outside the pinned scope`,
            });
        }

        // wait for observed writes
        await this.reach(context, scope);
    }

    /** Wait until the database has the caller's watermarks in the scope chain. */
    async reach(context: ServiceContext, scope: string): Promise<void> {
        // collect the scope chain
        const watermarks = context.bookmark.watermarks;
        if (watermarks.length === 0) {
            return;
        }
        const chain = new Set([
            scope,
            ...(await Scope.chain(Snapshot.live(this.database), scope)).map(
                (entry) => entry.object.id,
            ),
        ]);

        // wait for each watermark
        for (const watermark of watermarks.filter((entry) => chain.has(entry.scope))) {
            const signal = AbortSignal.any([
                context.signal,
                AbortSignal.timeout(BOOKMARK_TIMEOUT_MILLISECONDS),
            ]);
            const isReached = await this.#reach(scope, watermark, signal);
            if (!isReached) {
                throw new ServiceError("SERVICE_UNAVAILABLE", {
                    message: `${watermark.scope} has not reached the required sequence`,
                });
            }
        }
    }

    /** Read the watermark the scope's log has now. */
    async watermark(scope: string): Promise<Watermark> {
        return { scope, ...(await this.database.log.position()) };
    }

    /** Wait until the scope's log or a copy reaches a watermark. */
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

        // wait on the copy of an enclosing scope
        return await sync.Replica.wait(this.database, watermark.scope, watermark, signal);
    }

    /** The served durable object types. */
    get durable(): ObjectType[] {
        return this.objects.filter((object) => object.storage === "durable");
    }

    /** Read the memory store with the ephemeral objects, failing without one. */
    requireEphemeral(): EphemeralStorage {
        if (this.ephemeral === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: "no ephemeral objects are served here",
            });
        }

        return this.ephemeral;
    }

    /** Execute ephemeral calls of a request in one memory transaction as their writer. */
    async #mutateEphemeral(
        calls: readonly { object: ObjectType; name: string; input: JsonObject }[],
        scope: string,
        caller: Caller,
        requestId: string,
        writerId: string | undefined,
    ): Promise<unknown[]> {
        // require the writer owning the rows
        if (writerId === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "ephemeral objects are written by a writer",
            });
        }

        // admit the caller, then execute the calls in order over a transaction of the writer's rows, committed in its write
        const authorization = await this.admit(Transaction.reading(this.database), scope, caller);

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

    /** Resolve a recorded call against the served types. */
    #resolve(entry: ObjectCall, mutates: boolean, caller: Caller): ReturnType<typeof Call.resolve> {
        return Call.resolve(this.objects, entry, mutates, Caller.isRemote(caller));
    }

    /** Read the scope a procedure call acts in: an object method's own scope, else the decided target's or the context's. */
    #procedureScope(call: ProcedureCall<ServiceContext>): string | undefined {
        // read the routed object's scope field from the input
        const object = this.routed.find((each) => each.key === call.path[0]);
        const input = call.input === undefined ? undefined : PROCEDURE_INPUT.parse(call.input);
        const field = object?.route.field;
        const named = field === undefined ? undefined : input?.[field];

        return typeof named === "string"
            ? named
            : (call.context.target?.scope ?? call.context.scope);
    }

    /** Read the scope a call carries in its route field. */
    #scope(object: ObjectType, input: JsonObject): string {
        // read the field, or the universe
        const { field } = object.route;
        const named = field === undefined ? Scope.universe.id : input[field];
        if (typeof named !== "string") {
            throw new ServiceError("BAD_REQUEST", { message: `call names no ${field}` });
        }

        return named;
    }

    /** Build one call with its permitted target at the given revision. */
    async #call(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: Caller,
        options: Pick<CallOptions, "as" | "expansion"> = {},
    ): Promise<{ call: Call; method: Method; scope: string; targetId: string | undefined }> {
        // refuse changing rows kept here as copies from their home
        const method = object.method(name);
        const scope = this.#scope(object, input);
        if (method.mutates && (await object.isCopy(transaction.view(), scope))) {
            throw new ServiceError("CONFLICT", {
                message: `${object.name} rows of ${scope} are copies, which change at their home`,
            });
        }

        // parse the input and build the call in its scope through the view a read asks for
        const { database } = transaction;
        const { fields, id, revision, at, branch } = ObjectStore.#fields(object, name, input);
        authorization.requireScopeOf(object);
        const targetId = id === undefined ? undefined : schema.string().parse(id);
        const snapshot = await this.#view(database, authorization, scope, at, branch);
        const call = new Call({
            object,
            name,
            method,
            scope,
            chain: authorization.chain,
            input: fields,
            ...(targetId === undefined ? {} : { id: targetId }),
            database,
            caller:
                options.as === undefined
                    ? caller
                    : Caller.of(options.as, authorization.access.context.now),
            ...(options.expansion === undefined ? {} : { expansion: options.expansion }),
            now: authorization.access.context.now,
            location: {
                kind: "server",
                authorization,
                store: this,
                transaction,
                ...(snapshot === undefined ? {} : { snapshot }),
                ...(this.installation === undefined ? {} : { installation: this.installation }),
            },
            objects: this.objects,
        });

        // load the permitted target at the given revision
        const target = await ObjectStore.#target(call, targetId, snapshot, authorization);
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw new ServiceError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        // refuse callers' other methods on a trashed object
        const isTrashed =
            object.lifecycle.recoverable !== undefined &&
            target !== undefined &&
            target["deletionRequestedAt"] !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.isInternal !== true) {
            throw new ServiceError("CONFLICT", { message: `${object.name} is in the trash` });
        }

        return {
            call: target === undefined ? call : call.with({ target }),
            method,
            scope,
            targetId,
        };
    }

    /** Parse a call's input by its method's input schema, splitting off the fields routing it and the view a read asks for. */
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
        const { field } = object.route;
        const isViewed = method.kind === "get" || method.kind === "list";
        const { id, revision, ...named } = parsed.data;
        const { at, branch, ...viewed } = named;
        const fields = Object.fromEntries(
            Object.entries(isViewed ? viewed : named).filter(([property]) => property !== field),
        );

        return isViewed ? { fields, id, revision, at, branch } : { fields, id, revision };
    }

    /** Load a call's permitted target through the view a read asks for, absent for a call without one. */
    static async #target(
        call: Call,
        targetId: string | undefined,
        snapshot: Snapshot | undefined,
        authorization: Authorization,
    ): Promise<Row | undefined> {
        // skip a call without a target
        const { method } = call;
        if (!method.target || targetId === undefined) {
            return undefined;
        }

        // read the live target, or the target in the view
        if (snapshot === undefined) {
            return authorization.read(call, targetId);
        } else {
            return authorization.readIn(call, targetId, snapshot);
        }
    }

    /** Build the view a read asks for: the database at a position, under a branch's rows, or none for the live main line. */
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
            throw new ServiceError("BAD_REQUEST", { message: "this service serves no branches" });
        }
        const id = schema.string().parse(branch);
        const row = await Snapshot.live(database).row(types.object.table, { id });
        const permission = types.object.permission("read");
        const admitted =
            row === null
                ? undefined
                : await authorization.admitRows(types.object, permission, scope, [row]);
        if (admitted?.permitted.has(0) !== true) {
            throw new ServiceError("NOT_FOUND", { message: `no branch ${id}` });
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

    /** Prepare each call's external work of a request outside the transaction, a mutation's journaled request skipped once executed. */
    async #prepare(
        calls: readonly { object: ObjectType; name: string; input: JsonObject }[],
        scope: string,
        caller: Caller,
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
        const authorization = await this.admit(
            reading,
            scope,
            caller,
            request === undefined ? {} : { request: request.requestId },
        );

        return this.#prepareEach(
            requestId,
            calls.map((entry, index) => {
                const method = aligned(methods, index);
                if (method.prepare === undefined) {
                    return undefined;
                }

                return async () => {
                    const { call } = await this.#call(
                        reading,
                        authorization,
                        entry.object,
                        entry.name,
                        entry.input,
                        caller,
                    );
                    await method.authorize?.(call);

                    return call;
                };
            }),
        );
    }

    /** Prepare a request's built calls in order, reserving each settlement before its work starts and rolling back on failure. */
    async #prepareEach(
        requestId: string,
        builds: readonly ((() => Promise<Call>) | undefined)[],
    ): Promise<Prepared[]> {
        // prepare in order, keeping the reserved settlements
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
                prepared.push(await this.#prepareOne(reserved.call, reserved.settlement));
            }
        } catch (error) {
            // roll back prepared work
            await this.#settle(settlements);
            throw error;
        }

        return prepared;
    }

    /** Key a call's external work, the same on every retry of its request, and reserve its settlement when its method settles. */
    async #reserve(
        call: Call,
        key: string,
    ): Promise<{ readonly call: Call; readonly settlement: string | undefined }> {
        const keyed = call.with({ idempotencyKey: key });

        return {
            call: keyed,
            settlement: Method.settles(call.method)
                ? await Settlement.reserve(this.database, keyed, key)
                : undefined,
        };
    }

    /** Run a reserved call's prepare phase, parsing its value by the declared schema and recording it for its settlement. */
    async #prepareOne(call: Call, settlement: string | undefined): Promise<Prepared> {
        // run the prepare phase the method declares
        const { method } = call;
        const declared = method.prepared;
        if (method.prepare === undefined || declared === undefined) {
            throw new TypeError(`${call.object.name}.${call.name} prepares no declared work`);
        }
        const value: unknown = await method.prepare(call);

        // keep the value as a settlement reads it back
        if (settlement === undefined) {
            return { call, value: declared.parse(value), settlement };
        }
        const recorded = schema.json().parse(value);
        await Settlement.record(this.database, settlement, call, recorded);

        return { call, value: declared.parse(recorded), settlement };
    }

    /** Mark the settling calls of a transaction committed. */
    async #commit(transaction: DatabaseConnection, prepared: readonly Prepared[]): Promise<void> {
        for (const work of prepared) {
            if (work?.settlement !== undefined) {
                await Settlement.commit(transaction, work.settlement, work.call, this.clock.now());
            }
        }
    }

    /** Settle each reserved settlement as its row records the call's outcome, skipping one claimed elsewhere. */
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
        caller: Caller,
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

    /** Execute, redact and audit one call. */
    async #perform(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        caller: Caller,
        options: CallOptions,
    ): Promise<unknown> {
        // build the call on the served type to run its handlers
        const { prepared, from, writerId, requestId } = options;
        const { call, method, scope, targetId } = await this.#call(
            transaction,
            authorization,
            this.served(object),
            name,
            input,
            caller,
            options,
        );
        span.current()?.setAttributes({
            "destack.scope": scope,
            ...(targetId === undefined ? {} : { "destack.object.id": targetId }),
        });

        // run invoked calls in the call's transaction and scope
        const called = call.with({
            ...preparedFields(prepared),
            ...(writerId === undefined ? {} : { writerId }),
            ...(requestId === undefined ? {} : { requestId }),
            run: this.#invoker(transaction, call, options),
        });

        // authorize the call unless prepared, and execute
        if (prepared === undefined) {
            await method.authorize?.(called);
        }
        const executed = await method.execute(called);

        // present the result, track changes and record the call
        const presented = await this.#present(transaction, authorization, object, call, executed);
        const id = targetId ?? Call.resultId(executed);
        if (method.mutates && id !== undefined) {
            await ObjectStore.#track(transaction, object, call.with({ id }), from);
        }
        this.#record(
            transaction,
            object,
            call,
            id,
            presented,
            caller,
            options.request,
            options.component,
        );

        return presented;
    }

    /** Run the calls a method makes in its transaction and scope, under its authorization for its own caller, recorded under its request. */
    #invoker(transaction: Transaction, parent: Call, options: CallOptions): Run {
        const { from, writerId, requestId, component } = options;
        const recorded: CallOptions = {
            ...(from === undefined ? {} : { from }),
            ...(writerId === undefined ? {} : { writerId }),
            ...(requestId === undefined ? {} : { requestId }),
            ...(component === undefined ? {} : { component }),
        };

        return (entry, caller, mutates) =>
            this.#runInside(transaction, entry, caller, mutates, parent, recorded);
    }

    /** Redact and encode what a call returns: its value, its page of objects, or its object with their texts. */
    async #present(
        transaction: Transaction,
        authorization: Authorization,
        object: ObjectType,
        call: Call,
        executed: unknown,
    ): Promise<unknown> {
        const { method, scope } = call;
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

            return {
                ...page,
                ...(included === undefined ? {} : { included: rest }),
                items: redacted.map((row) => ({
                    ...table[TABLE].encode(row),
                    ...(chunks === undefined
                        ? {}
                        : Chunk.text(object, Chunk.included(chunks[IDENTIFIER.parse(row["id"])]))),
                })),
            };
        }
        // answer one object with its texts
        else {
            const row = aligned(await authorization.redact(object, [Row.parse(executed)]), 0);
            const id = IDENTIFIER.parse(row["id"]);
            const texts =
                object.text.length === 0
                    ? undefined
                    : await Chunk.texts(transaction, object, scope, [id]);

            return { ...table[TABLE].encode(row), ...texts?.get(id) };
        }
    }

    /** Record a tracked object's change by a call, from the log position before its mutation. */
    static async #track(
        transaction: Transaction,
        object: ObjectType,
        call: Call,
        from: LogPosition | undefined,
    ): Promise<void> {
        // skip untracked objects
        const history = object.lifecycle.tracked;
        if (history === undefined) {
            return;
        }

        // record the row after the call against the target before it
        const after = await transaction.row(object.table, {
            id: present(call.id, "the tracked object"),
        });
        if (after !== null) {
            const before = present(from, "the log position before a tracked call");
            await TrackedHistory.record(history, call, call.target, after, before);
        }
    }

    /** Record a call: audit durable changes, and keep every call of a request for retries. */
    #record(
        transaction: Transaction,
        object: ObjectType,
        call: Call,
        id: string | undefined,
        presented: unknown,
        caller: Caller,
        request: CallRequest | undefined,
        component: string | undefined,
    ): void {
        // describe the call's target and outcome
        const { method, scope, name } = call;
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

        // audit a durable change, or keep a request's other call for retries, once the mutation commits under its transaction
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
                await recorder.keep(committing, object.audit(name), recorded, stamped);
            }
        });
    }
}

/** Throw a failed settlement. */
function rethrow(error: unknown): never {
    throw error;
}

/** Read where a resumed sync continues. */
function resumed(
    after: LogPosition | undefined,
    lastEventId: string | undefined,
): LogPosition | undefined {
    // parse the last event identifier
    if (lastEventId !== undefined) {
        const [epoch, sequence] = lastEventId.split("/");
        const position = LogPosition.safeParse({ epoch, sequence: Number(sequence) });
        if (!position.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: `no page carries event ${lastEventId}`,
            });
        }

        return position.data;
    }

    return after;
}

/** Give a prepared creation without an identifier a fresh one, as its prepared work refers to it. */
function identified(call: Expansion): Expansion {
    const method = call.object.method(call.name);
    const isUnnamed =
        method.kind === "create" && method.prepare !== undefined && call.input["id"] === undefined;

    return isUnnamed
        ? { ...call, input: { ...call.input, id: `${call.object.identity}-${v7()}` } }
        : call;
}

/** Project a read's result onto the audit details its method declares. */
function detailsOf(
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
function preparedFields(work: Prepared): Partial<Pick<Call, "prepared" | "idempotencyKey">> {
    if (work === undefined) {
        return {};
    }
    const key = work.call.idempotencyKey;

    return { prepared: work.value, ...(key === undefined ? {} : { idempotencyKey: key }) };
}

/** List the settlements of prepared work, in call order. */
function settlementsOf(prepared: readonly Prepared[]): (string | undefined)[] {
    return prepared.map((work) => work?.settlement);
}

/** Report whether a publisher also receives the changes sent to the rows it publishes. */
function isUplink(publisher: sync.Publisher): publisher is sync.Uplink {
    return "receive" in publisher;
}
