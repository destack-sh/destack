import { schema } from "@destack/schema";
import { Call, Subject } from "@destack/sync";

/** A call a branch stores, with the principal who made it and the actors it called through. */
export const BranchEntry = Call.extend({
    /** The principal the call records as its caller. */
    author: Subject,
    /** The actors the author called through, none for a direct call. */
    actors: schema.array(Subject),
});
/** A call a branch stores, with the principal who made it and the actors it called through. */
export type BranchEntry = schema.Infer<typeof BranchEntry>;
