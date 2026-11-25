import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { asc, defineDatabase, defineTable, eq, sql, TABLE, text, unique } from "../index.ts";
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

/** Packages, which only the home stores. */
const parcel = defineTable("replicated_package", {
    /** The package's identifier. */
    id: text("id").primaryKey(),
});

/** Releases of packages, the home's beside replicas of other homes'. */
const release = defineTable(
    "replicated_release",
    {
        /** The release's identifier. */
        id: text("id").primaryKey(),
        /** The space storing the release. */
        scope: text("scope").notNull(),
        /** The released package, whose deletion deletes the home's releases. */
        parentId: text("parent_id")
            .notNull()
            .references(() => parcel.id, { onDelete: "cascade" }),
        /** The released version. */
        version: text("version").notNull(),
    },
    { log: {} },
);

/** Pins of releases' packages, the home's beside replicas of other homes'. */
const pin = defineTable(
    "replicated_pin",
    {
        /** The pin's identifier. */
        id: text("id").primaryKey(),
        /** The space storing the pin. */
        scope: text("scope").notNull(),
        /** The pinned package, which the home's pins keep from deletion. */
        packageId: text("package_id")
            .notNull()
            .references(() => parcel.id),
    },
    { log: {} },
);

test("require, cascade and refuse over a replicated table's home rows alone", async () => {
    // store packages here, and replicate releases and pins that reference them
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        defineDatabase({ name: "main", tables: [parcel], replicated: [release, pin] }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const write = (run: (transaction: DatabaseConnection) => PromiseLike<unknown>) =>
        database.transaction(async (transaction) => {
            await run(transaction);
        });

    // release two packages here, pin one, and copy another home's release and pin of both
    await write((transaction) =>
        transaction.insert(parcel).values([{ id: "notes" }, { id: "tasks" }]),
    );
    await write((transaction) =>
        transaction
            .insert(release)
            .values([{ id: "r1", scope: "home", parentId: "notes", version: "1.0.0" }]),
    );
    await write((transaction) =>
        transaction.insert(pin).values([{ id: "p1", scope: "home", packageId: "tasks" }]),
    );
    await write((transaction) =>
        transaction.log.asReplica(async () => {
            await transaction.upsert(release, [
                { id: "u1", scope: "other", parentId: "notes", version: "9.0.0" },
            ]);
            await transaction.upsert(pin, [{ id: "u2", scope: "other", packageId: "notes" }]);
        }),
    );

    // refuse a home release without a version, and copy a replica's without one
    const releases = sql.relation(release[TABLE].sqlName);
    const missing = await refusal(() =>
        write((transaction) =>
            transaction.run(
                sql`INSERT INTO ${releases} (id, scope, parent_id) VALUES ('r2', 'home', 'notes')`,
            ),
        ),
    );
    await write((transaction) =>
        transaction.log.asReplica(() =>
            transaction.run(
                sql`INSERT INTO ${releases} (id, scope, parent_id, is_replica) VALUES ('u3', 'other', 'tasks', 1)`,
            ),
        ),
    );

    // delete the released package, and refuse deleting the pinned one
    await write((transaction) => transaction.delete(parcel).where(eq(parcel.id, "notes")));
    const pinned = await refusal(() =>
        write((transaction) => transaction.delete(parcel).where(eq(parcel.id, "tasks"))),
    );

    // expect the home's release of the deleted package gone, and every replica kept
    const stored = await database.select({ id: release.id }).from(release).orderBy(asc(release.id));
    const pins = await database.select({ id: pin.id }).from(pin).orderBy(asc(pin.id));
    expect({ missing, pinned, stored, pins }).toEqual({
        missing: ["BAD_REQUEST", "RECORD_INVALID", "record is invalid: it fails a declared check"],
        pinned: [
            "PRECONDITION_FAILED",
            "REFERENCE_BLOCKED",
            "cannot change the record: the change would leave a reference to a missing record",
        ],
        stored: [{ id: "u1" }, { id: "u3" }],
        pins: [{ id: "p1" }, { id: "u2" }],
    });
});

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
