import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, index, integer, sql, text } from "../index.ts";
import { TABLE, type Table } from "../table/table.ts";
import { qualify } from "../table/namespace.ts";
import { defineDatabase } from "../declare/database.ts";
import { planMigration, planStates } from "./database.ts";
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
const taskOne = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        name: text("name").notNull(),
        urgent: integer("urgent").notNull(),
        note: text("note"),
    },
    { log: {} },
);

/** Tasks after renaming name to title, dropping the note and adding a due time and index. */
const taskTwo = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        title: text("title").notNull(),
        urgent: integer("urgent").notNull(),
        dueAt: integer("due_at"),
    },
    {
        log: {},
        moved: { columns: { title: "name" } },
        constraints: (task) => [index("task_due").on(task.dueAt)],
    },
);

/** Tasks after converting the urgent flag into a priority. */
const taskThree = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        title: text("title").notNull(),
        urgent: integer("urgent").notNull(),
        priority: text("priority").notNull().default("normal"),
        dueAt: integer("due_at"),
    },
    {
        log: {},
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

/** Tasks with an unfillable required column. */
const taskUnfilled = defineTable("task", {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    urgent: integer("urgent").notNull(),
    note: text("note"),
    owner: text("owner").notNull(),
});

/** Open an empty test database. */
async function open(dialect: Dialect, tables: readonly Table[]) {
    const test = await TestDatabase.create(dialect, tables);
    onTestFinished(() => test.close());

    return test.database;
}

/** Summarize a plan's steps. */
function review(plan: TablePlan) {
    return plan.steps.map((step) => `${step.risk} ${step.kind} ${step.target}: ${step.detail}`);
}

test.for(TEST_DIALECTS)(
    "create declared tables once and plan nothing afterwards on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);

        // create the table, then find the database current
        const plan = await database.migrate([taskOne]);
        expect(review(plan)).toEqual([`safe createTable ${table(taskOne)}: create table`]);
        expect(review(await planMigration(database, declareState([taskOne], dialect)))).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "rename, drop, add and index columns while keeping rows on %s",
    async (dialect) => {
        const database = await open(dialect, [taskTwo]);
        await database.migrate([taskOne]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, scope, name, urgent, note) VALUES ('a', 'inbox', 'Plan', 1, 'old')`,
        );

        // classify the rename, addition and dropped note
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

        // keep the row under the renamed column
        await applyPlan(database, plan);
        expect(await database.select().from(taskTwo)).toEqual([
            { id: "a", scope: "inbox", title: "Plan", urgent: 1, dueAt: null },
        ]);
        expect(review(await planMigration(database, declareState([taskTwo], dialect)))).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "convert rows to a new version and log the converted values on %s",
    async (dialect) => {
        const database = await open(dialect, [taskThree]);
        await database.migrate([taskTwo]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskTwo))} (id, scope, title, urgent, due_at) VALUES ('a', 'inbox', 'Plan', 1, NULL), ('b', 'inbox', 'Ship', 0, 5)`,
        );
        const before = (await database.log.position()).sequence;

        // convert every row through version two
        const plan = await database.migrate([taskThree]);
        expect(review(plan)).toEqual([
            `safe addColumn ${table(taskThree)}: add column priority`,
            `data-dependent convertRows ${table(taskThree)}: convert rows to version 2`,
        ]);
        expect(await database.select().from(taskThree).orderBy(taskThree.id)).toEqual([
            { id: "a", scope: "inbox", title: "Plan", urgent: 1, priority: "high", dueAt: null },
            { id: "b", scope: "inbox", title: "Ship", urgent: 0, priority: "normal", dueAt: 5 },
        ]);

        // log the conversion's update
        const changes = await database.log.read({ tables: [taskThree], after: before });
        expect(
            changes.changes.map((change) => [
                change.operation,
                change.key,
                change.before?.priority,
                change.after?.priority,
            ]),
        ).toEqual([["update", { id: "a" }, "normal", "high"]]);
    },
);

test.for(TEST_DIALECTS)(
    "name what the declarations must fix instead of planning on %s",
    async (dialect) => {
        const database = await open(dialect, [taskUnfilled]);
        await database.migrate([taskOne]);
        await database.execute(sql`CREATE TABLE ${sql.identifier(table(labelPlain))} (id TEXT)`);

        // name the unfillable column and the unmanaged table
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
        await database.migrate([folderOne, document]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(folderOne))} (id, extra) VALUES ('f', 'x')`,
        );
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(document))} (id, folder_id) VALUES ('d', 'f')`,
        );

        // drop the referenced column and keep the reference valid
        await database.migrate([folderTwo, document]);
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
        await database.migrate([labelPlain]);
        const name = sql.identifier(table(labelPlain));
        await database.execute(sql`INSERT INTO ${name} (id, code) VALUES ('a', 'x'), ('b', 'x')`);

        // fail to make duplicate codes unique and keep the state
        await expect(database.migrate([labelUnique])).rejects.toMatchObject({
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
    await database.migrate([labelPlain]);

    // add the unique constraint
    const plan = await planMigration(database, declareState([labelUnique], dialect));
    expect(review(plan)).toEqual(
        dialect === "sqlite"
            ? [`data-dependent rebuildTable ${table(labelUnique)}: rebuild table: constraints`]
            : [
                  `data-dependent alterConstraint ${table(labelUnique)}: add constraint ${table(labelUnique)}_code_unique`,
              ],
    );
    await applyPlan(database, plan);

    // plan nothing more
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

        // create a shared table once
        const plan = await planStates(database, [state([taskOne]), state([taskOne])]);
        expect(review(plan)).toEqual([`safe createTable ${table(taskOne)}: create table`]);
        await applyPlan(database, plan);

        // refuse conflicting column types
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
        await database.migrate([taskOne]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, scope, name, urgent, note) VALUES ('a', 'inbox', 'Plan', 1, 'old')`,
        );

        // expand the table for both releases and bridge the renamed column
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

        // connect both releases
        const release = (tables: readonly Table[]) => defineDatabase({ name: "main", tables });
        expect(await release([taskOne]).check(database)).toEqual([]);
        expect(await release([taskTwo]).check(database)).toEqual([]);

        // keep both names equal
        const name = sql.identifier(table(taskOne));
        await database.execute(
            sql`INSERT INTO ${name} (id, scope, name, urgent) VALUES ('b', 'inbox', 'Old', 0)`,
        );
        await database.execute(
            sql`INSERT INTO ${name} (id, scope, title, urgent) VALUES ('c', 'inbox', 'New', 0)`,
        );
        await database.execute(sql`UPDATE ${name} SET title = 'Plan again' WHERE id = 'a'`);
        expect(
            await database.execute(sql`SELECT id, name, title FROM ${name} ORDER BY id`),
        ).toEqual([
            { id: "a", name: "Plan again", title: "Plan again" },
            { id: "b", name: "Old", title: "Old" },
            { id: "c", name: "New", title: "New" },
        ]);

        // drop the old release's columns
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

        // report the missing table until applied
        expect(await declared.check(database)).toEqual([table(taskOne)]);
        await database.migrate([taskOne]);
        expect(await declared.check(database)).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "keep foreign keys to held tables and leave references elsewhere logical on %s",
    async (dialect) => {
        // describe documents with and without their folders in the same database
        const keys = (tables: readonly Table[]) =>
            declareState(tables, dialect)
                .find((state) => state.table.name === document[TABLE].sqlName)!
                .table.constraints.filter((constraint) => constraint.kind === "foreignKey")
                .map((constraint) => [constraint.kind, constraint.columns, constraint.table]);

        // migrate a database holding documents alone, whose folders live elsewhere
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "documents", tables: [document] }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await storage.database.insert(document).values({ id: "d1", folderId: "elsewhere" });

        expect([keys([folderTwo, document]), keys([document])]).toEqual([
            [["foreignKey", ["folder_id"], folderTwo[TABLE].sqlName]],
            [],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "let replicas leave out the sensitive columns their log never carries on %s",
    (dialect) => {
        // declare a required secret beside a required name
        const credential = defineTable("credential", {
            id: text("id").primaryKey(),
            name: text("name").notNull(),
            secret: text("secret").notNull().sensitive(),
        });
        const nullable = (isReplica: boolean) =>
            declareState([credential], dialect, { isReplica })[0]!.table.columns.map((column) => [
                column.name,
                column.nullable,
            ]);

        // keep the secret required where it is held, and optional in a replica
        expect([nullable(false), nullable(true)]).toEqual([
            [
                ["id", false],
                ["name", false],
                ["secret", false],
            ],
            [
                ["id", false],
                ["name", false],
                ["secret", true],
            ],
        ]);
    },
);
