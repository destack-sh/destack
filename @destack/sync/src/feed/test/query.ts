import { type Table, Condition, type Extras, Expression } from "@destack/db";
import type { Query } from "../../query/query.ts";
import { comment, page, project, relations, tag, task, taskTag } from "../../test/fixture.ts";
import { ConditionAudience } from "./audience.ts";

/** The scopes every query set follows. */
const ROUTES = ["inbox"];

/** Queries of every row: filters over each table. */
export const FILTERS: Readonly<Record<string, Query>> = {
    projects: { table: project, scopes: ROUTES, where: { name: { ne: "x" } } },
    open: {
        table: task,
        scopes: ROUTES,
        where: { state: "open", points: { isNotNull: true } },
    },
    comments: {
        table: comment,
        scopes: ROUTES,
        where: { OR: [{ position: { lt: 2 } }, { position: { gt: 4 } }] },
    },
};

/** Queries of ordered windows. */
export const WINDOWS: Readonly<Record<string, Query>> = {
    top: {
        table: task,
        scopes: ROUTES,
        where: { points: { gte: 2 } },
        orderBy: { rank: "desc" },
        limit: 4,
    },
    queue: {
        table: task,
        scopes: ROUTES,
        orderBy: { state: "asc", points: "desc", rank: "asc" },
        limit: 5,
    },
};

/** Queries of a tree of projects, tasks and comments with tallies. */
export const TREE: Readonly<Record<string, Query>> = {
    projects: {
        table: project,
        scopes: ROUTES,
        relations,
        where: { name: { ne: "x" } },
        with: {
            tasks: {
                where: { state: { in: ["open", "doing"] } },
                orderBy: { rank: "desc" },
                limit: 2,
                with: {
                    comments: {
                        orderBy: { position: "asc" },
                        limit: 2,
                    },
                },
            },
            stats: {
                aggregate: {
                    groupBy: ["state"],
                    values: {
                        tasks: { function: "count" },
                        ranks: { function: "sum", column: "rank" },
                        top: { function: "max", column: "rank" },
                        low: { function: "min", column: "rank" },
                        mean: { function: "avg", column: "points" },
                    },
                },
            },
        },
    },
    done: {
        table: task,
        scopes: ROUTES,
        relations,
        where: { state: "done" },
        with: { project: true },
    },
};

/** Queries of tallies over a root and per state. */
export const TALLIES: Readonly<Record<string, Query>> = {
    totals: {
        table: task,
        scopes: ROUTES,
        aggregate: {
            groupBy: ["state"],
            values: {
                tasks: { function: "count" },
                low: { function: "min", column: "rank" },
                high: { function: "max", column: "rank" },
                points: { function: "sum", column: "points" },
                mean: { function: "avg", column: "points" },
            },
        },
    },
    all: {
        table: comment,
        scopes: ROUTES,
        aggregate: { values: { comments: { function: "count" } } },
    },
};

/** Queries through junction and tree paths. */
export const PATHS: Readonly<Record<string, Query>> = {
    tagged: {
        table: task,
        scopes: ROUTES,
        relations,
        where: { state: { ne: "done" } },
        with: {
            tags: { orderBy: { name: "asc" }, limit: 2 },
            count: { aggregate: { values: { tags: { function: "count" } } } },
        },
    },
    roots: {
        table: page,
        scopes: ROUTES,
        relations,
        where: { parentId: { isNull: true } },
        with: {
            below: {
                orderBy: { rank: "asc" },
                limit: 3,
            },
            size: {
                aggregate: {
                    groupBy: ["rank"],
                    values: { pages: { function: "count" } },
                },
            },
        },
    },
    crumbs: {
        table: page,
        scopes: ROUTES,
        relations,
        where: { rank: { gte: 6 } },
        with: { above: true },
    },
};

/** Queries of limited tree includes whose members hold chains. */
export const CHAINS: Readonly<Record<string, Query>> = {
    roots: {
        table: page,
        scopes: ROUTES,
        relations,
        where: { parentId: { isNull: true } },
        with: {
            below: {
                where: { rank: { lt: 5 } },
                orderBy: { rank: "asc" },
                limit: 2,
            },
        },
    },
};

/** A task's formula values: a score, a ratio missing on zero, and a label. */
const FORMULAS: Extras = {
    score: Expression.multiply(
        Expression.column("rank"),
        Expression.coalesce(Expression.column("points"), Expression.literal(1)),
    ),
    ratio: Expression.divide(
        Expression.column("points"),
        Expression.subtract(Expression.column("rank"), Expression.literal(4)),
    ),
    label: Expression.coalesce(Expression.column("title"), Expression.literal("untitled")),
};

/** Queries over formula columns. */
export const COMPUTED: Readonly<Record<string, Query>> = {
    scored: {
        table: task,
        scopes: ROUTES,
        extras: FORMULAS,
        where: { score: { gt: 6 } },
        orderBy: { ratio: "desc", score: "asc" },
        limit: 4,
    },
    boards: {
        table: project,
        scopes: ROUTES,
        relations,
        with: {
            tasks: {
                extras: FORMULAS,
                orderBy: { score: "desc" },
                limit: 2,
            },
            scores: {
                extras: FORMULAS,
                aggregate: {
                    groupBy: ["ratio"],
                    values: {
                        tasks: { function: "count" },
                        total: { function: "sum", column: "score" },
                        best: { function: "max", column: "score" },
                        mean: { function: "avg", column: "ratio" },
                    },
                },
            },
        },
    },
    below: {
        table: page,
        scopes: ROUTES,
        relations,
        where: { parentId: { isNull: true } },
        with: {
            deep: {
                extras: {
                    weight: Expression.subtract(Expression.literal(10), Expression.column("rank")),
                },
                where: { weight: { gt: 4 } },
                orderBy: { weight: "asc" },
                limit: 2,
            },
        },
    },
};

/** Queries over tags and discussions. */
export const RELATED: Readonly<Record<string, Query>> = {
    tagged: {
        table: task,
        scopes: ROUTES,
        relations,
        where: { tags: { name: "a" } },
        orderBy: { rank: "desc" },
        limit: 3,
    },
    untagged: {
        table: task,
        scopes: ROUTES,
        relations,
        where: { NOT: { tags: {} }, state: { ne: "done" } },
    },
    either: {
        table: task,
        scopes: ROUTES,
        relations,
        where: { OR: [{ tags: { name: "b" } }, { comments: { position: { lt: 2 } } }] },
    },
    busy: {
        table: project,
        scopes: ROUTES,
        relations,
        where: { tasks: { state: "open", comments: {} } },
        with: {
            tasks: {
                orderBy: { rank: "asc" },
                limit: 2,
            },
        },
    },
    states: {
        table: task,
        scopes: ROUTES,
        relations,
        where: { tags: {} },
        aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } },
    },
};

/** A task's project's name. */
const PROJECT: Extras = { projectName: Expression.lookup("project", "name") };

/** Queries by the project's name. */
export const LOOKUPS: Readonly<Record<string, Query>> = {
    byProject: {
        table: task,
        scopes: ROUTES,
        relations,
        extras: PROJECT,
        orderBy: { projectName: "asc", rank: "desc" },
        limit: 4,
    },
    named: {
        table: task,
        scopes: ROUTES,
        relations,
        extras: PROJECT,
        where: { projectName: { ne: "x" }, state: { ne: "done" } },
    },
    counts: {
        table: task,
        scopes: ROUTES,
        relations,
        extras: PROJECT,
        aggregate: { groupBy: ["projectName"], values: { tasks: { function: "count" } } },
    },
    commented: {
        table: comment,
        scopes: ROUTES,
        relations,
        extras: { taskRank: Expression.lookup("task", "rank") },
        orderBy: { taskRank: "desc" },
        limit: 3,
    },
};

/** Queries by rollups of related rows. */
export const ROLLUPS: Readonly<Record<string, Query>> = {
    busiest: {
        table: project,
        scopes: ROUTES,
        relations,
        extras: {
            open: Expression.rollup("count", "tasks", undefined, { state: "open" }),
            effort: Expression.rollup("sum", "tasks", "points"),
            top: Expression.rollup("max", "tasks", "rank"),
            first: Expression.rollup("min", "tasks", "state"),
        },
        orderBy: { open: "desc", first: "asc" },
        limit: 3,
    },
    crowded: {
        table: project,
        scopes: ROUTES,
        relations,
        extras: { size: Expression.rollup("count", "tasks") },
        where: { size: { gt: 1 } },
    },
    discussed: {
        table: task,
        scopes: ROUTES,
        relations,
        extras: { replies: Expression.rollup("count", "comments") },
        where: { replies: { gte: 1 } },
        orderBy: { replies: "desc" },
        limit: 4,
    },
    labelled: {
        table: task,
        scopes: ROUTES,
        relations,
        extras: { labels: Expression.rollup("count", "tags") },
        aggregate: { groupBy: ["labels"], values: { tasks: { function: "count" } } },
    },
};

/** Queries of rows in the scopes selected rows are in: each project's own tags. */
export const WITHIN: Readonly<Record<string, Query>> = {
    folders: {
        table: project,
        scopes: ROUTES,
        relations,
        where: { name: { ne: "x" } },
        with: {
            tags: true,
        },
    },
};

/** Every query set at once. */
export const EVERYTHING: Readonly<Record<string, Query>> = {
    ...FILTERS,
    ...WINDOWS,
    ...TREE,
    ...TALLIES,
    ...PATHS,
    ...COMPUTED,
    ...RELATED,
    ...LOOKUPS,
    ...ROLLUPS,
    ...WITHIN,
};

/** The visible rows of each table. */
const VISIBLE = new Map<Table, Condition>([
    [task, { isSecret: false }],
    [comment, { position: { ne: 5 } }],
    [tag, { name: { ne: "hidden" } }],
    [taskTag, { tagId: { ne: "g3" } }],
    [page, { rank: { ne: 7 } }],
]);

/** The titles of high-ranked tasks. */
const CONCEALED = new Map<Table, { readonly when: Condition; readonly columns: readonly string[] }>(
    [[task, { when: { rank: { gt: 7 } }, columns: ["title"] }]],
);

/** An audience seeing the visible rows without the concealed titles. */
export const AUDIENCE = new ConditionAudience(VISIBLE, CONCEALED);

/** The same audience, deciding in memory. */
export const MEMORY_AUDIENCE = new ConditionAudience(VISIBLE, CONCEALED, { isInMemory: true });
