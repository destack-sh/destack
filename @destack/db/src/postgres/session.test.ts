import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import type { Session, SessionTransaction } from "../database/session.ts";
import { TestDatabase } from "../test/database.ts";

/** A read-write transaction at PostgreSQL's default isolation. */
const WRITE: SessionTransaction = { isolationLevel: "read committed", isReadOnly: false };

/** Open the session of an empty test database, closed after the test. */
async function openSession(): Promise<Session> {
    const storage = await TestDatabase.create([], {
        environment: testSimulation().environment("test"),
    });
    onTestFinished(() => storage.close());

    return storage.database.driver.session;
}

test("refuse a nested transaction beside an open sibling", async () => {
    // open two nested transactions at once
    const session = await openSession();
    const held = Promise.withResolvers<void>();
    const siblings = await refusal(
        session.transaction(
            (parent) =>
                Promise.all([
                    parent.transaction(() => held.promise, WRITE),
                    parent.transaction(async () => undefined, WRITE).finally(() => held.resolve()),
                ]),
            WRITE,
        ),
    );
    expect(siblings).toEqual([
        "BAD_REQUEST",
        "QUERY_INVALID",
        "a nested transaction began while its sibling is open: run them in turn",
    ]);
});

test("refuse a transaction the server rolled back at its commit, after its work caught a failed statement", async () => {
    // fail a statement, catch it, and return as if the transaction succeeded
    const session = await openSession();
    const committed = await refusal(
        session.transaction(async (transaction) => {
            await transaction.run("SELECT 1 / 0", []).catch(() => undefined);

            return "done";
        }, WRITE),
    );
    expect(committed).toEqual([
        "INTERNAL_SERVER_ERROR",
        "QUERY_FAILED",
        "the transaction rolled back at its commit, after a statement in it failed",
    ]);
});
