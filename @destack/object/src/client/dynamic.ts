import { Failure } from "@destack/error";
import { ObjectError } from "../error/error.ts";
import { type DatabaseConnection, TABLE, type TableState } from "@destack/db";
import { Digest, present, type JsonObject, type JsonValue, schema } from "@destack/schema";
import type { Client } from "../procedure/index.ts";
import type { ClientOptions } from "./package.ts";
import { refuseInput } from "../error/error.ts";
import { RequestId } from "@destack/sync";
import * as sync from "@destack/sync";
import { ObjectDescription } from "../object/description.ts";
import { ObjectClient } from "./client.ts";
import type { ObjectType } from "../object/object.ts";
import {
    EXTERNAL_PUBLICATION,
    ObjectQuery,
    QUERIES_PUBLICATION,
    type StoreProcedures,
} from "../replica/replica.ts";
import type { Environment } from "@destack/environment";

import type {} from "@destack/package/import-meta";
import { Method } from "../method/method.ts";

/** The hex digest digits of a watched query's identity: 64 bits, safe among watches. */
const QUERY_NAME_LENGTH = 16;

/** The local log window of a watch, in milliseconds: a minute, far above a watch's lag. */
const LOCAL_LOG_MILLISECONDS = 60_000;

/** The input field with the request a pushed call replays under. */
const REQUEST_FIELD = "requestId";

/** The rows a list method answers. */
const Listed = schema.looseObject({
    items: schema.array(schema.jsonObject()),
});

/** The results of a pushed mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.json());

/** An installation's objects, called through the package serving them. */
export interface DynamicObjectClientOptions {
    /** The object types the installation serves, as its manifest describes them. */
    readonly objects: readonly ObjectType[];
    /** The scope of the calls and watches, such as a space. */
    readonly scope: string;
    /** Where and how to call the package serving the objects. */
    readonly endpoint: ClientOptions;
    /** Where and how to call the home of another scope a change acts in, such as a space's. */
    readonly homes?: (scope: string) => ClientOptions;
    /** The environment the client runs in, whose clock the local log compacts at. */
    readonly environment: Environment;
}

/** An installation's objects called by name, with types from its manifest's descriptions. */
export class DynamicObjectClient {
    /** The installation the client calls, and its local database. */
    readonly options: DynamicObjectClientOptions;
    /** The store procedures of the package serving the objects. */
    readonly #store: Client<StoreProcedures>;
    /** Call the store procedures of a scope's home, the client's store for its scope. */
    readonly #home: (scope: string) => Client<StoreProcedures>;
    /** The local tables each watch's database has, created once per database. */
    readonly #migrated = new WeakMap<DatabaseConnection, Promise<unknown>>();
    /** When compacting each database's local log is next due. */
    readonly #compactions = new WeakMap<DatabaseConnection, number>();

    /** Connect to an installation's objects through the package serving them. */
    constructor(options: DynamicObjectClientOptions) {
        this.options = options;
        this.#store = ObjectClient.store(options.endpoint, options.environment);
        this.#home = ObjectClient.homes(
            options.scope,
            () => this.#store,
            options.homes,
            options.environment,
        );
    }

    /** Report whether an installation serves a type: one of its databases declares its table. */
    static serves(type: ObjectType, states: readonly TableState[]): boolean {
        return states.some((entry) => entry.table.name === type.table[TABLE].sqlName);
    }

    /** Find an object type by name, refusing one the installation does not serve. */
    object(type: string): ObjectType {
        const found = this.options.objects.find((entry) => entry.name === type);
        if (found === undefined) {
            throw ObjectError.of("TYPE_UNKNOWN", `unknown object type ${type}`);
        }

        return found;
    }

    /** Call a method by name with input validated by its JSON Schema, in the scope it acts in. */
    async call(type: string, method: string, input: JsonObject): Promise<JsonValue> {
        // find the method, refusing one the type does not declare or keeps from clients
        const object = this.object(type);
        const declared = object.methods[method];
        const procedure = object.procedures[method]?.["~orpc"];
        if (declared === undefined || !Method.isRouted(declared) || procedure === undefined) {
            throw ObjectError.of("METHOD_UNKNOWN", `${type} has no method ${method}`);
        }

        // set the scope and a change's request, and validate the input
        const field = object.route.field;
        const scoped = {
            ...(field === undefined ? {} : { [field]: this.options.scope }),
            ...(declared.mutates
                ? { [REQUEST_FIELD]: RequestId.create(this.options.environment) }
                : {}),
            ...input,
        };
        const validated = present(procedure.inputSchema, `the input of ${method}`).safeParse(
            scoped,
        );
        if (!validated.success) {
            throw refuseInput(scoped, validated.error.issues);
        }
        const called = schema.jsonObject().parse(validated.data);

        // read through the store procedures, in the scope the call reads
        if (!declared.mutates) {
            return this.#store.call({
                scope: object.readScope(method, called),
                call: object.call(method, called),
            });
        }

        // push a change as a mutation of one call under its request, at the home of the scope it acts in
        const { [REQUEST_FIELD]: request, ...rest } = called;
        const home = object.readScope(method, rest);
        const { outcomes } = await this.#home(home).push({
            scope: home,
            mutations: [
                {
                    id: schema.string().parse(request),
                    calls: [object.call(method, rest)],
                },
            ],
        });
        const [pushed] = outcomes;
        if (pushed === undefined) {
            throw ObjectError.of(
                "OUTCOME_MISSING",
                `the push of ${type}.${method} returned no outcome`,
            );
        } else if (!("value" in pushed.outcome)) {
            throw Failure.error(pushed.outcome.error);
        }
        const [result = null] = CALL_RESULTS.parse(pushed.outcome.value);

        return result;
    }

    /** Describe object types the store knows, every served one when given none. */
    async describe(types?: readonly sync.ObjectTypeReference[]): Promise<ObjectDescription[]> {
        // ask nothing for an empty list
        if (types?.length === 0) {
            return [];
        }

        // describe them at the package
        const { objects } = await this.#store.describe(
            types === undefined ? {} : { types: [...types] },
        );

        return objects.map((object) => ObjectDescription.parse(object));
    }

    /** List a type's rows through its list method, as JSON. */
    async list(
        type: string,
        query: Pick<ObjectQuery, "where" | "orderBy" | "limit"> = {},
    ): Promise<readonly JsonObject[]> {
        // find the type's list method
        const method = Object.entries(this.object(type).methods).find(
            ([, declared]) => declared.kind === "list" && Method.isRouted(declared),
        )?.[0];
        if (method === undefined) {
            throw ObjectError.of("LIST_METHOD_MISSING", `${type} has no list method`);
        }

        // call it with the query
        const input = schema.jsonObject().parse(query);
        const listed = await this.call(type, method, input);

        return Listed.parse(listed).items;
    }

    /** Follow a query's rows into a local database, yielding JSON after each change. */
    async *watch(
        type: string,
        query: Pick<ObjectQuery, "where" | "orderBy" | "limit">,
        database: DatabaseConnection,
        signal: AbortSignal,
    ): AsyncGenerator<readonly JsonObject[]> {
        // key the query and replica its rows apart from other watches
        const object = this.object(type);
        const { table } = object;
        const { scope } = this.options;
        await this.#migrate(database);
        const followed = ObjectQuery.parse({
            object: object.name,
            package: object.package.id,
            ...query,
        });
        const name = (await Digest.json(followed, database.state.environment.crypto)).slice(
            0,
            QUERY_NAME_LENGTH,
        );
        const replica = new sync.Replica({ name, scope, tables: [table] });
        const subscription: sync.Subscription = {
            name,
            publication: object.storage === "external" ? EXTERNAL_PUBLICATION : QUERIES_PUBLICATION,
            scope,
            target: scope,
            parameters: { queries: { [name]: schema.json().parse(followed) } },
            revision: await database.log.position(),
        };

        // follow the replica, stopping the watch at its first failure
        const stopping = new AbortController();
        const stopped = AbortSignal.any([signal, stopping.signal]);
        const following = this.#follow(database, replica, subscription, stopped).catch(
            (error: unknown) => {
                stopping.abort();
                throw error;
            },
        );
        const feed = new sync.Feed(database, [table, sync.replicaResult], {
            upstream: replica.upstream(database, undefined),
        });
        try {
            // read the rows once the replica has them, and again after every change
            await Promise.race([this.#replicated(database, replica, name, stopped), following]);
            const compiled: sync.Query = { table, scopes: [scope], ...query };
            for await (const items of feed.watch(name, compiled, stopped)) {
                yield items.map((item) => table[TABLE].encode(item.row));
            }
            await following;
        } finally {
            // stop the follow and drop the replica
            stopping.abort();
            await following.catch(() => {});
            await sync.Replica.drop(database, replica);
        }
    }

    /** Create the local tables of the installation's objects and replicas once per database. */
    #migrate(database: DatabaseConnection): Promise<unknown> {
        // migrate a database the first time a watch replicas into it
        let migrated = this.#migrated.get(database);
        if (migrated === undefined) {
            const tables = this.options.objects.map((object) => object.table);
            migrated = database.migrate([...tables, ...sync.replicaTables], { isReplica: true });
            this.#migrated.set(database, migrated);
        }

        return migrated;
    }

    /** Apply a subscription's pages until aborted, bounding the local log to its window. */
    async #follow(
        database: DatabaseConnection,
        replica: sync.Replica,
        subscription: sync.Subscription,
        signal: AbortSignal,
    ): Promise<void> {
        await replica.register(database);
        try {
            while (!signal.aborted) {
                // stream from where the replica is, and compact the log once due after a completed run
                const from = await replica.resume(database, subscription);
                const pages = await this.#store.stream({ ...subscription, ...from }, { signal });
                for await (const page of replica.apply(database, pages, {
                    subscription,
                    audience: "caller",
                })) {
                    if (page.complete) {
                        const due = await database.log.compactIfDue(
                            LOCAL_LOG_MILLISECONDS,
                            this.#compactions.get(database) ?? 0,
                            this.options.environment.clock.now(),
                        );
                        this.#compactions.set(database, due);
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

    /** Wait until the replica completed a run of the given query. */
    async #replicated(
        database: DatabaseConnection,
        replica: sync.Replica,
        name: string,
        signal: AbortSignal,
    ): Promise<void> {
        await database.log.until(async () => {
            const queries = (await replica.subscribed(database))?.parameters["queries"];

            return typeof queries === "object" && queries !== null && name in queries;
        }, signal);
    }
}
