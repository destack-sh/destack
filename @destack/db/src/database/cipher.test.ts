import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { connectBunSqlite } from "../bun/connection.ts";
import { Transaction } from "../log/commit.ts";
import { asc, eq, sql } from "../sql/index.ts";
import { boolean, json, text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { TEST_CIPHER, TEST_DIALECTS, TestDatabase } from "../test/database.ts";

/** Credentials with a secret sealed randomly and a token sealed alike for lookups. */
const credential = defineTable(
    "sealed_credential",
    {
        /** The credential's identifier. */
        id: text("id").primaryKey(),
        /** The space it lives in. */
        scope: text("scope").notNull(),
        /** The secret, sealed under a fresh nonce. */
        secret: text("secret").notNull().sensitive(),
        /** The bearer token, sealed alike so a lookup finds it by value. */
        token: text("token").notNull().sensitive("deterministic"),
    },
    { log: {} },
);

/** Push subscriptions with a sealed JSON endpoint and a sealed flag. */
const subscription = defineTable("sealed_subscription", {
    /** The subscription's identifier. */
    id: text("id").primaryKey(),
    /** The push endpoint and its keys, sealed. */
    endpoint: json(
        "endpoint",
        schema.object({
            url: schema.string(),
            keys: schema.record(schema.string(), schema.string()),
        }),
    ).sensitive(),
    /** Whether the subscription is muted, sealed. */
    isMuted: boolean("is_muted").notNull().sensitive(),
});

/** The stored text of each credential's sealed columns. */
const Stored = schema.array(schema.object({ secret: schema.string(), token: schema.string() }));

/** Open a migrated database with two credentials. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, [credential], { isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.insert(credential).values([
        { id: "a", scope: "s", secret: "first secret", token: "first token" },
        { id: "b", scope: "s", secret: "second secret", token: "second token" },
    ]);

    return database;
}

/** Read the stored text of the sealed columns in key order. */
async function stored(database: Awaited<ReturnType<typeof open>>) {
    const name = sql.identifier(credential[TABLE].sqlName);

    return Stored.parse(await database.execute(sql`SELECT secret, token FROM ${name} ORDER BY id`));
}

test.for(TEST_DIALECTS)(
    "seal sensitive columns at rest, open them on read, and find a deterministic one by value on %s",
    async (dialect) => {
        const database = await open(dialect);
        const [first, second] = await stored(database);

        // open the stored text with the database's key, which never holds the plaintext
        expect([
            TEST_CIPHER.open(first?.secret, "secret"),
            TEST_CIPHER.open(second?.token, "token"),
            first?.secret === "first secret" || first?.token === "first token",
        ]).toEqual(["first secret", "second token", false]);

        // read the plaintext back, and find a credential by its token
        expect([
            await database.select().from(credential).orderBy(asc(credential.id)),
            await database
                .select({ id: credential.id })
                .from(credential)
                .where(eq(credential.token, "second token")),
        ]).toEqual([
            [
                { id: "a", scope: "s", secret: "first secret", token: "first token" },
                { id: "b", scope: "s", secret: "second secret", token: "second token" },
            ],
            [{ id: "b" }],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "seal equal secrets apart and equal tokens alike, each bound to its column, on %s",
    async (dialect) => {
        const database = await open(dialect);
        await database.insert(credential).values([
            { id: "c", scope: "s", secret: "same", token: "same" },
            { id: "d", scope: "s", secret: "same", token: "same" },
        ]);
        const [, , third, fourth] = await stored(database);

        // differ for the secret and match for the token
        expect([third?.secret === fourth?.secret, third?.token === fourth?.token]).toEqual([
            false,
            true,
        ]);

        // refuse opening the token's text as the secret's column
        expect(() => TEST_CIPHER.open(third?.token, "secret")).toThrow(
            new TypeError("secret holds a value the database key cannot open"),
        );
    },
);

test.for(TEST_DIALECTS)(
    "seal the values one transaction updates together, as one statement writes them, on %s",
    async (dialect) => {
        const database = await open(dialect);

        // change both secrets in one transaction
        const transaction = await Transaction.open(database);
        for (const id of ["a", "b"]) {
            const before = await transaction.row(credential, { id });
            await transaction.write({
                table: credential,
                before,
                after: { id, scope: "s", secret: `${id} rotated`, token: `${id} token` },
            });
        }
        await transaction.commit();

        // keep both sealed, and open them as written
        const [first, second] = await stored(database);
        expect([
            TEST_CIPHER.open(first?.secret, "secret"),
            TEST_CIPHER.open(second?.secret, "secret"),
            first?.secret === "a rotated",
        ]).toEqual(["a rotated", "b rotated", false]);
    },
);

test.for(TEST_DIALECTS)(
    "return the opened values of the rows an insert and an update write, as a select reads them, on %s",
    async (dialect) => {
        // insert and update credentials, returning the written rows
        const database = await open(dialect);
        const inserted = await database
            .insert(credential)
            .values({ id: "c", scope: "s", secret: "third secret", token: "third token" })
            .returning();
        const updated = await database
            .update(credential)
            .set({ secret: "renewed secret" })
            .where(eq(credential.token, "third token"))
            .returning({ token: credential.token, secret: credential.secret });

        // return the plaintext the select reads too
        expect([inserted, updated]).toEqual([
            [{ id: "c", scope: "s", secret: "third secret", token: "third token" }],
            [{ token: "third token", secret: "renewed secret" }],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "open sealed JSON and boolean values as the values written, in every dialect, on %s",
    async (dialect) => {
        // write a sealed JSON value and a sealed flag
        const storage = await TestDatabase.create(dialect, [subscription], { isMigrated: true });
        onTestFinished(() => storage.close());
        const endpoint = { url: "https://push.example.com/1", keys: { auth: "a", p256dh: "b" } };
        await storage.database.insert(subscription).values({ id: "s", endpoint, isMuted: true });

        // read them back as written
        expect(await storage.database.select().from(subscription)).toEqual([
            { id: "s", endpoint, isMuted: true },
        ]);
    },
);

test("refuse writing a sensitive value into a database opened without its key", async () => {
    const database = await connectBunSqlite(":memory:", [credential]);
    onTestFinished(() => database.close());
    await database.migrate([credential]);

    await expect(
        database.insert(credential).values({ id: "a", scope: "s", secret: "x", token: "y" }),
    ).rejects.toThrow(new TypeError("secret is sensitive, but the database has no key"));
});
