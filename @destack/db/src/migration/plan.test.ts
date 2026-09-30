import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, index, integer, json, sql, text } from "../index.ts";
import { schema } from "@destack/schema";
import { TABLE, type Table } from "../table/table.ts";
import { qualify } from "../table/namespace.ts";
import { defineDatabase } from "../declare/database.ts";
import { mergeStates } from "./merge.ts";
import { applyPlan } from "./apply.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { declareState } from "./state.ts";
import type { TablePlan } from "./plan.ts";
import { Expression } from "../expression/expression.ts";
import { PlanError } from "@destack/resource/error";

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

/** This package's next release, declaring the converted tasks. */
const NEXT_RELEASE = { package: { ...taskOne[TABLE].package, version: "2026.10.0" } };

/** Tasks after converting the urgent flag into a priority, in the next release. */
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
        convert: {
            "2026.10.0": {
                priority: Expression.case(
                    Expression.column("urgent"),
                    [{ when: 1, then: Expression.literal("high") }],
                    Expression.literal("normal"),
                ),
            },
        },
        constraints: (task) => [index("task_due").on(task.dueAt)],
    },
    NEXT_RELEASE,
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
    return plan.steps.map((step) => `${step.risk} ${step.action} ${step.target}: ${step.detail}`);
}

test.for(TEST_DIALECTS)(
    "create declared tables once and plan nothing afterwards on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);

        // create the table, then find the database current
        const plan = await database.migrate([taskOne]);
        expect(review(plan)).toEqual([`safe create table/${table(taskOne)}: create table`]);
        expect(review(await database.plan({ declared: declareState([taskOne], dialect) }))).toEqual(
            [],
        );
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
        const plan = await database.plan({ declared: declareState([taskTwo], dialect) });
        expect(review(plan)).toEqual(
            dialect === "sqlite"
                ? [
                      `backward-incompatible rename table/${table(taskTwo)}/column/title: rename column name to title`,
                      `destructive replace table/${table(taskTwo)}: rebuild table: add due_at, drop note`,
                  ]
                : [
                      `backward-incompatible rename table/${table(taskTwo)}/column/title: rename column name to title`,
                      `safe create table/${table(taskTwo)}/column/due_at: add column due_at`,
                      `destructive delete table/${table(taskTwo)}/column/note: drop column note`,
                      `safe create table/${table(taskTwo)}/index/${indexName(taskTwo, "task_due")}: create index ${indexName(taskTwo, "task_due")}`,
                  ],
        );

        // keep the row under the renamed column
        await applyPlan(database, plan);
        expect(await database.select().from(taskTwo)).toEqual([
            { id: "a", scope: "inbox", title: "Plan", urgent: 1, dueAt: null },
        ]);
        expect(review(await database.plan({ declared: declareState([taskTwo], dialect) }))).toEqual(
            [],
        );
    },
);

test.for(TEST_DIALECTS)(
    "convert rows to a new release and log the converted values on %s",
    async (dialect) => {
        const database = await open(dialect, [taskThree]);
        await database.migrate([taskTwo]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskTwo))} (id, scope, title, urgent, due_at) VALUES ('a', 'inbox', 'Plan', 1, NULL), ('b', 'inbox', 'Ship', 0, 5)`,
        );
        const before = (await database.log.position()).sequence;

        // convert every row by the next release's conversion
        const plan = await database.migrate([taskThree]);
        expect(review(plan)).toEqual([
            `safe create table/${table(taskThree)}/column/priority: add column priority`,
            `data-dependent convert table/${table(taskThree)}: convert rows to 2026.10.0`,
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
            database.plan({ declared: declareState([taskUnfilled, labelPlain], dialect) }),
        ).rejects.toMatchObject({
            name: "PlanError",
            problems: [
                {
                    target: `table/${table(taskUnfilled)}`,
                    detail: "declare a default for the required column owner",
                },
                {
                    target: `table/${table(labelPlain)}`,
                    detail: "table exists without applied state",
                },
            ],
        });
    },
);

/** Preferences whose mode takes three values. */
const preferenceOne = defineTable("preference", {
    id: text("id").primaryKey(),
    mode: json("mode", schema.enum(["standard", "vim", "emacs"])),
});

/** Preferences in the next release, whose mode also takes a fourth value. */
const preferenceWide = defineTable(
    "preference",
    {
        id: text("id").primaryKey(),
        mode: json("mode", schema.enum(["standard", "vim", "emacs", "helix"])),
    },
    {},
    NEXT_RELEASE,
);

/** Preferences in the next release, dropping the emacs mode without a conversion. */
const preferenceNarrow = defineTable(
    "preference",
    { id: text("id").primaryKey(), mode: json("mode", schema.enum(["standard", "vim"])) },
    {},
    NEXT_RELEASE,
);

/** Preferences in the next release, converting the emacs mode to the standard one. */
const preferenceConverted = defineTable(
    "preference",
    { id: text("id").primaryKey(), mode: json("mode", schema.enum(["standard", "vim"])) },
    {
        convert: {
            "2026.10.0": {
                mode: Expression.case(
                    Expression.scalar(Expression.column("mode"), "text"),
                    [{ when: "emacs", then: Expression.json("standard") }],
                    Expression.column("mode"),
                ),
            },
        },
    },
    NEXT_RELEASE,
);

test.for(TEST_DIALECTS)(
    "widen column values freely and narrow them only by a conversion on %s",
    async (dialect) => {
        const database = await open(dialect, [preferenceConverted]);
        await database.migrate([preferenceOne]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(preferenceOne))} (id, mode) VALUES ('a', '"emacs"'), ('b', '"vim"')`,
        );

        // accept the widened mode, refuse the unconverted narrowing, and convert the declared one
        const wide = await database.plan({ declared: declareState([preferenceWide], dialect) });
        expect(review(wide)).toEqual([
            `safe update table/${table(preferenceWide)}/column/mode: wider values`,
        ]);
        await expect(
            database.plan({ declared: declareState([preferenceNarrow], dialect) }),
        ).rejects.toMatchObject({
            name: "PlanError",
            problems: [
                {
                    target: `table/${table(preferenceNarrow)}/column/mode`,
                    detail: "declare a conversion for 2026.10.0",
                },
            ],
        });
        const converted = await database.migrate([preferenceConverted]);
        expect(review(converted)).toEqual([
            `data-dependent convert table/${table(preferenceConverted)}: convert rows to 2026.10.0`,
        ]);
        expect(
            await database.select().from(preferenceConverted).orderBy(preferenceConverted.id),
        ).toEqual([
            { id: "a", mode: "standard" },
            { id: "b", mode: "vim" },
        ]);
    },
);

/** Themes the next release adds. */
const themeTable = defineTable("theme", { id: text("id").primaryKey() }, {}, NEXT_RELEASE);

/** Folders the next release renames to directories. */
const directoryTable = defineTable(
    "directory",
    { id: text("id").primaryKey(), extra: text("extra") },
    { moved: { table: "folder" } },
    NEXT_RELEASE,
);

/** Preferences in the next release, adding a theme beside the mode. */
const preferenceThemed = defineTable(
    "preference",
    {
        id: text("id").primaryKey(),
        mode: json("mode", schema.enum(["standard", "vim", "emacs"])),
        theme: text("theme"),
    },
    {},
    NEXT_RELEASE,
);

test.for(TEST_DIALECTS)(
    "roll back without contracting what a newer release added, and refuse tables the older release cannot read, on %s",
    async (dialect) => {
        // apply the next release adding a theme column and a theme table, then roll back to the first
        const database = await open(dialect, [preferenceOne]);
        await database.migrate([preferenceThemed, themeTable]);
        const rolledBack = await database.migrate([preferenceOne]);

        // refuse rolling back across a renamed table, which would create it again empty
        const moved = await open(dialect, [folderOne]);
        await moved.migrate([directoryTable]);
        const renamed = await moved.migrate([folderOne]).then(
            () => [],
            (error: PlanError) => error.problems,
        );

        // refuse rolling back across widened mode values the first release cannot read
        const widened = await open(dialect, [preferenceOne]);
        await widened.migrate([preferenceWide]);
        const refused = await widened.migrate([preferenceOne]).then(
            () => [],
            (error: PlanError) => error.problems,
        );

        // keep the theme column and table, since dropping them would lose them on the next upgrade
        expect({ rolledBack: review(rolledBack), refused, renamed }).toEqual({
            rolledBack: [],
            refused: [
                {
                    target: `table/${table(preferenceOne)}`,
                    detail: `rollback to 2026.9.0 cannot hold the table as ${NEXT_RELEASE.package.version} applied it`,
                },
            ],
            renamed: [
                {
                    target: `table/${table(folderOne)}`,
                    detail: `rollback to 2026.9.0 cannot read the table ${NEXT_RELEASE.package.version} renamed to ${table(directoryTable)}`,
                },
            ],
        });
    },
);

test("refuse conversions keyed by anything but a release up to the declaring one", () => {
    const refusal = (release: string) => {
        try {
            defineTable(
                "draft",
                { id: text("id").primaryKey() },
                { convert: { [release]: {} } },
                NEXT_RELEASE,
            );
        } catch (error) {
            return (error as Error).message;
        }
    };

    expect([refusal("2026.10.0"), refusal("2026.11.0"), refusal("next")]).toEqual([
        undefined,
        "conversion of draft is keyed by 2026.11.0, after its release 2026.10.0",
        "conversion key of draft is no release: next",
    ]);
});

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
    const plan = await database.plan({ declared: declareState([labelUnique], dialect) });
    expect(review(plan)).toEqual(
        dialect === "sqlite"
            ? [`data-dependent replace table/${table(labelUnique)}: rebuild table: constraints`]
            : [
                  `data-dependent create table/${table(labelUnique)}/constraint/${table(labelUnique)}_code_unique: add constraint ${table(labelUnique)}_code_unique`,
              ],
    );
    await applyPlan(database, plan);

    // plan nothing more
    expect(review(await database.plan({ declared: declareState([labelUnique], dialect) }))).toEqual(
        [],
    );
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
            defineDatabase({ name: "main", tables }).state().tables[dialect];

        // create a shared table once
        const plan = await database.plan(mergeStates([state([taskOne]), state([taskOne])]));
        expect(review(plan)).toEqual([`safe create table/${table(taskOne)}: create table`]);
        await applyPlan(database, plan);

        // refuse conflicting column types
        await expect(
            database.plan(mergeStates([state([taskOne]), state([taskText])])),
        ).rejects.toMatchObject({
            name: "PlanError",
            problems: [
                { target: `table/${table(taskOne)}`, detail: "releases disagree on column urgent" },
            ],
        });
    },
);

test.for(TEST_DIALECTS)(
    "expand for two running releases, bridge a renamed column, then contract on %s",
    async (dialect) => {
        const database = await open(dialect, [taskOne]);
        const state = (tables: readonly Table[]) =>
            defineDatabase({ name: "main", tables }).state().tables[dialect];
        await database.migrate([taskOne]);
        await database.execute(
            sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, scope, name, urgent, note) VALUES ('a', 'inbox', 'Plan', 1, 'old')`,
        );

        // expand the table for both releases and bridge the renamed column
        const expand = await database.plan(mergeStates([state([taskOne]), state([taskTwo])]));
        expect(review(expand)).toEqual(
            dialect === "sqlite"
                ? [
                      `safe replace table/${table(taskTwo)}: rebuild table: add title, add due_at, change name`,
                      `data-dependent convert table/${table(taskTwo)}/column/title: copy name into title`,
                  ]
                : [
                      `safe create table/${table(taskTwo)}/column/title: add column title`,
                      `safe create table/${table(taskTwo)}/column/due_at: add column due_at`,
                      `safe update table/${table(taskTwo)}/column/name: change column name`,
                      `safe create table/${table(taskTwo)}/index/${indexName(taskTwo, "task_due")}: create index ${indexName(taskTwo, "task_due")}`,
                      `data-dependent convert table/${table(taskTwo)}/column/title: copy name into title`,
                  ],
        );
        expect(
            review(await database.plan(mergeStates([state([taskTwo]), state([taskOne])]))),
        ).toEqual(review(expand));
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
        const contract = await database.plan(mergeStates([state([taskTwo])]));
        expect(review(contract)).toEqual(
            dialect === "sqlite"
                ? [
                      `destructive replace table/${table(taskTwo)}: rebuild table: drop name, drop note, change title`,
                  ]
                : [
                      `destructive delete table/${table(taskTwo)}/column/name: drop column name`,
                      `destructive delete table/${table(taskTwo)}/column/note: drop column note`,
                      `data-dependent update table/${table(taskTwo)}/column/title: change column title`,
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

        // migrate a database with documents but without their folders
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
