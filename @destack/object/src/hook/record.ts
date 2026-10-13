import { Caller } from "@destack/access";
import { Condition, OrderBy, Row, TABLE } from "@destack/db";
import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { errorOf } from "@destack/runtime/error";
import * as sync from "@destack/sync";
import { Failure, ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import type { Call } from "../method/call.ts";

/** A JSON object, such as a call's input or a row's columns. */
const JsonObject = schema.record(schema.string(), schema.json());

/** A listing's page in process: its rows beside its cursor and inclusions. */
const Page = schema.looseObject({ items: schema.array(Row) });

/** A listing's page as JSON. */
const EncodedPage = schema.looseObject({ items: schema.array(JsonObject) });

/** Calls a mutation runs as their authors, as an expansion lists them. */
export const BranchCalls = schema.array(sync.Call.extend({ author: Subject }));

/** A created object's first relationships and a new scope's owner. */
export const CreationRecord = schema.object({
    /** The relationships the object starts with. */
    relationships: schema
        .array(schema.object({ relation: schema.string(), subject: Subject }))
        .exactOptional(),
    /** The subject of a new scope's owner role. */
    owner: Subject.exactOptional(),
});

/** The schema of a call as a sandbox runs its hook. */
const callRecord = defineSchema(
    schema.object({
        /** The object type. */
        type: ObjectTypeReference,
        /** The method's name. */
        method: schema.string().min(1),
        /** The scope containing the object. */
        scope: schema.string().min(1),
        /** The scope and the scopes containing it, nearest first. */
        chain: schema.array(schema.string().min(1)),
        /** The method's own input fields. */
        input: JsonObject,
        /** The target's or created object's identifier. */
        id: schema.string().min(1).exactOptional(),
        /** The target at the start of the call, its columns as JSON. */
        target: JsonObject.exactOptional(),
        /** The caller. */
        caller: Caller.schema,
        /** The time of the call in UTC epoch milliseconds. */
        now: schema.number(),
        /** The request a durable mutation replays under. */
        requestId: schema.string().min(1).exactOptional(),
        /** The value the prepare phase produced, as JSON. */
        prepared: schema.json().exactOptional(),
        /** The idempotency key of the call's external work. */
        idempotencyKey: schema.string().min(1).exactOptional(),
        /** The calls the method's expansion ran before it in its mutation. */
        expansion: BranchCalls.exactOptional(),
    }),
);
/** A call as a sandbox runs its hook: its type, method, scope, input, target and caller. */
export type CallRecord = schema.Infer<typeof callRecord>;

/** A call as a sandbox runs its hook: its type, method, scope, input, target and caller. */
export const CallRecord = Object.assign(callRecord, {
    /** Record a call for its sandbox. */
    of(call: Call): CallRecord {
        const { object, target, prepared, id, requestId, idempotencyKey, expansion } = call;

        return {
            type: object.typeReference,
            method: call.name,
            scope: call.scope,
            chain: [...call.chain],
            input: JsonObject.parse(call.input),
            ...(id === undefined ? {} : { id }),
            ...(target === undefined ? {} : { target: object.table[TABLE].encode(target) }),
            caller: Caller.schema.parse(call.caller),
            now: call.now,
            ...(requestId === undefined ? {} : { requestId }),
            ...(prepared === undefined ? {} : { prepared: schema.json().parse(prepared) }),
            ...(idempotencyKey === undefined ? {} : { idempotencyKey }),
            ...(expansion === undefined ? {} : { expansion: BranchCalls.parse(expansion) }),
        };
    },
});

/** A store's request to an installation's sandbox to run one hook of a call. */
export const HandlerRequest = defineSchema(
    schema.object({
        /** The session of the hook's mutation, which its requests to the store present. */
        mutation: schema.string().min(1),
        /** The call the hook runs for. */
        call: CallRecord,
        /** The phase the hook runs, with the phase's own arguments. */
        hook: schema.discriminatedUnion("phase", [
            schema.object({ phase: schema.literal("authorize") }),
            schema.object({ phase: schema.literal("creation"), object: ObjectReference }),
            schema.object({ phase: schema.literal("expand") }),
            schema.object({ phase: schema.literal("prepare") }),
            schema.object({ phase: schema.literal("handle") }),
        ]),
    }),
);
/** A store's request to an installation's sandbox to run one hook of a call. */
export type HandlerRequest = schema.Infer<typeof HandlerRequest>;

/** The schema of an answer to a request. */
const handlerAnswer = defineSchema(
    schema.union([
        schema.object({
            /** The result as JSON, null for none. */
            result: schema.json(),
        }),
        schema.object({
            /** The failure. */
            error: Failure,
        }),
    ]),
);
/** The answer to a request: its result as JSON, or its failure. */
export type HandlerAnswer = schema.Infer<typeof handlerAnswer>;

/** The answer to a request: its result as JSON, or its failure. */
export const HandlerAnswer = Object.assign(handlerAnswer, {
    /** Answer what a request runs: its result, or what it threw. */
    async of(run: () => Promise<JsonValue>): Promise<HandlerAnswer> {
        try {
            return { result: await run() };
        } catch (error) {
            return { error: Failure.of(error) };
        }
    },

    /** Read an answer's result, throwing its failure as the call error it was. */
    result(answer: HandlerAnswer): JsonValue {
        if ("error" in answer) {
            throw errorOf(answer.error);
        }

        return answer.result;
    },
});

/** A hook's request to its store in its mutation's session. */
export const SessionRequest = defineSchema(
    schema.object({
        /** The session of the mutation the hook runs in. */
        mutation: schema.string().min(1),
        /** What the hook asks of its store. */
        request: schema.discriminatedUnion("kind", [
            schema.object({
                /** Run a call in the mutation. */
                kind: schema.literal("call"),
                /** Who the call runs as: the hook's own caller, or its installation. */
                as: schema.enum(["caller", "installation"]),
                /** The call. */
                call: sync.Call,
                /** Whether the call changes objects. */
                mutates: schema.boolean(),
            }),
            schema.object({
                /** Read one row by its key, absent unless the hook's caller may read it. */
                kind: schema.literal("row"),
                /** The table's SQL name. */
                table: schema.string().min(1),
                /** The row's key columns as JSON. */
                key: JsonObject,
            }),
            schema.object({
                /** Read the rows meeting a condition that the hook's caller may read. */
                kind: schema.literal("rows"),
                /** The table's SQL name. */
                table: schema.string().min(1),
                /** The condition. */
                where: Condition.schema,
                /** How the rows sort, completed by their key. */
                orderBy: OrderBy.schema.exactOptional(),
                /** The most rows read. */
                limit: schema.number().int().min(0).exactOptional(),
            }),
            schema.object({
                /** Run the behaviour the hook wraps with the call's input. */
                kind: schema.literal("next"),
                /** The input. */
                input: JsonObject,
            }),
        ]),
    }),
);
/** A hook's request to its store in its mutation's session. */
export type SessionRequest = schema.Infer<typeof SessionRequest>;

/** A method's result as JSON: its object's row, its page of rows, or its value. */
export const HookResult = {
    /** Write a call's result as JSON. */
    encode(call: Call, value: unknown): JsonValue {
        // write a value as it is
        const { method, object } = call;
        const table = object.table[TABLE];
        if (method.result === "value") {
            return schema.json().parse(value ?? null);
        }
        // write a page's rows
        else if (method.result === "page") {
            const { items, ...page } = Page.parse(value);

            return { ...JsonObject.parse(page), items: items.map((row) => table.encode(row)) };
        }

        return table.encode(Row.parse(value));
    },

    /** Read a call's result from JSON. */
    decode(call: Call, value: JsonValue): unknown {
        // read a value as it is
        const { method, object } = call;
        const table = object.table[TABLE];
        if (method.result === "value") {
            return value;
        }
        // read a page's rows
        else if (method.result === "page") {
            const { items, ...page } = EncodedPage.parse(value);

            return { ...page, items: items.map((row) => table.decode(row)) };
        }

        return table.decode(JsonObject.parse(value));
    },
};
