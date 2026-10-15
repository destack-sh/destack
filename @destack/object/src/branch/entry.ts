import { schema } from "@destack/schema";
import { Call, Subject } from "@destack/sync";

/** A call a branch keeps, with the principal who made it. */
export const BranchEntry = Call.extend({
    /** The principal the call records as its caller. */
    author: Subject,
});
/** A call a branch keeps, with the principal who made it. */
export type BranchEntry = schema.Infer<typeof BranchEntry>;
