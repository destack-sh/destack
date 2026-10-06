import postgres from "postgres";
import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { typedChannel } from "../channel/channel.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { applyPlan } from "../migration/apply.ts";
import { declareState } from "../migration/state.ts";
import { asc, sql, type SQL } from "../sql/index.ts";
import { integer, text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { connectPostgres, reportNotice } from "./connection.ts";
import { postgresChannel } from "./database.ts";
import { TEST_DIALECTS } from "../test/database.ts";

/** The test server's address, absent where the test dialects leave PostgreSQL out. */
const ADDRESS = TEST_DIALECTS.includes("postgresql")
    ? process.env["DESTACK_TEST_POSTGRES"]
    : undefined;

/** A message between parties, with a body of some size. */
const Note = schema.object({ from: schema.string(), body: schema.string() });
/** A message between parties, with a body of some size. */
type Note = schema.Infer<typeof Note>;

test.skipIf(ADDRESS === undefined)(
    "carry small and large messages whole through a PostgreSQL channel, splitting the large ones",
    async () => {
        // listen with one client and notify with another
        if (ADDRESS === undefined) {
            throw new Error("the test server has no address");
        }
        const listener = postgres(ADDRESS, { max: 1 });
        const sender = postgres(ADDRESS, { max: 4 });
        const name = `destack_test_${crypto.randomUUID().replaceAll("-", "")}`;
        const received: Note[] = [];
        const joined = Promise.withResolvers<void>();
        const isReceived = Promise.withResolvers<void>();
        const stop = typedChannel(postgresChannel(listener, name), Note).listen(
            (message) => {
                received.push(message);
                if (received.length === 2) {
                    isReceived.resolve();
                }
            },
            () => joined.resolve(),
        );
        onTestFinished(async () => {
            // stop listening before closing both clients
            stop();
            await Promise.all([listener.end(), sender.end()]);
        });
        await joined.promise;

        // send a small message and one of about 60 kB with multi-byte characters across fragments
        const large = "äöü€😀".repeat(5_000);
        const channel = typedChannel(postgresChannel(sender, name), Note);
        channel.notify({ from: "small", body: "hello" });
        channel.notify({ from: "large", body: large });
        await isReceived.promise;

        // receive both exactly, in any order
        expect(received.toSorted((left, right) => left.from.localeCompare(right.from))).toEqual([
            { from: "large", body: large },
            { from: "small", body: "hello" },
        ]);
    },
);

/** The test database whose default collation is ICU's English order, where `_` < `a` < `B`. */
const ICU_DATABASE = "destack_test_icu";

/** The keys of the listed files, in byte order: a capital, the underscore, small letters, an accented letter. */
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

test.skipIf(ADDRESS === undefined)(
    "list text keys in byte order through their index once migrated to byte collation, in a database collating by ICU",
    async () => {
        // create the ICU database once across concurrent runs
        if (ADDRESS === undefined) {
            throw new Error("the test server has no address");
        }
        const server = postgres(ADDRESS, { max: 1, onnotice: reportNotice });
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

        // connect to a schema of its own in the ICU database
        const address = new URL(ADDRESS);
        address.pathname = `/${ICU_DATABASE}`;
        const administration = postgres(address.href, { max: 1, onnotice: reportNotice });
        const name = `test_${crypto.randomUUID().replaceAll("-", "")}`;
        await administration.unsafe(`CREATE SCHEMA "${name}"`);
        const database = await connectPostgres(
            postgres(address.href, {
                max: 2,
                connection: { search_path: name },
                onnotice: reportNotice,
            }),
            [entry],
        );
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
        const listing = sql`SELECT key, size FROM ${sql.identifier(table)} ORDER BY key COLLATE "C" LIMIT 3`;
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
    },
);

/** List the node types of a statement's plan from the root down its first inputs, without sequential scans where an index serves. */
async function explain(database: DatabaseConnection, statement: SQL): Promise<string[]> {
    // explain the statement with sequential scans disabled
    const [explained] = await database.transaction(async (transaction) => {
        await transaction.execute(sql`SET LOCAL enable_seqscan = off`);

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
