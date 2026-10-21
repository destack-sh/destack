import { AccessContext, Caller, Decision, principal } from "@destack/access";
import { BranchEntry } from "../branch/entry.ts";
import { Condition, Key, OrderBy, RelationalQueryBuilder, TABLE, Table } from "@destack/db";
import { JsonValue, present, schema } from "@destack/schema";
import type { HookHandler, ServerContext } from "../workload/workload.ts";
import { Subject } from "@destack/sync";
import { MethodContext } from "../method/context.ts";
import type { Session } from "../method/reader.ts";
import { ObjectType } from "../object/object.ts";
import {
    type CallRecord,
    HandlerAnswer,
    HandlerRequest,
    HookResult,
    RehearsedRow,
    type SessionRequest,
} from "./record.ts";

/** An installation's object hooks, run in its sandbox for its space's store. */
export class HookServer implements HookHandler {
    /** The types whose hooks the server runs. */
    readonly types: readonly ObjectType[];
    /** The types the hooks' calls may name: the routed ones, their attachments and references. */
    readonly objects: readonly ObjectType[];
    /** Send a hook's request to the store in its mutation's session. */
    readonly #session: (request: SessionRequest) => Promise<HandlerAnswer>;
    /** The server context of the installation the sandbox runs as. */
    readonly #server: ServerContext;

    /** Run the hooks of the types packages route, reaching the store through a session. */
    constructor(options: {
        /** The packages whose routed types' hooks the server runs. */
        readonly packages: readonly { readonly objects: Readonly<Record<string, unknown>> }[];
        /** Send a hook's request to the store in its mutation's session. */
        readonly session: (request: SessionRequest) => Promise<HandlerAnswer>;
        /** The server context of the installation the sandbox runs as. */
        readonly server: ServerContext;
    }) {
        // run the hooks of the routed types, answering calls to them, their attachments and what they refer to
        const routed = options.packages
            .flatMap((served) => Object.values(served.objects))
            .filter((type) => type instanceof ObjectType);
        this.types = routed.filter((type) => Object.keys(type.hooks).length > 0);
        this.objects = HookServer.#attached(routed);
        this.#session = (request) => options.session(request);
        this.#server = options.server;
    }

    /** Answer a request to run a hook over HTTP. */
    async fetch(request: Request): Promise<Response> {
        const asked = HandlerRequest.parse(await request.json());

        return Response.json(await this.answer(asked));
    }

    /** Run a hook of a call its store sends, answering its result or its failure. */
    answer(request: HandlerRequest): Promise<HandlerAnswer> {
        return HandlerAnswer.of(async () => {
            // find the type's method and the code of the phase
            const { call: record, hook, mutation } = request;
            const call = this.#call(record, mutation);
            const code = call.method.hooks;

            // authorize the call
            if (hook.phase === "authorize" && code?.authorize !== undefined) {
                await code.authorize(call);

                return null;
            }
            // derive a creation's first relationships
            else if (hook.phase === "creation" && code?.creation !== undefined) {
                return schema.json().parse(await code.creation(call, hook.object));
            }
            // list the calls the mutation runs first
            else if (hook.phase === "expand" && code?.expand !== undefined) {
                return schema.json().parse(await code.expand(call));
            }
            // do the external work before the transaction
            else if (hook.phase === "prepare" && code?.prepare !== undefined) {
                return schema.json().parse(await code.prepare(call));
            }
            // change rows, the wrapped behaviour run at the store
            else if (hook.phase === "handle" && code?.handle !== undefined) {
                const next = async (changed: MethodContext = call) =>
                    HookResult.decode(
                        call,
                        await this.#request(mutation, { kind: "next", input: changed.input }),
                    );

                return HookResult.encode(call, await code.handle(call, next));
            }
            // refuse a phase the type does not implement
            else {
                throw new TypeError(`${call.object.name}.${call.name} has no ${hook.phase} hook`);
            }
        });
    }

    /** Rebuild a recorded call in the sandbox, its calls and reads sent to the store. */
    #call(record: CallRecord, mutation: string): MethodContext {
        // find the type and its method
        const object = ObjectType.find(this.types, record);
        if (object === undefined) {
            throw new TypeError(
                `this sandbox does not run hooks of ${record.type.packageId}/${record.type.type}`,
            );
        }
        const method = object.method(record.method);

        // rebuild the call, reading and running further calls in the mutation's session
        const context: MethodContext = new MethodContext({
            object,
            name: record.method,
            method,
            scope: record.scope,
            chain: record.chain,
            input: record.input,
            id: record.id,
            target:
                record.target === undefined ? undefined : Table.decode(object.table, record.target),
            caller: AccessContext.of(record.caller, record.now),
            location: {
                kind: "sandbox",
                session: this.#open(mutation),
                server: this.#server,
                ...(record.base === undefined ? {} : { base: record.base }),
            },
            objects: this.objects,
            replicas: [],
            ...(record.requestId === undefined ? {} : { requestId: record.requestId }),
            prepared:
                record.prepared === undefined
                    ? undefined
                    : present(
                          method.prepared,
                          `the prepared work of ${object.name}.${record.method}`,
                      ).parse(record.prepared),
            ...(record.idempotencyKey === undefined
                ? {}
                : { idempotencyKey: record.idempotencyKey }),
            ...(record.expansion === undefined ? {} : { expansion: record.expansion }),
            run: (entry, caller, mutates) =>
                this.#request(mutation, {
                    kind: "call",
                    as: this.#as(record, caller),
                    call: entry,
                    mutates,
                }),
        });

        return context;
    }

    /** Query a table in a mutation, decoding its rows as the store answers them. */
    #query<Definition extends Table, Read>(
        mutation: string,
        table: Definition,
        columns: "logged" | "all",
        decode: (row: JsonValue) => Read,
    ): RelationalQueryBuilder<Definition, Read> {
        // ask the store for the rows of a query
        const ask = async (
            kind: "findFirst" | "findMany",
            query: {
                readonly where?: unknown;
                readonly orderBy?: unknown;
                readonly limit?: number;
            },
        ) =>
            this.#request(mutation, {
                kind,
                table: table[TABLE].sqlName,
                ...(query.where === undefined
                    ? {}
                    : { where: Condition.schema.parse(query.where) }),
                ...(query.orderBy === undefined
                    ? {}
                    : { orderBy: OrderBy.schema.parse(query.orderBy) }),
                ...(query.limit === undefined ? {} : { limit: query.limit }),
                columns,
            });

        return new RelationalQueryBuilder(
            table,
            async (query) =>
                schema
                    .array(schema.json())
                    .parse(await ask("findMany", query))
                    .map((row) => decode(row)),
            async (key) => {
                const row = schema
                    .json()
                    .nullable()
                    .parse(await ask("findFirst", { where: Key.where(table, key) }));

                return row === null ? null : decode(row);
            },
        );
    }

    /** Reach the store in a mutation's session: its reads, decisions and positions. */
    #open(mutation: string): Session {
        return {
            read: (table) =>
                this.#query(mutation, table, "logged", (row) =>
                    Table.image(table, Table.decode(table, JsonValue.object(row))),
                ),
            reveal: (table) =>
                this.#query(mutation, table, "all", (row) =>
                    Table.complete(table, Table.decode(table, JsonValue.object(row))),
                ),
            events: (table, read) =>
                this.#request(mutation, { kind: "events", table: table[TABLE].sqlName, read }),
            check: async (permission, object) =>
                Decision.parse(
                    await this.#request(mutation, { kind: "check", permission, object }),
                ),
            require: async (permission, object) => {
                await this.#request(mutation, { kind: "require", permission, object });
            },
            rehearse: async (over, calls) =>
                schema.array(RehearsedRow).parse(
                    await this.#request(mutation, {
                        kind: "rehearse",
                        over: [...over],
                        calls: schema.array(BranchEntry).parse(calls),
                    }),
                ),
            lookup: async (lookup) =>
                schema
                    .array(Subject)
                    .parse(await this.#request(mutation, { kind: "lookup", lookup })),
            takePosition: async () =>
                schema
                    .number()
                    .int()
                    .parse(await this.#request(mutation, { kind: "position" })),
        };
    }

    /** Read whom a hook's call runs as: the hook's own caller, or its installation. */
    #as(record: CallRecord, caller: AccessContext): "caller" | "installation" {
        // run as the hook's own caller
        const server = this.#server;
        if (Caller.key(caller) === Caller.key(record.caller)) {
            return "caller";
        }
        // run as the installation
        else if (
            caller.subject !== undefined &&
            principal.installation.is(caller.subject) &&
            caller.subject.id === server.installation &&
            caller.subject.scope === server.space
        ) {
            return "installation";
        }
        // refuse any other caller
        else {
            throw new TypeError("a hook must call as its call's caller or as its installation");
        }
    }

    /** Send a request to the store in a mutation's session, throwing its failure. */
    async #request(mutation: string, request: SessionRequest["request"]): Promise<JsonValue> {
        return HandlerAnswer.result(await this.#session({ mutation, request }));
    }

    /** List some types with their attachments, transitively, and the types they refer to. */
    static #attached(types: readonly ObjectType[]): ObjectType[] {
        // walk the attachments of each type found
        const attached: ObjectType[] = [...types];
        const pending = types.flatMap((type) => type.attachments.map((each) => each.object));
        for (let type = pending.shift(); type !== undefined; type = pending.shift()) {
            if (!attached.some((each) => each.same(type))) {
                attached.push(type);
                pending.push(...type.attachments.map((each) => each.object));
            }
        }

        return [...attached, ...ObjectType.referenced(attached)];
    }
}
