import { schema } from "@destack/schema";

/** A position in one scope's log, which a read waits for to be at least as fresh. */
export const Watermark = schema.object({
    /** The scope whose database has the log. */
    scope: schema.string().min(1),
    /** The log's identifier, which a restore keeps. */
    log: schema.string().min(1),
    /** The log's epoch, renewed when the database is restored. */
    epoch: schema.string().min(1),
    /** The log sequence reached within the epoch. */
    sequence: schema.number().int().nonnegative(),
});
/** A position in one scope's log. */
export type Watermark = schema.Infer<typeof Watermark>;
