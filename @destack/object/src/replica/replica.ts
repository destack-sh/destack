import { defineSchema, schema } from "@destack/schema";
import { Expression, LogPosition, Condition, OrderBy, Scalar, type FindOptions } from "@destack/db";
import { eventIterator } from "@destack/runtime";
import { Watermark } from "@destack/runtime/bookmark";
import { defineProcedure } from "@destack/runtime/procedure";
import {
    Call,
    Failure,
    Mutation,
    ObjectReference,
    ObjectTypeReference,
    Page,
    Subscription,
    Aggregate,
} from "@destack/sync";

/** The view a read sees: a log position, a branch over the main line, or both. */
export const ViewShape = {
    /** The log position the read sees, the latest when absent. */
    at: LogPosition.exactOptional(),
    /** The branch whose rows the read sees over the main line, absent for the main line. */
    branch: schema.string().min(1).exactOptional(),
};

/** The fields of a relation query as servers follow it, without client-selected columns. */
const RelationShape = {
    /** Values computed from each row's fields and relations, read like fields. */
    extras: schema.record(schema.string().min(1), Expression.schema).exactOptional(),
    /** The condition the rows meet, over their logged fields. */
    where: Condition.schema.exactOptional(),
    /** How the rows sort, completed by their identifier. */
    orderBy: OrderBy.schema.exactOptional(),
    /** The most rows selected, per selected row for an include. */
    limit: schema.number().int().positive().exactOptional(),
    /** The related rows each row includes, by relation. */
    with: schema
        .lazy(() =>
            schema.record(schema.string(), schema.union([schema.literal(true), RelationQuery])),
        )
        .exactOptional(),
};

/** The query of a relation's rows. */
const RelationQuery: schema.Schema<FindOptions> = schema.lazy(() => schema.object(RelationShape));

/** The fields of a query of one object type's rows: its trashed rows, or aggregates of them. */
export const QueryShape = {
    ...RelationShape,
    /** Which trashed rows of a recoverable type to select, none by default. */
    deleted: schema.enum(["exclude", "include", "only"]).exactOptional(),
    /** Aggregates of the rows, replicated instead of the rows. */
    aggregate: Aggregate.exactOptional(),
};

/** One aggregate group a list measures. */
const GroupSchema = schema.object({
    /** The group's values in their JSON form, by field. */
    group: schema.record(schema.string(), Scalar),
    /** The group's measures by name. */
    values: schema.record(schema.string(), Scalar),
});

/** What a list returns beside its rows. */
export const ListShape = {
    /** Each row's extras, by row identifier. */
    extras: schema.record(schema.string(), schema.record(schema.string(), Scalar)).exactOptional(),
    /** What each row includes, in its JSON form, by include name and row identifier. */
    included: schema.record(schema.string(), schema.jsonObject()).exactOptional(),
    /** The groups of an aggregate query. */
    groups: schema.array(GroupSchema).exactOptional(),
};

/** The most mutations one push sends: at 1 to 5 ms each, about half a second. */
export const PUSH_MUTATIONS = 100;

/** The most object types one description request lists: every type of a large platform. */
export const DESCRIBED_TYPES = 1000;

/** The identifier an object client generates for itself as the writer of its ephemeral objects. */
export const WriterId = schema.string().min(1).max(64);

/** A query of one object type's rows in the scope a replica follows, or in every scope. */
export interface ObjectQuery extends FindOptions {
    /** The object type, by name. */
    readonly object: string;
    /** Every scope the follower may read, instead of the followed scope's chain. */
    readonly scopes?: "every";
    /** Which trashed rows of a recoverable type to select, none by default. */
    readonly deleted?: "exclude" | "include" | "only";
    /** Aggregates of the rows, replicated instead of the rows. */
    readonly aggregate?: Aggregate;
    /** The permission the caller also holds on each row, by its name on the type. */
    readonly permission?: string;
}

/** A query of one object type's rows in the scope a replica follows, or in every scope. */
export const ObjectQuery: schema.Schema<ObjectQuery> = schema.object({
    object: schema.string().min(1),
    scopes: schema.literal("every").exactOptional(),
    ...QueryShape,
    permission: schema.string().min(1).exactOptional(),
});

/** The publication of a scope's durable objects a caller follows. */
export const QUERIES_PUBLICATION = "queries";

/** The publication of a scope's ephemeral objects a client follows. */
export const EPHEMERAL_PUBLICATION = "ephemeral";

/** The publication of a scope's external objects a caller follows. */
export const EXTERNAL_PUBLICATION = "external";

/** The publication of the access rows a caller's own checks read in a scope's chain. */
export const ACCESS_PUBLICATION = "access";

/** The parameters of the durable objects publication: named queries, else every listed type. */
export const QueriesParameters = defineSchema(
    schema.object({
        /** The named queries, every durable table without them. */
        queries: schema.record(schema.string(), ObjectQuery).exactOptional(),
    }),
);
/** The parameters of the durable objects publication. */
export type QueriesParameters = schema.Infer<typeof QueriesParameters>;

/** The parameters of the ephemeral objects publication: queries, and the rows' writer. */
export const EphemeralParameters = defineSchema(
    schema.object({
        queries: schema.record(schema.string(), ObjectQuery).exactOptional(),
        writerId: WriterId,
    }),
);

/** The outcomes of a push and the watermark with their changes. */
export const PushResult = schema.object({
    /** Each mutation's outcome, in the order pushed. */
    outcomes: schema.array(
        schema.object({
            /** The mutation's request identifier. */
            id: schema.string(),
            /** The results of its calls in order, or its final failure. */
            outcome: schema.union([
                schema.object({
                    /** The results of the calls in order. */
                    value: schema.json(),
                }),
                schema.object({
                    /** The final failure. */
                    error: Failure,
                }),
            ]),
        }),
    ),
    /** The server log sequence with every executed mutation's changes. */
    watermark: Watermark,
});
/** The outcomes of a push and the watermark with their changes. */
export type PushResult = schema.Infer<typeof PushResult>;

/** The procedures a client replica or a database replicating the served one uses. */
export const storeProcedures = {
    push: defineProcedure({ authentication: "identity", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/push" })
        .input(
            schema.object({
                /** The scope the mutations change. */
                scope: schema.string().min(1),
                /** The mutations, in the order the client committed them. */
                mutations: schema.array(Mutation).min(1).max(PUSH_MUTATIONS),
                /** The client instance writing ephemeral objects. */
                writerId: WriterId.exactOptional(),
            }),
        )
        .output(PushResult),
    watermark: defineProcedure({ authentication: "identity", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/watermark" })
        .input(
            schema.object({
                /** The scope whose watermark to read. */
                scope: schema.string().min(1),
                /** The watermarks of enclosing scopes' homes the replica waits for. */
                homes: schema.array(Watermark),
            }),
        )
        .output(Watermark),
    call: defineProcedure({ authentication: "public", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/call" })
        .input(
            schema.object({
                /** The scope the call reads. */
                scope: schema.string().min(1),
                /** The call of a reading method. */
                call: Call,
            }),
        )
        .output(schema.json()),
    receive: defineProcedure({ authentication: "identity", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/receive" })
        .input(
            schema.object({
                /** The mutation of replicated rows a follower sends toward their home. */
                mutation: Mutation,
            }),
        )
        .output(schema.object({})),
    broadcast: defineProcedure({ authentication: "identity", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/broadcast" })
        .input(
            schema.object({
                /** The scope with the object. */
                scope: schema.string().min(1),
                /** The object whose readers receive the event. */
                object: ObjectReference.omit({ scope: true }),
                /** The unstored event. */
                event: schema.json(),
            }),
        )
        .output(schema.object({})),
    stream: defineProcedure({ authentication: "public", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/stream" })
        .input(Subscription)
        .output(eventIterator(Page)),
    describe: defineProcedure({ authentication: "public", permission: "rows", audit: false })
        .route({ method: "POST", path: "/store/describe" })
        .input(
            schema.object({
                /** The object types to describe, every served one when absent. */
                types: schema.array(ObjectTypeReference).max(DESCRIBED_TYPES).exactOptional(),
            }),
        )
        .output(
            schema.object({
                /** The descriptions, each type once, as `ObjectDescription` reads them. */
                objects: schema.array(schema.jsonObject()),
            }),
        ),
};

/** The procedures a client replica or a database replicating the served one uses. */
export type StoreProcedures = typeof storeProcedures;
