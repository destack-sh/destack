import { type Table } from "@destack/db";
import { Condition, Expression, type Computed } from "@destack/db/query";
import type { Query } from "../../query/query.ts";
import { comment, page, project, tag, task, taskTag } from "../../test/fixture.ts";
import type { Path, Relation } from "../../query/query.ts";
import { ConditionAudience } from "./audience.ts";

/** The scopes every query set follows. */
const ROUTES = ["inbox"];

/** Queries of every row they hold: filters over each table, no windows or tallies. */
export const FILTERS: Readonly<Record<string, Query>> = {
    projects: { table: project, scopes: ROUTES, where: Condition.ne("name", "x") },
    open: {
        table: task,
        scopes: ROUTES,
        where: Condition.all(
            Condition.eq("state", "open"),
            Condition.not(Condition.missing("points")),
        ),
    },
    comments: {
        table: comment,
        scopes: ROUTES,
        where: Condition.any(Condition.lt("position", 2), Condition.gt("position", 4)),
    },
};

/** Queries of the first rows of orders: descending, multi-key, over missing values. */
export const WINDOWS: Readonly<Record<string, Query>> = {
    top: {
        table: task,
        scopes: ROUTES,
        where: Condition.gte("points", 2),
        order: [{ column: "rank", direction: "desc" }],
        limit: 4,
    },
    queue: {
        table: task,
        scopes: ROUTES,
        order: [
            { column: "state", direction: "asc" },
            { column: "points", direction: "desc" },
            { column: "rank", direction: "asc" },
        ],
        limit: 5,
    },
};

/** Queries of a tree: projects with their first tasks, those tasks' first comments and project, and per-project tallies. */
export const TREE: Readonly<Record<string, Query>> = {
    projects: {
        table: project,
        scopes: ROUTES,
        where: Condition.ne("name", "x"),
        include: {
            tasks: {
                table: task,
                on: { kind: "key", column: "projectId", parent: "id" },
                where: Condition.oneOf("state", ["open", "doing"]),
                order: [{ column: "rank", direction: "desc" }],
                limit: 2,
                include: {
                    comments: {
                        table: comment,
                        on: { kind: "key", column: "taskId", parent: "id" },
                        order: [{ column: "position", direction: "asc" }],
                        limit: 2,
                    },
                },
            },
            stats: {
                table: task,
                on: { kind: "key", column: "projectId", parent: "id" },
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
        where: Condition.eq("state", "done"),
        include: {
            project: { table: project, on: { kind: "key", column: "id", parent: "projectId" } },
        },
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

/** The tags of a task, through its task tags. */
const TAGS: Path = {
    kind: "junction",
    table: taskTag,
    from: { column: "taskId", key: "id" },
    to: { column: "tagId", key: "id" },
};

/** Queries through paths: tags of tasks through a join table, pages below root pages, and pages above ranked pages. */
export const PATHS: Readonly<Record<string, Query>> = {
    tagged: {
        table: task,
        scopes: ROUTES,
        where: Condition.ne("state", "done"),
        include: {
            tags: { table: tag, on: TAGS, order: [{ column: "name", direction: "asc" }], limit: 2 },
            count: { table: tag, on: TAGS, aggregate: { values: { tags: { function: "count" } } } },
        },
    },
    roots: {
        table: page,
        scopes: ROUTES,
        where: Condition.missing("parentId"),
        include: {
            below: {
                table: page,
                on: { kind: "descendants", column: "parentId" },
                order: [{ column: "rank", direction: "asc" }],
                limit: 3,
            },
            size: {
                table: page,
                on: { kind: "descendants", column: "parentId" },
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
        where: Condition.gte("rank", 6),
        include: { above: { table: page, on: { kind: "ancestors", column: "parentId" } } },
    },
};

/** Queries of pages below root pages, the first of an order: members deeper than the rows between them carry those rows. */
export const CHAINS: Readonly<Record<string, Query>> = {
    roots: {
        table: page,
        scopes: ROUTES,
        where: Condition.missing("parentId"),
        include: {
            below: {
                table: page,
                on: { kind: "descendants", column: "parentId" },
                where: Condition.lt("rank", 5),
                order: [{ column: "rank", direction: "asc" }],
                limit: 2,
            },
        },
    },
};

/** A task's formula values, as a database view computes them: a score, a ratio missing on zero, and a label. */
const FORMULAS: Computed = {
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

/** Queries of a database view with formula columns: filtered, sorted and grouped by them, in a root and per project. */
export const COMPUTED: Readonly<Record<string, Query>> = {
    scored: {
        table: task,
        scopes: ROUTES,
        compute: FORMULAS,
        where: Condition.gt("score", 6),
        order: [
            { column: "ratio", direction: "desc" },
            { column: "score", direction: "asc" },
        ],
        limit: 4,
    },
    boards: {
        table: project,
        scopes: ROUTES,
        include: {
            tasks: {
                table: task,
                on: { kind: "key", column: "projectId", parent: "id" },
                compute: FORMULAS,
                order: [{ column: "score", direction: "desc" }],
                limit: 2,
            },
            scores: {
                table: task,
                on: { kind: "key", column: "projectId", parent: "id" },
                compute: FORMULAS,
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
        where: Condition.missing("parentId"),
        include: {
            deep: {
                table: page,
                on: { kind: "descendants", column: "parentId" },
                compute: {
                    weight: Expression.subtract(Expression.literal(10), Expression.column("rank")),
                },
                where: Condition.gt("weight", 4),
                order: [{ column: "weight", direction: "asc" }],
                limit: 2,
            },
        },
    },
};

/** The relations of a task: its tags, through its task tags, and its comments. */
const TASK_RELATIONS: Readonly<Record<string, Relation>> = {
    tags: { table: tag, on: TAGS },
    comments: { table: comment, on: { kind: "key", column: "taskId", parent: "id" } },
};

/** Queries of an issue tracker's filters: tagged, untagged and discussed tasks, projects with open discussed work, and counts of tagged tasks per state. */
export const RELATED: Readonly<Record<string, Query>> = {
    tagged: {
        table: task,
        scopes: ROUTES,
        relations: TASK_RELATIONS,
        where: Condition.exists("tags", Condition.eq("name", "a")),
        order: [{ column: "rank", direction: "desc" }],
        limit: 3,
    },
    untagged: {
        table: task,
        scopes: ROUTES,
        relations: TASK_RELATIONS,
        where: Condition.all(
            Condition.not(Condition.exists("tags")),
            Condition.ne("state", "done"),
        ),
    },
    either: {
        table: task,
        scopes: ROUTES,
        relations: TASK_RELATIONS,
        where: Condition.any(
            Condition.exists("tags", Condition.eq("name", "b")),
            Condition.exists("comments", Condition.lt("position", 2)),
        ),
    },
    busy: {
        table: project,
        scopes: ROUTES,
        relations: {
            tasks: {
                table: task,
                on: { kind: "key", column: "projectId", parent: "id" },
                relations: TASK_RELATIONS,
            },
        },
        where: Condition.exists(
            "tasks",
            Condition.all(Condition.eq("state", "open"), Condition.exists("comments")),
        ),
        include: {
            tasks: {
                table: task,
                on: { kind: "key", column: "projectId", parent: "id" },
                order: [{ column: "rank", direction: "asc" }],
                limit: 2,
            },
        },
    },
    states: {
        table: task,
        scopes: ROUTES,
        relations: TASK_RELATIONS,
        where: Condition.exists("tags"),
        aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } },
    },
};

/** A task's project, and the project's name each task sorts and groups by. */
const PROJECT: {
    readonly relations: Readonly<Record<string, Relation>>;
    readonly compute: Computed;
} = {
    relations: {
        project: { table: project, on: { kind: "key", column: "id", parent: "projectId" } },
    },
    compute: { projectName: Expression.lookup("project", "name") },
};

/** Queries of tasks sorted, filtered and counted by their project's name, in a root and within a tree. */
export const LOOKUPS: Readonly<Record<string, Query>> = {
    byProject: {
        table: task,
        scopes: ROUTES,
        ...PROJECT,
        order: [
            { column: "projectName", direction: "asc" },
            { column: "rank", direction: "desc" },
        ],
        limit: 4,
    },
    named: {
        table: task,
        scopes: ROUTES,
        ...PROJECT,
        where: Condition.all(Condition.ne("projectName", "x"), Condition.ne("state", "done")),
    },
    counts: {
        table: task,
        scopes: ROUTES,
        ...PROJECT,
        aggregate: { groupBy: ["projectName"], values: { tasks: { function: "count" } } },
    },
    commented: {
        table: comment,
        scopes: ROUTES,
        relations: { task: { table: task, on: { kind: "key", column: "id", parent: "taskId" } } },
        compute: { taskRank: Expression.lookup("task", "rank") },
        order: [{ column: "taskRank", direction: "desc" }],
        limit: 3,
    },
};

/** A project's tasks, which projects measure. */
const PROJECT_TASKS: Readonly<Record<string, Relation>> = {
    tasks: { table: task, on: { kind: "key", column: "projectId", parent: "id" } },
};

/** Queries of projects and tasks sorted, filtered and grouped by what their related rows measure. */
export const ROLLUPS: Readonly<Record<string, Query>> = {
    busiest: {
        table: project,
        scopes: ROUTES,
        relations: PROJECT_TASKS,
        compute: {
            open: Expression.rollup("count", "tasks", undefined, Condition.eq("state", "open")),
            effort: Expression.rollup("sum", "tasks", "points"),
            top: Expression.rollup("max", "tasks", "rank"),
            first: Expression.rollup("min", "tasks", "state"),
        },
        order: [
            { column: "open", direction: "desc" },
            { column: "first", direction: "asc" },
        ],
        limit: 3,
    },
    crowded: {
        table: project,
        scopes: ROUTES,
        relations: PROJECT_TASKS,
        compute: { size: Expression.rollup("count", "tasks") },
        where: Condition.gt("size", 1),
    },
    discussed: {
        table: task,
        scopes: ROUTES,
        relations: TASK_RELATIONS,
        compute: { replies: Expression.rollup("count", "comments") },
        where: Condition.gte("replies", 1),
        order: [{ column: "replies", direction: "desc" }],
        limit: 4,
    },
    labelled: {
        table: task,
        scopes: ROUTES,
        relations: TASK_RELATIONS,
        compute: { labels: Expression.rollup("count", "tags") },
        aggregate: { groupBy: ["labels"], values: { tasks: { function: "count" } } },
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
};

/** The query sets random workloads run, by name. */
export const QUERY_SETS: Readonly<Record<string, Readonly<Record<string, Query>>>> = {
    filters: FILTERS,
    windows: WINDOWS,
    tree: TREE,
    tallies: TALLIES,
    paths: PATHS,
    chains: CHAINS,
    computed: COMPUTED,
    related: RELATED,
    lookups: LOOKUPS,
    rollups: ROLLUPS,
    everything: EVERYTHING,
};

/** An audience that never sees secret tasks, fifth comments, hidden tags, the fourth tag's task tags or pages ranked seven, and never reads the titles of high-ranked tasks. */
export const AUDIENCE = new ConditionAudience(
    new Map<Table, Condition>([
        [task, Condition.eq("isSecret", false)],
        [comment, Condition.ne("position", 5)],
        [tag, Condition.ne("name", "hidden")],
        [taskTag, Condition.ne("tagId", "g3")],
        [page, Condition.ne("rank", 7)],
    ]),
    new Map([[task as Table, { when: Condition.gt("rank", 7), columns: ["title"] }]]),
);
