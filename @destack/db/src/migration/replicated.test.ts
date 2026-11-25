import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { asc, defineDatabase, defineTable, text, unique } from "../index.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { TestDatabase } from "../test/database.ts";

/** Tags pointing at versions, the home's beside replicas of other homes'. */
const tag = defineTable(
    "replicated_tag",
    {
        /** The tag's identifier. */
        id: text("id").primaryKey(),
        /** The space storing the tag. */
        scope: text("scope").notNull(),
        /** The package with the tag. */
        parentId: text("parent_id").notNull(),
        /** The tag's name. */
        name: text("name").notNull(),
        /** The version the tag points at. */
        version: text("version").notNull(),
    },
    { log: {}, constraints: (row) => [unique("replicated_tag_name").on(row.parentId, row.name)] },
);

test("keep a replicated table's keys unique among the home's rows and trust a replica's", async () => {
    // replicate tags into a database that writes its own
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        defineDatabase({ name: "main", tables: [], replicated: [tag] }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const write = (run: (transaction: DatabaseConnection) => PromiseLike<unknown>) =>
        database.transaction(async (transaction) => {
            await run(transaction);
        });

    // tag a version, then move the tag by its unique key
    await write((transaction) =>
        transaction
            .insert(tag)
            .values([
                { id: "t1", scope: "home", parentId: "notes", name: "latest", version: "1.0.0" },
            ]),
    );
    await write((transaction) =>
        transaction
            .insert(tag)
            .values([
                { id: "t2", scope: "home", parentId: "notes", name: "latest", version: "1.1.0" },
            ])
            .onConflictDoUpdate({ target: [tag.parentId, tag.name], set: { version: "1.1.0" } }),
    );

    // refuse a second home tag of one name
    const duplicate = await refusal(() =>
        write((transaction) =>
            transaction.insert(tag).values([
                {
                    id: "t3",
                    scope: "home",
                    parentId: "notes",
                    name: "latest",
                    version: "1.0.0",
                },
            ]),
        ),
    );

    // copy another home's tag of the same name
    await write((transaction) =>
        transaction.log.asReplica(() =>
            transaction.upsert(tag, [
                { id: "u1", scope: "other", parentId: "notes", name: "latest", version: "9.0.0" },
            ]),
        ),
    );

    // expect the home's duplicate refused and the replica kept beside the home's moved tag
    const tags = await database
        .select({ id: tag.id, version: tag.version })
        .from(tag)
        .orderBy(asc(tag.id));
    expect({ duplicate, tags }).toEqual({
        duplicate: ["CONFLICT", "ROW_EXISTS", "a record with the same unique key already exists"],
        tags: [
            { id: "t1", version: "1.1.0" },
            { id: "u1", version: "9.0.0" },
        ],
    });
});
