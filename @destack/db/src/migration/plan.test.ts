import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, index, integer, sql, text } from "../index.ts";
import { TABLE, type Table } from "../table/table.ts";
import { qualify } from "../table/namespace.ts";
import { defineDatabase } from "../declare/database.ts";
import { migrate, planMigration, planStates } from "./database.ts";
import { applyPlan } from "./apply.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { declareState } from "./state.ts";
import type { TablePlan } from "./plan.ts";

/** Folders holding documents, as first released. */
const folderOne = defineTable("folder", {
    id: text("id").primaryKey(),
    extra: text("extra"),
});

/** Folders after dropping their extra column. */
const folderTwo = defineTable("folder", {
    id: text("id").primaryKey(),
});

/** Documents referencing their folder. */
const document = defineTable("document", {
    id: text("id").primaryKey(),
    folderId: text("folder_id")
        .notNull()
        .references(() => folderTwo.id),
});

/** Labels whose codes may repeat. */
const labelPlain = defineTable("label", {
    id: text("id").primaryKey(),
    code: text("code").notNull(),
});

/** Labels with unique codes. */
const labelUnique = defineTable("label", {
    id: text("id").primaryKey(),
    code: text("code").notNull().unique(),
});

/** Tasks as first released. */
const taskOne = defineTable("task", {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    urgent: integer("urgent").notNull(),
    note: text("note"),
});

/** Tasks after renaming name to title, dropping the note, adding a due time and an index. */
const taskTwo = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        title: text("title").notNull(),
        urgent: integer("urgent").notNull(),
        dueAt: integer("due_at"),
    },
    {
        moved: { columns: { title: "name" } },
        constraints: (task) => [index("task_due").on(task.dueAt)],
    },
);

/** Tasks after converting the urgent flag into a priority. */
const taskThree = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        title: text("title").notNull(),
        urgent: integer("urgent").notNull(),
        priority: text("priority").notNull().default("normal"),
        dueAt: integer("due_at"),
    },
    {
        version: 2,
        convert: {
            2: (task) => ({
                priority: sql`CASE WHEN ${task.urgent} = 1 THEN 'high' ELSE 'normal' END`,
            }),
        },
        constraints: (task) => [index("task_due").on(task.dueAt)],
    },
);

/** Tasks declaring the urgent flag as text. */
const taskText = defineTable("task", {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    urgent: text("urgent").notNull(),
    note: text("note"),
});

/** Tasks with a required column no existing row can fill. */
const taskUnfilled = defineTable("task", {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    urgent: integer("urgent").notNull(),
    note: text("note"),
    owner: text("owner").notNull(),
});

/** Open an empty test database in a dialect. */
async function open(dialect: Dialect, tables: readonly Table[]) {
    const test = await TestDatabase.create(dialect, tables);
    onTestFinished(() => test.close());

    return test.database;
}

/** Summarize a plan's steps as reviewers see them. */
function review(plan: TablePlan) {
    return plan.steps.map((step) => `${step.risk} ${step.kind} ${step.target}: ${step.detail}`);
}

test.for(TEST_DIALECTS)(
    "create declared tables once and plan nothing afterwards on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);

        // create the table with its log and state, then find the database current
        const plan = await migrate(database, [taskOne]);
        expect(review(plan)).toEqual([`safe createTable ${table(taskOne)}: create table`]);
        expect(review(await planMigration(database, declareState([taskOne], dialect)))).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "rename, drop, add and index columns while keeping rows on %s",
    async (dialect) => {
        const database = await open(dialect, [taskTwo]);
        await migrate(database, [taskOne]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, name, urgent, note) VALUES ('a', 'Plan', 1, 'old')`,
        );

        // classify the rename as backward-incompatible, the addition as safe and the dropped note as destructive
        const plan = await planMigration(database, declareState([taskTwo], dialect));
        expect(review(plan)).toEqual(
            dialect === "sqlite"
                ? [
                      `backward-incompatible renameColumn ${table(taskTwo)}: rename column name to title`,
                      `destructive rebuildTable ${table(taskTwo)}: rebuild table: add due_at, drop note`,
                  ]
                : [
                      `backward-incompatible renameColumn ${table(taskTwo)}: rename column name to title`,
                      `safe addColumn ${table(taskTwo)}: add column due_at`,
                      `destructive dropColumn ${table(taskTwo)}: drop column note`,
                      `safe createIndex ${table(taskTwo)}: create index ${indexName(taskTwo, "task_due")}`,
                  ],
        );

        // keep the row under its renamed column and find the database current
        await applyPlan(database, plan);
        expect(await database.select().from(taskTwo)).toEqual([
            { id: "a", title: "Plan", urgent: 1, dueAt: null },
        ]);
        expect(review(await planMigration(database, declareState([taskTwo], dialect)))).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "convert rows to a new version and log the converted values on %s",
    async (dialect) => {
        const database = await open(dialect, [taskThree]);
        await migrate(database, [taskTwo]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskTwo))} (id, title, urgent, due_at) VALUES ('a', 'Plan', 1, NULL), ('b', 'Ship', 0, 5)`,
        );
        const before = await database.log.latest();

        // add the priority column, then convert every row through version two
        const plan = await migrate(database, [taskThree]);
        expect(review(plan)).toEqual([
            `safe addColumn ${table(taskThree)}: add column priority`,
            `data-dependent convertRows ${table(taskThree)}: convert rows to version 2`,
        ]);
        expect(await database.select().from(taskThree).orderBy(taskThree.id)).toEqual([
            { id: "a", title: "Plan", urgent: 1, priority: "high", dueAt: null },
            { id: "b", title: "Ship", urgent: 0, priority: "normal", dueAt: 5 },
        ]);

        // record the conversion's update in the log like any other write
        const changes = await database.log.read({ tables: [taskThree], after: before });
        expect(
            changes.changes.map((change) => [change.operation, change.key, change.previous]),
        ).toEqual([["update", { id: "a" }, { priority: "normal" }]]);
    },
);

test.for(TEST_DIALECTS)(
    "name what the declarations must fix instead of planning on %s",
    async (dialect) => {
        const database = await open(dialect, [taskUnfilled]);
        await migrate(database, [taskOne]);
        await database.execute(sql`CREATE TABLE ${sql.identifier(table(labelPlain))} (id TEXT)`);

        // name the unfillable column and the table created outside any plan, all at once
        await expect(
            planMigration(database, declareState([taskUnfilled, labelPlain], dialect)),
        ).rejects.toMatchObject({
            name: "PlanError",
            problems: [
                {
                    target: table(taskUnfilled),
                    detail: "declare a default for the required column owner",
                },
                { target: table(labelPlain), detail: "table exists without applied state" },
            ],
        });
    },
);

/** Read a table's SQL name. */
function table(declaration: Table): string {
    return declaration[TABLE].sqlName;
}

/** Read a declared index's SQL name. */
function indexName(declaration: Table, name: string): string {
    return qualify(declaration[TABLE].package, name);
}

test.for(TEST_DIALECTS)(
    "rebuild a table other tables reference, keeping their rows, on %s",
    async (dialect) => {
        const database = await open(dialect, [folderOne, document]);
        await migrate(database, [folderOne, document]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(folderOne))} (id, extra) VALUES ('f', 'x')`,
        );
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(document))} (id, folder_id) VALUES ('d', 'f')`,
        );

        // drop the referenced table's column and keep the referencing row valid
        await migrate(database, [folderTwo, document]);
        expect(
            await database.execute(
                sql`SELECT id, folder_id FROM ${sql.identifier(table(document))}`,
            ),
        ).toEqual([{ id: "d", folder_id: "f" }]);
        expect(
            await database.execute(sql`SELECT id FROM ${sql.identifier(table(folderTwo))}`),
        ).toEqual([{ id: "f" }]);
    },
);

test.for(TEST_DIALECTS)(
    "report a failing migration statement as a failed migration on %s",
    async (dialect) => {
        const database = await open(dialect, [labelPlain]);
        await migrate(database, [labelPlain]);
        const name = sql.identifier(table(labelPlain));
        await database.execute(sql`INSERT INTO ${name} (id, code) VALUES ('a', 'x'), ('b', 'x')`);

        // fail to make duplicated codes unique, keeping the rows and the applied state
        await expect(migrate(database, [labelUnique])).rejects.toMatchObject({
            code: "MIGRATION_FAILED",
        });
        expect(await database.execute(sql`SELECT id FROM ${name} ORDER BY id`)).toEqual([
            { id: "a" },
            { id: "b" },
        ]);
    },
);

test.for(TEST_DIALECTS)("make a column unique and enforce it on %s", async (dialect) => {
    const database = await open(dialect, [labelPlain]);
    await migrate(database, [labelPlain]);

    // add the column's unique constraint, which depends on the existing rows
    const plan = await planMigration(database, declareState([labelUnique], dialect));
    expect(review(plan)).toEqual(
        dialect === "sqlite"
            ? [`data-dependent rebuildTable ${table(labelUnique)}: rebuild table: constraints`]
            : [
                  `data-dependent alterConstraint ${table(labelUnique)}: add constraint ${table(labelUnique)}_code_unique`,
              ],
    );
    await applyPlan(database, plan);

    // hold nothing more to apply, and keep one row per code
    expect(review(await planMigration(database, declareState([labelUnique], dialect)))).toEqual([]);
    const name = sql.identifier(table(labelUnique));
    await database.execute(
        sql`INSERT INTO ${name} (id, code) VALUES ('a', 'x'), ('b', 'x') ON CONFLICT DO NOTHING`,
    );
    expect(await database.execute(sql`SELECT id, code FROM ${name}`)).toEqual([
        { id: "a", code: "x" },
    ]);
});

test.for(TEST_DIALECTS)(
    "plan the union of declarations sharing a database on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);
        const state = (tables: readonly Table[]) =>
            defineDatabase({ name: "main", tables }).state();

        // create a shared table once when two declarations require it identically
        const plan = await planStates(database, [state([taskOne]), state([taskOne])]);
        expect(review(plan)).toEqual([`safe createTable ${table(taskOne)}: create table`]);
        await applyPlan(database, plan);

        // refuse a table two releases declare with different column types, naming the column
        await expect(
            planStates(database, [state([taskOne]), state([taskText])]),
        ).rejects.toMatchObject({
            name: "PlanError",
            problems: [{ target: table(taskOne), detail: "releases disagree on column urgent" }],
        });
    },
);

test.for(TEST_DIALECTS)(
    "expand for two running releases, bridge a renamed column, then contract on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);
        const state = (tables: readonly Table[]) =>
            defineDatabase({
                name: "main",
                tables,
            }).state();
        await migrate(database, [taskOne]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, name, urgent, note) VALUES ('a', 'Plan', 1, 'old')`,
        );

        // add the new release's columns beside the old release's, relax the old title it omits, and bridge them
        const expand = await planStates(database, [state([taskOne]), state([taskTwo])]);
        expect(review(expand)).toEqual(
            dialect === "sqlite"
                ? [
                      `safe rebuildTable ${table(taskTwo)}: rebuild table: add title, add due_at, change name`,
                      `data-dependent bridgeColumn ${table(taskTwo)}: copy name into title`,
                  ]
                : [
                      `safe addColumn ${table(taskTwo)}: add column title`,
                      `safe addColumn ${table(taskTwo)}: add column due_at`,
                      `safe alterColumn ${table(taskTwo)}: change column name`,
                      `safe createIndex ${table(taskTwo)}: create index ${indexName(taskTwo, "task_due")}`,
                      `data-dependent bridgeColumn ${table(taskTwo)}: copy name into title`,
                  ],
        );
        expect(review(await planStates(database, [state([taskTwo]), state([taskOne])]))).toEqual(
            review(expand),
        );
        await applyPlan(database, expand);

        // let both releases connect to the expanded table
        const release = (tables: readonly Table[]) => defineDatabase({ name: "main", tables });
        expect(await release([taskOne]).check(database)).toEqual([]);
        expect(await release([taskTwo]).check(database)).toEqual([]);

        // keep both names equal for writes from either release
        const name = sql.identifier(table(taskOne));
        await database.execute(sql`INSERT INTO ${name} (id, name, urgent) VALUES ('b', 'Old', 0)`);
        await database.execute(sql`INSERT INTO ${name} (id, title, urgent) VALUES ('c', 'New', 0)`);
        await database.execute(sql`UPDATE ${name} SET title = 'Plan again' WHERE id = 'a'`);
        expect(
            await database.execute(sql`SELECT id, name, title FROM ${name} ORDER BY id`),
        ).toEqual([
            { id: "a", name: "Plan again", title: "Plan again" },
            { id: "b", name: "Old", title: "Old" },
            { id: "c", name: "New", title: "New" },
        ]);

        // drop the old release's columns once only the new release runs
        const contract = await planStates(database, [state([taskTwo])]);
        expect(review(contract)).toEqual(
            dialect === "sqlite"
                ? [
                      `destructive rebuildTable ${table(taskTwo)}: rebuild table: drop name, drop note, change title`,
                  ]
                : [
                      `destructive dropColumn ${table(taskTwo)}: drop column name`,
                      `destructive dropColumn ${table(taskTwo)}: drop column note`,
                      `data-dependent alterColumn ${table(taskTwo)}: change column title`,
                  ],
        );
        await applyPlan(database, contract);
        expect(await release([taskOne]).check(database)).toEqual([table(taskOne)]);
        expect(await release([taskTwo]).check(database)).toEqual([]);
        expect(await database.execute(sql`SELECT id, title FROM ${name} ORDER BY id`)).toEqual([
            { id: "a", title: "Plan again" },
            { id: "b", title: "Old" },
            { id: "c", title: "New" },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "name declared tables a database has not applied on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);
        const declared = defineDatabase({
            name: "main",
            tables: [taskOne],
        });

        // report the missing table, then nothing once applied
        expect(await declared.check(database)).toEqual([table(taskOne)]);
        await migrate(database, [taskOne]);
        expect(await declared.check(database)).toEqual([]);
    },
);
