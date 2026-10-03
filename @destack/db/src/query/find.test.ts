import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { boolean, integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { defineDatabase } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import { defineRelations } from "./relation.ts";

/** Projects holding tasks. */
const project = defineTable("project", {
    /** The project identity. */
    id: text("id").primaryKey(),
    /** The project name. */
    name: text("name").notNull(),
});

/** Tasks of a project, with a rank and a lead. */
const task = defineTable("task", {
    /** The task identity. */
    id: text("id").primaryKey(),
    /** The task's project. */
    projectId: text("project_id"),
    /** The task title. */
    title: text("title").notNull(),
    /** The task's place in its project. */
    rank: integer("rank").notNull(),
    /** Whether the task is done. */
    isDone: boolean("is_done").notNull(),
});

/** Labels for tasks. */
const tag = defineTable("tag", {
    /** The tag identity. */
    id: text("id").primaryKey(),
    /** The label. */
    label: text("label").notNull(),
});

/** The tags of each task. */
const taskTag = defineTable("task_tag", {
    /** The tagged task. */
    taskId: text("task_id").primaryKey(),
    /** The tag. */
    tagId: text("tag_id").primaryKey(),
});

/** Pages nested in one forest per space. */
const page = defineTable(
    "page",
    {
        /** The page identity. */
        id: text("id").primaryKey(),
        /** The page's space. */
        space: text("space").notNull(),
        /** The page above, absent for a root. */
        parentId: text("parent_id"),
    },
    { tree: { id: "id", scope: "space", parent: "parentId" } },
);

/** The relations between the tables. */
const relations = defineRelations({ project, task, tag, taskTag, page }, (relate) => ({
    project: {
        tasks: relate.many.task({ from: relate.project.id, to: relate.task.projectId }),
        open: relate.many.task({
            from: relate.project.id,
            to: relate.task.projectId,
            where: { isDone: false },
        }),
    },
    task: {
        project: relate.one.project({ from: relate.task.projectId, to: relate.project.id }),
        tags: relate.many.tag({
            from: relate.task.id.through(relate.taskTag.taskId),
            to: relate.tag.id.through(relate.taskTag.tagId),
        }),
    },
}));

/** The database keeping the tables and their relations. */
const database = defineDatabase({
    name: "work",
    tables: [project, task, tag, taskTag, page],
    relations,
});

/** Open the database with two projects, five tasks, two tags and a page forest. */
async function open(dialect: Dialect) {
    const opened = await TestDatabase.create(dialect, database, { isMigrated: true });
    onTestFinished(() => opened.close());
    const connection = opened.database;
    await connection.insert(project).values([
        { id: "p1", name: "Garden" },
        { id: "p2", name: "House" },
    ]);
    await connection.insert(task).values([
        { id: "t1", projectId: "p1", title: "Dig", rank: 2, isDone: true },
        { id: "t2", projectId: "p1", title: "Plant", rank: 1, isDone: false },
        { id: "t3", projectId: "p1", title: "Water", rank: 3, isDone: false },
        { id: "t4", projectId: "p2", title: "Paint", rank: 1, isDone: true },
        { id: "t5", projectId: null, title: "Rest", rank: 1, isDone: false },
    ]);
    await connection.insert(tag).values([
        { id: "g1", label: "outdoor" },
        { id: "g2", label: "slow" },
    ]);
    await connection.insert(taskTag).values([
        { taskId: "t1", tagId: "g1" },
        { taskId: "t1", tagId: "g2" },
        { taskId: "t2", tagId: "g1" },
    ]);
    await connection.insert(page).values([
        { id: "a", space: "s", parentId: null },
        { id: "b", space: "s", parentId: "a" },
        { id: "c", space: "s", parentId: "b" },
    ]);

    return connection;
}

test.for(TEST_DIALECTS)(
    "include each project's first tasks by rank, each task's project or null, and each task's tags through their join table on %s",
    async (dialect) => {
        const connection = await open(dialect);

        // limit each project's tasks separately, in rank order
        expect(
            await connection.query.project.findMany({
                columns: { name: true },
                orderBy: { name: "asc" },
                with: { tasks: { columns: { title: true }, orderBy: { rank: "asc" }, limit: 2 } },
            }),
        ).toEqual([
            { name: "Garden", tasks: [{ title: "Plant" }, { title: "Dig" }] },
            { name: "House", tasks: [{ title: "Paint" }] },
        ]);

        // read a task without a project as null, and tags through the join table
        expect(
            await connection.query.task.findMany({
                columns: { title: true },
                where: { rank: 1 },
                orderBy: { title: "asc" },
                with: { project: { columns: { name: true } }, tags: { columns: { label: true } } },
            }),
        ).toEqual([
            { title: "Paint", project: { name: "House" }, tags: [] },
            { title: "Plant", project: { name: "Garden" }, tags: [{ label: "outdoor" }] },
            { title: "Rest", project: null, tags: [] },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "select rows by their relations, and count and look up through relations, a relation's condition included, on %s",
    async (dialect) => {
        const connection = await open(dialect);

        // select the projects with an open task, counting their open tasks
        expect(
            await connection.query.project.findMany({
                columns: { name: true },
                where: { open: true },
                extras: { pending: { kind: "rollup", function: "count", via: "open", where: {} } },
            }),
        ).toEqual([{ name: "Garden", pending: 2 }]);

        // select the tagged tasks, reading each task's project name
        expect(
            await connection.query.task.findMany({
                columns: { title: true },
                where: { tags: { label: "outdoor" } },
                orderBy: { title: "asc" },
                extras: { projectName: { kind: "lookup", via: "project", column: "name" } },
            }),
        ).toEqual([
            { title: "Dig", projectName: "Garden" },
            { title: "Plant", projectName: "Garden" },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "include a tree's descendants and ancestors through its index, each node without itself, on %s",
    async (dialect) => {
        const connection = await open(dialect);

        // read every node below and above each node
        const pages = await connection.query.page.findMany({
            columns: { id: true },
            orderBy: { id: "asc" },
            with: {
                descendants: { columns: { id: true }, orderBy: { id: "asc" } },
                ancestors: { columns: { id: true }, orderBy: { id: "asc" } },
            },
        });
        expect(pages).toEqual([
            { id: "a", descendants: [{ id: "b" }, { id: "c" }], ancestors: [] },
            { id: "b", descendants: [{ id: "c" }], ancestors: [{ id: "a" }] },
            { id: "c", descendants: [], ancestors: [{ id: "a" }, { id: "b" }] },
        ]);
    },
);

test("read the first row by order, none when no row meets the condition, and refuse an undeclared relation", async () => {
    const connection = await open("sqlite");

    // read the first and a missing row
    expect([
        await connection.query.task.findFirst({ columns: { id: true }, orderBy: { rank: "desc" } }),
        await connection.query.task.findFirst({ where: { title: "Sweep" } }),
    ]).toEqual([{ id: "t3" }, undefined]);

    // refuse a relation the schema does not declare
    await expect(connection.query.tag.findMany({ with: { tasks: true } })).rejects.toEqual(
        new DatabaseError("INVALID_QUERY", "tag has no relation tasks"),
    );
});

test("refuse relations naming a table the database does not keep", () => {
    expect(() => defineDatabase({ name: "projects", tables: [project], relations })).toThrow(
        "relations name table task, which the database does not keep",
    );
});
