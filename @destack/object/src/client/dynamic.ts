import { type DatabaseConnection, TABLE, type TableState } from "@destack/db";
import type { Package } from "@destack/package";
import { Clock, Digest, present, type JsonObject, type JsonValue, schema } from "@destack/schema";
import type { Client } from "@destack/service";
import { createClient, type ClientOptions } from "@destack/service/client";
import { errorOf, refuseInput, ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import {
    EXTERNAL_SHAPE,
    ObjectQuery,
    QUERIES_SHAPE,
    replicaProcedures,
    type OpenQueryOptions,
    type ReplicaProcedures,
} from "../replica/replica.ts";

/** The hex digest digits of a watched query's identity: 64 bits, safe among watches. */
const QUERY_NAME_LENGTH = 16;

/** The local log window of a watch, in milliseconds: a minute, far above a watch's lag. */
const LOCAL_LOG_MILLISECONDS = 60_000;

/** The input field holding the request a pushed call replays under. */
const REQUEST_FIELD = "requestId";

/** The rows a list method answers. */
const Listed = schema.looseObject({
    items: schema.array(schema.record(schema.string(), schema.json())),
});

/** The results of a pushed mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.json());

/** An installation's objects, reached through the service serving them. */
export interface DynamicObjectClientOptions {
    /** The installed release, which every call is made against. */
    readonly package: Package;
    /** The object types the installation serves, as its manifest describes them. */
    readonly objects: readonly ObjectType[];
    /** The scope the calls and watches carry, such as a space. */
    readonly scope: string;
    /** Where and how to reach the service serving the objects. */
    readonly endpoint: ClientOptions;
    /** The local database a watch copies rows into. */
    readonly database: DatabaseConnection;
    /** The clock the local log compacts at. */
    readonly clock: Clock;
}

/** An installation's objects called by name, with types from its manifest's descriptions. */
export class DynamicObjectClient {
    /** The installation the client reaches, and its local database. */
    readonly options: DynamicObjectClientOptions;
    /** The replica procedures of the service serving the objects. */
    readonly #service: Client<ReplicaProcedures>;
    /** The local tables a watch needs, created once. */
    #migrated: Promise<unknown> | undefined;

    /** Reach an installation's objects through the service serving them. */
    constructor(options: DynamicObjectClientOptions) {
        this.options = options;
        this.#service = createClient(
            { package: options.package, router: { replica: replicaProcedures } },
            options.endpoint,
        ).replica;
    }

    /** Report whether an installation serves a type: one of its databases declares its table. */
    static serves(type: ObjectType, states: readonly TableState[]): boolean {
        return states.some((entry) => entry.table.name === type.table[TABLE].sqlName);
    }

    /** Find an object type by name, refusing one the installation does not serve. */
    object(type: string): ObjectType {
        const found = this.options.objects.find((entry) => entry.name === type);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no object type ${type}` });
        }

        return found;
    }

    /** Call a method by name with input validated by its JSON Schema, in the client's scope. */
    async call(type: string, method: string, input: JsonObject): Promise<JsonValue> {
        // find the method, refusing one the type does not declare or keeps internal
        const object = this.object(type);
        const declared = object.methods[method];
        const procedure = object.procedures[method]?.["~orpc"];
        if (declared === undefined || declared.isInternal === true || procedure === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${type} has no method ${method}` });
        }

        // set the scope and a change's request, and validate the input
        const field = object.route.field;
        const scoped = {
            ...(field === undefined ? {} : { [field]: this.options.scope }),
            ...(declared.mutates
                ? { [REQUEST_FIELD]: RequestId.create(this.options.clock.now()) }
                : {}),
            ...input,
        };
        const validated = present(procedure.inputSchema, `the input of ${method}`).safeParse(
            scoped,
        );
        if (!validated.success) {
            throw refuseInput(scoped, validated.error.issues);
        }
        const called = schema.record(schema.string(), schema.json()).parse(validated.data);
        const release = this.options.package.version;
        const scope = this.options.scope;

        // read through the service
        if (!declared.mutates) {
            return this.#service.call({
                scope,
                call: { method: `${type}.${method}`, input: called, release },
            });
        }

        // push a change as a mutation of one call under its request
        const { [REQUEST_FIELD]: request, ...rest } = called;
        const { outcomes } = await this.#service.push({
            scope,
            mutations: [
                {
                    id: schema.string().parse(request),
                    calls: [{ method: `${type}.${method}`, input: rest, release }],
                },
            ],
        });
        const [pushed] = outcomes;
        if (pushed === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `the push of ${type}.${method} answered no outcome`,
            });
        } else if (!("value" in pushed.outcome)) {
            throw errorOf(pushed.outcome.error);
        }
        const [result = null] = CALL_RESULTS.parse(pushed.outcome.value);

        return result;
    }

    /** List a type's rows through its list method, as JSON. */
    async list(
        type: string,
        query: Pick<OpenQueryOptions, "where" | "orderBy" | "limit"> = {},
    ): Promise<readonly JsonObject[]> {
        // find the type's list method
        const method = Object.entries(this.object(type).methods).find(
            ([, declared]) => declared.kind === "list" && declared.isInternal !== true,
        )?.[0];
        if (method === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${type} has no list method` });
        }

        // call it with the query
        const input = schema.record(schema.string(), schema.json()).parse(query);
        const listed = await this.call(type, method, input);

        return Listed.parse(listed).items;
    }

    /** Follow a query's rows into the local database, yielding JSON after each change. */
    async *watch(
        type: string,
        query: Pick<OpenQueryOptions, "where" | "orderBy" | "limit">,
        signal: AbortSignal,
    ): AsyncGenerator<readonly JsonObject[]> {
        // key the query and copy its rows apart from other watches
        const object = this.object(type);
        const { table } = object;
        const { database, scope } = this.options;
        await this.#migrate();
        const followed = ObjectQuery.parse({ object: object.name, ...query });
        const name = (await Digest.json(followed)).slice(0, QUERY_NAME_LENGTH);
        const replica = new sync.Replica({ name, scope, tables: [table] });
        const subscription: sync.Subscription = {
            name,
            shape: object.storage === "external" ? EXTERNAL_SHAPE : QUERIES_SHAPE,
            scope,
            below: scope,
            parameters: { queries: { [name]: schema.json().parse(followed) } },
        };

        // follow the copy, stopping the watch at its first failure
        const stopping = new AbortController();
        const stopped = AbortSignal.any([signal, stopping.signal]);
        const following = this.#follow(replica, subscription, stopped).catch((error: unknown) => {
            stopping.abort();
            throw error;
        });
        const feed = new sync.Feed(database, [table, sync.replicaResult], {
            upstream: replica.upstream(database, undefined),
            clock: this.options.clock,
        });
        try {
            // read the rows once the copy has them, and again after every change
            await Promise.race([this.#copied(replica, name, stopped), following]);
            const compiled: sync.Query = { table, scopes: [scope], ...query };
            for await (const items of feed.watch(name, compiled, stopped)) {
                yield items.map((item) => table[TABLE].encode(item.row));
            }
            await following;
        } finally {
            // stop the follow, which this abort rejects, and drop the copy
            stopping.abort();
            await following.catch(() => {});
            await replica.drop(database);
        }
    }

    /** Create the local tables of the installation's objects and their copies once. */
    #migrate(): Promise<unknown> {
        this.#migrated ??= this.options.database.migrate(
            [...this.options.objects.map((object) => object.table), ...sync.replicaTables],
            { isReplica: true },
        );

        return this.#migrated;
    }

    /** Apply a subscription's pages until aborted, keeping the local log to its window. */
    async #follow(
        replica: sync.Replica,
        subscription: sync.Subscription,
        signal: AbortSignal,
    ): Promise<void> {
        const { database } = this.options;
        await replica.register(database, { clock: this.options.clock });
        try {
            while (!signal.aborted) {
                // stream from where the copy is, and compact the log after each completed run
                const from = await replica.resume(database, subscription);
                const pages = await this.#service.stream({ ...subscription, ...from }, { signal });
                for await (const page of replica.apply(database, pages, {
                    subscription,
                    clock: this.options.clock,
                })) {
                    if (page.complete) {
                        const now = this.options.clock.now();
                        await database.log.compact(now - LOCAL_LOG_MILLISECONDS, now);
                    }
                }
            }
        } catch (error) {
            // end quietly once stopped, failing otherwise
            if (!signal.aborted) {
                throw error;
            }
        }
    }

    /** Wait until the copy completed a run of the given query. */
    async #copied(replica: sync.Replica, name: string, signal: AbortSignal): Promise<void> {
        const { database } = this.options;
        await database.log.until(async () => {
            const queries = (await replica.subscribed(database))?.parameters["queries"];

            return typeof queries === "object" && queries !== null && name in queries;
        }, signal);
    }
}
