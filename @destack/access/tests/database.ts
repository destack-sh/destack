import * as turso from "@destack/db/turso";
import * as postgres from "@destack/db/postgres";
import { sql } from "@destack/db";
import { fixtureSchema, node, cell, entity, mappings, item, rows } from "./fixture.ts";
import { AccessModel, type AccessContext } from "../src/index.ts";
import { AccessQuery, GrantStore } from "../src/database/index.ts";
import { prepare } from "@destack/db/migration";

/** Open an isolated database and remove it after the example completes. */
async function openFixtureDatabase() {
    const address = process.env.DESTACK_TEST_POSTGRES;
    if (!address) {
        const database = await prepare(await turso.connect(":memory:", fixtureSchema), [
            fixtureSchema,
        ]);

        return { database, address: undefined, close: () => database.close() };
    }

    // isolate each run so migrations and destructive cases cannot affect another database
    const administration = await postgres.connect(address);
    const name = `access_${crypto.randomUUID().replaceAll("-", "")}`;
    await administration.execute(sql`CREATE DATABASE ${sql.identifier(name)}`);
    const url = new URL(address);
    url.pathname = `/${name}`;
    try {
        const database = await prepare(await postgres.connect(url.href, fixtureSchema), [
            fixtureSchema,
        ]);

        return {
            database,
            address: url.href,
            async close() {
                await database.close();
                await administration.execute(sql`DROP DATABASE ${sql.identifier(name)}`);
                await administration.close();
            },
        };
    } catch (error) {
        await administration.execute(sql`DROP DATABASE ${sql.identifier(name)}`);
        await administration.close();
        throw error;
    }
}

/** Open a migrated application database with independent users and explicit sharing. */
export async function openFixture() {
    // open the application database and prepare equivalent SQL and memory declarations
    const opened = await openFixtureDatabase();
    const { database } = opened;
    const model = new AccessModel([node, cell, entity]);
    const query = new AccessQuery(model, mappings);

    // authenticate two independent users and record committed sharing changes
    const alice: AccessContext = {
        subjects: [{ kind: "user", authority: "global", id: "alice" }],
        now: 1000,
        attributes: {},
    };
    const bob: AccessContext = {
        ...alice,
        subjects: [{ kind: "user", authority: "global", id: "bob" }],
    };
    const changes: string[] = [];
    const store = new GrantStore(database, query, async (change) => {
        changes.push(change.operation);
    });

    // populate the migrated database and grant access to one note
    try {
        await database.insert(item).values(rows);
        await store.grant(
            {
                object: node.reference("personal", "b"),
                relation: "editor",
                subject: bob.subjects[0],
            },
            alice,
        );

        return { ...opened, model, query, alice, bob, changes, store };
    } catch (error) {
        await opened.close();

        throw error;
    }
}
