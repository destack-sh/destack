import type { Interaction } from "@destack/package/declare";
import { schema } from "@destack/schema";

/** The query sets a feed scenario follows, by the name the feed's tests give them. */
export const QuerySet = schema.enum([
    "filters-windows",
    "snapshot",
    "lookups",
    "everything",
    "discussed",
]);
/** A query set a feed scenario follows. */
export type QuerySet = schema.Infer<typeof QuerySet>;

/** A step of a feed world: rows written to a database, followed by subscribers of a feed. */
export const FeedStep = schema.discriminatedUnion("action", [
    schema.object({
        /** Write random projects, tasks and comments. */
        action: schema.literal("seed"),
        /** The rows. */
        rows: schema.number().int().positive(),
    }),
    schema.object({
        /** File open tasks in one inbox, in a named project when given. */
        action: schema.literal("file"),
        /** The tasks. */
        tasks: schema.number().int().positive(),
        /** The project's identity, the tasks in none when absent. */
        project: schema.string().min(1).exactOptional(),
    }),
    schema.object({
        /** Start followers of a query set, each with its replica, until each has the rows. */
        action: schema.literal("follow"),
        /** The query set. */
        queries: QuerySet,
        /** The followers. */
        followers: schema.number().int().positive(),
        /** The distinct audiences the followers fall into, one shared audience when absent. */
        audiences: schema.number().int().positive().exactOptional(),
    }),
    schema.object({
        /** Subscribe to a query set's pages directly, until each subscriber is complete. */
        action: schema.literal("subscribe"),
        /** The query set. */
        queries: QuerySet,
        /** The subscribers. */
        subscribers: schema.number().int().positive(),
    }),
    schema.object({
        /** Snapshot a query set into a new follower's memory, then stop it. */
        action: schema.literal("snapshot"),
        /** The query set. */
        queries: QuerySet,
    }),
    schema.object({
        /** Snapshot a query set into a new replica database, then stop its follower. */
        action: schema.literal("replicate"),
        /** The query set. */
        queries: QuerySet,
    }),
    schema.object({
        /** Write one random row, until every follower and subscriber has it. */
        action: schema.literal("write"),
    }),
    schema.object({
        /** Rank a task anew by the write's number, until every follower and subscriber has it. */
        action: schema.literal("rank"),
        /** The task's identity. */
        task: schema.string().min(1),
    }),
    schema.object({
        /** Rename a project back and forth, until every follower and subscriber has it. */
        action: schema.literal("rename"),
        /** The project's identity. */
        project: schema.string().min(1),
    }),
    schema.object({
        /** Comment on one of the first tasks at random, until every follower has it. */
        action: schema.literal("comment"),
        /** The tasks the comment picks from. */
        among: schema.number().int().positive(),
    }),
]);
/** A step of a feed world. */
export type FeedStep = schema.Infer<typeof FeedStep>;

/** An observation of a feed world: the rows a follower or a snapshot holds. */
export const FeedObservation = schema.object({
    /** Count the rows the first follower holds, or the last snapshot held. */
    count: schema.enum(["followed", "snapshot"]),
});
/** An observation of a feed world. */
export type FeedObservation = schema.Infer<typeof FeedObservation>;

/** The interaction of feed worlds, which the feed's driver plays. */
export const feedInteraction: Interaction<
    FeedStep,
    FeedObservation,
    Record<string, never>
> = Object.freeze({
    name: "feed",
    step: FeedStep,
    observation: FeedObservation,
    globals: schema.object({}),
});
