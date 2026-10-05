import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import {
    Aggregate,
    Expression,
    LogPosition,
    Condition,
    OrderBy,
    Scalar,
    type Extras,
} from "@destack/db";
import { eventIterator } from "@destack/service";
import { Watermark } from "@destack/service/bookmark";
import { defineProcedure } from "@destack/service/procedure";
import { Call, Failure, Mutation, ObjectReference, Page, Subscription } from "@destack/sync";

/** A relational query of rows of a type known at runtime, the open form of `QueryOptions` callers send. */
export type OpenQueryOptions = {
    /** The fields each result keeps: only those set true, or all but those set false. */
    readonly columns?: Readonly<Record<string, boolean>>;
    /** Values computed from each row's fields and relations, read like fields. */
    readonly extras?: Extras;
    /** The condition the rows meet, over their logged fields. */
    readonly where?: Condition;
    /** How the rows sort, completed by their identifier. */
    readonly orderBy?: OrderBy;
    /** The most rows selected, per selected row for an include. */
    readonly limit?: number;
    /** The related rows each row includes, by relation: all of them, or those a query selects. */
    readonly with?: Readonly<Record<string, true | RelationOptions>>;
    /** Aggregates of the rows, kept instead of the rows, for a query's own rows only. */
    readonly aggregate?: Aggregate;
    /** Which trashed rows of a recoverable type to select, none by default. */
    readonly deleted?: "exclude" | "include" | "only";
};

/** The open query of a relation's rows, which keeps rows and no aggregates. */
export type RelationOptions = Omit<OpenQueryOptions, "aggregate">;

/** The view a read sees: a log position, a branch over the main line, or both. */
export const ViewShape = {
    /** The log position the read sees, the latest when absent. */
    at: LogPosition.exactOptional(),
    /** The branch whose rows the read sees over the main line, absent for the main line. */
    branch: schema.string().min(1).exactOptional(),
};

/** The fields of a query of a relation's rows as servers follow it, without the columns clients select. */
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
    /** Which trashed rows of a recoverable type to select, none by default. */
    deleted: schema.enum(["exclude", "include", "only"]).exactOptional(),
};

/** The query of a relation's rows. */
const RelationQuery: schema.Schema<RelationOptions> = schema.lazy(() =>
    schema.object(RelationShape),
);

/** The fields of a query of one object type's rows, which may aggregate them. */
export const QueryShape = {
    ...RelationShape,
    /** Aggregates of the rows, kept instead of the rows. */
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
    included: schema
        .record(schema.string(), schema.record(schema.string(), schema.json()))
        .exactOptional(),
    /** The groups of an aggregate query. */
    groups: schema.array(GroupSchema).exactOptional(),
};

/** The most mutations one push sends: at 1 to 5 ms each, about half a second. */
export const PUSH_MUTATIONS = 100;

/** The identifier an object client generates for itself. */
export const ClientId = schema.string().min(1).max(64);

/** A query of one object type's rows in the scope a replica follows. */
export type ObjectQuery = OpenQueryOptions & {
    /** The object type, by name. */
    readonly object: string;
};

/** A query of one object type's rows in the scope a replica follows. */
export const ObjectQuery: schema.Schema<ObjectQuery> = schema.object({
    object: schema.string().min(1),
    ...QueryShape,
});

/** The shape of a scope's durable objects a caller follows. */
export const QUERIES_SHAPE = "queries";

/** The shape of a scope's ephemeral objects a client follows. */
export const EPHEMERAL_SHAPE = "ephemeral";

/** The shape of a scope's external objects a caller follows. */
export const EXTERNAL_SHAPE = "external";

/** The shape of the access rows a caller's own checks read in a scope's chain. */
export const ACCESS_SHAPE = "access";

/** The shape of the rows of a type a home's residents receive, which the home projects. */
export const PROJECTION_SHAPE = "projection";

/** The parameters of a projection shape: the installation keeping the rows, the type, its recipient field, and the residents by subject key. */
export const ProjectionParameters = defineSchema(
    schema.object({
        /** The installation in the scope keeping the rows. */
        installation: schema.identifier("installation"),
        /** The package declaring the type. */
        packageId: PackageId,
        /** The type's name. */
        type: schema.string().min(1),
        /** The type's recipient field. */
        to: schema.string().min(1),
        /** The residents receiving the rows, as subject keys. */
        recipients: schema.array(schema.string().min(1)).min(1),
    }),
);
/** The parameters of a projection shape. */
export type ProjectionParameters = schema.Infer<typeof ProjectionParameters>;

/** The parameters of the shape of a scope's durable objects: the queries by name, every listed type of the scope's level when absent. */
export const QueriesParameters = defineSchema(
    schema.object({ queries: schema.record(schema.string(), ObjectQuery).exactOptional() }),
);
/** The parameters of the shape of a scope's durable objects. */
export type QueriesParameters = schema.Infer<typeof QueriesParameters>;

/** The parameters of the shape of a scope's ephemeral objects: the queries, and the client owning the rows it writes. */
export const EphemeralParameters = defineSchema(
    schema.object({
        queries: schema.record(schema.string(), ObjectQuery).exactOptional(),
        client: ClientId,
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

/** The procedures a client replica or a database copying the served one uses. */
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
                client: ClientId.exactOptional(),
            }),
        )
        .output(PushResult),
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
    receive: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/receive" })
        .input(
            schema.object({
                /** The mutation of copied rows a follower sends toward their home. */
                mutation: Mutation,
            }),
        )
        .output(schema.object({})),
    broadcast: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/broadcast" })
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
    stream: defineProcedure({ authentication: "public", permission: null, audit: false })
        .route({ method: "POST", path: "/replica/stream" })
        .input(Subscription)
        .output(eventIterator(Page)),
};

/** The procedures a client replica or a database copying the served one uses. */
export type ReplicaProcedures = typeof replicaProcedures;
