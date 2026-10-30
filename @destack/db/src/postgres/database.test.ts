import postgres from "postgres";
import { expect, onTestFinished, test } from "@destack/test";
import { present, schema, Uuid } from "@destack/schema";
import { Commit, typedChannel } from "../channel/channel.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { applyPlan } from "../migration/apply.ts";
import { declareState } from "../migration/state.ts";
import { asc, eq, sql, type SQL } from "../sql/index.ts";
import { integer, text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { connectPostgres } from "./connection.ts";
import { PostgresPool, reportNotices } from "./pool.ts";
import { postgresChannel } from "./database.ts";
import { TestDatabase } from "../test/database.ts";
import { first, note, revision } from "../log/test/fixture.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import { testSimulation } from "@destack/test/world";

/** A random UUID, as a transaction's token is. */
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

/** A message between parties, with a body of some size. */
const Note = schema.object({ from: schema.string(), body: schema.string() });
/** A message between parties, with a body of some size. */
type Note = schema.Infer<typeof Note>;

test("send small and large messages whole through a PostgreSQL channel, splitting the large ones", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // listen with one client and notify with another
    const address = TestDatabase.address();
    const listener = new PostgresPool(address, world, { connections: 1 });
    const sender = new PostgresPool(address, world, { connections: 4 });
    const name = `destack_test_${Uuid.v4(world.random).replaceAll("-", "")}`;
    const received: Note[] = [];
    const joined = Promise.withResolvers<void>();
    const isReceived = Promise.withResolvers<void>();
    const listening = typedChannel(postgresChannel(listener, Promise.resolve(name), world), Note);
    const sending = typedChannel(postgresChannel(sender, Promise.resolve(name), world), Note);
    const stop = listening.listen({
        receive: (message) => {
            received.push(message);
            if (received.length === 2) {
                isReceived.resolve();
            }
        },
        resume: () => joined.resolve(),
    });
    onTestFinished(async () => {
        // stop listening and close both channels before ending both pools
        stop();
        await Promise.all([listening.close(), sending.close()]);
        await Promise.all([listener.end(), sender.end()]);
    });
    await joined.promise;

    // send a small message and one of about 60 kB with multi-byte characters across fragments
    const large = "äöü€😀".repeat(5_000);
    sending.notify({ from: "small", body: "hello" });
    sending.notify({ from: "large", body: large });
    await isReceived.promise;

    // receive both exactly, in any order
    expect(received.toSorted((left, right) => left.from.localeCompare(right.from))).toEqual([
        { from: "large", body: large },
        { from: "small", body: "hello" },
    ]);
});

/** The test database whose default collation is ICU's English order, where `_` < `a` < `B`. */
const ICU_DATABASE = "destack_test_icu";

/** The keys of the listed files in byte order: capital, underscore, small, accented letter. */
const KEYS = ["B", "_", "a", "b", "é"];

/** Files listed by key, as a bucket's catalogue lists them. */
const entry = defineTable("entry", {
    key: text("key").primaryKey(),
    size: integer("size").notNull(),
});

/** A plan node as `EXPLAIN (FORMAT JSON)` writes it: its operation and the nodes it reads from. */
const PlanNode = schema.looseObject({
    "Node Type": schema.string(),
    Plans: schema.array(schema.unknown()).exactOptional(),
});

/** The plan of a statement as `EXPLAIN (FORMAT JSON)` returns it. */
const QueryPlan = schema.object({
    "QUERY PLAN": schema.array(schema.object({ Plan: schema.json() })),
});

test("list text keys in byte order through their index once migrated to byte collation, in a database collating by ICU", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // create the ICU database once across concurrent runs
    const address = TestDatabase.address();
    const server = postgres(address, {
        max: 1,
        onnotice: reportNotices(world.telemetry),
    });
    onTestFinished(() => server.end());
    const [existing] = await server`SELECT 1 FROM pg_database WHERE datname = ${ICU_DATABASE}`;
    if (existing === undefined) {
        await server
            .unsafe(
                `CREATE DATABASE ${ICU_DATABASE} TEMPLATE template0 LOCALE_PROVIDER icu ICU_LOCALE 'en-US' LOCALE 'C'`,
            )
            .catch((error: unknown) => {
                if (!(error instanceof postgres.PostgresError && error.code === "42P04")) {
                    throw error;
                }
            });
    }

    // connect to a separate schema in the ICU database
    const icu = new URL(address);
    icu.pathname = `/${ICU_DATABASE}`;
    const administration = postgres(icu.href, {
        max: 1,
        onnotice: reportNotices(world.telemetry),
    });
    const name = `test_${Uuid.v4(world.random).replaceAll("-", "")}`;
    await administration.unsafe(`CREATE SCHEMA "${name}"`);
    const database = await connectPostgres(icu.href, [entry], { environment: world, schema: name });
    onTestFinished(async () => {
        await database.close();
        await administration.unsafe(`DROP SCHEMA "${name}" CASCADE`);
        await administration.end();
    });

    // create the table under the database's collation as declared before byte collation
    const declared = declareState([entry], "postgresql");
    const earlier = declared.map((state) => ({
        ...state,
        table: {
            ...state.table,
            columns: state.table.columns.map(({ collation: _collation, ...column }) => column),
        },
    }));
    await applyPlan(database, await database.plan({ declared: earlier }));
    await database.insert(entry).values(KEYS.toReversed().map((key) => ({ key, size: 1 })));
    const table = entry[TABLE].sqlName;
    const listing = sql`SELECT key, size FROM ${sql.relation(table)} ORDER BY key COLLATE "C" LIMIT 3`;
    const list = async () =>
        (await database.select().from(entry).orderBy(asc(entry.key)).limit(3)).map(
            (row) => row.key,
        );
    const before = { keys: await list(), plan: await explain(database, listing) };

    // collate the key by byte in one safe step
    const plan = await database.plan({ declared });
    expect(
        plan.steps.map((step) => [step.risk, step.action, step.target, step.statements]),
    ).toEqual([
        [
            "safe",
            "update",
            `table/${table}/column/key`,
            [`ALTER TABLE "${table}" ALTER COLUMN "key" TYPE text COLLATE "C"`],
        ],
    ]);
    await applyPlan(database, plan);

    // sort beside the index before the migration and read the index in byte order after it
    expect({
        before,
        after: { keys: await list(), plan: await explain(database, listing) },
    }).toEqual({
        before: { keys: ["_", "a", "b"], plan: ["Limit", "Sort", "Seq Scan"] },
        after: { keys: ["B", "_", "a"], plan: ["Limit", "Index Scan"] },
    });
});

test("announce a schema's commits on its channel alone, never to a database of another schema", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // open databases of other tables in two schemas of one server, listening to the second
    const here = await TestDatabase.create([revision], {
        environment: world,
        isMigrated: true,
    });
    const there = await TestDatabase.create([note], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([here.close(), there.close()]);
    });
    const row = {
        id: "a",
        title: "First",
        scope: "inbox",
        subtitle: null,
        views: 1n,
        labels: [],
        editedAt: 1,
        attachment: new Uint8Array(),
    };
    const received: unknown[] = [];
    const isReceived = Promise.withResolvers<void>();
    const joined = Promise.withResolvers<void>();
    const channel = there.database.state.openChannel?.(LOG_TOPIC);
    if (channel === undefined) {
        throw new Error("the database announces no commits");
    }
    const stop = channel.listen({
        receive: (message) => {
            received.push(message);
            isReceived.resolve();
        },
        resume: () => joined.resolve(),
    });
    onTestFinished(stop);
    await joined.promise;

    // commit in both schemas, receiving only the second's commit at its sequence with its token
    await here.database.transaction((transaction) =>
        transaction
            .insert(revision)
            .values({ scope: "inbox", noteId: "a", number: 1, title: "First" })
            .execute(),
    );
    await there.database.transaction((transaction) =>
        transaction.insert(note).values(row).execute(),
    );
    await isReceived.promise;
    const commits = received.map((message) => {
        const { token, ...commit } = Commit.parse(message);

        return { ...commit, isToken: token !== undefined && UUID.test(token) };
    });
    expect(commits).toEqual([
        { kind: "commit", tables: [note[TABLE].sqlName], sequence: 1, isToken: true },
    ]);
});

test("count a commit in flight until a watching connection receives it, though its writer never listens", async () => {
    // write on one connection of a simulated world, and watch the log on another
    const simulation = testSimulation();
    const storage = await TestDatabase.create([note], {
        environment: simulation.environment("store"),
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const watching = await storage.connect([note]);
    await watching.state.commits.listen();
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    let isSeen = false;
    void watching.log
        .until(async () => (await watching.select().from(note)).length > 0, stopping.signal)
        .then(() => {
            isSeen = true;
        });

    // commit a note, and run the world until the watcher saw it
    void storage.database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({
                id: "a",
                title: "First",
                scope: "inbox",
                subtitle: null,
                views: 1n,
                labels: [],
                editedAt: 1,
                attachment: new Uint8Array(),
            })
            .execute(),
    );
    await simulation.until(() => isSeen);

    // leave the clock where it started, as the commit stays in flight until delivered
    expect(simulation.clock.now()).toBe(Date.UTC(2026, 9, 1));
});

test("send a listening writer's commit token in its commit notification, counted in flight until delivered, and none for a rolled back or unlogged transaction", async () => {
    // open a database on a simulated clock whose commit watch listens, and a second listener
    const simulation = testSimulation();
    const { clock } = simulation;
    const environment = simulation.environment("store");
    const storage = await TestDatabase.create([note], { environment, isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.state.commits.listen();
    const received: unknown[] = [];
    const isReceived = Promise.withResolvers<void>();
    const joined = Promise.withResolvers<void>();
    const openChannel = present(database.state.openChannel, "the database's channels");
    const stop = openChannel(LOG_TOPIC).listen({
        receive: (message) => {
            received.push(message);
            isReceived.resolve();
        },
        resume: () => joined.resolve(),
    });
    onTestFinished(stop);
    await joined.promise;

    // roll one transaction back, change nothing in one, and commit a note in a third
    const row = first;
    const refused = await database
        .transaction(async (transaction) => {
            await transaction.insert(note).values(row).execute();
            throw new Error("roll back");
        })
        .catch((error: Error) => error.message);
    await database.transaction((transaction) =>
        transaction.update(note).set({ title: "None" }).where(eq(note.id, "missing")).execute(),
    );
    await database.transaction((transaction) => transaction.insert(note).values(row).execute());

    // read the notification, then the work in flight once the world is idle
    await isReceived.promise;
    await simulation.idle();
    const announced = schema
        .object({
            kind: schema.literal("commit"),
            tables: schema.array(schema.string()),
            sequence: schema.number(),
            token: schema.string(),
        })
        .parse(received[0]);

    // receive one commit carrying its token, and wait for nothing else
    expect({
        refused,
        received: [{ ...announced, token: UUID.test(announced.token) }, ...received.slice(1)],
        inFlight: clock.inFlight,
    }).toEqual({
        refused: "roll back",
        received: [{ kind: "commit", tables: [note[TABLE].sqlName], sequence: 1, token: true }],
        inFlight: 0,
    });
});

/** List a statement's plan node types from the root down its first inputs. */
async function explain(database: DatabaseConnection, statement: SQL): Promise<string[]> {
    // explain the statement with sequential scans disabled
    const [explained] = await database.transaction(async (transaction) => {
        await transaction.run(sql`SET LOCAL enable_seqscan = off`);

        return transaction.execute(sql`EXPLAIN (FORMAT JSON) ${statement}`, QueryPlan);
    });

    // follow each node's first input
    const types: string[] = [];
    let node: unknown = explained?.["QUERY PLAN"][0]?.Plan;
    while (node !== undefined) {
        const parsed = PlanNode.parse(node);
        types.push(parsed["Node Type"]);
        node = parsed.Plans?.[0];
    }

    return types;
}
