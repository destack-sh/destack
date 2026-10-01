import {
    Authorizer,
    principalOf,
    type Subject,
    type AccessContext,
    type GrantReader,
    Policy,
    type Access,
    type TableMapping,
} from "@destack/access";
import { Scope, type ScopeLink } from "@destack/sync";
import { LogPosition, Snapshot } from "@destack/db/log";
import { AuditRecorder } from "@destack/audit";
import {
    encodeRow,
    eq,
    TABLE,
    type DatabaseConnection,
    type JsonValue,
    type Table,
} from "@destack/db";

import { DatabaseError } from "@destack/db/error";
import { schema } from "@destack/schema";
import type { Watermark } from "@destack/service/bookmark";
import {
    CompactionController,
    Journal,
    type JournalKey,
    type defineJournal,
} from "@destack/service/database";
import { ServiceError } from "@destack/service/error";
import { type RequestIdentity } from "@destack/service/request";
import { v7 } from "uuid";
import {
    implement,
    type Router,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { withEventMeta, type Service } from "@destack/service";
import * as sync from "@destack/sync";
import { Call, RUNS } from "../method/call.ts";
import type { Method, MethodKind } from "../method/method.ts";
import type { ObjectProcedures, ObjectSchema } from "../method/procedure.ts";
import {
    ClientId,
    replicaProcedures,
    type ObjectQuery,
    type PushResult,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ObjectType, REPLICATE, REPRESENT } from "../object/object.ts";
import type { ObjectController } from "../object/controller.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { camelCase } from "../object/name.ts";
import { Authorization, SystemAuthorization } from "./authorization.ts";
import { WatchController } from "./watch.ts";
import { Outbox } from "@destack/service/outbox";
import type { Directory } from "@destack/directory";
import { DirectoryDatabase } from "@destack/directory";
import { ClaimController, Reservation } from "../claim/index.ts";
import { EphemeralStorage } from "./ephemeral.ts";
import { ObjectSource } from "./source.ts";

import { tracked } from "../trait/tracked.ts";
import { recoverable } from "../trait/recoverable.ts";
import { expiring } from "../trait/expiring.ts";
import { addressed } from "../trait/addressed.ts";
import type { Controller } from "@destack/service/control";
import type { RunClient, TriggerOf } from "@destack/service/trigger";

import { Settlement } from "./settlement.ts";
import { telemetry } from "@destack/telemetry";

/** The object call spans. */
const { span } = telemetry.scope(import.meta.destack.package);
/** The recent pages sharing grant readers: 64 readers of a few KiB, well under a megabyte. */
const SHARED_READERS = 64;

/** The methods an object in the trash still takes. */
const TRASH_METHODS: ReadonlySet<MethodKind> = new Set([
    "get",
    "restore",
    "purge",
    "relationships",
    "proposals",
]);

/** How long a request waits for its caller's watermarks, in milliseconds: 5 s, above sub-second copy lag. */
const BOOKMARK_TIMEOUT_MILLISECONDS = 5000;

/** The work a call prepared outside its transaction. */
/** The copies a database keeps, and the source streaming them. */
export interface ObjectReplicas {
    /** The source streaming each copy's pages. */
    readonly source: sync.ReplicaSource;
    /** List the requests of the copies the database keeps now. */
    requests(): Promise<readonly Omit<sync.ReplicaRequest, "after">[]>;
}

/** The work a call prepared outside its transaction. */
type Prepared = { readonly value: unknown; readonly call: Call } | undefined;

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
    /** The database holding the objects. */
    readonly database: DatabaseConnection;
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
    /** The journal of executed mutations. */
    readonly journal: Journal;
    /** The feed serving every sync. */
    readonly feed: sync.Feed;
    /** The directory holding the claims of the served objects' unique indexes. */
    readonly directory?: Directory;
    /** The memory store holding the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** The copies the database keeps, and the source streaming them. */
    readonly replicas?: ObjectReplicas;
    /** The cell recording the objects' runs: the calls methods send, and the calls of the objects' watches. */
    readonly runs?: RunClient;
    /** The watches of the served objects, each change they admit recorded as a run. */
    readonly watches: readonly TriggerOf<"watch">[];
    /** The outbox delivering the calls methods send, absent where no cell records runs. */
    readonly #sends?: Outbox;
    /** Derive a request's verified authorization inputs within a scope. */
    readonly #context: (context: ServiceContext, scope: string) => AccessContext;
    /** Open an audit recorder for a scope and request. */
    readonly #audit: (scope: string, context?: ServiceContext) => AuditRecorder<DatabaseConnection>;
    /** Report failed settlements. */
    readonly #report: (error: unknown) => void;
    /** Whether reads of the objects record access events. */
    readonly isAccessAudited: boolean;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, GrantReader>();
    /** The input schemas of pushed calls, by method. */
    readonly #inputs = new Map<string, schema.Object<Record<string, schema.Schema>>>();
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
        /** The database holding the objects. */
        readonly database: DatabaseConnection;
        /** Derive a request's verified authorization inputs within a scope, the request's own access by default. */
        readonly context?: (context: ServiceContext, scope: string) => AccessContext;
        /** The record of every mutation executed. */
        readonly journal: Journal;
        /** The directory holding the claims of unique indexes, in the global database. */
        readonly directory?: Directory;
        /** The memory store holding the served ephemeral objects. */
        readonly ephemeral?: EphemeralStorage;
        /** The copies the database keeps, and the source streaming them. */
        readonly replicas?: ObjectReplicas;
        /** The cell recording the objects' runs: the calls methods send, and the calls of the objects' watches. */
        readonly runs?: RunClient;
        /** The watches of the served objects, which need a cell recording their runs. */
        readonly watches?: readonly TriggerOf<"watch">[];
        /** Open an audit recorder for a scope and request. */
        readonly audit: (
            scope: string,
            context?: ServiceContext,
        ) => AuditRecorder<DatabaseConnection>;
        /** Report failed settlements, thrown when absent. */
        readonly report?: (error: unknown) => void;
        /** Whether reads of the objects record access events, as the space's audit setting asks. */
        readonly isAccessAudited?: boolean;
    }) {
        // require each object type under its own name
        for (const [key, object] of Object.entries(options.objects)) {
            if (key !== camelCase(object.name)) {
                throw new TypeError(`object ${object.name} is served under ${key}`);
            } else if (Object.hasOwn(object.shared, key)) {
                throw new TypeError(
                    `object ${object.name} takes the name of the shared ${key} procedures`,
                );
            }
        }

        // require the directory, in another database, for indexed objects
        this.#routed = Object.values(options.objects);
        this.objects = ObjectType.served(this.#routed);
        const indexed = this.objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.directory === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but no directory keeps their claims`,
            );
        } else if (
            options.directory instanceof DirectoryDatabase &&
            options.directory.database === options.database
        ) {
            throw new TypeError(
                "the directory keeps claims across databases, apart from the objects'",
            );
        }
        if (options.directory !== undefined) {
            this.directory = options.directory;
        }
        if (options.replicas !== undefined) {
            this.replicas = options.replicas;
        }
        if (options.runs !== undefined) {
            this.runs = options.runs;
            this.#sends = new Outbox(options.database);
        }
        this.watches = options.watches ?? [];
        if (this.watches.length > 0 && options.runs === undefined) {
            throw new TypeError("watches need a cell recording their runs");
        }

        // require a store for ephemeral objects
        const ephemeral = this.objects.filter((object) => object.storage === "ephemeral");
        const unheld = ephemeral.find(
            (object) => options.ephemeral?.objects.some((held) => held.same(object)) !== true,
        );
        if (unheld !== undefined) {
            throw new TypeError(`ephemeral object ${unheld.name} needs a store holding it`);
        }
        if (options.ephemeral !== undefined) {
            this.ephemeral = options.ephemeral;
        }

        // build the authorizer over the objects and every scope type enclosing them, and the feed
        const others = options.policies ?? [];
        this.database = options.database;
        this.authorizer = new Authorizer(
            [
                ...this.objects.flatMap((object) => [
                    object.policy,
                    ...object.scopes
                        .flatMap((scope) => [scope, ...scope.ancestors])
                        .map((scope) => scope.policy),
                ]),
                ...others.map((other) => (other instanceof Policy ? other : other.policy)),
            ],
            [
                ...[...this.objects, ...others.filter((other) => other instanceof ObjectType)].map(
                    (object) => object.mapping,
                ),
                ...others.flatMap((other) =>
                    other instanceof Policy || other instanceof ObjectType ? [] : [other],
                ),
            ],
            { copies: (table) => options.database.copies(table) },
        );
        tracked.require(this.objects, this.authorizer);
        this.journal = options.journal;
        this.#context = options.context ?? ((context, scope) => context.access(scope));
        this.#audit = options.audit;
        this.isAccessAudited = options.isAccessAudited ?? false;
        this.#report =
            options.report ??
            ((error) => {
                throw error;
            });
        this.feed = new sync.Feed(options.database, [
            ...new Set(
                [...this.objects, ...others.filter((other) => other instanceof ObjectType)]
                    .filter((object) => object.storage === "durable")
                    .flatMap((object) => object.tables),
            ),
            options.journal.table,
        ]);
        this.#schemas = new Map(
            this.objects.map((object) => [object.name, { object, schema: object.schema }]),
        );
    }

    /** Serve a service's object types over one database. */
    static serve(
        service: Service,
        options: {
            /** The database holding the objects. */
            readonly database: DatabaseConnection;
            /** The service's journal. */
            readonly journal: ReturnType<typeof defineJournal>;
            /** Read the deployment's key that sensitive inputs are fingerprinted under. */
            readonly journalKey: JournalKey;
            /** Open an audit recorder for a scope and request. */
            readonly audit: (
                scope: string,
                context?: ServiceContext,
            ) => AuditRecorder<DatabaseConnection>;
            /** The directory holding the claims of the objects' unique indexes. */
            readonly directory?: Directory;
            /** The memory store holding the ephemeral objects. */
            readonly ephemeral?: EphemeralStorage;
            /** Further controllers running alongside the objects'. */
            readonly controllers?: readonly Controller[];
            /** The source of the copies of the objects' space: its chain, and what its objects point at. */
            readonly replicas?: { readonly scope: string; readonly source: sync.ReplicaSource };
            /** The cell recording the objects' runs: the calls methods send, and the calls of the objects' watches. */
            readonly runs?: RunClient;
            /** The watches of the served objects. */
            readonly watches?: readonly TriggerOf<"watch">[];
        },
    ): ServiceImplementation {
        // serve the service's objects, keeping the copies of their space
        const replicas = options.replicas;
        const objects: ObjectServer = new ObjectServer({
            objects: service.objects as Readonly<Record<string, ObjectType>>,
            database: options.database,
            audit: options.audit,
            journal: new Journal(options.journal),
            ...(options.directory === undefined ? {} : { directory: options.directory }),
            ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
            ...(replicas === undefined
                ? {}
                : {
                      replicas: {
                          source: replicas.source,
                          requests: () =>
                              objects.source.replicaRequests(replicas.scope, { isHome: false }),
                      },
                  }),
            ...(options.runs === undefined ? {} : { runs: options.runs }),
            ...(options.watches === undefined ? {} : { watches: options.watches }),
        });

        return objects.implement(service, options.controllers);
    }

    /** Implement a service with the served objects' access, audit, controllers and router, running further controllers alongside. */
    implement(service: Service, controllers: readonly Controller[] = []): ServiceImplementation {
        return {
            service,
            access: this.access,
            audit: AuditRecorder.procedure(({ context }) =>
                this.audit(context.scope ?? Scope.universe.id, context),
            ),
            controllers: [...this.controllers(), ...controllers],
            router: this.router(),
            clock: this.clock,
        };
    }

    /** The service access deciding the served objects. */
    get access(): ServiceAccess {
        return { authorizer: this.authorizer, database: this.database };
    }

    /** Route every served object's methods and the replica procedures. */
    router(): ObjectRouter<Objects> {
        const routers: Record<string, unknown> = { replica: this.#replica() };
        for (const object of this.#routed) {
            routers[camelCase(object.name)] = this.#route(object);
        }

        return routers as ObjectRouter<Objects>;
    }

    /** Route an object's methods to the call pipeline. */
    #route(object: ObjectType): unknown {
        // bind each method's procedure to the call pipeline
        const contract = implement(object.procedures).$context<ServiceContext>();
        const routes: Record<string, unknown> = {};
        for (const [name, method] of Object.entries(
            object.methods as Readonly<Record<string, Method>>,
        )) {
            if (method.isSystem) {
                continue;
            }
            const procedure = (contract as Record<string, { handler: Function }>)[name]!;
            routes[name] = procedure.handler(
                ({ input, context }: { input: Record<string, unknown>; context: ServiceContext }) =>
                    this.call(object, name, input, context),
            );
        }

        return (contract as unknown as { router(value: unknown): unknown }).router(routes);
    }

    /** Route the replica procedures. */
    #replica(): Router<ReplicaProcedures, ServiceContext> {
        const replica = implement(replicaProcedures).$context<ServiceContext>();

        return replica.router({
            push: replica.push.handler(({ input, context }) =>
                this.push(input.scope, input.mutations, context, input.client),
            ),
            sync: replica.sync.handler(({ input, context, lastEventId }) => {
                const { scope, after, ...options } = input;
                const position = resumed(after, lastEventId);

                return this.source.sync(scope, context, {
                    ...options,
                    ...(position === undefined ? {} : { after: position }),
                });
            }),
            call: replica.call.handler(async ({ input, context }) => {
                // read in the named scope
                const { object, name, input: called } = this.#resolve(input.call, false);
                if (this.#scope(object, called) !== input.scope) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `${object.name}.${name} reads another scope than ${input.scope}`,
                    });
                }

                return (await this.call(object, name, called, context)) as JsonValue;
            }),
            broadcast: replica.broadcast.handler(async ({ input, context }) => {
                await this.source.broadcast(input.scope, input.object, input.event, context);

                return {};
            }),
            stream: replica.stream.handler(({ input, context }) =>
                this.source.relayed(input, context),
            ),
        });
    }

    /** Execute one method as a single-call mutation or a query. */
    async call(
        object: ObjectType,
        name: string,
        input: Record<string, unknown>,
        context: ServiceContext,
    ): Promise<unknown> {
        // query a reading method
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
        const isEphemeral = object.storage === "ephemeral";
        const { [isEphemeral ? "client" : "requestId"]: named, ...rest } = input;
        if (!method.mutates) {
            return this.query(object, name, rest, context);
        }

        // mutate with one call
        const calls = [Call.record(object, name, rest)];
        const [result] = isEphemeral
            ? await this.mutate({ id: crypto.randomUUID(), calls }, context, ClientId.parse(named))
            : await this.mutate({ id: schema.string().parse(named), calls }, context);

        return result;
    }

    /** Read through a method in a read-only transaction. */
    async query(
        object: ObjectType,
        name: string,
        input: Record<string, unknown>,
        context: ServiceContext,
    ): Promise<unknown> {
        // wait for the caller's watermarks
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
        const scope = this.#scope(object, input);
        await this.enter(context, scope);
        const isEphemeral = object.storage === "ephemeral";
        const database = isEphemeral ? this.store().database : this.database;
        const read = async () => {
            // prepare, read, commit and settle
            const [prepared] = await this.#prepare([{ object, name, input }], scope, context);
            let value: unknown;
            try {
                value = await readIn(prepared);
                await this.#commit(this.database, [prepared]);
            } catch (error) {
                await this.#settle([prepared], false);
                throw error;
            }
            await this.#settle([prepared], true);

            return value;
        };
        const readIn = (prepared: Prepared) =>
            database.transaction(
                async (transaction) => {
                    // admit the caller against the durable database
                    const authorization = await this.admit(
                        isEphemeral ? this.database : transaction,
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
        // record a read as one access event, with the details its result holds
        else {
            return recorder.read(
                action,
                values,
                read,
                details &&
                    ((value) =>
                        Object.fromEntries(
                            Object.keys(details.shape).map((field) => [
                                field,
                                (value as Record<string, unknown>)[field],
                            ]),
                        )),
            );
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
        const scope = this.#scope(originals[0]!.object, originals[0]!.input);
        if (originals.some((call) => this.#scope(call.object, call.input) !== scope)) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation's calls act in one scope",
            });
        }
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

        // expand calls into the calls they run after, and mint prepared creations' identifiers
        const calls = (await this.#expand(originals, scope, context)).map((call) => {
            const method = (call.object.methods as Readonly<Record<string, Method>>)[call.name]!;

            return method.kind === "create" &&
                method.prepare !== undefined &&
                call.input.id === undefined
                ? { ...call, input: { ...call.input, id: `${call.object.identity}-${v7()}` } }
                : call;
        });

        // identify the mutation by caller, scope and request identifier
        const request = {
            caller: context.requireCaller().id,
            scope,
            requestId: mutation.id,
        };
        const sensitive: unknown[] = [];
        const fingerprint = await this.journal.fingerprint(
            {
                id: mutation.id,
                calls: mutation.calls.map((entry, index) => ({
                    method: entry.method,
                    input: schema.redact(
                        Call.input(originals[index]!.object, originals[index]!.name, true),
                        entry.input,
                        (found) => sensitive.push(found),
                    ),
                })),
            },
            sensitive,
        );

        // prepare external work
        const prepared = await this.#prepare(calls, scope, context, request);

        // execute through the journal and keep the call running
        let authorization: Authorization | undefined;
        let running: number | undefined;
        let reservation: Reservation | undefined;
        let isExecuted = false;
        let results: unknown[];
        try {
            results = (await this.journal.execute(this.database, request, fingerprint, {
                authorize: async (transaction) => {
                    // guard the scope chain and admit the caller
                    const chain = await Scope.chain(Snapshot.live(transaction), scope);
                    await Scope.guard(
                        transaction,
                        chain.map((link) => link.object.id),
                    );
                    authorization = await this.admit(
                        transaction,
                        scope,
                        context,
                        mutation.id,
                        chain,
                    );
                },
                run: async (transaction) => {
                    // execute the calls in order
                    isExecuted = true;
                    const from = calls.some((call) => call.object.tracked !== undefined)
                        ? await transaction.log.position()
                        : undefined;
                    const executed: unknown[] = [];
                    for (const [index, call] of calls.entries()) {
                        running = index;
                        executed.push(
                            await this.#execute(
                                transaction,
                                authorization!,
                                call.object,
                                call.name,
                                call.input,
                                context,
                                {
                                    prepared: prepared[index],
                                    from,
                                    as: call.as,
                                    expansion: call.expansion,
                                },
                            ),
                        );
                    }

                    // commit prepared work
                    running = undefined;
                    await this.#commit(transaction, prepared);

                    // release request-bound relationships
                    await this.authorizer.release(transaction, mutation.id);

                    // reserve the names the written objects claim
                    reservation =
                        this.directory &&
                        (await Reservation.open(
                            this.directory,
                            transaction,
                            this.objects,
                            mutation.id,
                            authorization!.access.context.now,
                        ));

                    return executed;
                },
            })) as unknown[];
        } catch (error) {
            // release keys and cancel prepared work
            await this.directory?.release(mutation.id).catch((failure: unknown) => {
                throw new AggregateError([error, failure], "mutation and its key release failed");
            });
            await this.#settle(prepared, false);

            // record the failed call, leaving denials to the procedure layer
            const failed = running === undefined ? undefined : calls[running];
            const result = AuditRecorder.result(error);
            if (failed !== undefined && result.outcome !== "denied") {
                const { action, values } = failed.object.auditCall(
                    failed.name,
                    failed.input,
                    scope,
                );
                await this.audit(scope, context).record(undefined, action, {
                    ...values,
                    ...result,
                });
            }
            throw error;
        }

        // confirm keys and settle prepared work
        await reservation?.confirm();
        await this.#settle(prepared, isExecuted);

        // report the watermark
        context.observed.observe(await this.watermark(scope));

        return results;
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
                const first = this.#resolve(mutation.calls[0]!, true);
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
                outcomes.push({ id: mutation.id, outcome: failure });
            }
        }

        return { outcomes, watermark: await this.watermark(scope) };
    }

    /** Find the served copy of an object type with the server's handlers. */
    served<Type extends ObjectType>(object: Type): Type {
        const found = this.objects.find((served) => served.same(object));
        if (found === undefined) {
            throw new TypeError(`object server serves no ${object.name}`);
        }

        return found as Type;
    }

    /** Execute one system call inside an open transaction, returning its result. */
    async invoke(
        transaction: DatabaseConnection,
        scope: string,
        object: ObjectType,
        name: string,
        input: Readonly<Record<string, unknown>>,
        now: number,
    ): Promise<unknown> {
        // refuse external work inside an open transaction
        const method = (this.served(object).methods as Readonly<Record<string, Method>>)[name];
        if (method?.isSystem !== true || method.prepare !== undefined) {
            throw new TypeError(`${object.name}.${name} is no system method without external work`);
        }

        // guard the scope, then execute as the system
        const authorization = await SystemAuthorization.open(
            this.authorizer,
            transaction,
            scope,
            now,
        );
        await guard(transaction, authorization, scope);

        return this.#execute(
            transaction,
            authorization,
            object,
            name,
            scoped(object, input, scope),
            undefined,
        );
    }

    /** Execute system calls in one transaction, returning each call's result. */
    async executeAsSystem(
        object: ObjectType,
        name: string,
        calls: readonly SystemCall[],
        now: number,
    ): Promise<unknown[]> {
        // build each call as the system, on the served type
        const served = this.served(object);
        const method = (served.methods as Readonly<Record<string, Method>>)[name]!;
        const systems = new Map<string, Promise<SystemAuthorization>>();
        const system = (database: DatabaseConnection, scope: string) => {
            // reuse the authorization per database and scope
            const key = `${database === this.database ? "database" : "transaction"} ${scope}`;
            const known = systems.get(key);
            if (known !== undefined) {
                return known;
            }
            const opened = SystemAuthorization.open(this.authorizer, database, scope, now);
            systems.set(key, opened);

            return opened;
        };
        const calling = async (database: DatabaseConnection, entry: SystemCall) => {
            // identify the target, or the creation's chosen identifier
            const authorization = await system(database, entry.scope);
            const target = entry.target;
            const id = target === undefined ? entry.id : String(target.id);

            return new Call({
                object: served,
                name,
                method,
                scope: entry.scope,
                chain: authorization.chain,
                input: entry.input ?? {},
                ...(id === undefined ? {} : { id }),
                ...(target === undefined ? {} : { target: target as never }),
                database,
                now,
                isPredicted: false,
                authorization,
                objects: this.objects,
                ...(this.#sends === undefined ? {} : { sends: this.#sends }),
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
        };

        // prepare external work
        const prepared: Prepared[] = [];
        try {
            for (const entry of calls) {
                const call = await this.#reserve(await calling(this.database, entry));
                prepared.push(
                    method.prepare === undefined
                        ? undefined
                        : { value: await method.prepare(call), call },
                );
            }
        } catch (error) {
            await this.#settle(prepared, false);
            throw error;
        }

        // execute and audit the calls in one transaction
        let reservation: Reservation | undefined;
        const results: unknown[] = [];
        try {
            reservation = await this.database.transaction(async (transaction) => {
                // guard each scope chain and refuse moved scopes, as pushes do
                for (const scope of new Set(calls.map((entry) => entry.scope))) {
                    await guard(transaction, await system(transaction, scope), scope);
                }

                // execute each call
                for (const [index, entry] of calls.entries()) {
                    const call = await calling(transaction, entry);
                    const work = prepared[index];
                    const result = await method.execute(
                        work === undefined
                            ? call
                            : call.with({
                                  prepared: work.value,
                                  ...(work.call.key === undefined ? {} : { key: work.call.key }),
                              }),
                    );
                    results.push(result);
                    const id = call.id ?? Call.resultId(result);
                    await this.audit(call.scope).record(transaction, served.audit(name), {
                        targets: {
                            [served.auditTarget]:
                                id === undefined
                                    ? { type: "scope", id: call.scope }
                                    : { type: object.name, id },
                        },
                        details: {},
                        outcome: "success",
                    });
                }

                // commit the external work and reserve unique keys
                await this.#commit(transaction, prepared);

                return (
                    this.directory &&
                    Reservation.open(
                        this.directory,
                        transaction,
                        this.objects,
                        crypto.randomUUID(),
                        now,
                    )
                );
            });
        } catch (error) {
            await this.#settle(prepared, false);
            throw error;
        }

        // confirm keys and settle prepared work
        await reservation?.confirm();
        await this.#settle(prepared, true);

        return results;
    }

    /** List the controllers the served objects need. */
    controllers(): readonly Controller[] {
        // compact the log, and pick the controllers the objects need
        const isRecoverable = this.objects.some((object) => object.recoverable !== undefined);
        const isExpiring = this.objects.some((object) => object.expiring !== undefined);
        const isSettled = this.objects.some((object) =>
            Object.values(object.methods as Readonly<Record<string, Method>>).some(
                (method) => method.settle !== undefined,
            ),
        );

        return [
            new CompactionController(this.database),
            ...(this.directory === undefined
                ? []
                : [new ClaimController(this.directory, this.database, this.objects)]),
            ...(isRecoverable ? [recoverable.controller(this)] : []),
            ...(isExpiring ? [expiring.controller(this)] : []),
            ...(this.objects.some((object) => object.addressed !== undefined)
                ? [addressed.controller(this)]
                : []),
            ...(isSettled ? [Settlement.controller(this)] : []),
            ...(this.#sends === undefined || this.runs === undefined
                ? []
                : [this.#sends.controller(RUNS.to(this.runs, this.#report))]),
            ...(this.runs === undefined || this.watches.length === 0
                ? []
                : [new WatchController(this.database, this.watches, this.runs, this.#report)]),
            ...(this.replicas === undefined ? [] : [this.#replicate(this.replicas)]),
            ...this.objects.flatMap((object) =>
                object.controller === undefined ? [] : [this.#control(object, object.controller)],
            ),
        ];
    }

    /** Build the controller a type declares, reconciling or following its pending objects by key. */
    #control(object: ObjectType, declared: ObjectController): Controller {
        // key each pending object by its declared fields
        const table = object.table as Table;
        const match = Condition.compile(declared.pending, table);
        const keyOf = (fields: Readonly<Record<string, unknown>>) => canonicalize(fields);
        const pending = (key: string) => {
            const fields = Object.entries(JSON.parse(key) as Readonly<Record<string, Scalar>>);

            return Condition.all(
                declared.pending,
                ...fields.map(([name, value]) => Condition.eq(name, value)),
            );
        };
        const watches = new Map((declared.watches ?? []).map((watch) => [watch.table, watch]));
        const keys = async (change: Change) => {
            // select the keys of a watched row, or the key of a changed object left pending
            const row = (change.after ?? change.before) as Readonly<Record<string, unknown>>;
            const watch = watches.get(change.table);
            const selected =
                watch !== undefined
                    ? await watch.keys(row, this.database)
                    : change.after !== undefined && Condition.matches(match, row)
                      ? [declared.key?.(row) ?? { id: row.id }]
                      : [];

            return selected.map(keyOf);
        };

        return {
            name: object.name,
            watches: [table, ...watches.keys()],
            ...(declared.concurrency === undefined ? {} : { concurrency: declared.concurrency }),
            ...(declared.mode === undefined ? {} : { mode: declared.mode }),
            keys,
            list: async () => {
                // list the keys of every pending object
                const rows = await Snapshot.live(this.database).rows(table, declared.pending);

                return [
                    ...new Set(rows.map((row) => keyOf(declared.key?.(row) ?? { id: row.id }))),
                ];
            },
            reconcile: async (key, reconciliation) => {
                // reconcile the key's pending objects, if any are left
                const rows = await Snapshot.live(this.database).rows(table, pending(key));
                if (rows.length === 0) {
                    return undefined;
                }
                const now = Date.now();

                return declared.reconcile({
                    rows,
                    now,
                    database: this.database,
                    server: this as ObjectServer,
                    signal: reconciliation.signal,
                    ...(reconciliation.epoch === undefined ? {} : { epoch: reconciliation.epoch }),
                    changed: () => reconciliation.changed(),
                    execute: (method, targets) =>
                        this.executeAsSystem(
                            object,
                            method,
                            targets.map((row) => SystemCall.of(row)),
                            now,
                        ),
                });
            },
        };
    }

    /** Build the controller following each requested copy from its source, one key per request. */
    #replicate(replicas: ObjectReplicas): Controller {
        // list the requests again once the scopes above change
        const keyOf = async (request: Omit<sync.ReplicaRequest, "after">) =>
            `${request.name} ${request.scope} ${await digest(request)}`;

        return {
            name: "replica",
            mode: "follow",
            watches: [Scope.table],
            concurrency: Infinity,
            list: async () => Promise.all((await replicas.requests()).map(keyOf)),
            reconcile: async (key, { signal }) => {
                // find the key's request, waiting for the loop to stop one no longer listed
                const requests = await replicas.requests();
                const keys = await Promise.all(requests.map(keyOf));
                const request = requests[keys.indexOf(key)];
                if (request === undefined) {
                    await until(signal);

                    return undefined;
                }

                // follow the copy from the position it reached for the same request
                const replica = this.authorizer.replicaOf(request);
                await replica.follow(
                    this.database,
                    (after, stream) =>
                        replicas.source.stream(
                            { ...request, ...(after === undefined ? {} : { after }) },
                            stream,
                        ),
                    signal,
                    { request },
                );

                return undefined;
            },
        };
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
            this.#readers.delete(this.#readers.keys().next().value!);
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
            context.caller?.authentication.delegation,
        );
    }

    /** Authorize a principal in a scope, as a follow of its copies does. */
    async authorizeSubject(
        database: DatabaseConnection,
        scope: string,
        subject: Subject,
    ): Promise<Authorization> {
        const bind = (): AccessContext => ({
            subjects: [subject],
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

    /** Wait until the database holds the caller's watermarks in the scope chain. */
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

    /** Read the watermark the scope's log holds now. */
    async watermark(scope: string): Promise<Watermark> {
        return { scope, ...(await this.database.log.position()) };
    }

    /** Wait until the scope's log or a copy reaches a watermark. */
    async #reach(scope: string, watermark: Watermark, signal: AbortSignal): Promise<boolean> {
        try {
            // wait on the scope's own log
            if (watermark.scope === scope) {
                if (watermark.epoch !== (await this.database.log.epoch())) {
                    throw new DatabaseError("STALE_EPOCH", `${scope} holds another epoch`);
                }

                return await this.database.log.wait(watermark.sequence, signal);
            }

            // wait on the copy of an enclosing scope
            return await sync.Replica.reach(this.database, watermark.scope, watermark, signal);
        } catch (error) {
            // report a stale epoch
            if (error instanceof DatabaseError && error.code === "STALE_EPOCH") {
                throw new ServiceError("STALE_EPOCH", {
                    status: 410,
                    message: `${watermark.scope} no longer holds the watermark's history`,
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
        calls: readonly { object: ObjectType; name: string; input: Record<string, unknown> }[],
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

    /** Read the scope a call names in its route field. */
    #scope(object: ObjectType, input: Record<string, unknown>): string {
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
        input: Record<string, unknown>,
        options: Pick<CallOptions, "as" | "expansion"> = {},
    ): Promise<{ call: Call; method: Method; scope: string; targetId: string | undefined }> {
        // refuse changing the rows of a type kept as copies from its home
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
        if (method.mutates && this.database.copies(object.table)) {
            throw new ServiceError("CONFLICT", {
                message: `${object.name} is a copy, which changes at its home`,
            });
        }

        // split off routing fields
        const { field } = object.route;
        const { id, revision, ...rest } = input;
        const fields = Object.fromEntries(Object.entries(rest).filter(([name]) => name !== field));
        const scope = this.#scope(object, input);
        authorization.requireScopeOf(object);
        const targetId = id === undefined ? undefined : schema.string().parse(id);
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
            isPredicted: false,
            authorization,
            objects: this.objects,
            ...(this.#sends === undefined ? {} : { sends: this.#sends }),
            ...(snapshot === undefined ? {} : { snapshot }),
        });

        // load the permitted target at the named revision, through the view a read names
        const target =
            method.target &&
            (method.permission !== null || method.isSystem) &&
            targetId !== undefined
                ? snapshot === undefined
                    ? await authorization.read(call, targetId)
                    : await authorization.readIn(call, targetId, snapshot)
                : undefined;
        if (revision !== undefined && target?.revision !== revision) {
            throw new ServiceError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        // refuse callers' other methods on a trashed object
        const isTrashed =
            object.recoverable !== undefined &&
            target !== undefined &&
            (target as Record<string, unknown>).deletionRequestedAt !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind) && method.isSystem !== true) {
            throw new ServiceError("CONFLICT", { message: `${object.name} is in the trash` });
        }

        // refuse changing a copy
        const isCopy =
            object.addressed !== undefined &&
            target !== undefined &&
            (target as Record<string, unknown>).origin !== null;
        if (isCopy && method.mutates) {
            throw new ServiceError("CONFLICT", {
                message: `${object.name} is a copy, which changes at its origin`,
            });
        }

        return {
            call: target === undefined ? call : call.with({ target: target as never }),
            method,
            scope,
            targetId,
        };
    }

    /** Prepare each call's external work outside the transaction. */
    async #prepare(
        calls: readonly { object: ObjectType; name: string; input: Record<string, unknown> }[],
        scope: string,
        context: ServiceContext,
        request?: RequestIdentity,
    ): Promise<Prepared[]> {
        // skip when nothing prepares or the request executed
        const methods = calls.map(
            (call) => (call.object.methods as Readonly<Record<string, Method>>)[call.name]!,
        );
        if (
            methods.every((method) => method.prepare === undefined) ||
            (request !== undefined &&
                (await this.journal.outcome(this.database, request)) !== undefined)
        ) {
            return calls.map(() => undefined);
        }

        // admit the caller
        const authorization = await this.admit(this.database, scope, context, request?.requestId);
        const prepared: Prepared[] = [];
        try {
            for (const [index, entry] of calls.entries()) {
                // prepare in order
                const method = methods[index]!;
                if (method.prepare === undefined) {
                    prepared.push(undefined);
                    continue;
                }
                const { call } = await this.#call(
                    this.database,
                    authorization,
                    entry.object,
                    entry.name,
                    entry.input,
                );
                await method.authorize?.(call);
                const reserved = await this.#reserve(call);
                prepared.push({ value: await method.prepare(reserved), call: reserved });
            }
        } catch (error) {
            // cancel prepared work
            await this.#settle(prepared, false);
            throw error;
        }

        return prepared;
    }

    /** Key a call's external work and reserve its settlement. */
    async #reserve(call: Call): Promise<Call> {
        // key the external work
        const keyed = call.with({ key: call.method.key?.(call) ?? v7() });

        return call.method.settle === undefined ? keyed : Settlement.reserve(this.database, keyed);
    }

    /** Commit the prepared values of settling calls. */
    async #commit(transaction: DatabaseConnection, prepared: readonly Prepared[]): Promise<void> {
        for (const work of prepared) {
            if (work?.call.method.settle !== undefined) {
                await Settlement.commit(transaction, work.call, work.value);
            }
        }
    }

    /** Confirm or cancel each call's prepared work. */
    async #settle(prepared: readonly Prepared[], isCommitted: boolean): Promise<void> {
        for (const work of prepared) {
            // settle each reserved settlement
            if (work?.call.method.settle === undefined) {
                continue;
            }
            try {
                // skip one claimed elsewhere
                if (
                    (await Settlement.claim(this.database, work.call.key!, Date.now())) ===
                    undefined
                ) {
                    continue;
                }
                await work.call.method.settle!(
                    work.call.with({ prepared: work.value }),
                    work.value,
                    isCommitted,
                );
                await Settlement.forget(this.database, work.call.key!);
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
        input: Record<string, unknown>,
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
        input: Record<string, unknown>,
        context: ServiceContext | undefined,
        options: CallOptions,
    ): Promise<unknown> {
        // build the call on the served type to run its handlers
        const { prepared, from, client } = options;
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

        // run invoked system methods as the system
        let system: Promise<SystemAuthorization> | undefined;
        const called = call.with({
            ...(prepared === undefined ? {} : { prepared: prepared.value }),
            ...(prepared?.call.key === undefined ? {} : { key: prepared.call.key }),
            ...(client === undefined ? {} : { client }),
            run: async (invoked, invokedName, invokedInput, invoking) => {
                // run system methods and system-authority calls as the system
                const isSystem =
                    invoking.authority === "system" ||
                    (invoked.methods as Readonly<Record<string, Method>>)[invokedName]?.isSystem;
                const running = isSystem
                    ? await (system ??= SystemAuthorization.open(
                          this.authorizer,
                          transaction,
                          scope,
                          call.now,
                      ))
                    : authorization;

                return this.#execute(
                    transaction,
                    running,
                    invoked,
                    invokedName,
                    scoped(invoked, invokedInput, scope),
                    context,
                    { from, client, ...(invoking.as === undefined ? {} : { as: invoking.as }) },
                );
            },
        });

        // authorize a caller unless prepared, then execute
        if (prepared === undefined && !(authorization instanceof SystemAuthorization)) {
            await method.authorize?.(called);
        }
        const executed = await method.execute(called);

        // redact and encode returned objects with their texts
        const table = object.table as Table;
        let presented: unknown;
        if (method.result === "value") {
            presented = executed;
        } else if (method.result === "page") {
            const { items, included, ...page } = executed as {
                items: Record<string, unknown>[];
                included?: Record<string, Record<string, unknown>>;
            };
            const { [CHUNKS]: chunks, ...rest } = included ?? {};
            presented = {
                ...page,
                ...(included === undefined ? {} : { included: rest }),
                items: (await authorization.redact(object, items)).map((row) => ({
                    ...encodeRow(table, row),
                    ...(chunks === undefined
                        ? {}
                        : Chunk.text(object, chunks[String(row.id)] as never)),
                })),
            };
        } else {
            const [row] = await authorization.redact(object, [executed as Record<string, unknown>]);
            const texts =
                object.text.length === 0
                    ? undefined
                    : await Chunk.texts(transaction, object, scope, [String(row!.id)]);
            presented = { ...encodeRow(table, row!), ...texts?.get(String(row!.id)) };
        }

        // track changes
        const id = targetId ?? Call.resultId(executed);
        if (method.mutates && object.tracked !== undefined && id !== undefined) {
            const [after] = (await transaction
                .select()
                .from(table)
                .where(eq(table[TABLE].columns.id!, id))) as Record<string, unknown>[];
            if (after !== undefined) {
                await tracked.record(call.with({ id }), call.target, after, from!);
            }
        }

        // audit durable changes
        if (method.mutates && object.storage === "durable") {
            await this.audit(scope, context).record(transaction, object.audit(name), {
                targets: {
                    [object.auditTarget]:
                        id === undefined ? { type: "scope", id: scope } : { type: object.name, id },
                },
                details: {},
                outcome: "success",
            });
        }

        return presented;
    }
}

/** Name a scope in an invoked call's input. */
function scoped(
    object: ObjectType,
    input: Readonly<Record<string, unknown>>,
    scope: string,
): Record<string, unknown> {
    const { field } = object.route;

    return field === undefined ? { ...input } : { ...input, [field]: scope };
}

/** Guard a scope chain against moves for the transaction, and refuse a moved scope. */
async function guard(
    transaction: DatabaseConnection,
    authorization: Authorization,
    scope: string,
): Promise<void> {
    const chain = await Scope.chain(Snapshot.live(transaction), scope);
    await Scope.guard(
        transaction,
        chain.map((link) => link.object.id),
    );
    authorization.requireUnmoved();
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
    const caller = context.subject ?? principalOf(context);

    return caller === undefined ? {} : { caller };
}

/** Build system calls. */
export const SystemCall = {
    /** Call on an existing object's row, in its scope. */
    of(
        row: Readonly<Record<string, unknown>>,
        input?: Readonly<Record<string, unknown>>,
    ): SystemCall {
        return { scope: String(row.scope), target: row, ...(input === undefined ? {} : { input }) };
    },
};

/** The parts of an object server that run system calls. */
export type SystemServer = Pick<ObjectServer, "database" | "executeAsSystem">;

/** One call the system makes. */
export interface SystemCall {
    /** The scope the call acts in. */
    readonly scope: string;
    /** The object the call acts on, absent for a creation or a call on the collection. */
    readonly target?: Readonly<Record<string, unknown>>;
    /** The identifier a creation takes, minted when absent. */
    readonly id?: string;
    /** The method's input. */
    readonly input?: Readonly<Record<string, unknown>>;
}
