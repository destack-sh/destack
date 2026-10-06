import { type DatabaseConnection, Table, TABLE, type TableState } from "@destack/db";
import type { Package } from "@destack/package";
import { Digest, fromJsonSchema, schema, type JsonObject, type JsonValue } from "@destack/schema";
import type { Client } from "@destack/service";
import { createClient, type ClientOptions } from "@destack/service/client";
import { errorOf, refuseInput, ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import * as sync from "@destack/sync";
import type { MethodDescription, ObjectDescription } from "../inspect/object.ts";
import {
    EXTERNAL_SHAPE,
    ObjectQuery,
    QUERIES_SHAPE,
    replicaProcedures,
    type OpenQueryOptions,
    type ReplicaProcedures,
} from "../replica/replica.ts";

/** The hexadecimal digest digits naming a watched query: 64 bits, safe among a client's watches. */
const QUERY_NAME_LENGTH = 16;

/** The local log window of a watch, in milliseconds: a minute, far above a watch's lag. */
const LOCAL_LOG_MILLISECONDS = 60_000;

/** The input field naming the request a pushed call replays under. */
const REQUEST_FIELD = "requestId";

/** The rows a list method answers. */
const Listed = schema.looseObject({
    items: schema.array(schema.record(schema.string(), schema.json())),
});

/** The results of a pushed mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.json());

/** An object type an installation serves, as its manifest describes it, with the table a watch copies its rows into. */
export interface RemoteObject {
    /** The object type as its package's manifest describes it. */
    readonly description: ObjectDescription;
    /** The table a watch copies the type's rows into. */
    readonly table: Table;
}

/** Installed object types paired with the tables their databases declare. */
export const RemoteObject = {
    /** Pair an installed type's description with its table, built from its database's declared state under the type's properties. */
    of(description: ObjectDescription, states: readonly TableState[]): RemoteObject {
        const properties = Object.fromEntries(
            Object.entries(description.columns).map(([property, column]) => [column, property]),
        );

        return {
            description,
            table: Table.describe(RemoteObject.state(description, states), properties),
        };
    },

    /** Report whether an installation serves a type its release declares: whether one of its databases declares the type's table, which a type its package declares for other packages' databases lacks. */
    isServed(description: ObjectDescription, states: readonly TableState[]): boolean {
        return states.some((entry) => entry.table.name === description.table);
    },

    /** Find the declared state of a type's table, refusing a type whose table no database declares. */
    state(description: ObjectDescription, states: readonly TableState[]): TableState {
        const state = states.find((entry) => entry.table.name === description.table);
        if (state === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `no database declares table ${description.table} of ${description.plural}`,
            });
        }

        return state;
    },
};

/** An installation's objects, reached through the service serving them. */
export interface RemoteInstallation {
    /** The installed release, which every call is made against. */
    readonly package: Package;
    /** The object types the installation serves. */
    readonly objects: readonly RemoteObject[];
    /** The scope the calls and watches name, such as a space. */
    readonly scope: string;
    /** Where and how to reach the service serving the objects. */
    readonly endpoint: ClientOptions;
    /** The local database a watch copies rows into. */
    readonly database: DatabaseConnection;
}

/** An installation's objects called by name, as their descriptions declare them, without their object types. */
export class RemoteClient {
    /** The installation the client reaches. */
    readonly installation: RemoteInstallation;
    /** The replica procedures of the service serving the objects. */
    readonly #service: Client<ReplicaProcedures>;
    /** The validators of method inputs, built once per method. */
    readonly #validators = new Map<MethodDescription, schema.Schema>();
    /** The local tables a watch needs, created once. */
    #migrated: Promise<unknown> | undefined;

    /** Reach an installation's objects through the service serving them. */
    constructor(installation: RemoteInstallation) {
        this.installation = installation;
        this.#service = createClient(
            { package: installation.package, router: { replica: replicaProcedures } },
            installation.endpoint,
        ).replica;
    }

    /** Find an object type by name, refusing one the installation does not serve. */
    object(type: string): RemoteObject {
        const found = this.installation.objects.find((entry) => entry.description.name === type);
        if (found === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no object type ${type}` });
        }

        return found;
    }

    /** Call a method by name with its input validated against the method's JSON Schema, in the client's scope. */
    async call(type: string, method: string, input: JsonObject): Promise<JsonValue> {
        // find the method, refusing one the type does not declare
        const { description } = this.object(type);
        const declared = description.methods[method];
        if (declared === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${type} has no method ${method}` });
        }

        // name the scope and a change's request, and validate the input
        const scoped = {
            ...(description.scopeField === undefined
                ? {}
                : { [description.scopeField]: this.installation.scope }),
            ...(declared.mutates ? { [REQUEST_FIELD]: RequestId.create() } : {}),
            ...input,
        };
        const validated = this.#validator(declared).safeParse(scoped);
        if (!validated.success) {
            throw refuseInput(scoped, validated.error.issues);
        }
        const called = schema.record(schema.string(), schema.json()).parse(validated.data);
        const release = this.installation.package.version;
        const scope = this.installation.scope;

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
        const { description } = this.object(type);
        const method = Object.entries(description.methods).find(
            ([, declared]) => declared.kind === "list",
        )?.[0];
        if (method === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `${type} has no list method` });
        }

        // call it with the query
        const input = schema.record(schema.string(), schema.json()).parse(query);
        const listed = await this.call(type, method, input);

        return Listed.parse(listed).items;
    }

    /** Follow a type's rows a query selects into the local database, yielding them as JSON after every change, until the signal aborts. */
    async *watch(
        type: string,
        query: Pick<OpenQueryOptions, "where" | "orderBy" | "limit">,
        signal: AbortSignal,
    ): AsyncGenerator<readonly JsonObject[]> {
        // name the query and copy its rows apart from other watches
        const { description, table } = this.object(type);
        const { database, scope } = this.installation;
        await this.#migrate();
        const followed = ObjectQuery.parse({ object: description.name, ...query });
        const name = (await Digest.json(followed)).slice(0, QUERY_NAME_LENGTH);
        const replica = new sync.Replica({ name, scope, tables: [table] });
        const subscription: sync.Subscription = {
            name,
            shape: description.storage === "external" ? EXTERNAL_SHAPE : QUERIES_SHAPE,
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
            stopping.abort();
            await following.catch(() => {});
            await replica.drop(database);
        }
    }

    /** Create the local tables of the installation's objects and their copies once. */
    #migrate(): Promise<unknown> {
        this.#migrated ??= this.installation.database.migrate(
            [...this.installation.objects.map((entry) => entry.table), ...sync.replicaTables],
            { isReplica: true },
        );

        return this.#migrated;
    }

    /** Apply the service's pages of a subscription until the signal aborts, keeping the local log to its window. */
    async #follow(
        replica: sync.Replica,
        subscription: sync.Subscription,
        signal: AbortSignal,
    ): Promise<void> {
        const { database } = this.installation;
        await replica.register(database);
        try {
            while (!signal.aborted) {
                // stream from where the copy is, and compact the log after each completed run
                const from = await replica.resume(database, subscription);
                const pages = await this.#service.stream({ ...subscription, ...from }, { signal });
                for await (const page of replica.apply(database, pages, { subscription })) {
                    if (page.complete) {
                        await database.log.compact(Date.now() - LOCAL_LOG_MILLISECONDS);
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

    /** Wait until the copy completed a run of the named query. */
    async #copied(replica: sync.Replica, name: string, signal: AbortSignal): Promise<void> {
        const { database } = this.installation;
        await database.log.until(async () => {
            const queries = (await replica.subscribed(database))?.parameters["queries"];

            return typeof queries === "object" && queries !== null && name in queries;
        }, signal);
    }

    /** Build the validator of a method's input from its JSON Schema once. */
    #validator(method: MethodDescription): schema.Schema {
        // take the validator built earlier
        const known = this.#validators.get(method);
        if (known !== undefined) {
            return known;
        }

        // build it from the method's JSON Schema
        const description = schema.record(schema.string(), schema.json()).parse(method.input);
        const built = fromJsonSchema(description);
        this.#validators.set(method, built);

        return built;
    }
}
