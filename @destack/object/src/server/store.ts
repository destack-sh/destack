import type { EventStore } from "@destack/event";
import {
    Authorizer,
    AccessContext,
    decisionTables,
    type TupleReader,
    Policy,
    type Access,
    type TableMapping,
    Caller,
    CONTAINED,
    GrantPermission,
    principal,
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
} from "@destack/db";
import { objectUnmask } from "../audit/unmask.ts";
import { Replay } from "../audit/replay.ts";
import {
    AuditRecorder,
    type AuditOrigin,
    type AuditCalled,
    type CallRequest,
} from "../audit/recorder.ts";
import { Journal } from "../audit/journal.ts";
import type { ExchangeAsk } from "@destack/runtime/authentication";
import type { CallKey } from "@destack/runtime/request";

import { aligned, Clock, type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import type { InstallationContext } from "@destack/runtime/workload";
import type { Watermark } from "@destack/runtime/bookmark";
import { CompactionController } from "@destack/runtime/database";
import { refuseInput, CallError } from "@destack/runtime/error";
import { RequestId, type RequestIdentity } from "@destack/runtime/request";
import { v7 } from "uuid";
import {
    implement,
    type Router,
    type ServerAccess,
    type RequestContext,
    type PackageImplementation,
    type ProcedureCall,
} from "@destack/runtime/server";
import type { PackageHandle, ProcedureRouter } from "@destack/runtime";
import * as sync from "@destack/sync";
import { Branch, type BranchCall, type BranchType } from "../branch/index.ts";
import {
    Call,
    CallCode,
    OutboxCall,
    type ObjectCall,
    type ResultsOf,
    type Run,
} from "../method/call.ts";
import { Method, type MethodKind } from "../method/method.ts";
import { callableProcedures } from "../method/procedure.ts";
import { InputSchema } from "../method/input.ts";
import {
    WriterId,
    replicaProcedures,
    type PushResult,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ACCESS_KINDS, ObjectType } from "../object/object.ts";
import { INTRINSIC } from "../object/intrinsic.ts";
import { ObjectAccess } from "../object/access.ts";
import {
    DeclarationIndex,
    ObjectDescription,
    type KnownDeclaration,
} from "../object/description.ts";
import { ObjectController } from "../object/controller.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { Authorization } from "./authorization.ts";
import { ChangeController } from "./change.ts";
import { Outbox } from "@destack/runtime/outbox";
import { ClaimController } from "../claim/controller.ts";
import { type Claimer, type ClaimHome, Reservation } from "../claim/reservation.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import { ExternalStorage } from "./external.ts";
import { ObjectSource } from "./source.ts";

import { Provisioned } from "../trait/provisioned.ts";
import { ProvisionedController, type ProvisionedOptions } from "./provisioned.ts";
import { ExpiringController } from "./expiring.ts";
import { RecoverableController } from "./recoverable.ts";
import { TrackedHistory } from "./tracked.ts";
import type { Controller } from "@destack/runtime/control";
import { Trigger, type ChangeTrigger, type RunClient } from "@destack/runtime/trigger";

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

/** How long a request waits for its caller's watermarks: 5 s, above sub-second copy lag. */
const BOOKMARK_TIMEOUT_MILLISECONDS = 5000;

/** A database's copy: the subscription following it, and the publisher streaming its pages. */
export interface SubscriberEntry {
    /** The subscription following the copy. */
    readonly subscription: sync.Subscription;
    /** The publisher streaming the copy's pages, one step closer to its rows' home. */
    readonly publisher: sync.Publisher;
}

/** The subscriptions a database follows for its copies, each from its own publisher. */
export interface Subscriber {
    /** The tables whose changes relist subscriptions, beside scopes and residents' homes. */
    readonly watches?: readonly Table[];
    /** The link to the copies' store, receiving this store's changes to them, if it sends any. */
    readonly uplink?: sync.Uplink;
    /** List the subscriptions of the copies the database keeps now, each with its publisher. */
    subscriptions(): Promise<readonly SubscriberEntry[]>;
}

/** Build the subscriber of a database's copies. */
export const Subscriber = {
    /** Keep some subscriptions' copies, each over one uplink receiving their changes. */
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

/** A call's prepared external work: the keyed call, parsed value and settlement, if any. */
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
    /** Each call's input as its record keeps it. */
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

/** How a store admits a caller beyond its authorization. */
export interface AdmitOptions extends Omit<AuthorizeOptions, "within"> {
    /** The object types the admitted calls run on, whose owned types need no visible scope. */
    readonly objects?: readonly ObjectType[];
}

/** How a mutation runs beyond its caller. */
export interface MutateOptions {
    /** The running mutation to join without external work, which its owner commits, if any. */
    readonly transaction?: Transaction;
    /** The client instance writing ephemeral objects. */
    readonly writerId?: string;
    /** The code making the calls for a principal's own code, such as a controller's name. */
    readonly component?: string;
    /** The scope the calls carrying none run in. */
    readonly scope?: string;
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
    readonly expansion?: readonly BranchCall[];
    /** The request the call belongs to. */
    readonly request?: CallRequest;
    /** The code making the call for a principal's own code, such as a controller's name. */
    readonly component?: string;
};

/** The options of an object store: its objects over one database, and adjacent stores. */
export interface ObjectStoreOptions {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** Further policies and table mappings to authorize. */
    readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The principal of the store's own work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The one partition the store serves, every partition of its database when absent. */
    readonly partition?: string;
    /** The journal's key for fingerprinting sensitive call inputs, set when types serve methods. */
    readonly callKey?: CallKey;
    /** The package recording the calls. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The claim type keeping the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimHome;
    /** The installations serving objects and their projections, by type reference key. */
    readonly installed?: ReadonlyMap<string, InstallationContext>;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** The event log with the served events types, over the store's database. */
    readonly events?: EventStore;
    /** The files with the served external objects, each keeping some of their types. */
    readonly external?: readonly ExternalStorage[];
    /** The subscriptions the database follows to keep its copies. */
    readonly subscriber?: Subscriber;
    /** The shapes served and followed beside the objects' and policies', such as package copies. */
    readonly shapes?: readonly sync.Shape[];
    /** The machine recording runs: the calls methods send and their change triggers' calls. */
    readonly runs?: RunClient;
    /** The served package's triggers, whose change triggers need a machine recording runs. */
    readonly triggers?: readonly Trigger[];
    /** Report failed settlements, thrown when absent. */
    readonly report?: (error: unknown) => void;
    /** Whether reads of the objects record access events, as the space's audit setting asks. */
    readonly isAccessAudited?: boolean;
    /** The clock calls run and controllers reconcile at. */
    readonly clock: Clock;
    /** The scope's branch types, among the objects, whose branches reads can see. */
    readonly branch?: BranchType;
    /** The providers reconciling the served kinds' resources, if the store reconciles any. */
    readonly provisioned?: ProvisionedOptions;
}

/** A store's routes: each served object's procedures by its key, and the replica procedures. */
export type ObjectRouter = Readonly<Record<string, Router<ProcedureRouter, RequestContext>>> & {
    readonly replica: Router<ReplicaProcedures, RequestContext>;
};

/** A store serving object types' methods over one database. */
export class ObjectStore {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The principal of the store's own work: controllers, settlements and in-process changes. */
    readonly principal: Subject;
    /** The one partition the store serves, every partition of its database when absent. */
    readonly #partition: string | undefined;
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
    /** The journal of executed mutations, kept only by a store serving methods. */
    readonly #journal: Journal | undefined;
    /** The package the store serves, as the audit records it. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The feed serving every sync. */
    readonly feed: sync.Feed;
    /** The claim type keeping the names of the served objects' unique indexes. */
    readonly claims: ClaimHome | undefined;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral: EphemeralStorage | undefined;
    /** The event log with the served events types. */
    readonly events: EventStore | undefined;
    /** The files with the served external objects. */
    readonly external: readonly ExternalStorage[];
    /** The subscriptions the database follows to keep its copies. */
    readonly subscriber: Subscriber | undefined;
    /** The durable types the store serves or decides, whose rows stand for their principals. */
    readonly decided: readonly ObjectType[];
    /** The shapes the store serves and follows beside its objects' and policies' shapes. */
    readonly shapes: readonly sync.Shape[];
    /** The installations serving objects and their projections, by type reference key. */
    readonly #installed: ReadonlyMap<string, InstallationContext>;
    /** The served or authorized durable types whose rows the database copies from home. */
    readonly copied: readonly ObjectType[];
    /** The machine recording runs: the calls methods send and their change triggers' calls. */
    readonly runs: RunClient | undefined;
    /** Stream served row copies to an in-process follower, running its changes as principal. */
    readonly uplink: sync.Uplink;
    /** The change triggers of the served objects, each change they admit recorded as a run. */
    readonly triggers: readonly ChangeTrigger[];
    /** The outbox of sent calls and copied row changes, absent if nothing receives them. */
    readonly #sends?: Outbox;
    /** Open an audit recorder for a scope and caller, with its calls' request and component. */
    readonly audit: (scope: string, caller: Caller, called?: AuditCalled) => AuditRecorder;
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
    /** The declarations installed object types resolve against: the types, policies and kinds. */
    readonly known: readonly KnownDeclaration[];
    /** Whether reads of the objects record access events. */
    readonly isAccessAudited: boolean;
    /** The copies the objects stream to clients and databases below. */
    readonly source: ObjectSource;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, TupleReader>();
    /** The scopes each caller lives inside, read once per caller and database. */
    readonly #inside = new WeakMap<Caller, Map<DatabaseConnection, Promise<readonly Subject[]>>>();
    /** The object types routed by name, without the chunk types serving them. */
    readonly routed: readonly ObjectType[];
    /** The wait for the copies gating readiness, shared by every implemented package. */
    #readiness: Promise<boolean> | undefined;

    /** Serve object types over one database. */
    constructor(options: ObjectStoreOptions) {
        // require each object type once apart from the shared procedures, and the stores its storage needs
        this.routed = options.objects;
        this.objects = ObjectType.served(this.routed);
        ObjectStore.#requireRoutes(options.objects);
        ObjectStore.#requireStores(this.objects, options);

        // keep the optional stores and machines
        this.claims = options.claims;
        this.subscriber = options.subscriber;
        this.shapes = options.shapes ?? [];
        this.#installed = options.installed ?? new Map();
        this.uplink = this.#uplink();
        this.runs = options.runs;
        this.triggers = ObjectStore.#changeTriggers(options);
        this.ephemeral = options.ephemeral;
        this.events = options.events;
        this.external = options.external ?? [];

        // build the authorizer over the objects and the scope types enclosing them
        this.database = options.database;
        this.principal = options.principal;
        this.#partition = options.partition;
        this.authorizer = ObjectStore.#authorizer(this.objects, options);
        TrackedHistory.require(this.objects, this.authorizer);

        // keep the journal, the audit and the request context
        this.#journal = ObjectStore.#journalOf(this.routed, options);
        this.origin = options.origin;
        this.audit = (scope, caller, called) =>
            AuditRecorder.scoped(this.journal, options.origin)(scope, caller, called);
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
        this.known = ObjectStore.known(options.objects, options.policies ?? []);
        if (followed.sends !== undefined) {
            this.#sends = followed.sends;
        }
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
        // index the known types by reference, beside the intrinsic policies every client knows
        const known = new Map(
            this.known
                .filter((declaration) => declaration instanceof ObjectType)
                .map((type) => [ObjectTypeReference.key(type.typeReference), type]),
        );
        const intrinsic = DeclarationIndex.of([]);

        // describe each asked type, then each type a description refers to beyond the intrinsic policies
        const served = this.routed.filter((type) => !(INTRINSIC in type.source));
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
                pending.push(...ObjectDescription.references(description, intrinsic));
            }
            reference = pending.pop();
        }

        return [...described.values()];
    }

    /** Follow durable tables and decision tables in one feed, with an outbox for leaving calls. */
    static #follow(
        objects: readonly ObjectType[],
        options: Pick<
            ObjectStoreOptions,
            "database" | "policies" | "runs" | "subscriber" | "clock" | "installed"
        >,
    ): {
        readonly decided: ObjectType[];
        readonly copied: ObjectType[];
        readonly feed: sync.Feed;
        readonly sends?: Outbox;
    } {
        // find the durable and copied object types, an installation's types owned where it serves them
        const decided = ObjectStore.#durable(objects, options.policies ?? []);
        const installed = options.installed ?? new Map();
        const copied = decided.filter(
            (object) =>
                options.database.copies(object.table) &&
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

        // keep an outbox for the machine recording runs and the copies' home
        const isUplinked = options.subscriber?.uplink !== undefined && copied.length > 0;
        const isSending = options.runs !== undefined || isUplinked;

        return isSending
            ? { decided, copied, feed, sends: new Outbox(options.database) }
            : { decided, copied, feed };
    }

    /** Stream served row copies to in-process followers, and run the changes they send. */
    #uplink(): sync.Uplink {
        return {
            stream: (subscription, signal) => this.source.publish(subscription, signal),
            receive: async (mutation, signer) => {
                // run the changes as the follower's principal
                if (signer === undefined) {
                    throw new TypeError("a change sent in process names the principal sending it");
                }
                await this.receive(mutation, signer);
            },
        };
    }

    /** Control each served kind's resources with its trait's loop, if it has providers. */
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
        // list each durable type once, though several extensions name it
        const durable = [...objects, ...policies.filter((other) => other instanceof ObjectType)];

        return durable.filter(
            (object, index) =>
                object.storage === "durable" &&
                durable.findIndex((each) => each.same(object)) === index,
        );
    }

    /** Keep the served package's change triggers, which need a machine recording runs. */
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

    /** Build the authorizer over the objects, their scope types and other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        options: Pick<ObjectStoreOptions, "database" | "policies" | "clock" | "partition">,
    ): Authorizer {
        return ObjectAccess.authorizer(
            objects,
            options.policies ?? [],
            (table) => options.database.copies(table),
            options.partition,
        );
    }

    /** Keep a method store's journal with its call key, refusing a call key without methods. */
    static #journalOf(
        routed: readonly ObjectType[],
        options: Pick<ObjectStoreOptions, "database" | "callKey" | "clock" | "events">,
    ): Journal | undefined {
        // require the call key and the event log keeping the calls of a store serving methods
        const isServing = routed.length > 0;
        const { callKey, events } = options;
        if (isServing && callKey === undefined) {
            throw new TypeError("a store serving methods needs the call key of their journal");
        } else if (isServing && (events === undefined || !events.keeps(Journal.kind))) {
            throw new TypeError("a store serving methods needs an event log keeping its calls");
        } else if (!isServing && callKey !== undefined) {
            throw new TypeError(
                "a store serving no methods keeps no journal and takes no call key",
            );
        }

        return callKey === undefined || events === undefined
            ? undefined
            : new Journal(options.database, callKey, { events, clock: options.clock });
    }

    /** Read the journal of executed mutations, failing for a store serving no methods. */
    get journal(): Journal {
        if (this.#journal === undefined) {
            throw new TypeError("a store serving no methods keeps no journal");
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
                    `object ${object.name} takes the name of the shared ${object.key} procedures`,
                );
            }
            routed.add(key);
        }
    }

    /** Refuse another principal's local types, and require claims, memory stores and files. */
    static #requireStores(
        objects: readonly ObjectType[],
        options: Pick<
            ObjectStoreOptions,
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
            throw new TypeError(`local object ${local.name} lives in its scope's own store alone`);
        }

        // require claims for indexed objects
        const indexed = objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.claims === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but no claim type keeps their names`,
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

        // require the event log over the store's database for events types
        const unlogged = objects.find(
            (object) =>
                object.storage === "events" &&
                (options.events?.keeps(object.eventKind()) !== true ||
                    options.events.database !== options.database),
        );
        if (unlogged !== undefined) {
            throw new TypeError(
                `events object ${unlogged.name} needs the store's event log keeping it`,
            );
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
                    ObjectStore.#requestCaller(call.context),
                    { requestId: call.context.requestId },
                ),
            ),
            controllers,
            ...(subscriber === undefined
                ? {}
                : { ready: (signal) => this.#awaitReady(subscriber, signal) }),
            router: this.router(routed),
        };
    }

    /** Wait for the copies gating readiness, checked once for all packages, or until stopped. */
    async #awaitReady(subscriber: Subscriber, signal: AbortSignal): Promise<void> {
        // share one check of the copies, checking again after the checking server stopped first
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

    /** Wait until every readiness copy is a complete snapshot, reporting sync before the signal. */
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

    /** Route the served objects' methods, or some of them, and the replica procedures. */
    router(routed: readonly ObjectType[] = this.routed): ObjectRouter {
        // route each object under its key, which its package's handle keeps unique
        const routers: Record<string, Router<ProcedureRouter, RequestContext>> = {};
        for (const object of routed) {
            routers[object.key] = this.#route(object);
        }

        return { replica: this.#replica(), ...routers };
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

    /** Route the replica procedures. */
    #replica(): Router<ReplicaProcedures, RequestContext> {
        const replica = implement(replicaProcedures).$context<RequestContext>();

        return replica.router({
            push: replica.push.handler(({ input, context }) =>
                this.#push(input.scope, input.mutations, context, input.writerId),
            ),
            watermark: replica.watermark.handler(async ({ input, context }) => {
                // read the scope's watermark once its copies have the homes' commits
                await this.enter(context, input.scope);
                await this.#reachHomes(context, input.scope, input.homes);

                return this.watermark(input.scope);
            }),
            call: replica.call.handler(async ({ input, context }) => {
                // read in the given scope
                const {
                    object,
                    name,
                    input: called,
                } = this.#resolve(input.call, false, context.caller(input.scope));
                if (object.scopeOf(name, called) !== input.scope) {
                    throw new CallError("BAD_REQUEST", {
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
            describe: replica.describe.handler(({ input }) => ({
                objects: this.describe(input.types),
            })),
        });
    }

    /** Refuse a method of a type in a scope that the caller's grant binds to other methods. */
    static #requireReached(caller: Caller, object: ObjectType, scope: string, name: string): void {
        const { grant } = caller;
        const type = { packageId: object.policy.definition.packageId, type: object.policy.name };
        if (grant !== undefined && !GrantPermission.reaches(type, scope, name, grant.permissions)) {
            throw new CallError("FORBIDDEN", {
                message: `grant ${grant.id} reaches no ${object.name}.${name} in ${scope}`,
            });
        }
    }

    /** Read a request's caller for its audit: anonymous when its credential failed. */
    static #requestCaller(context: RequestContext): Caller {
        return context.authenticationError === undefined ? context.caller() : { subjects: [] };
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
        const scope = object.scopeOf(name, rest);
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
            throw new CallError("INTERNAL_SERVER_ERROR", {
                message: `invalid output of ${object.name}.${name}`,
                data: { issues: validated.issues },
            });
        }

        return validated.value;
    }

    /** Read through a method as a caller in a read-only transaction, recording audited reads. */
    async query<Result>(caller: Caller, call: ObjectCall<Result>): Promise<Result>;
    /**
     * Read through a method as a caller.
     *
     * @construct the result is the call method's answer, which ObjectCall keeps from type.call.
     */
    async query(caller: Caller, call: ObjectCall): Promise<unknown> {
        // resolve the call, reading only a method the caller's grant reaches
        const { object, name, input } = this.#resolve(call, false, caller);
        const method = object.method(name);
        const scope = object.scopeOf(name, input);
        ObjectStore.#requireReached(caller, object, scope, name);
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
            const authorization = await this.admit(admitting, scope, caller, {
                objects: [object],
            });

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

    /** Run a mutation's calls as a caller in one transaction, once per caller and identifier. */
    async mutate<const Calls extends readonly ObjectCall[]>(
        caller: Caller,
        transaction: { readonly id: string; readonly calls: Calls },
        options?: MutateOptions,
    ): Promise<ResultsOf<Calls>>;
    /**
     * Run a mutation's calls as a caller in one transaction.
     *
     * @construct each result is its method's answer, which ObjectCall keeps from type.call.
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

        // resolve each call in one scope, of methods the caller's grant reaches
        const originals = mutation.calls.map((entry) => this.#resolve(entry, true, caller));
        const scope = this.#mutationScope(originals);
        for (const { object, name } of originals) {
            ObjectStore.#requireReached(caller, object, scope, name);
        }

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

    /** Place a call without a scope in one, by its served or copied type's route field. */
    #place(entry: ObjectCall, scope: string): ObjectCall {
        // find the route field of the call's method, which resolution refuses when unknown
        const type = Call.typeOf([...this.objects, ...this.copied], entry);
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
                await this.#runInside(
                    transaction,
                    entry,
                    caller,
                    true,
                    CallCode.store,
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
        entry: ObjectCall,
        caller: Caller,
        mutates: boolean,
        code: CallCode,
        parent: Call | undefined,
        options: CallOptions,
    ): Promise<unknown> {
        // place the call in the calling method's scope unless it carries another
        const placed = parent === undefined ? entry : this.#place(entry, parent.scope);

        // read whether the call's type is served here or only copied from its home
        const copied = Call.typeOf(this.copied, placed);
        const isServed = Call.typeOf(this.objects, placed) !== undefined;
        const served = parent?.location.kind === "server" ? parent.location : undefined;

        // refuse a change of a type only copied here, which its home changes
        if (mutates && !isServed && copied !== undefined) {
            throw new CallError("CONFLICT", {
                message: `${copied.name} rows are copies here, so send their changes to their home`,
            });
        }

        // resolve the call and refuse external work or another storage inside the transaction
        const { object, name, input } = Call.resolve(this.objects, placed, mutates, code);
        if (this.served(object).method(name).prepare !== undefined) {
            throw new TypeError(
                `${object.name}.${name} does external work, which a call inside a transaction cannot`,
            );
        } else if (object.storage !== (parent?.object.storage ?? "durable")) {
            throw new TypeError(`${object.name} is stored apart from the transaction`);
        }

        // run it under the calling method's authorization, else admit the caller in its scope
        const scope = object.scopeOf(name, input);
        const authorization =
            served !== undefined && parent?.caller === caller && parent.scope === scope
                ? served.authorization
                : await this.admit(
                      transaction,
                      scope,
                      caller,
                      parent === undefined
                          ? {
                                links: await Scope.chain(transaction.view(), scope),
                                objects: [object],
                            }
                          : { now: parent.now, objects: [object] },
                  );

        return this.#execute(transaction, authorization, object, name, input, caller, options);
    }

    /** Read the one scope a mutation's calls act in. */
    #mutationScope(originals: readonly ReturnType<typeof Call.resolve>[]): string {
        // require every call to act in the first call's scope
        const first = aligned(originals, 0);
        const scope = first.object.scopeOf(first.name, first.input);
        if (originals.some((call) => call.object.scopeOf(call.name, call.input) !== scope)) {
            throw new CallError("BAD_REQUEST", {
                message: "a mutation's calls act in one scope",
            });
        }

        return scope;
    }

    /** Identify a durable mutation by caller, scope and request id, with its redacted calls. */
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

    /** Execute a planned mutation once: return a recorded outcome, else run and commit it. */
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

            // roll back prepared work and release the reserved names
            await this.#settle(settlementsOf(planned.prepared));
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

    /** Admit a mutation's caller in its chain, refusing installation writes to capped spaces. */
    async #authorizeRun(transaction: Transaction, planned: MutationRun): Promise<Authorization> {
        // admit the caller in the scope chain the mutation reads
        const chain = await Scope.chain(transaction.view(), planned.scope);
        const authorization = await this.admit(transaction, planned.scope, planned.caller, {
            request: planned.id,
            links: chain,
            objects: planned.calls.map((call) => call.object),
        });

        // refuse an installation's writes to a space whose storage is capped, the submitted calls deciding
        if (principal.installation.is(this.principal)) {
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
                claimer: this.claimer(transaction),
            }));
        progress.reservation = reservation;

        // commit prepared work and claims with the mutation, releasing request-bound relationships
        transaction.record(async (committing) => {
            await this.#commit(committing, run.prepared);
            await this.authorizer.release(committing, run.id);
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
        const objects = calls.map((call) => call.object);
        const authorization = await this.admit(reading, scope, caller, { objects });
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

    /** Run a client's pushed mutations in order as its caller, returning the scope's watermark. */
    async #push(
        scope: string,
        mutations: readonly sync.Mutation[],
        context: RequestContext,
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

                // require the pushed scope to be the home running the mutation
                const first = this.#resolve(aligned(mutation.calls, 0), true, caller);
                if (first.object.homeOf(first.name, first.input) !== scope) {
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

    /** Find the served copy of an object type with the store's handlers. */
    served<Type extends ObjectType>(object: Type): Type;
    /**
     * Find the served copy of an object type.
     *
     * @construct it is the declared type with this store's handlers, matched by package and name.
     */
    served(object: ObjectType): ObjectType {
        const found = this.objects.find((served) => served.same(object));
        if (found === undefined) {
            throw new TypeError(`object store serves no ${object.name}`);
        }

        return found;
    }

    /** Decide whether a scope's rows of a type are copies here, which their home alone writes. */
    async isCopy(object: ObjectType, snapshot: Snapshot, scope: string): Promise<boolean> {
        // keep the rows of a type an installation serves here, though the database copies its table elsewhere
        const isInstalled = this.#installed.has(ObjectTypeReference.key(object.typeReference));

        return (
            object.storage === "durable" &&
            ((snapshot.database.copies(object.table) && !isInstalled) ||
                (await sync.Replica.isCopied(snapshot, scope, object.table)))
        );
    }

    /** Send a call to run at its rows' home after this database's transaction commits. */
    async send(
        call: sync.Call,
        transaction: Transaction,
        sending: { readonly at?: number; readonly exchange?: ExchangeAsk } = {},
    ): Promise<void> {
        // require an outbox delivering the store's sends
        if (this.#sends === undefined) {
            throw new TypeError(
                `the store sends ${sync.Call.label(call)}, but nothing receives its sends`,
            );
        }

        // record the send once the mutation commits
        const requestId = RequestId.create(this.clock.now());
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

    /** Run a mutation another in-process store sent once, as its principal. */
    async receive(mutation: sync.Mutation, as: Subject): Promise<void> {
        // pass changes of copied rows to their home, else run them here under the sent request, which replays a redelivery
        if (!(await this.#forward(mutation))) {
            await this.mutate(Caller.of(as, this.clock.now()), mutation);
        }
    }

    /** Run a follower's mutation of the served rows as the caller. */
    async receiveFrom(transaction: sync.Mutation, context: RequestContext): Promise<void> {
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
            transaction.calls.every((call) => this.#isUplinked(call))
        );
    }

    /** Report whether a call's type's rows are copied over the uplink rather than kept here. */
    #isUplinked(call: sync.Call): boolean {
        const object = Call.typeOf(this.copied, call);

        return object !== undefined && !this.objects.some((served) => served.same(object));
    }

    /** Find the one publisher receiving a copied rows mutation, refusing one spanning homes. */
    async #homeOfAll(transaction: sync.Mutation): Promise<sync.Uplink> {
        // require one home for every call, refusing calls nothing here receives
        const homes = await Promise.all(transaction.calls.map((call) => this.#homeOf(call)));
        const [home] = homes;
        if (homes.includes(undefined)) {
            throw new CallError("BAD_REQUEST", {
                message: "nothing here receives changes of the copied rows",
            });
        } else if (home === undefined || homes.some((each) => each !== home)) {
            throw new CallError("BAD_REQUEST", {
                message: "a mutation of copied rows changes rows of one home",
            });
        }

        return home;
    }

    /** Refuse a mutation changing rows this store only copies, which their home alone writes. */
    #requireServed(transaction: { readonly calls: readonly ObjectCall[] }): void {
        // find each call's type that no served object keeps
        const copies = transaction.calls
            .filter((call) => Call.typeOf(this.objects, call) === undefined)
            .map((call) => call.type.type);
        if (copies.length === 0) {
            return;
        }

        throw new CallError("BAD_REQUEST", {
            message: `the rows ${[...new Set(copies)].join(", ")} change are copies here, written only at their home`,
        });
    }

    /** Find the publisher receiving a change of copied rows. */
    async #homeOf(call: sync.Call): Promise<sync.Uplink | undefined> {
        // find the copy of the call's table in its scope whose publisher receives changes
        const object = Call.typeOf(this.copied, call);
        const scope = object?.scopeOf(call.method, call.input);
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

    /** Pass a mutation of copied rows to their one home once per request, returning success. */
    async #forward(mutation: sync.Mutation): Promise<boolean> {
        // leave a mutation of rows the database keeps itself, and refuse one mixing both
        const view = Snapshot.live(this.database);
        const copied = await Promise.all(
            mutation.calls.map(async (call) => {
                const object = Call.typeOf(this.copied, call);

                return (
                    object !== undefined &&
                    (await this.isCopy(object, view, object.scopeOf(call.method, call.input)))
                );
            }),
        );
        if (copied.every((isCopied) => !isCopied)) {
            return false;
        } else if (copied.some((isCopied) => !isCopied)) {
            throw new CallError("BAD_REQUEST", {
                message: "a mutation changes rows of one home, kept here or copied",
            });
        }

        // pass it to the publisher the copies follow, signed as this store's principal
        await (await this.#homeOfAll(mutation)).receive(mutation, this.principal);

        return true;
    }

    /** Deliver a sent call to the store keeping its rows, or to the machine recording runs. */
    async #deliver(sent: OutboxCall, signal: AbortSignal): Promise<void> {
        // deliver a change of copied rows toward their home
        const { call } = sent.request;
        if (await this.#forward({ id: sent.requestId, calls: [call] })) {
            return;
        }
        // record any other sent call's run at the machine
        else if (this.runs !== undefined) {
            await this.runs.send(sent.request, { requestId: sent.requestId, signal });
        }
        // refuse a call nothing receives
        else {
            throw new CallError("BAD_REQUEST", {
                message: `nothing receives the sent call ${sync.Call.label(call)}`,
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
            ...(this.#journal === undefined ? [] : [this.#journal.controller()]),
            ...(this.events?.controllers ?? []),
            ...(this.claims === undefined
                ? []
                : [
                      new ClaimController(this.claims, this.database, this.objects, {
                          clock: this.clock,
                          claimer: (object) => this.claimerOf(object),
                      }),
                  ]),
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

        return new Authorization(this.authorizer, transaction, bind, access);
    }

    /** Admit a caller in a scope, requiring an unmoved scope that a remote caller also sees. */
    async admit(
        transaction: Transaction,
        scope: string,
        caller: Caller,
        options: AdmitOptions = {},
    ): Promise<Authorization> {
        // resolve its access, then require an unmoved scope
        const { objects, ...authorizing } = options;
        const authorization = await this.authorize(transaction, scope, caller, authorizing);
        authorization.requireUnmoved();
        authorization.requirePartition(this.#partition);

        // require a remote caller to see the scope, except for calls on owned types, whose owners need no view of the home
        const isOwned =
            objects !== undefined &&
            objects.length > 0 &&
            objects.every((object) => object.owner !== undefined);
        if (Caller.isRemote(caller) && !isOwned) {
            await authorization.requireVisible(this.objects);
        }

        return authorization;
    }

    /** Enter a scope: require the pinned scope and wait for observed writes. */
    async enter(context: RequestContext, scope: string): Promise<void> {
        // require the pinned scope
        if (context.scope !== undefined && context.scope !== scope) {
            throw new CallError("NOT_FOUND", {
                message: `scope ${scope} is outside the pinned scope`,
            });
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

    /** Wait until a scope's chain copies have their homes' commits, refusing a home outside it. */
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
                message: `${outside.scope} is outside the chain of ${scope}, so no copy here follows it`,
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

    /** Wait until the scope's log or its copies reach each watermark, in a bounded wait each. */
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

    /** Read the principal claiming a type's names: the installation serving it, else the store. */
    claimerOf(object: ObjectType): Claimer {
        // claim as the store for a type no installation serves
        const serving = this.#installed.get(ObjectTypeReference.key(object.typeReference));
        if (serving === undefined) {
            return { kind: "store" };
        }

        // claim as the installation serving it
        const subject = principal.installation.reference(serving.scope, serving.id);

        return { kind: "installation", signer: { subject, database: this.database } };
    }

    /** Read the principal claiming the names a transaction writes, the store for none. */
    claimer(transaction: Transaction): Claimer {
        // find the claimers of the written indexed types
        const tables = new Set(transaction.changes.changes.map((change) => change.table));
        const written = this.objects.filter(
            (object) => Object.keys(object.indexes).length > 0 && tables.has(object.table),
        );
        const claimers = new Map(
            written.map((object) => {
                const claimer = this.claimerOf(object);
                const key =
                    claimer.kind === "store" ? "the store" : Subject.key(claimer.signer.subject);

                return [key, claimer];
            }),
        );

        // claim as the store for no written names, and refuse names claimed by several principals at once
        const [first, ...others] = claimers.values();
        if (first === undefined) {
            return { kind: "store" };
        } else if (others.length > 0) {
            throw new TypeError(
                `a mutation claims names as ${[...claimers.keys()].join(" and ")} at once`,
            );
        }

        return first;
    }

    /** Read the installation serving a type inside the store, absent for a type it serves. */
    installationOf(object: ObjectType): InstallationContext | undefined {
        return this.#installed.get(ObjectTypeReference.key(object.typeReference));
    }

    /** Read the event log with the events types, failing without one. */
    requireEvents(): EventStore {
        if (this.events === undefined) {
            throw new CallError("NOT_FOUND", { message: "no events types are served here" });
        }

        return this.events;
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
        caller: Caller,
        requestId: string,
        writerId: string | undefined,
    ): Promise<unknown[]> {
        // require the writer owning the rows
        if (writerId === undefined) {
            throw new CallError("BAD_REQUEST", {
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
        return Call.resolve(this.objects, entry, mutates, CallCode.of(caller));
    }

    /** Read a procedure call's scope: a method's own, else the decided target's or context's. */
    #procedureScope(
        call: ProcedureCall<RequestContext>,
        routed: readonly ObjectType[],
    ): string | undefined {
        // read the scope field of the object the package routes the call to
        const object = routed.find((each) => each.key === call.path[0]);
        const input = call.input === undefined ? undefined : PROCEDURE_INPUT.parse(call.input);
        const field = object?.route.field;
        const named = field === undefined ? undefined : input?.[field];

        return typeof named === "string"
            ? named
            : (call.context.target?.scope ?? call.context.scope);
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
        // refuse row changes to copies from their home, leaving access writes to the authorization
        const method = object.method(name);
        const scope = object.scopeOf(name, input);
        const isRowChange = method.mutates && !ACCESS_KINDS.has(method.kind);
        if (isRowChange && (await this.isCopy(object, transaction.view(), scope))) {
            throw new CallError("CONFLICT", {
                message: `${object.name} rows of ${scope} are copies here, so send their changes to their home`,
            });
        }

        // parse the input and build the call in its scope as the installation serving its type
        const { database } = transaction;
        const installation = this.installationOf(object);
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
                ...(installation === undefined ? {} : { installation }),
            },
            objects: this.objects,
        });

        // load the permitted target at the given revision
        const target = await ObjectStore.#target(call, targetId, snapshot, authorization);
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw new CallError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        // refuse callers' other methods on a trashed object
        const isTrashed =
            object.lifecycle.recoverable !== undefined &&
            target !== undefined &&
            target["deletionRequestedAt"] !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.isInternal !== true) {
            throw new CallError("CONFLICT", { message: `${object.name} is in the trash` });
        }

        return {
            call: target === undefined ? call : call.with({ target }),
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
            throw new CallError("BAD_REQUEST", { message: "this store serves no branches" });
        }
        const id = schema.string().parse(branch);
        const row = await Snapshot.live(database).row(types.object.table, { id });
        const permission = types.object.permission("read");
        const admitted =
            row === null
                ? undefined
                : await authorization.admitRows(types.object, permission, scope, [row]);
        if (admitted?.permitted.has(0) !== true) {
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
        const objects = calls.map((call) => call.object);
        const authorization = await this.admit(
            reading,
            scope,
            caller,
            request === undefined ? { objects } : { request: request.requestId, objects },
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

    /** Prepare a request's calls in order, reserving settlements first, rolling back on failure. */
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

    /** Key a call's external work, stable across retries, and reserve a settlement if needed. */
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

    /** Run a reserved call's prepare phase, parsing its value and recording it for settlement. */
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

    /** Run a method's calls in its transaction and scope, authorized for its caller and request. */
    #invoker(transaction: Transaction, parent: Call, options: CallOptions): Run {
        const { from, writerId, requestId, component } = options;
        const recorded: CallOptions = {
            ...(from === undefined ? {} : { from }),
            ...(writerId === undefined ? {} : { writerId }),
            ...(requestId === undefined ? {} : { requestId }),
            ...(component === undefined ? {} : { component }),
        };

        return (entry, caller, mutates, code) =>
            this.#runInside(transaction, entry, caller, mutates, code, parent, recorded);
    }

    /** Record a read that showed guarded personal values unmasked, of one object or a scope's. */
    async recordUnmask(read: {
        readonly scope: string;
        readonly caller: Caller;
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
            if (redacted.isRevealing) {
                await this.recordUnmask({
                    scope,
                    caller: call.caller,
                    object,
                    operation: call.name,
                    ...(call.requestId === undefined ? {} : { requestId: call.requestId }),
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
            const row = aligned(redacted.rows, 0);
            const id = IDENTIFIER.parse(row["id"]);
            if (redacted.isRevealing) {
                await this.recordUnmask({
                    scope,
                    caller: call.caller,
                    object,
                    operation: call.name,
                    ...(call.requestId === undefined ? {} : { requestId: call.requestId }),
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
            throw new CallError("BAD_REQUEST", {
                message: `no page carries event ${lastEventId}`,
            });
        }

        return position.data;
    }

    return after;
}

/** Give a prepared creation without an identifier a fresh one, for its prepared work. */
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
