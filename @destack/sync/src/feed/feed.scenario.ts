import { defineScenario } from "@destack/package/declare";
import { Feed } from "./feed.ts";
import { feedInteraction } from "../scenario/interaction.ts";

/** Snapshot thousands of rows of every query set into memory, and into a replica database. */
export const snapshotThousandsOfRows = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "snapshot-thousands-of-rows",
    description:
        "snapshot thousands of rows of every query set into memory, and into a replica database",
    given: { examples: [] },
    parameters: { rows: 4000 },
    when: [
        { action: "seed", rows: "{rows}" },
        {
            operation: "feed.snapshot",
            times: 1,
            warmup: 1,
            step: { action: "snapshot", queries: "snapshot" },
            then: [
                { metric: { name: "db.client.operation.duration", host: "database" }, atMost: 9 },
            ],
        },
        {
            operation: "replica.snapshot",
            times: 1,
            warmup: 1,
            step: { action: "replicate", queries: "snapshot" },
            then: [
                { metric: { name: "db.client.operation.duration", host: "database" }, atMost: 55 },
            ],
        },
    ],
    then: [{ observe: { count: "snapshot" }, is: 3798 }],
});

/** Write rows a hundred followers of one audience follow. */
export const writeToHundredSubscribers = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "write-to-hundred-subscribers",
    description: "write rows a hundred followers of one audience follow, each until all have it",
    given: { examples: [] },
    parameters: { rows: 500, followers: 100, warmup: 20, writes: 40 },
    when: [
        { action: "seed", rows: "{rows}" },
        { action: "follow", queries: "filters-windows", followers: "{followers}" },
        {
            operation: "row.write",
            times: "{writes}",
            warmup: "{warmup}",
            step: { action: "write" },
            then: [
                {
                    metric: { name: "db.client.operation.duration", host: "database" },
                    atMost: 6.15,
                },
            ],
        },
    ],
    then: [{ observe: { count: "followed" }, is: 224 }],
});

/** Rank one task a thousand direct subscribers follow. */
export const writeToThousandRowSubscribers = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "rank-for-thousand-subscribers",
    description: "rank one task anew while a thousand direct subscribers follow it",
    given: { examples: [] },
    parameters: { rows: 200, subscribers: 1000, warmup: 20, writes: 40 },
    when: [
        { action: "seed", rows: "{rows}" },
        { action: "subscribe", queries: "filters-windows", subscribers: "{subscribers}" },
        {
            operation: "task.rank",
            times: "{writes}",
            warmup: "{warmup}",
            step: { action: "rank", task: "s1" },
            then: [
                { metric: { name: "db.client.operation.duration", host: "database" }, atMost: 5.1 },
            ],
        },
    ],
});

/** Write rows three hundred followers of fifty distinct audiences follow. */
export const writeAcrossDistinctAccess = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "write-across-distinct-access",
    description: "write rows three hundred followers of fifty distinct audiences follow",
    given: { examples: [] },
    parameters: { rows: 500, followers: 300, audiences: 50, warmup: 20, writes: 40 },
    when: [
        { action: "seed", rows: "{rows}" },
        {
            action: "follow",
            queries: "filters-windows",
            followers: "{followers}",
            audiences: "{audiences}",
        },
        {
            operation: "row.write",
            times: "{writes}",
            warmup: "{warmup}",
            step: { action: "write" },
            then: [
                {
                    metric: { name: "db.client.operation.duration", host: "database" },
                    atMost: 6.15,
                },
            ],
        },
    ],
    then: [{ observe: { count: "followed" }, is: 313 }],
});

/** Rename a project back and forth, flipping its tasks in and out of a lookup. */
export const flipRowsThroughLookup = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "flip-rows-through-lookup",
    description:
        "rename a project back and forth, flipping its fifteen hundred tasks in and out of a lookup",
    given: { examples: [] },
    parameters: { tasks: 1500, warmup: 20, flips: 6 },
    when: [
        { action: "file", tasks: "{tasks}", project: "p0" },
        { action: "follow", queries: "lookups", followers: 1 },
        {
            operation: "project.rename",
            times: "{flips}",
            warmup: "{warmup}",
            step: { action: "rename", project: "p0" },
            then: [
                { metric: { name: "db.client.operation.duration", host: "database" }, atMost: 6.5 },
            ],
        },
    ],
    then: [{ observe: { count: "followed" }, is: 4 }],
});

/** Write rows one follower of every query set follows at scale. */
export const followEveryQuerySet = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "follow-every-query-set",
    description: "write rows one follower of every query set follows over three thousand rows",
    given: { examples: [] },
    parameters: { rows: 3000, warmup: 20, writes: 48 },
    when: [
        { action: "seed", rows: "{rows}" },
        { action: "follow", queries: "everything", followers: 1 },
        {
            operation: "row.write",
            times: "{writes}",
            warmup: "{warmup}",
            step: { action: "write" },
            then: [
                {
                    metric: { name: "db.client.operation.duration", host: "database" },
                    atMost: 8.375,
                },
            ],
        },
    ],
    then: [{ observe: { count: "followed" }, is: 2857 }],
});

/** Comment on tasks a window of the ten most commented among ten thousand follows. */
export const reorderTasksByRollup = defineScenario({
    of: Feed,
    interaction: feedInteraction,
    name: "reorder-tasks-by-rollup",
    description: "comment on tasks a window of the ten most commented among ten thousand follows",
    given: { examples: [] },
    parameters: { tasks: 10_000, warmup: 20, comments: 60, among: 40 },
    when: [
        { action: "file", tasks: "{tasks}" },
        { action: "follow", queries: "discussed", followers: 1 },
        {
            operation: "comment.create",
            times: "{comments}",
            warmup: "{warmup}",
            step: { action: "comment", among: "{among}" },
            then: [
                { metric: { name: "db.client.operation.duration", host: "database" }, atMost: 6 },
            ],
        },
    ],
    then: [{ observe: { count: "followed" }, is: 10 }],
});
