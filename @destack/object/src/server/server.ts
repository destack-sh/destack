import {
    Authorizer,
    objectKey,
    principalOf,
    GLOBAL_SCOPE,
    type AccessContext,
    type GrantReader,
    type ObjectReference,
    Policy,
    type Access,
    type TableMapping,
    Scope,
    type ScopeLink,
} from "@destack/access";
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
import { Condition } from "@destack/db/query";
import { DatabaseError } from "@destack/db/error";
import { schema } from "@destack/schema";
import type { Watermark } from "@destack/service/bookmark";
import { Journal, Outcome, type defineJournal } from "@destack/service/database";
import { ServiceError } from "@destack/service/error";
import { RequestFingerprint, type RequestIdentity } from "@destack/service/request";
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
import { Call } from "../method/call.ts";
import type { Method, MethodKind } from "../method/method.ts";
import type { ObjectProcedures, ObjectSchema } from "../method/procedure.ts";
import {
    ClientId,
    replicaProcedures,
    type ObjectQuery,
    type PushResult,
    type ReplicaProcedures,
} from "../replica/replica.ts";
import { ObjectType } from "../object/object.ts";
import { Chunk, CHUNKS } from "../text/chunk.ts";
import { camelCase } from "../object/name.ts";
import { Authorization, SystemAuthorization } from "./authorization.ts";
import type { KeyIndex, Reservation } from "../key/key.ts";
import type { EphemeralStorage } from "./ephemeral.ts";
import { Duration } from "../object/duration.ts";
import { ObjectAudience } from "./audience.ts";
import { tracked } from "../trait/tracked.ts";
import { recoverable } from "../trait/recoverable.ts";
import { expiring } from "../trait/expiring.ts";
import { addressed } from "../trait/addressed.ts";
import type { Controller } from "@destack/service/control";
import { Settlement } from "./settlement.ts";

/** The reserved name of the query following a caller's journal entries. */
const JOURNAL_QUERY = "#journal";

/** The name of the query following a scope's own object. */
const SCOPE_QUERY = "#scope";

/** The recent pages sharing grant readers: 64 readers of a few KiB, well under a megabyte. */
const SHARED_READERS = 64;

/** The largest broadcast event, in bytes: 16 KiB, far above the tens of bytes a cursor takes. */
const BROADCAST_BYTES = 16 * 1024;

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

/** Where a sync starts and what it follows. */
export interface SyncOptions {
    /** The position the subscriber holds, absent before its first snapshot. */
    readonly after?: LogPosition;
    /** The queries to follow by name, every listed object type when absent. */
    readonly queries?: Readonly<Record<string, ObjectQuery>>;
    /** The queries the subscriber followed before. */
    readonly previous?: Readonly<Record<string, ObjectQuery>>;
    /** How often merged pages arrive. */
    readonly refresh?: { readonly every: Duration };
    /** The client following ephemeral objects, absent for durable ones. */
    readonly client?: string;
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
    /** The key index keeping the served objects' unique indexes. */
    readonly index?: KeyIndex;
    /** The memory store holding the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** Derive a request's verified authorization inputs within a scope. */
    readonly #context: (context: ServiceContext, scope: string) => AccessContext;
    /** Open an audit recorder for a scope and request. */
    readonly #audit: (scope: string, context?: ServiceContext) => AuditRecorder<DatabaseConnection>;
    /** Report failed settlements. */
    readonly #report: (error: unknown) => void;
    /** The shared grant readers, by scope and page position. */
    readonly #readers = new Map<string, GrantReader>();
    /** The input schemas of pushed calls, by method. */
    readonly #inputs = new Map<string, schema.Schema>();
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
        /** Derive a request's verified authorization inputs within a scope. */
        readonly context: (context: ServiceContext, scope: string) => AccessContext;
        /** The record of every mutation executed. */
        readonly journal: Journal;
        /** The key index, in another database. */
        readonly index?: KeyIndex;
        /** The memory store holding the served ephemeral objects. */
        readonly ephemeral?: EphemeralStorage;
        /** Open an audit recorder for a scope and request. */
        readonly audit: (
            scope: string,
            context?: ServiceContext,
        ) => AuditRecorder<DatabaseConnection>;
        /** Report failed settlements, thrown when absent. */
        readonly report?: (error: unknown) => void;
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

        // require a separate key index for indexed objects
        this.#routed = Object.values(options.objects);
        this.objects = ObjectType.served(this.#routed);
        const indexed = this.objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.index === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but no key index keeps them`,
            );
        } else if (options.index?.database === options.database) {
            throw new TypeError(
                "the key index keeps indexes across databases, apart from the objects'",
            );
        }
        if (options.index !== undefined) {
            this.index = options.index;
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

        // build the authorizer and the feed
        const others = options.policies ?? [];
        this.database = options.database;
        this.authorizer = new Authorizer(
            [
                ...this.objects.flatMap((object) => [
                    object.policy,
                    ...object.scopes.map((scope) => scope.policy),
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
        );
        tracked.require(this.objects, this.authorizer);
        this.journal = options.journal;
        this.#context = options.context;
        this.#audit = options.audit;
        this.#report =
            options.report ??
            ((error) => {
                throw error;
            });
        this.feed = new sync.Feed(options.database, [
            ...new Set(
                this.objects
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
            /** Open an audit recorder for a scope and request. */
            readonly audit: (
                scope: string,
                context?: ServiceContext,
            ) => AuditRecorder<DatabaseConnection>;
            /** The key index keeping the objects' unique indexes. */
            readonly index?: KeyIndex;
            /** The memory store holding the ephemeral objects. */
            readonly ephemeral?: EphemeralStorage;
        },
    ): ServiceImplementation {
        // serve the service's objects
        const objects = new ObjectServer({
            objects: service.objects as Readonly<Record<string, ObjectType>>,
            database: options.database,
            context: (context: ServiceContext) => context.access(),
            audit: options.audit,
            journal: new Journal(options.journal),
            ...(options.index === undefined ? {} : { index: options.index }),
            ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
        });

        return {
            service,
            access: objects.access,
            audit: AuditRecorder.procedure(({ context }) =>
                options.audit(context.scope ?? GLOBAL_SCOPE, context),
            ),
            controllers: objects.controllers(),
            router: objects.router(),
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

                return this.sync(scope, context, {
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
                await this.broadcast(input.scope, input.object, input.event, context);

                return {};
            }),
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
        await this.#enter(context, scope);
        const isEphemeral = object.storage === "ephemeral";
        const database = isEphemeral ? this.#store().database : this.database;
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

                    return this.#execute(
                        transaction,
                        authorization,
                        object,
                        name,
                        input,
                        context,
                        prepared,
                    );
                },
                { isReadOnly: true },
            );

        // read unaudited objects directly
        if (!object.isReadAudited && method.audited !== true) {
            return read();
        }
        // record the read of one object
        else if (method.target) {
            const target = {
                [object.auditTarget]: { type: object.name, id: schema.string().parse(input.id) },
            };
            const details = method.audit?.details;

            return this.#audit(scope, context).attempt(
                object.audit(name),
                { targets: target, details: {} },
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
        // record the read of the scope's collection
        else {
            return this.#audit(scope, context).attempt(
                object.audit(name, "collection"),
                collection(object, scope),
                read,
            );
        }
    }

    /** Execute a mutation's calls in one transaction, returning each call's result. */
    async mutate(
        mutation: sync.Mutation,
        context: ServiceContext,
        client?: string,
    ): Promise<unknown[]> {
        // resolve each call and mint prepared creations' identifiers
        const calls = mutation.calls.map((entry) => {
            const call = this.#resolve(entry, true);
            const method = (call.object.methods as Readonly<Record<string, Method>>)[call.name]!;

            return method.kind === "create" &&
                method.prepare !== undefined &&
                call.input.id === undefined
                ? { ...call, input: { ...call.input, id: `${call.object.identity}-${v7()}` } }
                : call;
        });
        const scope = this.#scope(calls[0]!.object, calls[0]!.input);
        if (calls.some((call) => this.#scope(call.object, call.input) !== scope)) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation's calls act in one scope",
            });
        }
        await this.#enter(context, scope);

        // write ephemeral objects apart
        const ephemeral = calls.filter((call) => call.object.storage === "ephemeral").length;
        if (ephemeral === calls.length) {
            return this.#mutateEphemeral(calls, scope, context, client);
        } else if (ephemeral > 0) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a mutation writes durable or ephemeral objects, not both",
            });
        }

        // identify the mutation by caller, scope and request identifier
        const request = {
            caller: context.requireCaller().id,
            scope,
            requestId: mutation.id,
        };
        const fingerprint = await RequestFingerprint.hash(schema.json(), {
            id: mutation.id,
            calls: mutation.calls.map((entry) => ({
                method: entry.method,
                input: schema.redact(this.#inputs.get(entry.method)!, entry.input),
            })),
        });

        // prepare external work
        const prepared = await this.#prepare(calls, scope, context, request);

        // execute through the journal
        let authorization: Authorization | undefined;
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
                        executed.push(
                            await this.#execute(
                                transaction,
                                authorization!,
                                call.object,
                                call.name,
                                call.input,
                                context,
                                prepared[index],
                                from,
                            ),
                        );
                    }

                    // commit prepared work
                    await this.#commit(transaction, prepared);

                    // release request-bound relationships
                    await this.authorizer.release(transaction, mutation.id);

                    // reserve written keys
                    reservation = await this.index?.reserve(
                        transaction,
                        this.objects,
                        mutation.id,
                        authorization!.access.context.now,
                    );

                    return executed;
                },
            })) as unknown[];
        } catch (error) {
            // release keys and cancel prepared work
            await this.index?.release(mutation.id).catch((failure: unknown) => {
                throw new AggregateError([error, failure], "mutation and its key release failed");
            });
            await this.#settle(prepared, false);
            throw error;
        }

        // confirm keys and settle prepared work
        if (reservation !== undefined) {
            await this.index!.confirm(reservation);
        }
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
        for (const mutation of mutations) {
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

    /** Follow queries of a scope's objects from a resumed position. */
    sync(
        scope: string,
        context: ServiceContext,
        options: SyncOptions = {},
    ): AsyncGenerator<sync.QueryPage> {
        return options.client === undefined
            ? this.#syncDurable(scope, context, options)
            : this.#syncEphemeral(scope, context, options.client, options);
    }

    /** Follow queries of a scope's durable objects with the caller's journal entries. */
    async *#syncDurable(
        scope: string,
        context: ServiceContext,
        { after, queries, previous, refresh }: SyncOptions,
    ): AsyncGenerator<sync.QueryPage> {
        // enter and open the audience
        await this.#enter(context, scope);
        const audience = await ObjectAudience.open(this, scope, context);

        // follow the caller's journal
        const caller = context.caller?.id;
        const journal: Record<string, sync.Query> =
            caller === undefined
                ? {}
                : {
                      [JOURNAL_QUERY]: {
                          table: this.journal.table,
                          scopes: [scope],
                          where: Condition.eq("caller", caller),
                      },
                  };

        // follow the scope's own object
        const scoped = this.#scoped(audience.scope);
        const compiled = {
            ...ObjectType.queries(this.#durable, queries, scope),
            ...scoped,
            ...journal,
        };
        const earlier =
            previous === undefined
                ? undefined
                : { ...ObjectType.queries(this.#durable, previous, scope), ...scoped, ...journal };
        const table = this.journal.table[TABLE].sqlName;
        const follow = async function* (feed: sync.Feed): AsyncGenerator<sync.QueryPage> {
            for await (const page of feed.subscribe(compiled, after, context.signal, {
                audience,
                ...(earlier === undefined ? {} : { previous: earlier }),
                ...(refresh === undefined ? {} : { every: Duration.milliseconds(refresh.every) }),
            })) {
                yield withOutcomes(page, table);
            }
        };

        // audit watching each audited object type
        const tables = new Set(Object.values(compiled).flatMap(tablesOf));
        const audited = this.objects.filter(
            (object) => object.isReadAudited && tables.has(object.table),
        );
        const pages = audited.reduce(
            (source, object) => () =>
                this.#audit(scope, context).stream(
                    object.audit("watch", "collection"),
                    collection(object, scope),
                    source,
                ),
            () => follow(this.feed),
        );
        try {
            yield* pages();
        } catch (error) {
            // report capacity failures as service failures
            throw serviceFailure(error);
        }
    }

    /** Follow queries of a scope's ephemeral objects as a client, restarting when access changes. */
    async *#syncEphemeral(
        scope: string,
        context: ServiceContext,
        client: string,
        { after, queries, previous, refresh }: SyncOptions,
    ): AsyncGenerator<sync.QueryPage> {
        // enter and hold the client's rows
        const store = this.#store();
        await this.#enter(context, scope);
        const release = store.hold(this.#clientKey(context, client));
        const compiled = ObjectType.queries(store.objects, queries, scope);
        let earlier =
            previous === undefined ? undefined : ObjectType.queries(store.objects, previous, scope);
        try {
            let position = after;
            while (!context.signal.aborted) {
                // open the audience and watch access rows
                const revised = new AbortController();
                const signal = AbortSignal.any([context.signal, revised.signal]);
                const since = (await this.database.log.position()).sequence;
                const audience = await ObjectAudience.open(this, scope, context, "ephemeral");
                const { authorization, chain } = audience;
                const tables = this.authorizer.watch(chain).map((watch) => watch.table);
                let failure: unknown;
                const watching = (async () => {
                    for await (const _ of this.database.log.follow(
                        { tables, scopes: chain, after: since },
                        signal,
                    )) {
                        revised.abort();
                    }
                })().catch((error: unknown) => {
                    failure = error;
                    revised.abort();
                });

                // follow until access changes
                const pages = store.feed.subscribe(compiled, position, signal, {
                    audience,
                    ...(earlier === undefined ? {} : { previous: earlier }),
                    ...(refresh === undefined
                        ? {}
                        : { every: Duration.milliseconds(refresh.every) }),
                });
                yield* this.#interleave(pages, scope, authorization, signal);
                revised.abort();
                await watching;
                if (failure !== undefined) {
                    throw failure;
                }
                position = undefined;
                earlier = undefined;
            }
        } catch (error) {
            // report capacity failures as service failures
            throw serviceFailure(error);
        } finally {
            release();
        }
    }

    /** Send an unstored event to an object's readers. */
    async broadcast(
        scope: string,
        target: Omit<ObjectReference, "scope">,
        event: JsonValue,
        context: ServiceContext,
    ): Promise<void> {
        // require a small event and a reader
        const store = this.#store();
        if (new TextEncoder().encode(JSON.stringify(event)).length > BROADCAST_BYTES) {
            throw new ServiceError("BAD_REQUEST", {
                message: `a broadcast holds at most ${BROADCAST_BYTES} bytes`,
            });
        }
        await this.#enter(context, scope);
        const reference = { ...target, scope };
        const authorization = await this.admit(this.database, scope, context);
        if (!(await this.#lists(authorization, reference))) {
            throw new ServiceError("NOT_FOUND", { message: `${target.type} not found` });
        }

        // send it to the scope's streams
        store.tracker.broadcast(scope, { ...reference, event });
    }

    /** Execute system calls in one transaction, returning each call's result. */
    async executeAsSystem(
        object: ObjectType,
        name: string,
        calls: readonly SystemCall[],
        now: number,
    ): Promise<unknown[]> {
        // build each call as the system
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
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
            const authorization = await system(database, entry.scope);
            const target = entry.target;

            return new Call({
                object,
                name,
                method,
                scope: entry.scope,
                input: entry.input ?? {},
                ...(target === undefined ? {} : { id: String(target.id), target: target as never }),
                database,
                now,
                isPredicted: false,
                authorization,
                objects: this.objects,
                run: (invoked, invokedName, invokedInput) =>
                    this.#execute(
                        database,
                        authorization,
                        invoked,
                        invokedName,
                        scoped(invoked, invokedInput, entry.scope),
                        undefined,
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
                    await this.#audit(call.scope).record(transaction, object.audit(name), {
                        targets: {
                            [object.auditTarget]:
                                id === undefined
                                    ? { type: "scope", id: call.scope }
                                    : { type: object.name, id },
                        },
                        details: {},
                        outcome: "success",
                    });
                }

                await this.#commit(transaction, prepared);

                return this.index?.reserve(transaction, this.objects, crypto.randomUUID(), now);
            });
        } catch (error) {
            await this.#settle(prepared, false);
            throw error;
        }

        // confirm keys and settle prepared work
        if (reservation !== undefined) {
            await this.index!.confirm(reservation);
        }
        await this.#settle(prepared, true);

        return results;
    }

    /** List the controllers the served objects need. */
    controllers(): readonly Controller[] {
        // pick the needed controllers
        const isRecoverable = this.objects.some((object) => object.recoverable !== undefined);
        const isExpiring = this.objects.some((object) => object.expiring !== undefined);
        const isSettled = this.objects.some((object) =>
            Object.values(object.methods as Readonly<Record<string, Method>>).some(
                (method) => method.settle !== undefined,
            ),
        );

        return [
            ...(this.index === undefined ? [] : [this.index.controller(this)]),
            ...(isRecoverable ? [recoverable.controller(this)] : []),
            ...(isExpiring ? [expiring.controller(this)] : []),
            ...(this.objects.some((object) => object.addressed !== undefined)
                ? [addressed.controller(this)]
                : []),
            ...(isSettled ? [Settlement.controller(this)] : []),
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
            const caller = this.#context(context, bound);

            return request === undefined ? caller : { ...caller, request };
        };
        const access = await this.authorizer.resolve(
            Snapshot.live(database),
            scope,
            bind(scope),
            links,
        );

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
    async #enter(context: ServiceContext, scope: string): Promise<void> {
        // require the pinned scope
        if (context.scope !== undefined && context.scope !== scope) {
            throw new ServiceError("NOT_FOUND");
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

    /** Decide whether a caller may list a served durable object. */
    async #lists(authorization: Authorization, reference: ObjectReference): Promise<boolean> {
        const object = this.objects.find(
            (entry) =>
                entry.storage === "durable" &&
                entry.name === reference.type &&
                entry.policy.definition.packageId === reference.packageId,
        );
        const permission = object?.listing;

        return (
            permission !== undefined && (await authorization.check(permission, reference)).isAllowed
        );
    }

    /** Key a client by its caller and identifier. */
    #clientKey(context: ServiceContext, client: string): string {
        return JSON.stringify([context.caller?.id ?? null, client]);
    }

    /** The served durable object types. */
    get #durable(): ObjectType[] {
        return this.objects.filter((object) => object.storage === "durable");
    }

    /** Read the ephemeral store, failing without one. */
    #store(): EphemeralStorage {
        if (this.ephemeral === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: "no ephemeral objects are served here",
            });
        }

        return this.ephemeral;
    }

    /** Interleave a stream's pages with the readable objects' events. */
    async *#interleave(
        pages: AsyncGenerator<sync.QueryPage>,
        scope: string,
        authorization: Authorization,
        signal: AbortSignal,
    ): AsyncGenerator<sync.QueryPage> {
        // collect the scope's events
        const received: JsonValue[] = [];
        const readable = new Map<string, boolean>();
        let wake = () => {};
        let stop = () => {};
        let last: sync.QueryPage | undefined;
        let next = pages.next();
        try {
            while (!signal.aborted) {
                // yield the next page unless events wait
                const arrived =
                    received.length > 0
                        ? undefined
                        : await Promise.race([
                              next,
                              new Promise<undefined>(
                                  (resolve) => (wake = () => resolve(undefined)),
                              ),
                          ]);
                if (arrived !== undefined) {
                    if (arrived.done) {
                        return;
                    }
                    if (last === undefined) {
                        stop = this.#store().tracker.listen(scope, (event) => {
                            received.push(event);
                            wake();
                        });
                    }
                    last = arrived.value;
                    next = pages.next();
                    yield arrived.value;
                    continue;
                }

                // yield the readable events
                const broadcasts: sync.Broadcast[] = [];
                for (const sent of received.splice(0)) {
                    const { event, ...reference } = sent as ObjectReference & { event: JsonValue };
                    const topic = objectKey(reference);
                    let isRead = readable.get(topic);
                    if (isRead === undefined) {
                        isRead = await this.#lists(authorization, reference);
                        readable.set(topic, isRead);
                    }
                    if (isRead) {
                        broadcasts.push({ topic, event });
                    }
                }
                if (broadcasts.length > 0 && last !== undefined) {
                    yield {
                        reset: false,
                        complete: true,
                        changes: [],
                        position: last.position,
                        broadcasts,
                    };
                }
            }
        } finally {
            stop();
            await pages.return(undefined);
        }
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

        return this.#store().write(this.#clientKey(context, client), async (transaction) => {
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
                        undefined,
                        undefined,
                        client,
                    ),
                );
            }

            return executed;
        });
    }

    /** Resolve a recorded call's object type and method and validate its input. */
    #resolve(
        entry: sync.Call,
        mutates: boolean,
    ): {
        readonly object: ObjectType;
        readonly name: string;
        readonly input: Record<string, unknown>;
    } {
        // find the object type and method
        const separator = entry.method.lastIndexOf(".");
        const served = this.#schemas.get(entry.method.slice(0, separator));
        const name = entry.method.slice(separator + 1);
        const method = (served?.object.methods as Readonly<Record<string, Method>> | undefined)?.[
            name
        ];
        if (!served || method === undefined || (mutates && !method.mutates)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `no ${mutates ? "mutating " : ""}method ${entry.method}`,
            });
        } else if (!mutates && method.mutates) {
            throw new ServiceError("BAD_REQUEST", {
                message: `${entry.method} changes objects, so a client pushes it`,
            });
        }

        // validate the input
        let input = this.#inputs.get(entry.method);
        if (input === undefined) {
            const procedure = method.procedure(name, served.schema).input as schema.Object<
                Record<string, schema.Schema>
            >;
            const replay =
                served.object.storage === "ephemeral" ? { client: true } : { requestId: true };
            input = mutates ? procedure.omit(replay as never) : procedure;
            this.#inputs.set(entry.method, input);
        }
        const parsed = input.safeParse(Call.upcast(served.object, name, entry));
        if (!parsed.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: `invalid input to ${entry.method}`,
                data: { issues: parsed.error.issues },
            });
        }

        return { object: served.object, name, input: parsed.data as Record<string, unknown> };
    }

    /** Compile the query of a scope's own object. */
    #scoped(own: ObjectReference | undefined): Record<string, sync.Query> {
        const object =
            own === undefined ? undefined : this.objects.find((entry) => entry.policy.is(own));

        return own === undefined || object?.listing === undefined
            ? {}
            : {
                  [SCOPE_QUERY]: {
                      ...object.query({ where: Condition.eq("id", own.id) }, this.objects),
                      scopes: [own.scope],
                  },
              };
    }

    /** Read the scope a call names in its route field. */
    #scope(object: ObjectType, input: Record<string, unknown>): string {
        // read the field, or the global scope
        const { field } = object.route;
        const named = field === undefined ? GLOBAL_SCOPE : input[field];
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
    ): Promise<{ call: Call; method: Method; scope: string; targetId: string | undefined }> {
        // split off routing fields
        const method = (object.methods as Readonly<Record<string, Method>>)[name]!;
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
            input: fields,
            ...(targetId === undefined ? {} : { id: targetId }),
            database,
            ...callerOf(authorization),
            now: authorization.access.context.now,
            isPredicted: false,
            authorization,
            objects: this.objects,
        });

        // load the permitted target at the named revision
        const target =
            method.target &&
            (method.permission !== null || method.isSystem) &&
            targetId !== undefined
                ? await authorization.read(call, targetId)
                : undefined;
        if (revision !== undefined && target?.revision !== revision) {
            throw new ServiceError("CONFLICT", { message: `${object.name} revision has changed` });
        }

        // refuse other methods on a trashed object
        const isTrashed =
            object.recoverable !== undefined &&
            target !== undefined &&
            (target as Record<string, unknown>).deletionRequestedAt !== null;
        if (isTrashed && !TRASH_METHODS.has(method.kind)) {
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

    /** Execute, redact and audit one call. */
    async #execute(
        transaction: DatabaseConnection,
        authorization: Authorization,
        object: ObjectType,
        name: string,
        input: Record<string, unknown>,
        context: ServiceContext | undefined,
        prepared: Prepared,
        from?: LogPosition,
        client?: string,
    ): Promise<unknown> {
        // build the call
        const { call, method, scope, targetId } = await this.#call(
            transaction,
            authorization,
            object,
            name,
            input,
        );
        // run invoked system methods as the system
        let system: Promise<SystemAuthorization> | undefined;
        const called = call.with({
            ...(prepared === undefined ? {} : { prepared: prepared.value }),
            ...(prepared?.call.key === undefined ? {} : { key: prepared.call.key }),
            ...(client === undefined ? {} : { client }),
            run: async (invoked, invokedName, invokedInput) =>
                this.#execute(
                    transaction,
                    (invoked.methods as Readonly<Record<string, Method>>)[invokedName]?.isSystem
                        ? await (system ??= SystemAuthorization.open(
                              this.authorizer,
                              transaction,
                              scope,
                              call.now,
                          ))
                        : authorization,
                    invoked,
                    invokedName,
                    scoped(invoked, invokedInput, scope),
                    context,
                    undefined,
                    from,
                    client,
                ),
        });

        // authorize unless prepared, then execute
        if (prepared === undefined) {
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
            await this.#audit(scope, context).record(transaction, object.audit(name), {
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

/** Convert capacity failures to service failures. */
function serviceFailure(error: unknown): unknown {
    return error instanceof sync.SyncError
        ? new ServiceError(
              error.code === "OVERLOADED" ? "SERVICE_UNAVAILABLE" : "UNPROCESSABLE_CONTENT",
              { message: error.message },
          )
        : error;
}

/** Name a type's objects in a scope as an audit collection target. */
function collection(object: ObjectType, scope: string) {
    return { targets: { collection: { type: object.plural, id: scope } }, details: {} };
}

/** List the tables a query reads. */
function tablesOf(query: sync.Query | sync.Include | sync.Relation): Table[] {
    return [
        query.table,
        ...Object.values(("include" in query ? query.include : undefined) ?? {}).flatMap(tablesOf),
        ...Object.values(query.relations ?? {}).flatMap(tablesOf),
    ];
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

/** Replace a page's journal rows with mutation outcomes. */
function withOutcomes(page: sync.QueryPage, journal: string): sync.QueryPage {
    // move journal changes to outcomes
    const outcomes = page.changes.filter((change) => change.table === journal).flatMap(settled);
    const settledPage: sync.QueryPage = {
        ...page,
        changes: page.changes.filter((change) => change.table !== journal),
        ...(outcomes.length === 0 ? {} : { outcomes: [...(page.outcomes ?? []), ...outcomes] }),
    };

    return settledPage.complete
        ? withEventMeta(settledPage, {
              id: `${settledPage.position.epoch}/${settledPage.position.sequence}`,
          })
        : settledPage;
}

/** Read the mutation outcome a journal change records. */
function settled(change: sync.RowChange): sync.MutationOutcome[] {
    // skip removed entries and claims without an outcome
    if (
        change.operation === "delete" ||
        change.row.outcome === null ||
        change.row.outcome === undefined
    ) {
        return [];
    }

    // read the outcome
    const outcome = Outcome.parse(change.row.outcome);
    const id = schema.string().parse(change.row.requestId);

    return "error" in outcome ? [{ id, error: outcome.error }] : [{ id }];
}

/** Build system calls. */
export const SystemCall = {
    /** Call on an existing object's row, in its scope. */
    of(row: Readonly<Record<string, unknown>>): SystemCall {
        return { scope: String(row.scope), target: row };
    },
};

/** One call the system makes. */
export interface SystemCall {
    /** The scope the call acts in. */
    readonly scope: string;
    /** The object the call acts on, absent for a creation or a call on the collection. */
    readonly target?: Readonly<Record<string, unknown>>;
    /** The method's input. */
    readonly input?: Readonly<Record<string, unknown>>;
}
