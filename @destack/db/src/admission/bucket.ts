import { present } from "@destack/schema";
import type { DatabaseConnection } from "../database/index.ts";
import { and, eq, sql } from "../sql/index.ts";
import { defineTable, real, text } from "../table/index.ts";

/** The token buckets of rate limits, one row per limit and key, each one GCRA arrival time. */
export const bucket = defineTable("destack_bucket", {
    /** The limit, such as a method's or a workflow's. */
    name: text("name").primaryKey(),
    /** Whose tokens they are within the limit, such as a space or a subject. */
    key: text("key").primaryKey(),
    /** The theoretical arrival time of the next token, in UTC epoch milliseconds. */
    tat: real("tat").notNull(),
});

/** A request for tokens of a limit's key. */
export interface BucketRequest {
    /** The limit. */
    readonly name: string;
    /** Whose tokens they are within the limit. */
    readonly key: string;
    /** The sustained rate: so many tokens every so many milliseconds. */
    readonly rate: { readonly count: number; readonly every: number };
    /** The most tokens one key takes at once after idling. */
    readonly burst: number;
    /** The tokens to take, one by default. */
    readonly tokens?: number;
    /** The current time, in UTC epoch milliseconds. */
    readonly now: number;
}

/** Tokens taken, or the wait until they would be. */
export type BucketOutcome =
    | {
          /** The tokens were taken. */
          readonly kind: "taken";
      }
    | {
          /** The limit refused the tokens. */
          readonly kind: "limited";
          /** How long until the tokens would be taken, in milliseconds. */
          readonly retryAfter: number;
      };

/** Rate limits as generic cell rate algorithm buckets, one statement per take. */
export const Bucket = {
    /** Take tokens of a key when its rate and burst allow. */
    async take(database: DatabaseConnection, request: BucketRequest): Promise<BucketOutcome> {
        // refuse more tokens at once than the burst admits ever
        const { name, key, rate, burst, tokens = 1, now } = request;
        if (tokens > burst) {
            throw new RangeError(`${tokens} tokens exceed the burst of ${burst} of limit ${name}`);
        }

        // advance the arrival time by the tokens' interval while it stays within the burst
        const interval = rate.every / rate.count;
        const tolerance = burst * interval;
        const cost = tokens * interval;
        const start = sql`CASE WHEN ${bucket.tat} > ${now} THEN ${bucket.tat} ELSE ${now} END`;
        const [taken] = await database
            .insert(bucket)
            .values({ name, key, tat: now + cost })
            .onConflictDoUpdate({
                target: [bucket.name, bucket.key],
                set: { tat: sql`${start} + ${cost}` },
                setWhere: sql`${start} + ${cost} - ${tolerance} <= ${now}`,
            })
            .returning({ tat: bucket.tat });
        if (taken !== undefined) {
            return { kind: "taken" };
        }

        // read how long until the tokens would fit within the burst
        const [limited] = await database
            .select({ tat: bucket.tat })
            .from(bucket)
            .where(and(eq(bucket.name, name), eq(bucket.key, key)));
        const tat = Math.max(present(limited, `the bucket of ${name} ${key}`).tat, now);

        return { kind: "limited", retryAfter: tat + cost - tolerance - now };
    },
};
