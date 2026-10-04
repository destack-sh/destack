import { onTestFinished } from "@destack/test";
import {
    bigint,
    binary,
    boolean,
    defineRelations,
    defineTable,
    index,
    integer,
    json,
    text,
    type DatabaseConnection,
    type Relation,
    Relations,
    type Table,
} from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { replicaTables, type Replica } from "../replica/replica.ts";
import type { Prediction } from "../prediction/prediction.ts";
import type { Page } from "../query/page.ts";
import { schema } from "@destack/schema";

/** Test notes. */
export const note = defineTable(
    "note",
    {
        /** The note identity. */
        id: text("id").primaryKey(),
        /** The title. */
        title: text("title").notNull(),
        /** The folder the note lives in. */
        scope: text("scope").notNull(),
        /** An optional summary. */
        summary: text("summary"),
        /** An exact counter beyond the safe integer range. */
        views: bigint("views").notNull(),
        /** Structured labels. */
        labels: json("labels", schema.array(schema.string())).notNull(),
        /** The last edit time. */
        editedAt: integer("edited_at").notNull(),
        /** Attached bytes, logged and synced in base64 form. */
        attachment: binary("attachment"),
    },
    { log: {} },
);

/** Assets with required binary content and a required secret outside the log. */
export const asset = defineTable(
    "asset",
    {
        /** The asset identity. */
        id: text("id").primaryKey(),
        /** The folder the asset lives in. */
        scope: text("scope").notNull(),
        /** The required binary content. */
        content: binary("content").notNull(),
        /** The required secret, left out of the log. */
        secret: text("secret").notNull().sensitive(),
    },
    { log: {} },
);

/** Projects with tasks. */
export const project = defineTable(
    "project",
    {
        /** The project identity. */
        id: text("id").primaryKey(),
        /** The folder the project lives in. */
        scope: text("scope").notNull(),
        /** The name. */
        name: text("name").notNull(),
    },
    { log: {} },
);

/** Tasks of projects. */
export const task = defineTable(
    "task",
    {
        /** The task identity. */
        id: text("id").primaryKey(),
        /** The folder the task lives in. */
        scope: text("scope").notNull(),
        /** The project the task belongs to. */
        projectId: text("project_id"),
        /** The state. */
        state: text("state").notNull(),
        /** The rank within lists. */
        rank: integer("rank").notNull(),
        /** The estimate. */
        points: integer("points"),
        /** The title, concealed from some audiences. */
        title: text("title"),
        /** Whether only the audience's insiders see the task. */
        isSecret: boolean("is_secret").notNull(),
    },
    {
        log: {},
        constraints: (table) => [
            index("task_project").on(table.projectId, table.rank),
            index("task_rank").on(table.scope, table.rank),
            index("task_queue").on(table.scope, table.state, table.points, table.rank),
        ],
    },
);

/** Comments on tasks. */
export const comment = defineTable(
    "comment",
    {
        /** The comment identity. */
        id: text("id").primaryKey(),
        /** The folder the comment lives in. */
        scope: text("scope").notNull(),
        /** The task commented on. */
        taskId: text("task_id"),
        /** The creation order. */
        position: integer("position").notNull(),
    },
    {
        log: {},
        constraints: (table) => [index("comment_task").on(table.taskId, table.position)],
    },
);

/** Tags of tasks. */
export const tag = defineTable(
    "tag",
    {
        /** The tag's key. */
        id: text("id").primaryKey(),
        /** The folder the tag lives in. */
        scope: text("scope").notNull(),
        /** The tag's name. */
        name: text("name").notNull(),
    },
    { log: {} },
);

/** The tags of tasks: one row per task and tag. */
export const taskTag = defineTable(
    "task_tag",
    {
        /** The row's key. */
        id: text("id").primaryKey(),
        /** The folder the row lives in. */
        scope: text("scope").notNull(),
        /** The tagged task. */
        taskId: text("task_id").notNull(),
        /** The tag. */
        tagId: text("tag_id").notNull(),
    },
    {
        log: {},
        constraints: (table) => [
            index("task_tag_task").on(table.taskId),
            index("task_tag_tag").on(table.tagId),
        ],
    },
);

/** Pages nested under other pages through their parent. */
export const page = defineTable(
    "page",
    {
        /** The page's key. */
        id: text("id").primaryKey(),
        /** The folder the page lives in. */
        scope: text("scope").notNull(),
        /** The page above, absent for a root. */
        parentId: text("parent_id"),
        /** The page's rank among its siblings. */
        rank: integer("rank").notNull(),
    },
    {
        log: {},
        tree: { id: "id", scope: "scope", parent: "parentId" },
        constraints: (table) => [index("page_parent").on(table.parentId)],
    },
);

/** A page's pages below it, through the page tree. */
const BELOW: Relation = {
    table: page,
    cardinality: "many",
    on: { kind: "descendants", column: "parentId" },
};

/** The relations the test queries name. */
export const relations = Relations.merge(
    defineRelations({ project, task, comment, tag, taskTag }, (relate) => ({
        project: {
            tasks: relate.many.task({ from: relate.project.id, to: relate.task.projectId }),
            stats: relate.many.task({ from: relate.project.id, to: relate.task.projectId }),
            scores: relate.many.task({ from: relate.project.id, to: relate.task.projectId }),
            size: relate.many.task({ from: relate.project.id, to: relate.task.projectId }),
            states: relate.many.task({ from: relate.project.id, to: relate.task.projectId }),
            tags: relate.many.tag({ from: relate.project.id, to: relate.tag.scope }),
        },
        task: {
            project: relate.one.project({ from: relate.task.projectId, to: relate.project.id }),
            comments: relate.many.comment({ from: relate.task.id, to: relate.comment.taskId }),
            tags: relate.many.tag({
                from: relate.task.id.through(relate.taskTag.taskId),
                to: relate.tag.id.through(relate.taskTag.tagId),
            }),
            count: relate.many.tag({
                from: relate.task.id.through(relate.taskTag.taskId),
                to: relate.tag.id.through(relate.taskTag.tagId),
            }),
        },
        comment: {
            task: relate.one.task({ from: relate.comment.taskId, to: relate.task.id }),
        },
    })),
    new Relations(
        new Map([
            [
                page,
                {
                    below: BELOW,
                    size: BELOW,
                    deep: BELOW,
                    above: {
                        table: page,
                        cardinality: "many",
                        on: { kind: "ancestors", column: "parentId" },
                    },
                },
            ],
        ]),
    ),
);

/** The tables of each test database. */
export const TABLES = [note, project, task, comment, tag, taskTag, page, ...replicaTables];

/** Open a migrated test database. */
export async function open(
    dialect: (typeof TEST_DIALECTS)[number],
    tables: readonly Table[] = TABLES,
): Promise<DatabaseConnection> {
    const test = await TestDatabase.create(dialect, tables, { isMigrated: true });
    onTestFinished(() => test.close());

    return test.database;
}

/** Open a migrated copy database without trees and references. */
export async function openCopy(
    dialect: (typeof TEST_DIALECTS)[number],
    tables: readonly Table[] = TABLES,
): Promise<DatabaseConnection> {
    const test = await TestDatabase.create(dialect, tables, { isMigrated: true, isReplica: true });
    onTestFinished(() => test.close());

    return test.database;
}

/** A note with exact, structured and binary values. */
export const first = {
    id: "a",
    title: "First",
    scope: "inbox",
    summary: null,
    views: 9_007_199_254_740_993n,
    labels: ["draft"],
    editedAt: 1790244000123,
    attachment: new Uint8Array([1, 2, 3]),
};

/** Take pages from a subscription until one satisfies a condition, then stop. */
export async function take(
    pages: AsyncGenerator<Page>,
    isLast: (page: Page) => boolean,
): Promise<Page[]> {
    const taken: Page[] = [];
    for await (const read of pages) {
        taken.push(read);
        if (isLast(read)) {
            await pages.return(undefined);
        }
    }

    return taken;
}

/** Read a generator's next value, refusing its end. */
export async function nextValue<Value>(generator: AsyncGenerator<Value, unknown>): Promise<Value> {
    const read = await generator.next();
    if (read.done === true) {
        throw new TypeError("the generator ended");
    }

    return read.value;
}

/** Apply pages to a copy as one stream. */
export async function replicate(
    copy: Replica,
    database: DatabaseConnection,
    pages: readonly Page[],
    prediction?: Prediction,
): Promise<void> {
    const applied = copy.apply(database, pages, prediction === undefined ? {} : { prediction });
    while ((await applied.next()).done !== true) {
        // apply each page in order
    }
}

/** Read pages from a subscription until one satisfies a condition. */
export async function until(
    pages: AsyncGenerator<Page>,
    condition: (page: Page) => boolean,
): Promise<Page[]> {
    const read: Page[] = [];
    for (let next = await pages.next(); next.done !== true; next = await pages.next()) {
        read.push(next.value);
        if (condition(next.value)) {
            break;
        }
    }

    return read;
}
