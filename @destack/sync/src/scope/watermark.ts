import { schema } from "@destack/schema";

/** A position in one scope's log, which a read waits for to be at least as fresh. */
const watermarkSchema = schema.object({
    /** The scope whose database has the log. */
    scope: schema.string().min(1),
    /** The log's identifier, which a restore preserves. */
    log: schema.string().min(1),
    /** The log's epoch, renewed when the database is restored. */
    epoch: schema.string().min(1),
    /** The log sequence within the epoch. */
    sequence: schema.number().int().nonnegative(),
});
/** A position in one scope's log. */
export type Watermark = schema.Infer<typeof watermarkSchema>;

/** A position in one scope's log, which a read waits for to be at least as fresh. */
export const Watermark = Object.assign(watermarkSchema, {
    /** Take the later of two watermarks, the newer log's after a restore. */
    later(left: Watermark | null, right: Watermark | null): Watermark | null {
        return left === null ||
            (right !== null &&
                (right.log !== left.log ||
                    right.epoch !== left.epoch ||
                    right.sequence > left.sequence))
            ? (right ?? left)
            : left;
    },
});
