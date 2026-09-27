import { schema } from "@destack/schema";
import { RequestId } from "../request/index.ts";

/** An idempotency key scoped to the authenticated caller, procedure, and request contents. */
export const Creation = schema.object({
    /** The idempotency key, which conflicts when reused with different request contents. */
    requestId: RequestId.schema,
});

/** An idempotent mutation conditional on the observed record revision. */
export const Mutation = Creation.extend({
    /** The required current record revision. */
    revision: schema.number().int().positive(),
});
