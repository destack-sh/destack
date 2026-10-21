import { Caller, PermissionReference } from "@destack/access";
import { BranchEntry } from "../branch/entry.ts";
import { Condition, LogPosition, OrderBy, Row, Table, TABLE } from "@destack/db";
import { SERIES_FOLDS } from "@destack/event";
import { schema, type JsonValue } from "@destack/schema";

import * as sync from "@destack/sync";
import { ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import { Failure } from "@destack/error";
import type { MethodContext } from "../method/context.ts";

/** A JSON object, such as a call's input or a row's columns. */
const JsonObject = schema.jsonObject();

/** A listing's page in process: its rows beside its cursor and inclusions. */
const Page = schema.looseObject({ items: schema.array(Row) });

/** A listing's page as JSON. */
const EncodedPage = schema.looseObject({ items: schema.array(JsonObject) });

/** The query of a hook's read of a table's rows. */
const FIND = {
    /** The table's SQL name. */
    table: schema.string().min(1),
    /** The condition. */
    where: Condition.schema.exactOptional(),
    /** How the rows sort, completed by their key. */
    orderBy: OrderBy.schema.exactOptional(),
    /** The most rows read. */
    limit: schema.number().int().min(0).exactOptional(),
    /** The columns read: the logged ones, or all of a type of the hook's package. */
    columns: schema.enum(["logged", "all"]),
};

/** A row a rehearsal writes or leaves changed, its columns as JSON. */
export const RehearsedRow = schema.object({
    /** The SQL name of the row's table. */
    table: schema.string().min(1),
    /** The row's key columns. */
    key: JsonObject,
    /** The row after, null when removed. */
    after: JsonObject.nullable(),
    /** The row before, null when created. */
    before: JsonObject.nullable(),
});

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
const callRecord = schema.object({
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
    expansion: schema.array(BranchEntry).exactOptional(),
    /** The log position the call's mutation started at, absent for a query. */
    base: LogPosition.exactOptional(),
});
/** A call as a sandbox runs its hook: its type, method, scope, input, target and caller. */
export type CallRecord = schema.Infer<typeof callRecord>;

/** A call as a sandbox runs its hook: its type, method, scope, input, target and caller. */
export const CallRecord = Object.assign(callRecord, {
    /** Record a call for its sandbox. */
    of(context: MethodContext): CallRecord {
        // read the call's fields, and the start of a served mutation
        const { object, target, prepared, id, requestId, idempotencyKey, expansion } = context;
        const base = context.started;
        const { subject, actors, groups, contacts, grants } = context.caller;

        return {
            type: object.typeReference,
            method: context.name,
            scope: context.scope,
            chain: [...context.chain],
            input: JsonObject.parse(context.input),
            ...(id === undefined ? {} : { id }),
            ...(target === undefined ? {} : { target: object.table[TABLE].encode(target) }),
            caller: Caller.schema.parse(
                schema.defined({ subject, actors, groups, contacts, grants }),
            ),
            now: context.now,
            ...(requestId === undefined ? {} : { requestId }),
            ...(prepared === undefined ? {} : { prepared: schema.json().parse(prepared) }),
            ...(idempotencyKey === undefined ? {} : { idempotencyKey }),
            ...(expansion === undefined
                ? {}
                : { expansion: schema.array(BranchEntry).parse(expansion) }),
            ...(base === undefined ? {} : { base }),
        };
    },
});

/** A store's request to an installation's sandbox to run one hook of a call. */
export const HandlerRequest = schema.object({
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
});
/** A store's request to an installation's sandbox to run one hook of a call. */
export type HandlerRequest = schema.Infer<typeof HandlerRequest>;

/** The schema of an answer to a request. */
const handlerAnswer = schema.union([
    schema.object({
        /** The result as JSON, null for none. */
        result: schema.json(),
    }),
    schema.object({
        /** The failure. */
        error: Failure,
        /** The stack the failure was thrown at, the cause of the asking side's error. */
        stack: schema.string().exactOptional(),
    }),
]);
/** The answer to a request: its result as JSON, or its failure. */
export type HandlerAnswer = schema.Infer<typeof handlerAnswer>;

/** The answer to a request: its result as JSON, or its failure. */
export const HandlerAnswer = Object.assign(handlerAnswer, {
    /** Answer what a request runs: its result, or what it threw. */
    async of(run: () => Promise<JsonValue>): Promise<HandlerAnswer> {
        try {
            return { result: await run() };
        } catch (error) {
            const stack = error instanceof Error ? error.stack : undefined;

            return { error: Failure.of(error), ...(stack === undefined ? {} : { stack }) };
        }
    },

    /** Read an answer's result, throwing its failure as the call error it was. */
    result(answer: HandlerAnswer): JsonValue {
        // throw the failure, caused by an error at the answering side's stack
        if ("error" in answer) {
            const { stack } = answer;
            const cause = Object.assign(
                new Error(answer.error.message),
                stack === undefined ? {} : { stack },
            );
            throw Failure.error(answer.error, { cause });
        }

        return answer.result;
    },
});

/** The events of some scopes a read selects: the scopes, and a condition over their keys. */
const EventFilterRecord = schema.object({
    /** The scopes, at least one. */
    scopes: schema.tuple([schema.string().min(1)]).rest(schema.string().min(1)),
    /** The condition over `id`, `time` and the keys. */
    where: Condition.schema.exactOptional(),
});

/** A read of a package's own event type, as a hook sends it to its store. */
export const EventRequest = schema.discriminatedUnion("verb", [
    schema.object({
        /** Read a page of events in time order. */
        verb: schema.literal("query"),
        /** The selected events. */
        filter: EventFilterRecord,
        /** Where the page starts and ends, and its order. */
        page: schema.object({
            /** The previous page's last event. */
            after: schema
                .object({ time: schema.number(), id: schema.string().min(1) })
                .exactOptional(),
            /** The most events read. */
            limit: schema.number().int().min(1).exactOptional(),
            /** The order in time. */
            order: schema.enum(["ascending", "descending"]).exactOptional(),
        }),
    }),
    schema.object({
        /** Read one event by its identity. */
        verb: schema.literal("get"),
        /** The scopes the event may be in. */
        scopes: schema.tuple([schema.string().min(1)]).rest(schema.string().min(1)),
        /** The event's identity. */
        id: schema.string().min(1),
    }),
    schema.object({
        /** Read the events appended after a log sequence, in the order they were committed. */
        verb: schema.literal("changes"),
        /** The selected events. */
        filter: EventFilterRecord,
        /** The log sequence the read starts after. */
        after: schema.number().int().min(0),
        /** The most log entries read. */
        limit: schema.number().int().min(1).exactOptional(),
    }),
    schema.object({
        /** Read the events appended after a log sequence through another, as one page. */
        verb: schema.literal("range"),
        /** The selected events. */
        filter: EventFilterRecord,
        /** The log sequence the read starts after. */
        after: schema.number().int().min(0),
        /** The log sequence the read ends at. */
        through: schema.number().int().min(0),
    }),
    schema.object({
        /** Fold events into a series per group of key values. */
        verb: schema.literal("series"),
        /** The folded events. */
        filter: EventFilterRecord,
        /** The measured key, fold, grouping keys and step. */
        series: schema.object({
            /** The key folded. */
            measure: schema.string().min(1).exactOptional(),
            /** The fold. */
            fold: schema.enum(SERIES_FOLDS),
            /** The keys whose values each get a series of their own. */
            group: schema.array(schema.string().min(1)).exactOptional(),
            /** The width of each step, in milliseconds. */
            step: schema.number().exactOptional(),
        }),
    }),
]);
/** A read of a package's own event type, as a hook sends it to its store. */
export type EventRequest = schema.Infer<typeof EventRequest>;

/** A folded series as JSON: its group's values and its steps. */
export const SeriesRecord = schema.object({
    /** The value of each grouping key. */
    group: schema.record(
        schema.string(),
        schema.union([schema.string(), schema.number(), schema.null()]),
    ),
    /** The steps with events. */
    steps: schema.array(
        schema.object({
            /** The step's start, in Unix microseconds. */
            start: schema.number(),
            /** The folded value. */
            value: schema.number(),
            /** The events folded. */
            events: schema.number().int(),
        }),
    ),
});

/** A page of the subjects of a type with a permission on an object, as a hook asks for it. */
export const LookupRequest = schema.object({
    /** The permission. */
    permission: PermissionReference,
    /** The object. */
    object: ObjectReference,
    /** The type of the subjects listed. */
    type: ObjectTypeReference,
    /** Where the page starts and how many subjects it lists. */
    page: schema.object({
        /** The key of the previous page's last subject. */
        after: schema.string().min(1).exactOptional(),
        /** The most subjects listed. */
        limit: schema.number().int().min(1),
    }),
});
/** A page of the subjects of a type with a permission on an object, as a hook asks for it. */
export type LookupRequest = schema.Infer<typeof LookupRequest>;

/** A hook's request to its store in its mutation's session. */
export const SessionRequest = schema.object({
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
            /** Read the first row meeting a query, as the hook's package. */
            kind: schema.literal("findFirst"),
            ...FIND,
        }),
        schema.object({
            /** Read the rows meeting a query, as the hook's package. */
            kind: schema.literal("findMany"),
            ...FIND,
        }),
        schema.object({
            /** Read events of a type of the hook's package. */
            kind: schema.literal("events"),
            /** The event type's table SQL name. */
            table: schema.string().min(1),
            /** The read. */
            read: EventRequest,
        }),
        schema.object({
            /** Decide whether the hook's caller has a permission on an object. */
            kind: schema.literal("check"),
            /** The permission. */
            permission: PermissionReference,
            /** The object. */
            object: ObjectReference,
        }),
        schema.object({
            /** Require the hook's caller to have a permission on an object. */
            kind: schema.literal("require"),
            /** The permission. */
            permission: PermissionReference,
            /** The object. */
            object: ObjectReference,
        }),
        schema.object({
            /** Run calls as their authors over some rows and undo them. */
            kind: schema.literal("rehearse"),
            /** The rows written first. */
            over: schema.array(RehearsedRow),
            /** The calls run as their authors. */
            calls: schema.array(BranchEntry),
        }),
        schema.object({
            /** List a page of the subjects of a type with a permission on an object. */
            kind: schema.literal("lookup"),
            /** The lookup. */
            lookup: LookupRequest,
        }),
        schema.object({
            /** Take the position of the mutation's next derived identifier. */
            kind: schema.literal("position"),
        }),
        schema.object({
            /** Run the behaviour the hook wraps with the call's input. */
            kind: schema.literal("next"),
            /** The input. */
            input: JsonObject,
        }),
    ]),
});
/** A hook's request to its store in its mutation's session. */
export type SessionRequest = schema.Infer<typeof SessionRequest>;

/** A method's result as JSON: its object's row, its page of rows, or its value. */
export const HookResult = {
    /** Write a call's result as JSON. */
    encode(context: MethodContext, value: unknown): JsonValue {
        // write a value as it is
        const { method, object } = context;
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
    decode(context: MethodContext, value: JsonValue): unknown {
        // read a value as it is
        const { method, object } = context;
        if (method.result === "value") {
            return value;
        }
        // read a page's rows
        else if (method.result === "page") {
            const { items, ...page } = EncodedPage.parse(value);

            return { ...page, items: items.map((row) => Table.decode(object.table, row)) };
        }

        return Table.decode(object.table, JsonObject.parse(value));
    },
};
