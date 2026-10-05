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
} from "@destack/access";
import { type ObjectReference, Scope, type ScopeLink, type Subject } from "@destack/sync";
import {
    LogPosition,
    Snapshot,
    eq,
    TABLE,
    type DatabaseConnection,
    Row,
    type Select,
    type Table,
    DatabaseError,
} from "@destack/db";
import {
    AuditRecorder,
    Journal,
    type AuditDestination,
    type AuditOrigin,
    type CallRequest,
} from "@destack/audit";
import type { CallKey } from "@destack/service/request";

import { aligned, present, schema, type JsonObject, type JsonValue } from "@destack/schema";
import type { InstallationContext } from "@destack/service/workload";
import type { Watermark } from "@destack/service/bookmark";
import { CompactionController } from "@destack/service/database";
import { refuseInput, ServiceError } from "@destack/service/error";
import { type RequestIdentity } from "@destack/service/request";
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
import {
    Call,
    invoking,
    RUNS,
    type Invoke,
    type Invoker,
    type ResultOf,
    type Run,
} from "../method/call.ts";
import { Method, type MethodKind } from "../method/method.ts";
import {
    callableProcedures,
    type CallableName,
    type CallInput,
    type CallOutput,
    type MethodName,
    type ObjectProcedures,
    type ObjectSchema,
} from "../method/procedure.ts";
import {
    ClientId,
    ProjectionParameters,
    replicaProcedures,
    type PushResult,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ObjectType } from "../object/object.ts";
import { ObjectController } from "../object/controller.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { Authorization, SystemAuthorization } from "./authorization.ts";
import { ChangeController } from "./change.ts";
import { Outbox } from "@destack/service/outbox";
import type { Directory } from "@destack/directory";
import { DirectoryStore } from "@destack/directory";
import { ClaimController, Reservation } from "../claim/index.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import type { ExternalStorage } from "./external.ts";
import { ObjectSource } from "./source.ts";

import { tracked } from "../trait/tracked.ts";
import { recoverable } from "../trait/recoverable.ts";
import type { Residence } from "../trait/projected.ts";
import { expiring } from "../trait/expiring.ts";
import type { Controller } from "@destack/service/control";
import { Trigger, type ChangeTrigger, type RunClient } from "@destack/service/trigger";

import { Settlement } from "./settlement.ts";
import { telemetry } from "@destack/telemetry";

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
    /** List the subscriptions of the copies the database keeps now, each with its publisher. */
    subscriptions(): Promise<readonly SubscriberEntry[]>;
}

/** Build the subscriber of a database's copies. */
export const Subscriber = {
    /** Keep the copies some subscriptions list, each streamed from one publisher. */
    of(
        publisher: sync.Publisher,
        subscriptions: () => Promise<readonly sync.Subscription[]>,
    ): Subscriber {
        return {
            subscriptions: async () =>
                (await subscriptions()).map((subscription) => ({ subscription, publisher })),
        };
    },
};

/** A call's prepared external work: the keyed call, its parsed value and its settlement, absent without any. */
type Prepared =
    | { readonly call: Call; readonly value: unknown; readonly settlement: string | undefined }
    | undefined;

/** A resolved call with the author it runs as and the calls its expansion ran before it. */
type Expansion = ReturnType<typeof Call.resolve> & {
    /** The principal the call runs as, the caller's when absent. */
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
    /** The request the mutation runs for. */
    readonly context: ServiceContext;
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
    /** The client writing ephemeral objects. */
    readonly client?: string;
    /** The request a durable mutation replays under. */
    readonly requestId?: string;
    /** The principal the call records as its caller, the authorization's when absent. */
    readonly as?: Subject;
    /** The calls the method's expansion ran before it. */
    readonly expansion?: readonly BranchCall[];
    /** The request the call belongs to. */
    readonly request?: CallRequest;
};

/** Where system calls run: an open transaction, a scope and the time. */
export interface SystemContext {
    /** The open transaction. */
    readonly database: DatabaseConnection;
    /** The scope the calls run in. */
    readonly scope: string;
    /** The time the calls run at. */
    readonly now: number;
}

/** The routers of served object types by name, and the replica procedures. */
export type ObjectRouter<Objects extends Readonly<Record<string, ObjectType>>> = {
    readonly [Name in keyof Objects]: Router<ObjectProcedures<Objects[Name]>, ServiceContext>;
} & { readonly replica: Router<ReplicaProcedures, ServiceContext> };

/** A server of object types' methods over one database. */
export class ObjectServer<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
    /** The journal of executed mutations, kept only by a server serving methods. */
    readonly #journal: Journal | undefined;
    /** The package and service the server serves, as the audit names them. */
    readonly origin: Omit<AuditOrigin, "scope">;
    /** The feed serving every sync. */
    readonly feed: sync.Feed;
    /** The directory with the claims of the served objects' unique indexes. */
    readonly directory?: Directory;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** The files with the served external objects. */
    readonly external?: ExternalStorage;
    /** The subscriptions the database follows to keep its copies. */
    readonly subscriber?: Subscriber;
    /** The copied object types whose rows stand for their principals here, as at their home. */
    readonly standing: readonly ObjectType[];
    /** The shapes the server serves and follows beside its objects' and its policies' own. */
    readonly shapes: readonly sync.Shape[];
    /** The installation serving the objects, which sends and receives projected rows, absent for a cell. */
    readonly installation?: InstallationContext;
    /** The copied types naming the residents whose rows the served types project, absent where nothing projects. */
    readonly residence: Residence | undefined;
    /** The durable object types, served or authorized, whose rows the database keeps as copies from their home. */
    readonly copied: readonly ObjectType[];
    /** The cell recording the objects' runs: the calls methods send, and the calls of their change triggers. */
    readonly runs?: RunClient;
    /** The change triggers of the served objects, each change they admit recorded as a run. */
    readonly triggers: readonly ChangeTrigger[];
    /** The audit history the journal delivers calls to, absent where another server delivers them. */
    readonly #history: AuditDestination | undefined;
    /** The outbox delivering the calls methods send, absent where no cell records runs. */
    readonly #sends?: Outbox;
    /** Derive a request's verified authorization inputs within a scope. */
    readonly accessContext: (context: ServiceContext, scope: string) => AccessContext;
    /** Open an audit recorder for a scope and request. */
    readonly audit: (scope: string, context?: ServiceContext) => AuditRecorder<DatabaseConnection>;
    /** Report failed settlements. */
    readonly #report: (error: unknown) => void;
    /** Read the current time calls run and controllers reconcile at, in UTC epoch milliseconds. */
    readonly clock: () => number;
    /** The scope's branch types, absent for a server without branches. */
    readonly branch: BranchType | undefined;
    /** Whether reads of the objects record access events. */
    readonly isAccessAudited: boolean;
    /** The copies the objects stream to clients and databases below. */
    readonly source: ObjectSource;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, GrantReader>();
    /** The output validators of the methods called in process, by object type and method. */
    readonly #outputs = new WeakMap<ObjectType, Map<string, schema.StandardSchema>>();
    /** The object types routed by name, without the chunk types serving them. */
    readonly #routed: readonly ObjectType[];
    /** The object schemas, by object name. */
    readonly #schemas: ReadonlyMap<string, { object: ObjectType; schema: ObjectSchema }>;

    /** Serve object types over one database. */
    constructor(options: {
        /** The object types served, by name. */
        readonly objects: Objects;
        /** Further policies and table mappings to authorize. */
        readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
        /** The database with the objects. */
        readonly database: DatabaseConnection;
        /** Derive a request's verified authorization inputs within a scope, the request's own access by default. */
        readonly context?: (context: ServiceContext, scope: string) => AccessContext;
        /** The key sensitive call inputs are fingerprinted under in the journal, given exactly when object types serve methods. */
        readonly callKey?: CallKey;
        /** The package and service recording the calls. */
        readonly origin: Omit<AuditOrigin, "scope">;
        /** The audit history the journal delivers calls to, absent where another server delivers them. */
        readonly history?: AuditDestination;
        /** The directory with the claims of unique indexes, kept by the account service. */
        readonly directory?: Directory;
        /** The installation serving the objects, which sends and receives projected rows, absent for a cell. */
        readonly installation?: InstallationContext;
        /** The memory store with the served ephemeral objects. */
        readonly ephemeral?: EphemeralStorage;
        /** The files with the served external objects. */
        readonly external?: ExternalStorage;
        /** The subscriptions the database follows to keep its copies. */
        readonly subscriber?: Subscriber;
        /** The copied object types whose rows stand for their principals here, as at their home, such as a space's copied zone. */
        readonly standing?: readonly ObjectType[];
        /** The shapes the server serves and follows beside its objects' and its policies' own, such as a package's own copies. */
        readonly shapes?: readonly sync.Shape[];
        /** The cell recording the objects' runs: the calls methods send, and the calls of their change triggers. */
        readonly runs?: RunClient;
        /** The triggers of the served objects' package, whose change triggers need a cell recording their runs. */
        readonly triggers?: readonly Trigger[];
        /** Report failed settlements, thrown when absent. */
        readonly report?: (error: unknown) => void;
        /** Whether reads of the objects record access events, as the space's audit setting asks. */
        readonly isAccessAudited?: boolean;
        /** Read the current time calls run and controllers reconcile at, the system clock by default. */
        readonly clock?: () => number;
        /** The scope's branch types, among the objects, whose branches reads can see. */
        readonly branch?: BranchType;
    }) {
        // require each object type under its own name, and the stores its storage needs
        this.#routed = Object.values(options.objects);
        this.objects = ObjectType.served(this.#routed);
        ObjectServer.#requireNames(options.objects);
        ObjectServer.#requireStores(this.objects, options);

        // keep the optional stores and cells
        if (options.directory !== undefined) {
            this.directory = options.directory;
        }
        if (options.subscriber !== undefined) {
            this.subscriber = options.subscriber;
        }
        this.standing = options.standing ?? [];
        this.shapes = options.shapes ?? [];
        if (options.installation !== undefined) {
            this.installation = options.installation;
        }
        if (options.runs !== undefined) {
            this.runs = options.runs;
            this.#sends = new Outbox(options.database);
        }
        this.triggers = ObjectServer.#changeTriggers(options);
        if (options.ephemeral !== undefined) {
            this.ephemeral = options.ephemeral;
        }
        if (options.external !== undefined) {
            this.external = options.external;
        }

        // build the authorizer over the objects, the scope types enclosing them and the residence
        this.database = options.database;
        this.residence = ObjectServer.#residence(this.objects);
        this.authorizer = ObjectServer.#authorizer(this.objects, this.residence, options);
        tracked.require(this.objects, this.authorizer);

        // keep the journal, the audit and the request context
        this.#journal = ObjectServer.#journalOf(this.#routed, options);
        this.accessContext = options.context ?? ((context, scope) => context.access(scope));
        this.origin = options.origin;
        this.audit = (scope, context) =>
            AuditRecorder.service(this.journal, options.origin)(scope, context);
        this.#history = options.history;
        this.isAccessAudited = options.isAccessAudited ?? false;
        this.source = new ObjectSource(this);
        this.clock = options.clock ?? Date.now;
        this.branch = options.branch;
        this.#report = options.report ?? rethrow;

        // follow the durable objects' tables and the decision tables in one feed
        const durable = ObjectServer.#durable(this.objects, options.policies ?? []);
        this.copied = durable.filter((object) => options.database.copies(object.table));
        const followed = durable.flatMap((object) => object.tables);
        this.feed = new sync.Feed(options.database, [...new Set([...followed, ...decisionTables])]);
        this.#schemas = new Map(
            this.objects.map((object) => [object.name, { object, schema: object.schema }]),
        );
    }

    /** List the durable object types a server serves or decides. */
    static #durable(
        objects: readonly ObjectType[],
        policies: readonly (Policy | ObjectType | TableMapping)[],
    ): ObjectType[] {
        return [...objects, ...policies.filter((other) => other instanceof ObjectType)].filter(
            (object) => object.storage === "durable",
        );
    }

    /** Keep the change triggers of the served package, which need a cell recording their runs. */
    static #changeTriggers(options: {
        readonly runs?: RunClient;
        readonly triggers?: readonly Trigger[];
    }): ChangeTrigger[] {
        const triggers = (options.triggers ?? []).flatMap(
            (trigger) => Trigger.of(trigger, "change") ?? [],
        );
        if (triggers.length > 0 && options.runs === undefined) {
            throw new TypeError("change triggers need a cell recording their runs");
        }

        return triggers;
    }

    /** Build the authorizer over the objects, every scope type enclosing them, the residence and the other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        residence: Residence | undefined,
        options: {
            readonly database: DatabaseConnection;
            readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
        },
    ): Authorizer {
        // add the residence's types the server does not serve
        const residents = (
            residence === undefined ? [] : [residence.user, residence.address]
        ).filter((type) => !objects.some((served) => served.same(type)));

        return ObjectType.authorizer(
            objects,
            [...(options.policies ?? []), ...residents],
            (table) => options.database.copies(table),
        );
    }

    /** Keep the journal of a server serving methods, which needs the call key, and refuse a call key without methods. */
    static #journalOf(
        routed: readonly ObjectType[],
        options: { readonly database: DatabaseConnection; readonly callKey?: CallKey },
    ): Journal | undefined {
        const isServing = routed.length > 0;
        if (isServing && options.callKey === undefined) {
            throw new TypeError("a server serving methods needs the call key of their journal");
        } else if (!isServing && options.callKey !== undefined) {
            throw new TypeError(
                "a server serving no methods keeps no journal and takes no call key",
            );
        }

        return options.callKey === undefined
            ? undefined
            : new Journal(options.database, options.callKey);
    }

    /** Read the journal of executed mutations, failing for a server serving no methods. */
    get journal(): Journal {
        if (this.#journal === undefined) {
            throw new TypeError("a server serving no methods keeps no journal");
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

    /** Require the directory for indexed types, a memory store for ephemeral types and files for external types. */
    static #requireStores(
        objects: readonly ObjectType[],
        options: {
            readonly database: DatabaseConnection;
            readonly directory?: Directory;
            readonly ephemeral?: EphemeralStorage;
            readonly external?: ExternalStorage;
        },
    ): void {
        // require a directory in another database for indexed objects
        const indexed = objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.directory === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but no directory keeps their claims`,
            );
        } else if (
            options.directory instanceof DirectoryStore &&
            options.directory.database === options.database
        ) {
            throw new TypeError(
                "the directory keeps claims across databases, apart from the objects'",
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
                options.external?.objects.some((stored) => stored.same(object)) !== true,
        );
        if (unfiled !== undefined) {
            throw new TypeError(`external object ${unfiled.name} needs files keeping it`);
        }
    }

    /** Read the residence the projecting types share, refusing types naming different ones. */
    static #residence(objects: readonly ObjectType[]): Residence | undefined {
        // read each projecting type's residence, refusing a second one
        const [first, ...others] = objects.flatMap((object) =>
            object.projected === undefined ? [] : [object.projected.residence],
        );
        const differing = others.find(
            (other) => !other.user.same(first?.user) || !other.address.same(first?.address),
        );
        if (differing !== undefined) {
            throw new TypeError("the projecting types name different residences");
        }

        return first;
    }

    /** Serve a service's object types over one database. */
    static serve(
        service: Service<ServiceRouter, Readonly<Record<string, ObjectType>>>,
        options: {
            /** The database with the objects. */
            readonly database: DatabaseConnection;
            /** The key sensitive call inputs are fingerprinted under in the journal. */
            readonly callKey: CallKey;
            /** The audit history the journal delivers calls to. */
            readonly history?: AuditDestination;
            /** The directory with the claims of the objects' unique indexes. */
            readonly directory?: Directory;
            /** The memory store with the ephemeral objects. */
            readonly ephemeral?: EphemeralStorage;
            /** The files with the external objects. */
            readonly external?: ExternalStorage;
            /** Further controllers running alongside the objects'. */
            readonly controllers?: readonly Controller[];
            /** The installation serving the objects, keeping the copies of its space. */
            readonly installation?: InstallationContext;
            /** The cell recording the objects' runs: the calls methods send, and the calls of their change triggers. */
            readonly runs?: RunClient;
            /** The triggers of the served objects' package. */
            readonly triggers?: readonly Trigger[];
            /** The scope's branch types, among the served objects. */
            readonly branch?: BranchType;
            /** The handled types served in place of the service's own of the same tables. */
            readonly handled?: readonly ObjectType[];
            /** The types of other packages the database copies, such as the settings in its residents' scopes. */
            readonly policies?: readonly ObjectType[];
        },
    ): ServiceImplementation {
        // serve the service's objects with their handlers, keeping the copies of their space
        const { installation } = options;
        const objects: ObjectServer = new ObjectServer({
            objects: ObjectServer.#handle(service, options.handled ?? []),
            ...(options.policies === undefined ? {} : { policies: options.policies }),
            database: options.database,
            callKey: options.callKey,
            origin: { package: service.package, service: service.name },
            ...(options.history === undefined ? {} : { history: options.history }),
            ...(options.branch === undefined ? {} : { branch: options.branch }),
            ...(options.directory === undefined ? {} : { directory: options.directory }),
            ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
            ...(options.external === undefined ? {} : { external: options.external }),
            ...(installation === undefined
                ? {}
                : {
                      subscriber: ObjectServer.#subscriber(installation, () => objects.source),
                      installation,
                  }),
            ...(options.runs === undefined ? {} : { runs: options.runs }),
            ...(options.triggers === undefined ? {} : { triggers: options.triggers }),
        });

        return objects.implement(service, options.controllers);
    }

    /** Keep an installation's copies: its space's chain and universe rows from its cell, and the projections its residents receive from the installations keeping them. */
    static #subscriber(installation: InstallationContext, source: () => ObjectSource): Subscriber {
        return {
            subscriptions: async () => [
                ...(await source().subscriptions(installation.scope, { isHome: false })).map(
                    (subscription) => ({ subscription, publisher: installation.publisher }),
                ),
                ...(await source().projectionSubscriptions(installation.scope)).map(
                    (subscription) => {
                        const { installation: keeping } = ProjectionParameters.parse(
                            subscription.parameters,
                        );

                        return {
                            subscription,
                            publisher: installation.publisherAt(`${keeping}.${subscription.scope}`),
                        };
                    },
                ),
            ],
        };
    }

    /** Replace a service's object types with the handled ones of the same tables, refusing handled types it lacks. */
    static #handle(
        service: Service<ServiceRouter, Readonly<Record<string, ObjectType>>>,
        handled: readonly ObjectType[],
    ): Readonly<Record<string, ObjectType>> {
        // refuse a handled type the service does not serve
        const own = service.objects;
        const stray = handled.find((type) => !Object.values(own).some((each) => each.same(type)));
        if (stray !== undefined) {
            throw new TypeError(`service ${service.name} serves no object ${stray.name}`);
        }

        return Object.fromEntries(
            Object.entries(own).map(([key, type]) => [
                key,
                handled.find((each) => each.same(type)) ?? type,
            ]),
        );
    }

    /** Implement a service with the served objects' access, audit, controllers and router, running further controllers alongside. */
    implement(service: Service, controllers: readonly Controller[] = []): ServiceImplementation {
        return {
            service,
            access: this.access,
            audit: AuditRecorder.procedure((call) =>
                this.audit(this.#procedureScope(call) ?? Scope.universe.id, call.context),
            ),
            controllers: [...this.controllers(), ...controllers],
            router: this.router(),
            clock: this.clock,
        };
    }

    /** The service access deciding the served objects. */
    get access(): ServiceAccess {
        return {
            authorizer: this.authorizer,
            database: this.database,
            standing: (subject) => this.source.standing(subject),
        };
    }

    /** Route every served object's methods and the replica procedures. */
    router(): ObjectRouter<Objects>;
    /**
     * Route every served object's methods and the replica procedures.
     *
     * @construct each served object routes its own procedures under its key, which is how ObjectRouter maps the objects.
     */
    router(): Readonly<Record<string, unknown>> {
        const routers: Record<string, unknown> = { replica: this.#replica() };
        for (const object of this.#routed) {
            routers[object.key] = this.#route(object);
        }

        return routers;
    }

    /** Route an object's methods to the call pipeline. */
    #route(object: ObjectType): Readonly<Record<string, unknown>> {
        // bind each method's procedure to the call pipeline
        const routes: Record<string, unknown> = {};
        for (const [name, procedure] of Object.entries(callableProcedures(object))) {
            routes[name] = implement(procedure)
                .$context<ServiceContext>()
                .handler(({ input, context }) => this.call(object, name, input, context));
        }

        return routes;
    }

    /** Route the replica procedures. */
    #replica(): Router<ReplicaProcedures, ServiceContext> {
        const replica = implement(replicaProcedures).$context<ServiceContext>();

        return replica.router({
            push: replica.push.handler(({ input, context }) =>
                this.push(input.scope, input.mutations, context, input.client),
            ),
            call: replica.call.handler(async ({ input, context }) => {
                // read in the named scope
                const { object, name, input: called } = this.#resolve(input.call, false);
                if (this.#scope(object, called) !== input.scope) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `${object.name}.${name} reads another scope than ${input.scope}`,
                    });
                }

                return await this.call(object, name, called, context);
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

    /** Execute one method as a single-call mutation or a query, answering its procedure's output. */
    async call<Type extends ObjectType, Name extends CallableName<Type>>(
        object: Type,
        name: Name,
        input: JsonObject,
        context: ServiceContext,
    ): Promise<CallOutput<Type, Name>>;
    /**
     * Execute one method as a mutation or a query.
     *
     * @construct the result is validated by the method's procedure output, which CallOutput reads from the same procedure.
     */
    async call(
        object: ObjectType,
        name: string,
        input: JsonObject,
        context: ServiceContext,
    ): Promise<unknown> {
        // query a reading method, or mutate through a single-call mutation
        const method = object.method(name);
        const isEphemeral = object.storage === "ephemeral";
        const { [isEphemeral ? "client" : "requestId"]: named, ...rest } = input;
        const result = method.mutates
            ? await this.#mutateOne(object, name, rest, named, context)
            : await this.query(object, name, rest, context);

        // answer what the procedure answers remote callers
        const validated = await this.#output(object, name)["~standard"].validate(result);
        if (validated.issues !== undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `invalid output of ${object.name}.${name}`,
                data: { issues: validated.issues },
            });
        }

        return validated.value;
    }

    /** Read a method's procedure output validator, derived once per object type. */
    #output(object: ObjectType, name: string): schema.StandardSchema {
        // read the validator derived earlier
        const outputs = this.#outputs.get(object) ?? new Map<string, schema.StandardSchema>();
        this.#outputs.set(object, outputs);
        const known = outputs.get(name);
        if (known !== undefined) {
            return known;
        }

        // derive it from the method's procedure
        const output = schema
            .standard()
            .parse(object.method(name).procedure(name, object.schema).output);
        outputs.set(name, output);

        return output;
    }

    /** Execute one mutating call under the request or client a caller names. */
    async #mutateOne(
        object: ObjectType,
        name: string,
        input: JsonObject,
        named: unknown,
        context: ServiceContext,
    ): Promise<unknown> {
        const calls = [Call.record(object, name, input)];

        // write an ephemeral object as the named client
        if (object.storage === "ephemeral") {
            const mutation = { id: crypto.randomUUID(), calls };
            const [result] = await this.mutate(mutation, context, ClientId.parse(named));

            return result;
        }
        // write a durable object under the named request
        else {
            const mutation = { id: schema.string().parse(named), calls };
            const [result] = await this.mutate(mutation, context);

            return result;
        }
    }

    /** Read through a method in a read-only transaction. */
    async query(
        object: ObjectType,
        name: string,
        input: JsonObject,
        context: ServiceContext,
    ): Promise<unknown> {
        // wait for the caller's watermarks
        const method = object.method(name);
        const scope = this.#scope(object, input);
        await this.enter(context, scope);
        const database = await this.#databaseOf(object, scope);
        const read = async () => {
            // prepare, read, commit and settle
            const prepared = await this.#prepare([{ object, name, input }], scope, context);
            try {
                const value = await readIn(prepared[0]);
                await this.#commit(this.database, prepared);

                return value;
            } finally {
                await this.#settle(settlementsOf(prepared));
            }
        };
        const readIn = (prepared: Prepared) =>
            database.transaction(
                async (transaction) => {
                    // admit the caller against the durable database
                    const authorization = await this.admit(
                        object.storage === "durable" ? transaction : this.database,
                        scope,
                        context,
                    );

                    return this.#execute(transaction, authorization, object, name, input, context, {
                        prepared,
                    });
                },
                { isReadOnly: true },
            );

        // read unaudited objects directly
        if (!object.isReadAudited && method.audited !== true && !this.isAccessAudited) {
            return read();
        }

        // audit one object, or the scope's collection
        const recorder = this.audit(scope, context);
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
            return this.store().database;
        }
        // read external objects from the scope's files
        else if (object.storage === "external") {
            return present(this.external, "the external files").open(scope);
        }
        // read durable objects from the database
        else {
            return this.database;
        }
    }

    /** Execute a mutation's calls in one transaction, returning each call's result. */
    async mutate(
        mutation: sync.Mutation,
        context: ServiceContext,
        client?: string,
    ): Promise<unknown[]> {
        // resolve each call in one scope
        const originals = mutation.calls.map((entry) => this.#resolve(entry, true));
        const scope = this.#mutationScope(originals);
        await this.enter(context, scope);

        // write ephemeral objects apart
        const ephemeral = originals.filter((call) => call.object.storage === "ephemeral").length;
        if (ephemeral === originals.length) {
            return this.#mutateEphemeral(originals, scope, context, client);
        } else if (ephemeral > 0) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation writes durable or ephemeral objects, not both",
            });
        }

        // expand calls into the calls they run after, and identify prepared creations
        const calls = (await this.#expand(originals, scope, context)).map(identified);

        // identify the mutation and prepare external work
        const run = await this.#identify(mutation, originals, calls, scope, context);
        const prepared = await this.#prepare(calls, scope, context, run.request);
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

        // report the watermark and answer the submitted calls without their expansions
        context.observed.observe(await this.watermark(scope));

        return results.filter((_result, index) => aligned(calls, index).as === undefined);
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
        mutation: sync.Mutation,
        originals: readonly ReturnType<typeof Call.resolve>[],
        calls: readonly Expansion[],
        scope: string,
        context: ServiceContext,
    ): Promise<MutationRun> {
        return {
            id: mutation.id,
            scope,
            context,
            calls,
            prepared: [],
            request: { caller: context.requireAuthentication().id, scope, requestId: mutation.id },
            fingerprint: await this.#fingerprint(mutation, originals),
            redacted: calls.map((call) =>
                schema.redactFields(Call.input(call.object, call.name, true), call.input),
            ),
        };
    }

    /** Execute a planned mutation through the journal, undoing its prepared work and keys once it fails. */
    async #executeDurable(planned: MutationRun, progress: MutationProgress): Promise<unknown[]> {
        // run the calls in one journal transaction
        try {
            return await this.journal.execute(planned.request, planned.fingerprint, {
                authorize: async (transaction) => {
                    // guard the scope chain and admit the caller
                    const chain = await Scope.guard(transaction, planned.scope);

                    return this.admit(
                        transaction,
                        planned.scope,
                        planned.context,
                        planned.id,
                        chain,
                    );
                },
                run: (transaction, authorization) =>
                    this.#run(transaction, authorization, planned, progress),
            });
        } catch (error) {
            // roll back prepared work and release keys
            await this.#settle(settlementsOf(planned.prepared));
            await this.directory?.release(planned.id).catch((failure: unknown) => {
                throw new AggregateError([error, failure], "mutation and its key release failed", {
                    cause: error,
                });
            });

            // record the failed call for audit and retries
            if (progress.running !== undefined) {
                await this.#recordFailure(planned, progress.running, error);
            }
            throw error;
        }
    }

    /** Digest a mutation's calls with their sensitive inputs redacted. */
    async #fingerprint(
        mutation: sync.Mutation,
        originals: readonly ReturnType<typeof Call.resolve>[],
    ): Promise<string> {
        const sensitive: unknown[] = [];
        const calls = mutation.calls.map((entry, index) => {
            const { object, name } = aligned(originals, index);
            const fields = Call.input(object, name, true);

            return {
                method: entry.method,
                input: schema.redact(fields, entry.input, (found) => sensitive.push(found)),
            };
        });

        return this.journal.fingerprint({ id: mutation.id, calls }, sensitive);
    }

    /** Execute a mutation's calls in order inside the journal's transaction, recording each for retries. */
    async #run(
        transaction: DatabaseConnection,
        authorization: Authorization,
        run: MutationRun,
        progress: MutationProgress,
    ): Promise<unknown[]> {
        // execute the calls in order
        const stamp = await transaction.log.transaction();
        const from = run.calls.some((call) => call.object.lifecycle.tracked !== undefined)
            ? await transaction.log.position()
            : undefined;
        const executed: unknown[] = [];
        for (const [index, call] of run.calls.entries()) {
            progress.running = index;
            const options: CallOptions = {
                prepared: run.prepared[index],
                ...(from === undefined ? {} : { from }),
                ...(call.as === undefined ? {} : { as: call.as }),
                ...(call.expansion === undefined ? {} : { expansion: call.expansion }),
                requestId: run.request.requestId,
                request: {
                    ...ObjectServer.#callRequest(run, index),
                    ...(stamp === undefined ? {} : { transaction: stamp }),
                },
            };
            executed.push(
                await this.#execute(
                    transaction,
                    authorization,
                    call.object,
                    call.name,
                    call.input,
                    run.context,
                    options,
                ),
            );
        }

        // commit prepared work and release request-bound relationships
        progress.running = undefined;
        await this.#commit(transaction, run.prepared);
        await this.authorizer.release(transaction, run.id);

        // reserve the names the written objects claim
        progress.reservation =
            this.directory &&
            (await Reservation.open(this.directory, transaction, this.objects, run.id));

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
        await this.audit(run.scope, run.context).record(
            undefined,
            action,
            { ...values, outcome },
            "activity",
            ObjectServer.#callRequest(run, position),
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
        context: ServiceContext,
    ): Promise<Expansion[]> {
        // keep calls whose methods expand into nothing
        const methods = calls.map((call) => this.served(call.object).method(call.name));
        if (methods.every((method) => method.expand === undefined)) {
            return [...calls];
        }

        // list each expanding call's calls before it, as the caller reads them now
        const authorization = await this.admit(this.database, scope, context);
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
                    this.database,
                    authorization,
                    call.object,
                    call.name,
                    call.input,
                );
                const listed = await method.expand(built.call);
                for (const { author, ...entry } of listed) {
                    expanded.push({
                        ...this.#resolve(this.#within(entry, scope), true),
                        as: author,
                    });
                }
                expanded.push({ ...call, expansion: listed });
            }
        }

        return expanded;
    }

    /** Execute pushed mutations in order, stopping at a transient failure. */
    async push(
        scope: string,
        mutations: readonly sync.Mutation[],
        context: ServiceContext,
        client?: string,
    ): Promise<PushResult> {
        // execute each mutation in its own transaction
        const outcomes: PushResult["outcomes"][number][] = [];
        for (const pushed of mutations) {
            const mutation = {
                ...pushed,
                calls: pushed.calls.map((call) => this.#within(call, scope)),
            };
            try {
                // require the pushed scope
                const first = this.#resolve(aligned(mutation.calls, 0), true);
                if (this.#scope(first.object, first.input) !== scope) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `a mutation pushed to ${scope} acts in another scope`,
                    });
                }
                const value = schema.json().parse(await this.mutate(mutation, context, client));
                outcomes.push({ id: mutation.id, outcome: { value } });
            } catch (error) {
                // record final failures, rethrow transient ones
                const failure = Journal.failure(error);
                if (failure === undefined) {
                    throw error;
                }
                outcomes.push({ id: mutation.id, outcome: { error: failure.error } });
            }
        }

        return { outcomes, watermark: await this.watermark(scope) };
    }

    /** Find the served copy of an object type with the server's handlers. */
    served<Type extends ObjectType>(object: Type): Type;
    /**
     * Find the served copy of an object type.
     *
     * @construct the served copy is the declared type with this server's handlers, matched by package and name.
     */
    served(object: ObjectType): ObjectType {
        const found = this.objects.find((served) => served.same(object));
        if (found === undefined) {
            throw new TypeError(`object server serves no ${object.name}`);
        }

        return found;
    }

    /** Call an object type's system methods inside an open transaction, in a scope, at a time. */
    invoke<Object extends ObjectType>(object: Object, context: SystemContext): Invoker<Object> {
        return this.invoker(context)<Object>(object);
    }

    /** Call object types' system methods inside an open transaction, in a scope, at a time. */
    invoker(context: SystemContext): Invoke {
        return invoking((object, name, input) => this.#invoke(context, object, name, input));
    }

    /** Execute one system call inside an open transaction, returning its result. */
    async #invoke(
        context: SystemContext,
        object: ObjectType,
        name: string,
        input: JsonObject,
    ): Promise<unknown> {
        // refuse external work inside an open transaction
        const method = this.served(object).methods[name];
        if (method?.isSystem !== true || method.prepare !== undefined) {
            throw new TypeError(`${object.name}.${name} is no system method without external work`);
        }

        // guard the scope before executing as the system
        const { database, scope, now } = context;
        const chain = await Scope.guard(database, scope);
        const authorization = await SystemAuthorization.open(
            this.authorizer,
            database,
            scope,
            now,
            chain,
        );
        authorization.requireUnmoved();

        return this.#execute(
            database,
            authorization,
            object,
            name,
            scoped(object, input, scope),
            undefined,
        );
    }

    /** Execute system calls in one transaction, returning each call's result. */
    async executeAsSystem<Type extends ObjectType, Name extends MethodName<Type>>(
        object: Type,
        name: Name,
        calls: readonly SystemCall<Select<Type["table"]>, SystemInput<Type, NoInfer<Name>>>[],
        now: number,
    ): Promise<ResultOf<Type, Name>[]>;
    /**
     * Execute system calls in one transaction.
     *
     * @construct each result is the method's result, which ResultOf reads from the same method.
     */
    async executeAsSystem(
        object: ObjectType,
        name: string,
        calls: readonly SystemCall[],
        now: number,
    ): Promise<unknown[]> {
        // build each call as the system, on the served type
        const served = this.served(object);
        const method = ObjectServer.#preparing(served, name);
        const system = this.#systemAuthorizations(now);
        const calling = async (database: DatabaseConnection, entry: SystemCall) =>
            this.#systemCall(served, name, database, entry, {
                authorization: await system(database, entry.scope),
                now,
            });

        // prepare external work
        const prepared = await this.#prepareEach(
            calls.map((entry) =>
                method.prepare === undefined ? undefined : () => calling(this.database, entry),
            ),
        );
        const settlements = settlementsOf(prepared);

        // execute and audit the calls in one transaction
        let reservation: Reservation | undefined;
        const results: unknown[] = [];
        try {
            reservation = await this.database.transaction(async (transaction) => {
                // guard each scope chain and refuse moved scopes, as pushes do
                for (const scope of new Set(calls.map((entry) => entry.scope))) {
                    const chain = await Scope.guard(transaction, scope);
                    (await system(transaction, scope, chain)).requireUnmoved();
                }

                // execute and audit each call
                for (const [index, entry] of calls.entries()) {
                    const call = await calling(transaction, entry);
                    const result = await method.execute(call.with(preparedFields(prepared[index])));
                    results.push(result);
                    await this.#auditSystem(transaction, call, result);
                }

                // commit the external work and reserve unique keys
                await this.#commit(transaction, prepared);

                return (
                    this.directory &&
                    Reservation.open(this.directory, transaction, this.objects, crypto.randomUUID())
                );
            });
        } catch (error) {
            await this.#settle(settlements);
            throw error;
        }

        // confirm keys and settle prepared work
        try {
            await reservation?.confirm();
        } finally {
            await this.#settle(settlements);
        }

        return results;
    }

    /** Open system authorizations at a time, once per database and scope. */
    #systemAuthorizations(
        now: number,
    ): (
        database: DatabaseConnection,
        scope: string,
        links?: readonly ScopeLink[],
    ) => Promise<SystemAuthorization> {
        const systems = new Map<string, Promise<SystemAuthorization>>();

        return (database, scope, links) => {
            // reuse the authorization per database and scope
            const key = `${database === this.database ? "database" : "transaction"} ${scope}`;
            const known = systems.get(key);
            if (known !== undefined) {
                return known;
            }

            // open one
            const opened = SystemAuthorization.open(this.authorizer, database, scope, now, links);
            systems.set(key, opened);

            return opened;
        };
    }

    /** Build one system call on a served type, running invoked calls as the system. */
    #systemCall(
        served: ObjectType,
        name: string,
        database: DatabaseConnection,
        entry: SystemCall,
        system: { readonly authorization: SystemAuthorization; readonly now: number },
    ): Call {
        // identify the target or the creation's chosen identifier
        const { authorization, now } = system;
        const target = entry.target;
        const id = target === undefined ? entry.id : schema.string().parse(target["id"]);

        // parse the input naming it
        const named = { ...entry.input, ...(id === undefined ? {} : { id }) };
        const { fields } = ObjectServer.#fields(served, name, scoped(served, named, entry.scope));

        return new Call({
            object: served,
            name,
            method: served.method(name),
            scope: entry.scope,
            chain: authorization.chain,
            input: fields,
            ...(id === undefined ? {} : { id }),
            ...(target === undefined ? {} : { target }),
            database,
            now,
            authorization,
            objects: this.objects,
            ...(this.#sends === undefined ? {} : { sends: this.#sends }),
            ...(this.installation === undefined ? {} : { installation: this.installation }),
            run: (invoked, invokedName, invokedInput) =>
                this.#execute(
                    database,
                    authorization,
                    invoked,
                    invokedName,
                    scoped(invoked, invokedInput, entry.scope),
                    undefined,
                ),
        });
    }

    /** Audit one executed system call against its target, or its scope without one. */
    async #auditSystem(
        transaction: DatabaseConnection,
        call: Call,
        result: unknown,
    ): Promise<void> {
        const id = call.id ?? Call.resultId(result);
        const target =
            id === undefined ? { type: "scope", id: call.scope } : { type: call.object.name, id };
        await this.audit(call.scope).record(transaction, call.object.audit(call.name), {
            targets: { [call.object.key]: target },
            details: {},
            outcome: { kind: "success" },
        });
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
            new CompactionController(this.database),
            ...(this.#journal === undefined ? [] : [this.#journal.controller(this.#history)]),
            ...(this.directory === undefined
                ? []
                : [new ClaimController(this.directory, this.database, this.objects)]),
            ...(isRecoverable ? [recoverable.controller(this)] : []),
            ...(isExpiring ? [expiring.controller(this)] : []),
            ...(isSettled ? [Settlement.controller(this)] : []),
            ...(this.#sends === undefined || this.runs === undefined
                ? []
                : [this.#sends.controller(RUNS.to(this.runs, this.#report))]),
            ...(this.runs === undefined || this.triggers.length === 0
                ? []
                : [new ChangeController(this.database, this.triggers, this.runs, this.#report)]),
            ...(this.subscriber === undefined ? [] : [this.source.controller(this.subscriber)]),
            ...[this.source.addresses()].filter((controller) => controller !== undefined),
            ...this.objects.flatMap((object) =>
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

    /** Resolve a caller's access in a scope, bound to a request for mutations. */
    async authorize(
        database: DatabaseConnection,
        scope: string,
        context: ServiceContext,
        request?: string,
        links?: readonly ScopeLink[],
    ): Promise<Authorization> {
        // bind the caller per scope and request
        const bind = (bound: string) => {
            const caller = this.accessContext(context, bound);

            return request === undefined ? caller : { ...caller, request };
        };
        const access = await this.authorizer.resolve(
            Snapshot.live(database),
            scope,
            bind(scope),
            links,
        );

        return new Authorization(
            this.authorizer,
            database,
            bind,
            access,
            context.authentication?.claims.delegation,
        );
    }

    /** Authorize a principal living inside some scopes in a scope, as a follow of its copies does. */
    async authorizeSubject(
        database: DatabaseConnection,
        scope: string,
        subject: Subject,
        within: readonly ObjectReference[] = [],
    ): Promise<Authorization> {
        const bind = (): AccessContext => ({
            subjects: [subject, ...within.map((inside) => ({ ...inside, relation: CONTAINED }))],
            now: this.clock(),
            attributes: {},
        });
        const access = await this.authorizer.resolve(Snapshot.live(database), scope, bind());

        return new Authorization(this.authorizer, database, bind, access);
    }

    /** Admit a caller: resolve its access and require an unmoved, visible scope. */
    async admit(
        database: DatabaseConnection,
        scope: string,
        context: ServiceContext,
        request?: string,
        links?: readonly ScopeLink[],
    ): Promise<Authorization> {
        // resolve and require an unmoved, visible scope
        const authorization = await this.authorize(database, scope, context, request, links);
        authorization.requireUnmoved();
        await authorization.requireVisible(this.objects);

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
        try {
            // wait on the scope's own log
            if (watermark.scope === scope) {
                if (watermark.epoch !== (await this.database.log.epoch())) {
                    throw new DatabaseError("STALE_EPOCH", `${scope} has another epoch`);
                }

                return await this.database.log.wait(watermark.sequence, signal);
            }

            // wait on the copy of an enclosing scope
            return await sync.Replica.reach(this.database, watermark.scope, watermark, signal);
        } catch (error) {
            // report a stale epoch
            if (error instanceof DatabaseError && error.code === "STALE_EPOCH") {
                throw new ServiceError("STALE_EPOCH", {
                    message: `${watermark.scope} no longer has the watermark's history`,
                });
            }
            throw error;
        }
    }

    /** The served durable object types. */
    get durable(): ObjectType[] {
        return this.objects.filter((object) => object.storage === "durable");
    }

    /** Read the ephemeral store, failing without one. */
    store(): EphemeralStorage {
        if (this.ephemeral === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: "no ephemeral objects are served here",
            });
        }

        return this.ephemeral;
    }

    /** Execute ephemeral calls in one memory transaction as their client. */
    async #mutateEphemeral(
        calls: readonly { object: ObjectType; name: string; input: JsonObject }[],
        scope: string,
        context: ServiceContext,
        client: string | undefined,
    ): Promise<unknown[]> {
        // require the client owning the rows
        if (client === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "ephemeral objects are written by a client",
            });
        }

        // admit and execute the calls in order
        const authorization = await this.admit(this.database, scope, context);

        return this.store().write(
            EphemeralStorage.clientKey(context, client),
            async (transaction) => {
                const executed: unknown[] = [];
                for (const call of calls) {
                    executed.push(
                        await this.#execute(
                            transaction,
                            authorization,
                            call.object,
                            call.name,
                            call.input,
                            context,
                            { client },
                        ),
                    );
                }

                return executed;
            },
        );
    }

    /** Resolve a recorded call against the served types. */
    #resolve(entry: sync.Call, mutates: boolean): ReturnType<typeof Call.resolve> {
        return Call.resolve(this.objects, entry, mutates);
    }

    /** Run a pushed call that leaves out its scope field in the scope it is pushed to. */
    #within(call: sync.Call, scope: string): sync.Call {
        // leave a call naming its scope, or one of an unknown type the resolution refuses
        const served = this.#schemas.get(call.method.slice(0, call.method.lastIndexOf(".")));
        const field = served?.object.route.field;
        if (field === undefined || Object.hasOwn(call.input, field)) {
            return call;
        }

        return { ...call, input: { ...call.input, [field]: scope } };
    }

    /** Read the scope a procedure call acts in: an object method's own scope, else the decided target's or the context's. */
    #procedureScope(call: ProcedureCall<ServiceContext>): string | undefined {
        // read the routed object's scope field from the input
        const object = this.#routed.find((each) => each.key === call.path[0]);
        const input = call.input === undefined ? undefined : PROCEDURE_INPUT.parse(call.input);
        const field = object?.route.field;
        const named = field === undefined ? undefined : input?.[field];

        return typeof named === "string"
            ? named
            : (call.context.target?.scope ?? call.context.scope);
    }

    /** Read the scope a call names in its route field. */
    #scope(object: ObjectType, input: JsonObject): string {
        // read the field, or the universe
        const { field } = object.route;
        const named = field === undefined ? Scope.universe.id : input[field];
        if (typeof named !== "string") {
            throw new ServiceError("BAD_REQUEST", { message: `call names no ${field}` });
        }

        return named;
    }

    /** Build one call with its permitted target at the named revision. */
    async #call(
        database: DatabaseConnection,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        options: Pick<CallOptions, "as" | "expansion"> = {},
    ): Promise<{ call: Call; method: Method; scope: string; targetId: string | undefined }> {
        // refuse changing the rows of a type kept as copies from its home
        const method = object.method(name);
        if (method.mutates && this.database.copies(object.table)) {
            throw new ServiceError("CONFLICT", {
                message: `${object.name} is a copy, which changes at its home`,
            });
        }

        // parse the input and build the call in its scope through the view a read names
        const { fields, id, revision, at, branch } = ObjectServer.#fields(object, name, input);
        const scope = this.#scope(object, input);
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
            ...(options.as === undefined ? callerOf(authorization) : { caller: options.as }),
            ...(options.expansion === undefined ? {} : { expansion: options.expansion }),
            now: authorization.access.context.now,
            authorization,
            objects: this.objects,
            ...(this.#sends === undefined ? {} : { sends: this.#sends }),
            ...(snapshot === undefined ? {} : { snapshot }),
            ...(this.installation === undefined ? {} : { installation: this.installation }),
        });

        // load the permitted target at the named revision
        const target = await ObjectServer.#target(call, targetId, snapshot, authorization);
        if (revision !== undefined && target?.["revision"] !== revision) {
            throw new ServiceError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        // refuse callers' other methods on a trashed object
        const isTrashed =
            object.lifecycle.recoverable !== undefined &&
            target !== undefined &&
            target["deletionRequestedAt"] !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.isSystem !== true) {
            throw new ServiceError("CONFLICT", { message: `${object.name} is in the trash` });
        }

        return {
            call: target === undefined ? call : call.with({ target }),
            method,
            scope,
            targetId,
        };
    }

    /** Parse a call's input by its method's input schema, splitting off the fields routing it and the view a read names. */
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
        const parsed = Call.input(object, name, method.mutates).safeParse(input);
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

    /** Load a call's permitted target through the view a read names, absent for a call without one. */
    static async #target(
        call: Call,
        targetId: string | undefined,
        snapshot: Snapshot | undefined,
        authorization: Authorization,
    ): Promise<Row | undefined> {
        // skip a call without a decided target
        const { method } = call;
        const isDecided = method.permission !== null || method.isSystem === true;
        if (!method.target || !isDecided || targetId === undefined) {
            return undefined;
        }

        // read the live target, or the target in the view
        if (snapshot === undefined) {
            return authorization.read(call, targetId);
        } else {
            return authorization.readIn(call, targetId, snapshot);
        }
    }

    /** Build the view a read names: the database at a position, under a branch's rows, or none for the live main line. */
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

        return base.layer(await new Branch(types, database, id).overlay(base, this.objects));
    }

    /** Read a method about to run, refusing one that declares prepared work it never prepares. */
    static #preparing(object: ObjectType, name: string): Method {
        const method = object.method(name);
        if (method.prepared !== undefined && method.prepare === undefined) {
            throw new TypeError(`${object.name}.${name} declares prepared work it never prepares`);
        }

        return method;
    }

    /** Prepare each call's external work outside the transaction. */
    async #prepare(
        calls: readonly { object: ObjectType; name: string; input: JsonObject }[],
        scope: string,
        context: ServiceContext,
        request?: RequestIdentity,
    ): Promise<Prepared[]> {
        // skip when nothing prepares or the request executed
        const methods = calls.map((call) => ObjectServer.#preparing(call.object, call.name));
        if (
            methods.every((method) => method.prepare === undefined) ||
            (request !== undefined && (await this.journal.outcome(request)) !== undefined)
        ) {
            return calls.map(() => undefined);
        }

        // build and authorize each preparing call as the caller
        const authorization = await this.admit(this.database, scope, context, request?.requestId);

        return this.#prepareEach(
            calls.map((entry, index) => {
                const method = aligned(methods, index);
                if (method.prepare === undefined) {
                    return undefined;
                }

                return async () => {
                    const { call } = await this.#call(
                        this.database,
                        authorization,
                        entry.object,
                        entry.name,
                        entry.input,
                    );
                    await method.authorize?.(call);

                    return call;
                };
            }),
        );
    }

    /** Prepare built calls in order, reserving each settlement before its work starts and rolling back on failure. */
    async #prepareEach(
        builds: readonly ((() => Promise<Call>) | undefined)[],
    ): Promise<Prepared[]> {
        // prepare in order, keeping the reserved settlements
        const prepared: Prepared[] = [];
        const settlements: (string | undefined)[] = [];
        try {
            for (const build of builds) {
                // skip a call without external work
                if (build === undefined) {
                    prepared.push(undefined);
                    continue;
                }

                // reserve before preparing
                const reserved = await this.#reserve(await build());
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

    /** Key a call's external work, and reserve its settlement when its method settles. */
    async #reserve(
        call: Call,
    ): Promise<{ readonly call: Call; readonly settlement: string | undefined }> {
        const key = call.method.idempotencyKey?.(call) ?? v7();
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
                await Settlement.commit(transaction, work.settlement, work.call, Date.now());
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
                const now = Date.now();
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
        transaction: DatabaseConnection,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        context: ServiceContext | undefined,
        options: CallOptions = {},
    ): Promise<unknown> {
        const attributes = { "destack.object.type": object.name, "destack.object.method": name };

        return span("object.call", attributes, () =>
            this.#perform(transaction, authorization, object, name, input, context, options),
        );
    }

    /** Execute, redact and audit one call. */
    async #perform(
        transaction: DatabaseConnection,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: JsonObject,
        context: ServiceContext | undefined,
        options: CallOptions,
    ): Promise<unknown> {
        // build the call on the served type to run its handlers
        const { prepared, from, client, requestId } = options;
        const { call, method, scope, targetId } = await this.#call(
            transaction,
            authorization,
            this.served(object),
            name,
            input,
            options,
        );
        span.current()?.setAttributes({
            "destack.scope": scope,
            ...(targetId === undefined ? {} : { "destack.object.id": targetId }),
        });

        // run invoked calls in the call's transaction and scope
        const called = call.with({
            ...preparedFields(prepared),
            ...(client === undefined ? {} : { client }),
            ...(requestId === undefined ? {} : { requestId }),
            run: this.#invoker(transaction, authorization, scope, call.now, context, options),
        });

        // authorize a caller unless prepared, and execute
        if (prepared === undefined && !(authorization instanceof SystemAuthorization)) {
            await method.authorize?.(called);
        }
        const executed = await method.execute(called);

        // present the result, track changes and record the call
        const presented = await this.#present(transaction, authorization, object, call, executed);
        const id = targetId ?? Call.resultId(executed);
        if (method.mutates && id !== undefined) {
            await ObjectServer.#track(transaction, object, call.with({ id }), from);
        }
        await this.#record(transaction, object, call, id, presented, context, options.request);

        return presented;
    }

    /** Run the calls a call invokes in its transaction and scope, as the system for system methods and system authority. */
    #invoker(
        transaction: DatabaseConnection,
        authorization: Authorization,
        scope: string,
        now: number,
        context: ServiceContext | undefined,
        options: CallOptions,
    ): Run {
        const { from, client, requestId } = options;
        const system = this.#systemAuthorizations(now);

        return async (invoked, invokedName, invokedInput, invocation) => {
            // run system methods and system-authority calls as the system
            const isSystem =
                invocation.authority === "system" ||
                invoked.methods[invokedName]?.isSystem === true;
            const running = isSystem ? await system(transaction, scope) : authorization;

            return this.#execute(
                transaction,
                running,
                invoked,
                invokedName,
                scoped(invoked, invokedInput, scope),
                context,
                {
                    ...(from === undefined ? {} : { from }),
                    ...(client === undefined ? {} : { client }),
                    ...(requestId === undefined ? {} : { requestId }),
                    ...(invocation.as === undefined ? {} : { as: invocation.as }),
                },
            );
        };
    }

    /** Redact and encode what a call returns: its value, its page of objects, or its object with their texts. */
    async #present(
        transaction: DatabaseConnection,
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
        transaction: DatabaseConnection,
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
        const table = object.table;
        const rows: readonly Row[] = await transaction
            .select()
            .from(table)
            .where(eq(table[TABLE].column("id"), present(call.id, "the tracked object")));
        const [after] = rows;
        if (after !== undefined) {
            const before = present(from, "the log position before a tracked call");
            await tracked.record(history, call, call.target, after, before);
        }
    }

    /** Record a call: audit durable changes, and keep every call of a request for retries. */
    async #record(
        transaction: DatabaseConnection,
        object: ObjectType,
        call: Call,
        id: string | undefined,
        presented: unknown,
        context: ServiceContext | undefined,
        request: CallRequest | undefined,
    ): Promise<void> {
        // describe the call's target and outcome
        const { method, scope, name } = call;
        const isAudited = method.mutates && object.storage === "durable";
        const target = id === undefined ? { type: "scope", id: scope } : { type: object.name, id };
        const recorded: {
            targets: Record<string, typeof target>;
            details: {};
            outcome: sync.Outcome;
        } = {
            targets: { [object.key]: target },
            details: {},
            outcome: {
                kind: "success",
                ...(request === undefined ? {} : { value: schema.json().parse(presented ?? null) }),
            },
        };

        // audit a durable change
        const recorder = this.audit(scope, context);
        if (isAudited) {
            await recorder.record(transaction, object.audit(name), recorded, "activity", request);
        }
        // keep a request's other call for retries
        else if (request !== undefined) {
            await recorder.keep(transaction, object.audit(name), recorded, request);
        }
    }
}

/** Throw a failed settlement. */
function rethrow(error: unknown): never {
    throw error;
}

/** Name a scope in an invoked call's input. */
function scoped(object: ObjectType, input: JsonObject, scope: string): JsonObject {
    const { field } = object.route;

    return field === undefined ? { ...input } : { ...input, [field]: scope };
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

/** Read the calling principal from an authorization. */
function callerOf(authorization: Authorization): Pick<Call, "caller"> {
    const context = authorization.access.context;
    const caller = context.subject ?? Caller.principal(context);

    return caller === undefined ? {} : { caller };
}

/** Build system calls. */
export const SystemCall = {
    /** Call on an existing object's row, in its scope. */
    of<Row extends Select<Table>, Input extends Readonly<Record<string, unknown>>>(
        row: Row,
        input?: Input,
    ): SystemCall<Row, Input> {
        return {
            scope: schema.string().parse(row["scope"]),
            target: row,
            ...(input === undefined ? {} : { input }),
        };
    },
};

/** One call the system makes. */
export interface SystemCall<Row extends Select<Table> = Select<Table>, Input = JsonObject> {
    /** The scope the call acts in. */
    readonly scope: string;
    /** The object the call acts on, as its table selects it, absent for a creation or a call on the collection. */
    readonly target?: Row;
    /** The identifier a creation takes, a fresh one when absent. */
    readonly id?: string;
    /** The method's input, without the target's identifier. */
    readonly input?: Input;
}

/** The input of a system call: the method's call input without the target's identifier, any record for a type or method not known statically. */
type SystemInput<
    Object extends ObjectType,
    Name extends MethodName<Object>,
> = ObjectType extends Object
    ? JsonObject
    : string extends Name
      ? JsonObject
      : Omit<CallInput<Object, Name>, "id">;

/** Give a prepared creation without an identifier a fresh one, as its prepared work names it. */
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
