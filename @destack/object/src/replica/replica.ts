import { schema } from "@destack/schema";
import { eventIterator } from "@destack/service";
import { LogPosition } from "@destack/db/log";
import { Watermark } from "@destack/service/bookmark";
import { Outcome } from "@destack/service/database";
import { defineProcedure } from "@destack/service/procedure";
import { Condition, Expression, Order, Scalar, type Computed } from "@destack/db/query";
import { Aggregate, Call, Mutation, QueryPage, ObjectReference } from "@destack/sync";
import { Duration } from "../object/duration.ts";

/** Rows an include adds, or their aggregates. */
export type ObjectInclude = {
    /** The relation the include follows, its own name when absent. */
    readonly via?: string;
    /** Values computed from each row's fields, read like fields. */
    readonly compute?: Computed;
    /** The condition the rows meet, over their logged fields. */
    readonly where?: Condition;
    /** How the rows sort, completed by their identifier. */
    readonly order?: Order;
    /** The most rows held, per held row for an include. */
    readonly limit?: number;
    /** The includes of the rows, by relation. */
    readonly include?: Readonly<Record<string, ObjectInclude>>;
    /** Aggregates of the rows, held instead of the rows. */
    readonly aggregate?: Aggregate;
    /** Which trashed rows of a recoverable type to hold, none by default. */
    readonly deleted?: "exclude" | "include" | "only";
};

/** The fields of a query of one object type's rows. */
export const QueryShape = {
    /** Values computed from each row's fields, read like fields. */
    compute: schema.record(schema.string().min(1), Expression.schema).optional(),
    /** The condition the rows meet, over their logged fields. */
    where: Condition.schema.optional(),
    /** How the rows sort, completed by their identifier. */
    order: Order.schema.optional(),
    /** The most rows held, per held row for an include. */
    limit: schema.number().int().positive().optional(),
    /** The includes of the rows, by relation. */
    include: schema.lazy(() => schema.record(schema.string(), ObjectInclude)).optional(),
    /** Aggregates of the rows, held instead of the rows. */
    aggregate: Aggregate.optional(),
    /** Which trashed rows of a recoverable type to hold, none by default. */
    deleted: schema.enum(["exclude", "include", "only"]).optional(),
};

/** One aggregate group a list measures. */
const GroupSchema = schema.object({
    /** The group's values in their JSON form, by field. */
    group: schema.record(schema.string(), Scalar),
    /** The group's measures by name. */
    values: schema.record(schema.string(), Scalar),
});

/** What a list returns beside its rows. */
export const ListedShape = {
    /** Each row's computed values, by row identifier. */
    computed: schema.record(schema.string(), schema.record(schema.string(), Scalar)).optional(),
    /** What each row includes, in its JSON form, by include name and row identifier. */
    included: schema
        .record(schema.string(), schema.record(schema.string(), schema.json()))
        .optional(),
    /** The groups of an aggregate query. */
    groups: schema.array(GroupSchema).optional(),
};

/** Rows an include adds, or their aggregates. */
export const ObjectInclude: schema.Schema<ObjectInclude> = schema.lazy(() =>
    schema.object({ via: schema.string().min(1).optional(), ...QueryShape }),
) as schema.Schema<ObjectInclude>;

/** The most mutations one push carries: at 1 to 5 ms each, about half a second. */
export const PUSH_MUTATIONS = 100;

/** The identifier an object client mints for itself. */
export const ClientId = schema.string().min(1).max(64);

/** A query of one object type's rows in the scope a replica follows. */
export type ObjectQuery = Omit<ObjectInclude, "via"> & {
    /** The object type, by name. */
    readonly object: string;
};

/** A query of one object type's rows in the scope a replica follows. */
export const ObjectQuery: schema.Schema<ObjectQuery> = schema.object({
    object: schema.string().min(1),
    ...QueryShape,
}) as schema.Schema<ObjectQuery>;

/** The outcomes of a push and the watermark holding their changes. */
export const PushResult = schema.object({
    /** Each mutation's outcome, in the order pushed. */
    outcomes: schema.array(
        schema.object({
            /** The mutation's request identifier. */
            id: schema.string(),
            /** The results of its calls in order, or its final failure. */
            outcome: Outcome,
        }),
    ),
    /** The server log sequence holding every executed mutation's changes. */
    watermark: Watermark,
});
/** The outcomes of a push and the watermark holding their changes. */
export type PushResult = schema.Infer<typeof PushResult>;

/** The procedures a client replica uses. */
export const replicaProcedures = {
    push: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/push" })
        .input(
            schema.object({
                /** The scope the mutations change. */
                scope: schema.string().min(1),
                /** The mutations, in the order the client committed them. */
                mutations: schema.array(Mutation).min(1).max(PUSH_MUTATIONS),
                /** The client writing ephemeral objects. */
                client: ClientId.optional(),
            }),
        )
        .output(PushResult),
    sync: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/sync" })
        .input(
            schema.object({
                /** The scope to follow. */
                scope: schema.string().min(1),
                /** The queries to follow by name, every listed object type when absent. */
                queries: schema.record(schema.string(), ObjectQuery).optional(),
                /** The queries the subscriber followed before. */
                previous: schema.record(schema.string(), ObjectQuery).optional(),
                /** The log position the subscriber holds, absent before its first snapshot. */
                after: LogPosition.optional(),
                /** How often merged pages arrive. */
                refresh: schema.object({ every: Duration.schema }).optional(),
                /** The client following ephemeral objects, absent for durable ones. */
                client: ClientId.optional(),
            }),
        )
        .output(eventIterator(QueryPage)),
    call: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/call" })
        .input(
            schema.object({
                /** The scope the call reads. */
                scope: schema.string().min(1),
                /** The call of a reading method. */
                call: Call,
            }),
        )
        .output(schema.json()),
    broadcast: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/broadcast" })
        .input(
            schema.object({
                /** The scope holding the object. */
                scope: schema.string().min(1),
                /** The object whose readers receive the event. */
                object: ObjectReference.omit({ scope: true }),
                /** The unstored event. */
                event: schema.json(),
            }),
        )
        .output(schema.object({})),
};

/** The procedures a client replica uses. */
export type ReplicaProcedures = typeof replicaProcedures;
