import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Change, defineTable, index, integer, json, sql, text, uniqueIndex } from "../index.ts";
import { Package } from "@destack/package";
import { present, schema } from "@destack/schema";
import { TABLE, type Table } from "../table/table.ts";
import { qualify } from "../table/namespace.ts";
import { defineDatabase } from "../declare/database.ts";
import { mergeStates } from "./merge.ts";
import { applyPlan } from "./apply.ts";
import { TestDatabase } from "../test/database.ts";
import { declareState, STATE } from "./state.ts";
import type { TablePlan } from "./plan.ts";
import { Expression } from "../expression/expression.ts";
import { ResourceError } from "@destack/resource/error";
import { testSimulation } from "@destack/test/world";

/** Folders with documents, as first released. */
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

/** Another package's release, declaring its documents' attachments. */
const ATTACHMENT_RELEASE = {
    package: Package.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000f1",
        name: "@acme/attachment",
        version: "2026.10.0",
    }),
};

/** Attachments of another package, referencing their document. */
const attachment = defineTable(
    "attachment",
    {
        id: text("id").primaryKey(),
        documentId: text("document_id")
            .notNull()
            .references(() => document.id),
    },
    {},
    ATTACHMENT_RELEASE,
);

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

/** Units with the predefined rows every database has, in a logged table. */
const unit = defineTable(
    "unit",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        name: text("name").notNull(),
        factor: integer("factor").notNull(),
        aliases: json("aliases", schema.array(schema.string())).notNull(),
    },
    {
        log: {},
        predefined: [
            { id: "metre", scope: "si", name: "Metre", factor: 1, aliases: ["m"] },
            { id: "kilometre", scope: "si", name: "Kilometre", factor: 1000, aliases: ["km"] },
        ],
    },
);

/** Open an empty test database. */
async function open(tables: readonly Table[]) {
    const storage = await TestDatabase.create(tables, {
        environment: testSimulation().environment("test"),
    });
    onTestFinished(() => storage.close());

    return storage.database;
}

/** Summarize a plan's steps. */
function review(plan: TablePlan) {
    return plan.steps.map((step) => `${step.risk} ${step.action} ${step.target}: ${step.detail}`);
}

/** Declare one release of the main database over its tables. */
function release(tables: readonly Table[]) {
    return defineDatabase({ name: "main", tables });
}

test("create declared tables once and plan nothing afterwards", async () => {
    const database = await open([taskOne]);

    // create the table, then find the database current
    const plan = await database.migrate([taskOne]);
    expect(review(plan)).toEqual([`safe create table/${table(taskOne)}: create table`]);
    expect(
        review(await database.plan({ declared: declareState([taskOne], TestDatabase.target()) })),
    ).toEqual([]);
});

test("write a table's predefined rows in a fresh database, unlogged, and plan nothing afterwards", async () => {
    const database = await open([unit]);

    // create the table and upsert its predefined rows
    const plan = await database.migrate([unit]);
    expect(review(plan)).toEqual([
        `safe create table/${table(unit)}: create table`,
        `safe update table/${table(unit)}/predefined: keep 2 predefined rows`,
    ]);
    expect(await database.select().from(unit).orderBy(unit.id)).toEqual([
        { id: "kilometre", scope: "si", name: "Kilometre", factor: 1000, aliases: ["km"] },
        { id: "metre", scope: "si", name: "Metre", factor: 1, aliases: ["m"] },
    ]);

    // log only rows written after, and plan nothing once migrated
    await database.transaction((transaction) =>
        transaction
            .insert(unit)
            .values({
                id: "mile",
                scope: "imperial",
                name: "Mile",
                factor: 1609,
                aliases: ["mi"],
            })
            .execute(),
    );
    const changes = await database.log.read({ tables: [unit], after: 0 });
    expect(changes.changes.map((change) => change.key)).toEqual([{ id: "mile" }]);
    expect(
        review(await database.plan({ declared: declareState([unit], TestDatabase.target()) })),
    ).toEqual([]);
});

test("retain the applied tables a migration leaves out when asked, and drop them otherwise", async () => {
    const database = await open([]);
    await database.migrate([labelPlain, taskOne]);

    // change the labels and retain the tasks
    const kept = await database.migrate([labelUnique], { others: "retain" });
    expect(review(kept)).toEqual([
        TestDatabase.target() === "sqlite"
            ? `fallible replace table/${table(labelUnique)}: rebuild table: constraints`
            : `fallible create table/${table(labelUnique)}/constraint/${table(labelUnique)}_code_unique: add constraint ${table(labelUnique)}_code_unique`,
    ]);
    expect(await database.unapplied([taskOne, labelUnique])).toEqual([]);

    // drop the tasks a migration leaves out by default
    const dropped = await database.migrate([labelUnique]);
    expect(review(dropped)).toEqual([`destructive delete table/${table(taskOne)}: drop table`]);
});

test("rename, drop, add and index columns while preserving rows", async () => {
    const database = await open([taskTwo]);
    await database.migrate([taskOne]);
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, scope, name, urgent, note) VALUES ('a', 'inbox', 'Plan', 1, 'old')`,
            ),
    );

    // classify the rename, addition and dropped note
    const plan = await database.plan({ declared: declareState([taskTwo], TestDatabase.target()) });
    expect(review(plan)).toEqual(
        TestDatabase.target() === "sqlite"
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

    // move the row's value to the renamed column
    await applyPlan(database, plan);
    expect(await database.select().from(taskTwo)).toEqual([
        { id: "a", scope: "inbox", title: "Plan", urgent: 1, dueAt: null },
    ]);
    expect(
        review(await database.plan({ declared: declareState([taskTwo], TestDatabase.target()) })),
    ).toEqual([]);
});

test("convert rows to a new release and log the converted values", async () => {
    const database = await open([taskThree]);
    await database.migrate([taskTwo]);
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`INSERT INTO ${sql.identifier(table(taskTwo))} (id, scope, title, urgent, due_at) VALUES ('a', 'inbox', 'Plan', 1, NULL), ('b', 'inbox', 'Ship', 0, 5)`,
            ),
    );
    const before = (await database.log.position()).sequence;

    // convert every row by the next release's conversion
    const plan = await database.migrate([taskThree]);
    expect(review(plan)).toEqual([
        `safe create table/${table(taskThree)}/column/priority: add column priority`,
        `fallible convert table/${table(taskThree)}: convert rows to 2026.10.0`,
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
            Change.before(change)?.priority,
            Change.after(change)?.priority,
        ]),
    ).toEqual([["update", { id: "a" }, "normal", "high"]]);
});

test("name what the declarations must fix instead of planning", async () => {
    const database = await open([taskUnfilled]);
    await database.migrate([taskOne]);
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, scope, name, urgent) VALUES ('t', 's', 'Plan', 0)`,
            ),
    );
    await database.run(sql`CREATE TABLE ${sql.identifier(table(labelPlain))} (id TEXT)`);

    // report the column no task row fills and the unmanaged table
    expect(
        await refusal(
            database.plan({
                declared: declareState([taskUnfilled, labelPlain], TestDatabase.target()),
            }),
        ),
    ).toEqual([
        "CONFLICT",
        "PLAN_UNREACHABLE",
        "table/destack__db__task: declare a default for the required column owner; table/destack__db__label: table exists without applied state",
    ]);
});

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

test("widen column values freely and narrow them only by a conversion", async () => {
    const database = await open([preferenceConverted]);
    await database.migrate([preferenceOne]);
    await database.run(
        sql`INSERT INTO ${sql.identifier(table(preferenceOne))} (id, mode) VALUES ('a', '"emacs"'), ('b', '"vim"')`,
    );

    // accept the widened mode, refuse the unconverted narrowing and convert the declared one
    const wide = await database.plan({
        declared: declareState([preferenceWide], TestDatabase.target()),
    });
    expect(review(wide)).toEqual([
        `safe update table/${table(preferenceWide)}/column/mode: wider values`,
    ]);
    expect(
        await refusal(
            database.plan({ declared: declareState([preferenceNarrow], TestDatabase.target()) }),
        ),
    ).toEqual([
        "CONFLICT",
        "PLAN_UNREACHABLE",
        "table/destack__db__preference/column/mode: declare a conversion for 2026.10.0",
    ]);
    const converted = await database.migrate([preferenceConverted]);
    expect(review(converted)).toEqual([
        `fallible convert table/${table(preferenceConverted)}: convert rows to 2026.10.0`,
    ]);
    expect(
        await database.select().from(preferenceConverted).orderBy(preferenceConverted.id),
    ).toEqual([
        { id: "a", mode: "standard" },
        { id: "b", mode: "vim" },
    ]);
});

/** Layouts whose settings name a theme. */
const layoutOne = defineTable("layout", {
    id: text("id").primaryKey(),
    settings: json("settings", schema.object({ theme: schema.string() })),
});

/** Layouts in the next release, whose settings require a size the stored rows lack. */
const layoutSized = defineTable(
    "layout",
    {
        id: text("id").primaryKey(),
        settings: json(
            "settings",
            schema.object({ theme: schema.string(), size: schema.number() }),
        ),
    },
    {},
    NEXT_RELEASE,
);

test("refuse requiring a key stored values lack without a conversion, reading rows as stored", async () => {
    const database = await open([layoutSized]);
    await database.migrate([layoutOne]);
    await database.run(
        sql`INSERT INTO ${sql.identifier(table(layoutOne))} (id, settings) VALUES ('a', '{"theme":"dark"}')`,
    );

    // refuse the key every stored row lacks, and read a row exactly as stored
    expect(
        await refusal(
            database.plan({ declared: declareState([layoutSized], TestDatabase.target()) }),
        ),
    ).toEqual([
        "CONFLICT",
        "PLAN_UNREACHABLE",
        `table/${table(layoutSized)}/column/settings: declare a conversion for 2026.10.0`,
    ]);
    expect(await database.select().from(layoutOne)).toEqual([
        { id: "a", settings: { theme: "dark" } },
    ]);
});

/** Preferences in the next release, whose conversion writes a mode the column refuses. */
const preferenceMisconverted = defineTable(
    "preference",
    { id: text("id").primaryKey(), mode: json("mode", schema.enum(["standard", "vim"])) },
    {
        convert: {
            "2026.10.0": {
                mode: Expression.case(
                    Expression.scalar(Expression.column("mode"), "text"),
                    [{ when: "emacs", then: Expression.json("nano") }],
                    Expression.column("mode"),
                ),
            },
        },
    },
    NEXT_RELEASE,
);

test("refuse a conversion writing a value its column refuses, leaving the rows as they were", async () => {
    const database = await open([preferenceMisconverted]);
    await database.migrate([preferenceOne]);
    await database.run(
        sql`INSERT INTO ${sql.identifier(table(preferenceOne))} (id, mode) VALUES ('a', '"emacs"')`,
    );

    // refuse the conversion loudly and roll it back
    expect(await refusal(database.migrate([preferenceMisconverted]))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "MIGRATION_FAILED",
        `conversion writes a value that ${table(preferenceMisconverted)}.mode rejects: ✖ Invalid option: expected one of "standard"|"vim"`,
    ]);
    expect(
        await database.execute(
            sql`SELECT mode FROM ${sql.identifier(table(preferenceOne))}`,
            schema.object({ mode: schema.unknown() }),
        ),
    ).toEqual([{ mode: TestDatabase.target() === "sqlite" ? '"emacs"' : "emacs" }]);
});

/** Preferences rebuilt within the first release, converting the emacs mode it drops. */
const preferenceRebuilt = defineTable(
    "preference",
    { id: text("id").primaryKey(), mode: json("mode", schema.enum(["standard", "vim"])) },
    {
        convert: {
            [preferenceOne[TABLE].package.version]: {
                mode: Expression.case(
                    Expression.scalar(Expression.column("mode"), "text"),
                    [{ when: "emacs", then: Expression.json("standard") }],
                    Expression.column("mode"),
                ),
            },
        },
    },
);

test("convert rows by the conversion a rebuilt release adds, once", async () => {
    const database = await open([preferenceRebuilt]);
    await database.migrate([preferenceOne]);
    await database.run(
        sql`INSERT INTO ${sql.identifier(table(preferenceOne))} (id, mode) VALUES ('a', '"emacs"'), ('b', '"vim"')`,
    );

    // convert the rows the earlier build wrote, then find the database current
    const converted = await database.migrate([preferenceRebuilt]);
    const again = await database.migrate([preferenceRebuilt]);
    expect({ converted: review(converted), again: review(again) }).toEqual({
        converted: [
            `fallible convert table/${table(preferenceRebuilt)}: convert rows to ${preferenceOne[TABLE].package.version}`,
        ],
        again: [],
    });
    expect(await database.select().from(preferenceRebuilt).orderBy(preferenceRebuilt.id)).toEqual([
        { id: "a", mode: "standard" },
        { id: "b", mode: "vim" },
    ]);
});

/** Preferences in the next release, narrowing the mode and requiring a theme without a default. */
const preferenceStrict = defineTable(
    "preference",
    {
        id: text("id").primaryKey(),
        mode: json("mode", schema.enum(["standard", "vim"])),
        theme: text("theme").notNull(),
    },
    {},
    NEXT_RELEASE,
);

test("narrow values and require a new column without a default only over a table without rows", async () => {
    // change an empty table freely
    const empty = await open([preferenceStrict]);
    await empty.migrate([preferenceOne]);
    const changed = await empty.migrate([preferenceStrict]);

    // refuse the same change over a row
    const filled = await open([preferenceStrict]);
    await filled.migrate([preferenceOne]);
    await filled.run(
        sql`INSERT INTO ${sql.identifier(table(preferenceOne))} (id, mode) VALUES ('a', '"emacs"')`,
    );
    const refused = await filled.migrate([preferenceStrict]).then(
        () => [],
        (error: unknown) =>
            ResourceError.is(error, "PLAN_UNREACHABLE") ? error.data.problems : error,
    );
    expect({ changed: review(changed), refused }).toEqual({
        changed: [
            TestDatabase.target() === "sqlite"
                ? `safe replace table/${table(preferenceStrict)}: rebuild table: add theme`
                : `safe create table/${table(preferenceStrict)}/column/theme: add column theme`,
        ],
        refused: [
            {
                target: `table/${table(preferenceStrict)}`,
                detail: "declare a default for the required column theme",
            },
            {
                target: `table/${table(preferenceStrict)}/column/mode`,
                detail: "declare a conversion for 2026.10.0",
            },
        ],
    });
});

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

test("roll back without contracting what a newer release added, and refuse tables the older release cannot read", async () => {
    // apply the next release adding a theme column and a theme table, then roll back to the first
    const database = await open([preferenceOne]);
    await database.migrate([preferenceThemed, themeTable]);
    const rolledBack = await database.migrate([preferenceOne]);

    // refuse rolling back across a renamed table, which would create it again empty
    const moved = await open([folderOne]);
    await moved.migrate([directoryTable]);
    const renamed = await moved.migrate([folderOne]).then(
        () => [],
        (error: unknown) =>
            ResourceError.is(error, "PLAN_UNREACHABLE") ? error.data.problems : error,
    );

    // refuse rolling back across widened mode values the first release cannot read
    const widened = await open([preferenceOne]);
    await widened.migrate([preferenceWide]);
    const refused = await widened.migrate([preferenceOne]).then(
        () => [],
        (error: unknown) =>
            ResourceError.is(error, "PLAN_UNREACHABLE") ? error.data.problems : error,
    );

    // retain the theme column and table, since dropping them would lose them on the next upgrade
    expect({ rolledBack: review(rolledBack), refused, renamed }).toEqual({
        rolledBack: [],
        refused: [
            {
                target: `table/${table(preferenceOne)}`,
                detail: `rollback to 2026.9.0 cannot read the table as ${NEXT_RELEASE.package.version} applied it`,
            },
        ],
        renamed: [
            {
                target: `table/${table(folderOne)}`,
                detail: `rollback to 2026.9.0 cannot read the table ${NEXT_RELEASE.package.version} renamed to ${table(directoryTable)}`,
            },
        ],
    });
});

/** Declare the state of a database of some tables in the project's database. */
function stateOf(tables: readonly Table[]) {
    return defineDatabase({ name: "main", tables }).state().tables[TestDatabase.target()];
}

/** Read a table's SQL name. */
function table(declaration: Table): string {
    return declaration[TABLE].sqlName;
}

/** Read a declared index's SQL name. */
function indexName(declaration: Table, name: string): string {
    return qualify(declaration[TABLE].package, name);
}

test("rebuild a table other tables reference, preserving their rows", async () => {
    const database = await open([folderOne, document]);
    await database.migrate([folderOne, document]);
    await database.run(
        sql`INSERT INTO ${sql.identifier(table(folderOne))} (id, extra) VALUES ('f', 'x')`,
    );
    await database.run(
        sql`INSERT INTO ${sql.identifier(table(document))} (id, folder_id) VALUES ('d', 'f')`,
    );

    // drop the referenced column and leave the reference valid
    await database.migrate([folderTwo, document]);
    expect(
        await database.all(sql`SELECT id, folder_id FROM ${sql.identifier(table(document))}`),
    ).toEqual([{ id: "d", folder_id: "f" }]);
    expect(await database.all(sql`SELECT id FROM ${sql.identifier(table(folderTwo))}`)).toEqual([
        { id: "f" },
    ]);
});

test("report a failing migration statement as a failed migration", async () => {
    const database = await open([labelPlain]);
    await database.migrate([labelPlain]);
    const name = sql.identifier(table(labelPlain));
    await database.run(sql`INSERT INTO ${name} (id, code) VALUES ('a', 'x'), ('b', 'x')`);

    // fail to make duplicate codes unique and leave the state unchanged
    expect(await refusal(database.migrate([labelUnique]))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "MIGRATION_FAILED",
        "migration failed: a record with the same unique key already exists",
    ]);
    expect(await database.all(sql`SELECT id FROM ${name} ORDER BY id`)).toEqual([
        { id: "a" },
        { id: "b" },
    ]);
});

test("make a column unique and enforce it", async () => {
    const database = await open([labelPlain]);
    await database.migrate([labelPlain]);

    // add the unique constraint
    const plan = await database.plan({
        declared: declareState([labelUnique], TestDatabase.target()),
    });
    expect(review(plan)).toEqual(
        TestDatabase.target() === "sqlite"
            ? [`fallible replace table/${table(labelUnique)}: rebuild table: constraints`]
            : [
                  `fallible create table/${table(labelUnique)}/constraint/${table(labelUnique)}_code_unique: add constraint ${table(labelUnique)}_code_unique`,
              ],
    );
    await applyPlan(database, plan);

    // plan nothing more
    expect(
        review(
            await database.plan({ declared: declareState([labelUnique], TestDatabase.target()) }),
        ),
    ).toEqual([]);
    const name = sql.identifier(table(labelUnique));
    await database.run(
        sql`INSERT INTO ${name} (id, code) VALUES ('a', 'x'), ('b', 'x') ON CONFLICT DO NOTHING`,
    );
    expect(await database.all(sql`SELECT id, code FROM ${name}`)).toEqual([{ id: "a", code: "x" }]);
});

test("plan the union of declarations sharing a database", async () => {
    const database = await open([taskOne]);

    // create a shared table once
    const plan = await database.plan(mergeStates([stateOf([taskOne]), stateOf([taskOne])]));
    expect(review(plan)).toEqual([`safe create table/${table(taskOne)}: create table`]);
    await applyPlan(database, plan);

    // refuse conflicting column types
    expect(
        await refusal(database.plan(mergeStates([stateOf([taskOne]), stateOf([taskText])]))),
    ).toEqual([
        "CONFLICT",
        "PLAN_UNREACHABLE",
        `table/${table(taskOne)}: releases disagree on column urgent`,
    ]);
});

test("expand for two running releases, bridge a renamed column, then contract", async () => {
    const database = await open([taskOne]);
    await database.migrate([taskOne]);
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`INSERT INTO ${sql.identifier(table(taskOne))} (id, scope, name, urgent, note) VALUES ('a', 'inbox', 'Plan', 1, 'old')`,
            ),
    );

    // expand the table for both releases and bridge the renamed column
    const expand = await database.plan(mergeStates([stateOf([taskOne]), stateOf([taskTwo])]));
    expect(review(expand)).toEqual(
        TestDatabase.target() === "sqlite"
            ? [
                  `safe replace table/${table(taskTwo)}: rebuild table: add title, add due_at, change name`,
                  `fallible convert table/${table(taskTwo)}/column/title: copy name into title`,
              ]
            : [
                  `safe create table/${table(taskTwo)}/column/title: add column title`,
                  `safe create table/${table(taskTwo)}/column/due_at: add column due_at`,
                  `safe update table/${table(taskTwo)}/column/name: change column name`,
                  `safe create table/${table(taskTwo)}/index/${indexName(taskTwo, "task_due")}: create index ${indexName(taskTwo, "task_due")}`,
                  `fallible convert table/${table(taskTwo)}/column/title: copy name into title`,
              ],
    );
    expect(
        review(await database.plan(mergeStates([stateOf([taskTwo]), stateOf([taskOne])]))),
    ).toEqual(review(expand));
    await applyPlan(database, expand);

    // connect both releases
    expect(await release([taskOne]).check(database)).toEqual([]);
    expect(await release([taskTwo]).check(database)).toEqual([]);

    // make both names equal
    const name = sql.identifier(table(taskOne));
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`INSERT INTO ${name} (id, scope, name, urgent) VALUES ('b', 'inbox', 'Old', 0)`,
            ),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`INSERT INTO ${name} (id, scope, title, urgent) VALUES ('c', 'inbox', 'New', 0)`,
            ),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.run(sql`UPDATE ${name} SET title = 'Plan again' WHERE id = 'a'`),
    );
    expect(await database.all(sql`SELECT id, name, title FROM ${name} ORDER BY id`)).toEqual([
        { id: "a", name: "Plan again", title: "Plan again" },
        { id: "b", name: "Old", title: "Old" },
        { id: "c", name: "New", title: "New" },
    ]);

    // drop the old release's columns
    const contract = await database.plan(mergeStates([stateOf([taskTwo])]));
    expect(review(contract)).toEqual(
        TestDatabase.target() === "sqlite"
            ? [
                  `destructive replace table/${table(taskTwo)}: rebuild table: drop name, drop note, change title`,
              ]
            : [
                  `destructive delete table/${table(taskTwo)}/column/name: drop column name`,
                  `destructive delete table/${table(taskTwo)}/column/note: drop column note`,
                  `fallible update table/${table(taskTwo)}/column/title: change column title`,
              ],
    );
    await applyPlan(database, contract);
    expect(await release([taskOne]).check(database)).toEqual([table(taskOne)]);
    expect(await release([taskTwo]).check(database)).toEqual([]);
    expect(await database.all(sql`SELECT id, title FROM ${name} ORDER BY id`)).toEqual([
        { id: "a", title: "Plan again" },
        { id: "b", title: "Old" },
        { id: "c", title: "New" },
    ]);
});

test("name declared tables a database has not applied", async () => {
    const database = await open([taskOne]);
    const declared = defineDatabase({
        name: "main",
        tables: [taskOne],
    });

    // report the missing table until applied
    expect(await declared.check(database)).toEqual([table(taskOne)]);
    await database.migrate([taskOne]);
    expect(await declared.check(database)).toEqual([]);
});

test("add foreign keys to local tables and leave references elsewhere logical", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // describe documents with and without their folders in the same database
    const keys = (tables: readonly Table[]) =>
        declareState(tables, TestDatabase.target())
            .filter((state) => state.table.name === document[TABLE].sqlName)
            .flatMap((state) => state.table.constraints)
            .filter((constraint) => constraint.kind === "foreignKey")
            .map((constraint) => [constraint.kind, constraint.columns, constraint.table]);

    // migrate a database with documents but without their folders
    const storage = await TestDatabase.create(
        defineDatabase({ name: "documents", tables: [document] }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await storage.database.transaction((transaction) =>
        transaction.insert(document).values({ id: "d1", folderId: "elsewhere" }).execute(),
    );

    expect([keys([folderTwo, document]), keys([document])]).toEqual([
        [["foreignKey", ["folder_id"], folderTwo[TABLE].sqlName]],
        [],
    ]);
});

test("let replicas leave out the sensitive columns their log never records", () => {
    // declare a required secret beside a required name
    const credential = defineTable("credential", {
        id: text("id").primaryKey(),
        name: text("name").notNull(),
        secret: text("secret").notNull().sensitive(),
    });
    const nullable = (isReplica: boolean) =>
        declareState([credential], TestDatabase.target(), { isReplica }).flatMap((state) =>
            state.table.columns.map((column) => [column.name, column.nullable]),
        );

    // require the secret where it is stored, and make it optional in a replica
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
});

test("declare a replicated table without its unique constraints or the foreign keys to it", () => {
    // declare a list whose items reference it, both replicated by a reader
    const list = defineTable(
        "list",
        { id: text("id").primaryKey(), name: text("name").notNull() },
        { constraints: (columns) => [uniqueIndex("list_name").on(columns.name)] },
    );
    const item = defineTable("item", {
        id: text("id").primaryKey(),
        listId: text("list_id")
            .notNull()
            .references(() => list.id, { onDelete: "cascade" }),
    });
    const shape = (replicated: readonly string[]) =>
        declareState([list, item], TestDatabase.target(), { replicated: new Set(replicated) }).map(
            (state) => [
                state.table.name,
                state.table.constraints.map((constraint) => constraint.kind),
                state.table.indexes.map((described) => [described.name, described.unique]),
            ],
        );

    // add the declarer's keys and leave a replica's to its package, under SQLite's key check
    const [lists, items] = [list[TABLE].sqlName, item[TABLE].sqlName];
    const checks = TestDatabase.target() === "sqlite" ? ["check"] : [];
    expect([shape([]), shape([lists])]).toEqual([
        [
            [lists, ["primaryKey", ...checks], [[`${lists}_name`, true]]],
            [items, ["primaryKey", "foreignKey", ...checks], []],
        ],
        [
            [lists, ["primaryKey", ...checks], [[`${lists}_name`, false]]],
            [items, ["primaryKey", ...checks], []],
        ],
    ]);
});

test("refuse an applied state in a format this release no longer reads, listing its table and database", async () => {
    // apply the tasks, then store their state with columns no release reads
    const database = await open([taskOne]);
    await database.migrate([taskOne]);
    const [stored] = await database.execute(
        sql`SELECT state FROM ${sql.relation(STATE)}`,
        schema.object({ state: schema.string() }),
    );
    const state = schema
        .looseObject({ table: schema.looseObject({}) })
        .parse(JSON.parse(present(stored, "the stored state").state));
    const old = { ...state, table: { ...state.table, columns: "restrict" } };
    await database.transaction(
        async (transaction) =>
            await transaction.run(
                sql`UPDATE ${sql.relation(STATE)} SET state = ${JSON.stringify(old)}`,
            ),
    );

    // read the server's database and schema the state lives in
    const [server] =
        TestDatabase.target() === "sqlite"
            ? [{ name: "sqlite" }]
            : await database.execute(
                  sql`SELECT current_database() || '.' || current_schema() AS name`,
                  schema.object({ name: schema.string() }),
              );

    // report the table and the database
    expect(await refusal(database.unapplied([taskOne]))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "MIGRATION_INVALID",
        `the applied state of table ${table(taskOne)} in database ${present(server, "the database").name} has an unknown format: ✖ Invalid input: expected array, received string\n  → at table.columns`,
    ]);
});

test("describe one package's tables as the whole database describes them", async () => {
    // describe the attachments placed beside the documents they reference
    const tables = [folderTwo, document, attachment];
    const owner = attachment[TABLE].package.id;
    const whole = declareState(tables, TestDatabase.target());
    expect(declareState(tables, TestDatabase.target(), { package: owner })).toEqual(
        whole.filter((state) => state.package.id === owner),
    );
});
