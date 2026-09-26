import { defineSchema, schema } from "@destack/schema";

/** A position in the log: an epoch and a sequence within it, where a reader or copy continues. */
export const LogPosition = defineSchema(
    schema.object({
        /** The log's epoch, a time-ordered identifier renewed when the database is restored. */
        epoch: schema.string().min(1),
        /** The log sequence within the epoch. */
        sequence: schema.number().int().nonnegative(),
    }),
);
/** A position in the log: an epoch and a sequence within it, where a reader or copy continues. */
export type LogPosition = schema.Infer<typeof LogPosition>;

/** Report whether a position is later than another: a newer epoch, or a higher sequence within one. */
export function isAfter(position: LogPosition, other: LogPosition): boolean {
    return position.epoch === other.epoch
        ? position.sequence > other.sequence
        : position.epoch > other.epoch;
}
