import { Caller, principal } from "@destack/access";
import { Condition, OrderBy, type RowImage, TABLE, type Table } from "@destack/db";
import { JsonValue, present, schema } from "@destack/schema";
import type { HookHandler, InstallationContext } from "@destack/runtime/workload";
import { Call } from "../method/call.ts";
import type { SessionReader } from "../method/reader.ts";
import { ObjectType } from "../object/object.ts";
import {
    type CallRecord,
    HandlerAnswer,
    HandlerRequest,
    HookResult,
    type SessionRequest,
} from "./record.ts";

/** An installation's object hooks, run in its sandbox for its space's store. */
export class HookServer implements HookHandler {
    /** The types whose hooks the server runs. */
    readonly types: readonly ObjectType[];
    /** Send a hook's request to the store in its mutation's session. */
    readonly #session: (request: SessionRequest) => Promise<HandlerAnswer>;
    /** The installation the sandbox runs as. */
    readonly #installation: InstallationContext;

    /** Run the hooks of the types packages route, reaching the store through a session. */
    constructor(options: {
        /** The packages whose routed types' hooks the server runs. */
        readonly packages: readonly { readonly objects: Readonly<Record<string, unknown>> }[];
        /** Send a hook's request to the store in its mutation's session. */
        session(request: SessionRequest): Promise<HandlerAnswer>;
        /** The installation the sandbox runs as. */
        readonly installation: InstallationContext;
    }) {
        this.types = options.packages
            .flatMap((served) => Object.values(served.objects))
            .filter((routed) => routed instanceof ObjectType)
            .filter((type) => Object.keys(type.hooks).length > 0);
        this.#session = (request) => options.session(request);
        this.#installation = options.installation;
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
                const next = async (changed: Call = call) =>
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
    #call(record: CallRecord, mutation: string): Call {
        // find the type and its method
        const object = Call.typeOf(this.types, record);
        if (object === undefined) {
            throw new TypeError(
                `this sandbox runs no hooks of ${record.type.packageId}/${record.type.type}`,
            );
        }
        const method = object.method(record.method);

        // rebuild the call, reading and running further calls in the mutation's session
        const call: Call = new Call({
            object,
            name: record.method,
            method,
            scope: record.scope,
            chain: record.chain,
            input: record.input,
            ...(record.id === undefined ? {} : { id: record.id }),
            ...(record.target === undefined
                ? {}
                : { target: object.table[TABLE].decode(record.target) }),
            caller: record.caller,
            now: record.now,
            location: {
                kind: "sandbox",
                session: this.#reader(mutation),
                installation: this.#installation,
            },
            objects: this.types,
            ...(record.requestId === undefined ? {} : { requestId: record.requestId }),
            ...(record.prepared === undefined
                ? {}
                : {
                      prepared: present(
                          method.prepared,
                          `the prepared work of ${object.name}.${record.method}`,
                      ).parse(record.prepared),
                  }),
            ...(record.idempotencyKey === undefined
                ? {}
                : { idempotencyKey: record.idempotencyKey }),
            ...(record.expansion === undefined ? {} : { expansion: record.expansion }),
            run: (entry, caller, mutates) =>
                this.#request(mutation, {
                    kind: "call",
                    as: this.#as(record, caller),
                    call: Call.wire(entry),
                    mutates,
                }),
        });

        return call;
    }

    /** Read the store's tables in a mutation's session. */
    #reader(mutation: string): SessionReader {
        return {
            row: async (table, key) => {
                const encoded = table[TABLE].encode(key);
                const row = await this.#request(mutation, {
                    kind: "row",
                    table: table[TABLE].sqlName,
                    key: encoded,
                });

                return row === null ? null : HookServer.#decode(table, row);
            },
            rows: async (table, where, options) => {
                const rows = await this.#request(mutation, {
                    kind: "rows",
                    table: table[TABLE].sqlName,
                    where: Condition.schema.parse(where),
                    ...(options.orderBy === undefined
                        ? {}
                        : { orderBy: OrderBy.schema.parse(options.orderBy) }),
                    ...(options.limit === undefined ? {} : { limit: options.limit }),
                });

                return schema
                    .array(schema.json())
                    .parse(rows)
                    .map((row) => HookServer.#decode(table, row));
            },
        };
    }

    /** Read whom a hook's call runs as: the hook's own caller, or its installation. */
    #as(record: CallRecord, caller: Caller): "caller" | "installation" {
        // run as the hook's own caller
        const installation = this.#installation;
        if (Caller.key(caller) === Caller.key(record.caller)) {
            return "caller";
        }
        // run as the installation
        else if (
            caller.subject !== undefined &&
            principal.installation.is(caller.subject) &&
            caller.subject.id === installation.id &&
            caller.subject.scope === installation.scope
        ) {
            return "installation";
        }
        // refuse any other caller
        else {
            throw new TypeError("a hook calls as its call's caller or as its installation");
        }
    }

    /** Send a request to the store in a mutation's session, throwing its failure. */
    async #request(mutation: string, request: SessionRequest["request"]): Promise<JsonValue> {
        return HandlerAnswer.result(await this.#session({ mutation, request }));
    }

    /** Read the logged columns of a row the store sent as JSON. */
    static #decode<Definition extends Table>(
        table: Definition,
        row: JsonValue,
    ): RowImage<Definition>;
    /**
     * Read a row's logged columns, as the store encoded them.
     *
     * @construct the store encodes a row's logged columns of this table, which decoding rebuilds.
     */
    static #decode(table: Table, row: JsonValue): unknown {
        return table[TABLE].decode(JsonValue.object(row));
    }
}
