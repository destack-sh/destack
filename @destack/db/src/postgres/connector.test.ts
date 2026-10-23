import { TestDatabase } from "../test/database.ts";
import postgres from "postgres";
import { ResourceId } from "@destack/resource";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { defineDatabase } from "../declare/database.ts";
import { text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { connectPostgres } from "./connection.ts";
import { PostgresConnector } from "./connector.ts";

/** Whether the test's project runs against PostgreSQL, which these tests need. */
const isPostgresql = TestDatabase.target() === "postgresql";

/** Notes with a title. */
const note = defineTable("note", { id: text("id").primaryKey(), title: text("title") });

/** The database with the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

test.skipIf(!isPostgresql)(
    "refuse an unmigrated PostgreSQL database, then open the migrated one as its sole writer",
    async () => {
        // store the notes in a schema dropped after the test
        const address = TestDatabase.address();
        const administration = postgres(address, { max: 1 });
        const name = `test_${crypto.randomUUID().replaceAll("-", "")}`;
        await administration.unsafe(`CREATE SCHEMA "${name}"`);
        onTestFinished(async () => {
            await administration.unsafe(`DROP SCHEMA "${name}" CASCADE`);
            await administration.end();
        });
        const connector = new PostgresConnector(new URL(address));
        const binding = {
            resource: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000001"),
            kind: "database",
            provider: "postgresql",
            reference: name,
        };

        // refuse the schema before its tables exist
        expect(await refusal(connector.connect(binding, notes))).toEqual([
            "INTERNAL_SERVER_ERROR",
            "NOT_APPLIED",
            "database notes has not applied destack__db__note",
        ]);

        // migrate the schema and write a note through a sole writer
        const migration = await connectPostgres(connector.url(name), notes);
        await migration.migrate(notes.tables);
        await migration.close();
        await using connection = await connector.connect(binding, notes);
        await connection.insert(note).values({ id: "first", title: "Hello" });
        expect({
            openChannel: connection.state.openChannel,
            notes: await connection.select().from(note),
        }).toEqual({ openChannel: undefined, notes: [{ id: "first", title: "Hello" }] });
    },
);
