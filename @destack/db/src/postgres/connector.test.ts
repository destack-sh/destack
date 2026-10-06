import postgres from "postgres";
import { ResourceId } from "@destack/resource";
import { expect, onTestFinished, test } from "@destack/test";
import { defineDatabase } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import { text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { connectPostgres } from "./connection.ts";
import { postgresConnector } from "./connector.ts";
import { TEST_DIALECTS } from "../test/database.ts";

/** The test server's address, absent where the test dialects leave PostgreSQL out. */
const ADDRESS = TEST_DIALECTS.includes("postgresql")
    ? process.env["DESTACK_TEST_POSTGRES"]
    : undefined;

/** Notes with a title. */
const note = defineTable("note", { id: text("id").primaryKey(), title: text("title") });

/** The database with the notes. */
const notes = defineDatabase({ name: "notes", tables: [note] });

test.skipIf(ADDRESS === undefined)(
    "refuse an unmigrated PostgreSQL database, then open the migrated one as its sole writer",
    async () => {
        // keep the notes in a schema dropped after the test
        if (ADDRESS === undefined) {
            throw new Error("the test server has no address");
        }
        const administration = postgres(ADDRESS, { max: 1 });
        const name = `test_${crypto.randomUUID().replaceAll("-", "")}`;
        await administration.unsafe(`CREATE SCHEMA "${name}"`);
        onTestFinished(async () => {
            await administration.unsafe(`DROP SCHEMA "${name}" CASCADE`);
            await administration.end();
        });
        const url = new URL(ADDRESS);
        url.searchParams.set("search_path", name);
        const binding = {
            resource: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000001"),
            kind: "database",
            provider: "postgresql",
            reference: url.href,
        };

        // refuse the schema before its tables exist
        await expect(postgresConnector.connect(binding, notes)).rejects.toThrow(
            new DatabaseError("NOT_APPLIED", "database notes has not applied destack__db__note"),
        );

        // migrate the schema and write a note through a sole writer
        const migration = await connectPostgres(url.href, notes);
        await migration.migrate(notes.tables);
        await migration.close();
        await using connection = await postgresConnector.connect(binding, notes);
        await connection.insert(note).values({ id: "first", title: "Hello" });
        expect({
            openChannel: connection.state.openChannel,
            notes: await connection.select().from(note),
        }).toEqual({ openChannel: undefined, notes: [{ id: "first", title: "Hello" }] });
    },
);
