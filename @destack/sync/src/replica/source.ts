import { LogPosition } from "@destack/db/log";
import { Condition } from "@destack/db/query";
import { defineSchema, schema } from "@destack/schema";
import type { QueryPage } from "../query/index.ts";
import { ObjectTypeReference } from "../scope/reference.ts";

/** What a copy asks its source for: rows of one scope, decided for the scope the follower serves. */
export const ReplicaRequest = defineSchema(
    schema.object({
        /** The copy's name, shared with a relaying source's own copy. */
        name: schema.string().min(1),
        /** The copied scope. */
        scope: schema.string().min(1),
        /** The scope the follower serves, whose principal the source decides for. */
        below: schema.string().min(1),
        /** Whether the copy includes the scope's access rows. */
        access: schema.boolean(),
        /** The object types whose access rows live in the follower's own database. */
        held: schema.array(ObjectTypeReference),
        /** The object types the follower keeps copies of: the inherited rows of inherited types, and the scope's own row of scope types. */
        copied: schema.array(ObjectTypeReference),
        /** The global rows the follower reads, by object type, decided for it where they live. */
        rows: schema.array(
            schema.object({
                /** The object type. */
                type: ObjectTypeReference,
                /** The rows copied. */
                where: Condition.schema,
                /** The requested object types whose copied rows are the scopes of these rows, absent for rows of the copied scope. */
                within: schema.array(ObjectTypeReference).min(1).optional(),
            }),
        ),
        /** The log position the copy reached, absent before its first snapshot. */
        after: LogPosition.optional(),
    }),
);
/** What a copy asks its source for. */
export type ReplicaRequest = schema.Infer<typeof ReplicaRequest>;

/** Streams a copy's pages from the database one step closer to its rows' home. */
export interface ReplicaSource {
    /** Stream a copy's pages from a position, or a snapshot without one. */
    stream(request: ReplicaRequest, signal: AbortSignal): AsyncIterable<QueryPage>;
}
