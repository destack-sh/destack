import { schema } from "@destack/schema";
import { RequestId } from "../request/index.ts";

/** An idempotent creation. */
export const Creation = schema.object({
    /** The request identifier. */
    requestId: RequestId.schema,
});

/** An idempotent mutation of a record revision. */
export const Mutation = Creation.extend({
    /** The current record revision. */
    revision: schema.number().int().positive(),
});
